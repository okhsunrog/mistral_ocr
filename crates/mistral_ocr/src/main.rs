use clap::{Parser, ValueEnum};
use mistral_ocr::ImageMode;
use std::path::PathBuf;

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

    /// Where to write the output (.md file, or .zip when --images zip)
    #[arg(long, default_value = "ocr_output.md")]
    output: PathBuf,
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
    let image_mode: ImageMode = cli.images.into();

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .format_timestamp(None)
        .init();

    let api_key = get_api_key();
    if let Err(err) = mistral_ocr::run_ocr(&cli.input, image_mode, &cli.output, &api_key) {
        log::error!("{err:#}");
        std::process::exit(1);
    }
}
