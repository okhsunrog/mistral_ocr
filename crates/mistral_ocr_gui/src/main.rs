use eframe::egui;
use mistral_ocr::ImageMode;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tracing::error;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Custom tracing layer that appends messages to a shared string and triggers UI repaint.
struct GuiLayer {
    log: Arc<Mutex<String>>,
    ctx: Arc<Mutex<Option<egui::Context>>>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for GuiLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let message = visitor.0;

        let level = *event.metadata().level();
        let mut buf = self.log.lock().unwrap();
        if !buf.is_empty() {
            buf.push('\n');
        }
        if level == tracing::Level::ERROR {
            buf.push_str(&format!("ERROR: {message}"));
        } else {
            buf.push_str(&message);
        }

        if let Some(ctx) = self.ctx.lock().unwrap().as_ref() {
            ctx.request_repaint();
        }
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}

struct OcrApp {
    input_path: String,
    image_mode: ImageMode,
    output_path: String,
    api_key: String,
    log: Arc<Mutex<String>>,
    running: Arc<AtomicBool>,
    egui_ctx: Arc<Mutex<Option<egui::Context>>>,
}

impl OcrApp {
    fn new(log: Arc<Mutex<String>>, egui_ctx: Arc<Mutex<Option<egui::Context>>>) -> Self {
        let api_key = std::env::var("MISTRAL_API_KEY").unwrap_or_default();
        Self {
            input_path: String::new(),
            image_mode: ImageMode::None,
            output_path: "ocr_output.md".to_string(),
            api_key,
            log,
            running: Arc::new(AtomicBool::new(false)),
            egui_ctx,
        }
    }
}

const IMAGE_MODE_LABELS: &[(ImageMode, &str)] = &[
    (ImageMode::None, "None"),
    (ImageMode::Separate, "Separate files"),
    (ImageMode::Inline, "Inline (base64)"),
    (ImageMode::Zip, "Zip archive"),
];

impl eframe::App for OcrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Store the egui context so the tracing layer can request repaints
        *self.egui_ctx.lock().unwrap() = Some(ctx.clone());

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Mistral OCR");
            ui.add_space(8.0);

            egui::Grid::new("form")
                .num_columns(3)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    // Input file
                    ui.label("Input file:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.input_path)
                            .desired_width(400.0)
                            .hint_text("Path to PDF, image, or document..."),
                    );
                    if ui.button("Browse...").clicked() {
                        let mut dialog = rfd::FileDialog::new().set_title("Select input file");
                        dialog = dialog.add_filter(
                            "All supported",
                            &[
                                "pdf", "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp",
                                "doc", "docx", "odt", "rtf", "pptx", "ppt", "odp", "xlsx", "xls",
                                "ods", "csv", "epub",
                            ],
                        );
                        dialog = dialog.add_filter("PDF", &["pdf"]);
                        dialog = dialog.add_filter(
                            "Images",
                            &["jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp"],
                        );
                        dialog = dialog.add_filter(
                            "Documents",
                            &["doc", "docx", "odt", "rtf", "pptx", "ppt", "xlsx", "xls"],
                        );
                        if let Some(path) = dialog.pick_file() {
                            self.input_path = path.display().to_string();
                        }
                    }
                    ui.end_row();

                    // Output file
                    ui.label("Output file:");
                    ui.add(egui::TextEdit::singleline(&mut self.output_path).desired_width(400.0));
                    if ui.button("Browse...").clicked()
                        && let Some(path) = rfd::FileDialog::new()
                            .set_title("Save output as")
                            .set_file_name(&self.output_path)
                            .save_file()
                    {
                        self.output_path = path.display().to_string();
                    }
                    ui.end_row();

                    // API key
                    ui.label("API key:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.api_key)
                            .desired_width(400.0)
                            .password(true)
                            .hint_text("MISTRAL_API_KEY"),
                    );
                    ui.label("");
                    ui.end_row();

                    // Image mode
                    ui.label("Images:");
                    let current_label = IMAGE_MODE_LABELS
                        .iter()
                        .find(|(m, _)| *m == self.image_mode)
                        .map(|(_, l)| *l)
                        .unwrap_or("None");
                    egui::ComboBox::from_id_salt("image_mode")
                        .selected_text(current_label)
                        .width(400.0)
                        .show_ui(ui, |ui| {
                            for (mode, label) in IMAGE_MODE_LABELS {
                                ui.selectable_value(&mut self.image_mode, *mode, *label);
                            }
                        });
                    ui.label("");
                    ui.end_row();
                });

            ui.add_space(12.0);

            let is_running = self.running.load(Ordering::Relaxed);
            ui.horizontal(|ui| {
                let can_run =
                    !is_running && !self.input_path.is_empty() && !self.api_key.is_empty();
                if ui
                    .add_enabled(can_run, egui::Button::new("Run OCR"))
                    .clicked()
                {
                    self.start_ocr();
                }
                if is_running {
                    ui.spinner();
                    ui.label("Processing...");
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.label("Log:");

            let log_text = self.log.lock().unwrap().clone();
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut log_text.as_str())
                            .desired_width(f32::INFINITY)
                            .font(egui::TextStyle::Monospace),
                    );
                });
        });
    }
}

impl OcrApp {
    fn start_ocr(&mut self) {
        self.log.lock().unwrap().clear();
        self.running.store(true, Ordering::Relaxed);

        let input = PathBuf::from(&self.input_path);
        let image_mode = self.image_mode;
        let output = PathBuf::from(&self.output_path);
        let api_key = self.api_key.clone();
        let running = self.running.clone();

        std::thread::spawn(move || {
            if let Err(e) = mistral_ocr::run_ocr(&input, image_mode, &output, &api_key) {
                error!("{e:#}");
            }
            running.store(false, Ordering::Relaxed);
        });
    }
}

fn main() -> eframe::Result {
    let log_buf = Arc::new(Mutex::new(String::new()));
    let egui_ctx: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

    let gui_layer = GuiLayer {
        log: log_buf.clone(),
        ctx: egui_ctx.clone(),
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::INFO)
        .with(gui_layer)
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Mistral OCR",
        options,
        Box::new(move |_cc| Ok(Box::new(OcrApp::new(log_buf, egui_ctx)))),
    )
}
