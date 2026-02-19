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

    /// Mistral OCR model name
    #[arg(long, default_value = mistral_ocr::DEFAULT_MODEL)]
    model: String,

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

    let api_key = get_api_key();
    if let Err(err) =
        mistral_ocr::run_ocr(&cli.input, &cli.model, image_mode, &cli.output, &api_key)
    {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }

    if image_mode == ImageMode::Zip {
        println!(
            "OCR output written to {}",
            cli.output.with_extension("zip").display()
        );
    } else {
        println!("OCR markdown written to {}", cli.output.display());
    }
}
