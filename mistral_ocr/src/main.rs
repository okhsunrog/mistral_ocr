use clap::{Parser, ValueEnum};
use mistral_ocr::{ImageMode, OcrOptions};
use std::path::PathBuf;
use tracing::error;

fn get_api_key() -> String {
    std::env::var("MISTRAL_API_KEY").unwrap_or_else(|_| {
        eprintln!("Error: MISTRAL_API_KEY environment variable is not set");
        std::process::exit(1);
    })
}

#[derive(Parser)]
#[command(about = "Run Mistral OCR on a PDF, image, or document file")]
struct Cli {
    /// Path to the input file (PDF, image, or document: docx, odt, pptx, xlsx, etc.)
    input: PathBuf,

    /// How to handle images: none, separate (save to _images/ dir), inline (embed base64 in markdown), zip (bundle md + images into a .zip)
    #[arg(long, value_enum, default_value_t = CliImageMode::None)]
    images: CliImageMode,

    /// Where to write the output (.md file, or .zip when --images zip).
    /// Defaults to the input file name with an .md extension.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Do not insert `# Page N` headers between pages of multi-page documents
    #[arg(long)]
    no_page_headers: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliImageMode {
    None,
    Separate,
    Inline,
    Zip,
}

impl From<CliImageMode> for ImageMode {
    fn from(m: CliImageMode) -> Self {
        match m {
            CliImageMode::None => ImageMode::None,
            CliImageMode::Separate => ImageMode::Separate,
            CliImageMode::Inline => ImageMode::Inline,
            CliImageMode::Zip => ImageMode::Zip,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let options = OcrOptions {
        image_mode: cli.images.into(),
        page_headers: !cli.no_page_headers,
    };
    let output = cli.output.unwrap_or_else(|| cli.input.with_extension("md"));

    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        // rustls-platform-verifier warns about unreadable files in the system
        // cert store; harmless noise as long as roots load
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,rustls_platform_verifier=error".into()),
        )
        .init();

    let api_key = get_api_key();
    if let Err(err) = mistral_ocr::run_ocr(&cli.input, options, &output, &api_key) {
        error!("{err:#}");
        std::process::exit(1);
    }
}
