use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;

const API_URL: &str = "https://api.mistral.ai/v1/ocr";

#[derive(Parser)]
#[command(about = "Run Mistral OCR on a PDF")]
struct Cli {
    /// Path to the PDF file to process
    #[arg(long)]
    pdf: PathBuf,

    /// Mistral OCR model name
    #[arg(long, default_value = "mistral-ocr-latest")]
    model: String,

    /// How to handle images: none, separate (save to _images/ dir), inline (embed base64 in markdown), zip (bundle md + images into a .zip)
    #[arg(long, value_enum, default_value_t = ImageMode::None)]
    images: ImageMode,

    /// Where to write the output (.md file, or .zip when --images zip)
    #[arg(long, default_value = "ocr_output.md")]
    output: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ImageMode {
    /// Don't include images
    None,
    /// Save images as separate files in a _images/ directory
    Separate,
    /// Embed images as base64 data URIs inline in the markdown
    Inline,
    /// Bundle markdown + image files into a single .zip archive
    Zip,
}

#[derive(Serialize)]
struct OcrRequest {
    model: String,
    document: Document,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_image_base64: Option<bool>,
}

#[derive(Serialize)]
struct Document {
    r#type: String,
    document_url: String,
    document_name: String,
}

#[derive(Deserialize)]
struct OcrResponse {
    pages: Vec<OcrPage>,
}

#[derive(Deserialize)]
struct OcrPage {
    index: u32,
    markdown: String,
    #[serde(default)]
    images: Vec<OcrImage>,
}

#[derive(Deserialize)]
struct OcrImage {
    id: Option<String>,
    image_base64: Option<String>,
}

fn encode_pdf(pdf_path: &Path) -> Result<String> {
    let data =
        fs::read(pdf_path).with_context(|| format!("PDF not found: {}", pdf_path.display()))?;
    Ok(BASE64.encode(&data))
}

fn run_ocr(pdf_path: &Path, model: &str, image_mode: ImageMode, output_path: &Path) -> Result<()> {
    let api_key =
        std::env::var("MISTRAL_API_KEY").context("MISTRAL_API_KEY environment variable is not set")?;

    let base64_pdf = encode_pdf(pdf_path)?;

    let file_name = pdf_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let include_image_base64 = match image_mode {
        ImageMode::None => None,
        ImageMode::Separate | ImageMode::Inline | ImageMode::Zip => Some(true),
    };

    let request = OcrRequest {
        model: model.to_string(),
        document: Document {
            r#type: "document_url".to_string(),
            document_url: format!("data:application/pdf;base64,{base64_pdf}"),
            document_name: file_name,
        },
        include_image_base64,
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(API_URL)
        .bearer_auth(&api_key)
        .json(&request)
        .send()
        .context("OCR request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("OCR request failed (HTTP {status}): {body}");
    }

    let ocr: OcrResponse = response.json().context("Failed to parse OCR response")?;
    write_markdown(output_path, &ocr, image_mode)?;
    Ok(())
}

fn write_markdown(output_path: &Path, response: &OcrResponse, image_mode: ImageMode) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let stem = output_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());

    let images_dir = if image_mode == ImageMode::Separate {
        Some(
            output_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(format!("{stem}_images")),
        )
    } else {
        None
    };

    // For zip mode: image filename in zip -> decoded bytes
    let mut zip_images: Vec<(String, Vec<u8>)> = Vec::new();
    let images_subdir = "images";

    let mut output = String::new();

    for page in &response.pages {
        let mut md = page.markdown.trim_end().to_string();

        if image_mode != ImageMode::None {
            for img in &page.images {
                let (Some(id), Some(b64_data)) = (&img.id, &img.image_base64) else {
                    continue;
                };
                let old_ref = format!("]({id})");

                match image_mode {
                    ImageMode::Separate => {
                        let dir = images_dir.as_ref().unwrap();
                        let decoded = decode_image_base64(b64_data, id)?;
                        fs::create_dir_all(dir)?;
                        fs::write(dir.join(id), &decoded)
                            .with_context(|| format!("Failed to write image {id}"))?;
                        let dir_name = dir.file_name().unwrap().to_string_lossy();
                        md = md.replace(&old_ref, &format!("]({dir_name}/{id})"));
                    }
                    ImageMode::Inline => {
                        let data_uri = if b64_data.starts_with("data:") {
                            b64_data.clone()
                        } else {
                            format!("data:image/jpeg;base64,{b64_data}")
                        };
                        md = md.replace(&old_ref, &format!("]({data_uri})"));
                    }
                    ImageMode::Zip => {
                        let decoded = decode_image_base64(b64_data, id)?;
                        zip_images.push((id.clone(), decoded));
                        md = md.replace(&old_ref, &format!("]({images_subdir}/{id})"));
                    }
                    ImageMode::None => unreachable!(),
                }
            }
        }

        output.push_str(&format!("# Page {}\n\n", page.index + 1));
        output.push_str(&md);
        output.push_str("\n\n");
    }

    if image_mode == ImageMode::Zip {
        let zip_path = output_path.with_extension("zip");
        let file = fs::File::create(&zip_path).context("Failed to create zip file")?;
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let md_name = format!("{stem}.md");
        zip.start_file(&md_name, options)?;
        zip.write_all(output.as_bytes())?;

        for (name, data) in &zip_images {
            zip.start_file(format!("{images_subdir}/{name}"), options)?;
            zip.write_all(data)?;
        }

        zip.finish()?;
    } else {
        fs::write(output_path, &output).context("Failed to write markdown output")?;
    }

    Ok(())
}

fn decode_image_base64(b64_data: &str, id: &str) -> Result<Vec<u8>> {
    let raw = if let Some((_header, encoded)) = b64_data.split_once(',') {
        encoded
    } else {
        b64_data
    };
    BASE64
        .decode(raw)
        .with_context(|| format!("Failed to decode base64 for image {id}"))
}

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run_ocr(&cli.pdf, &cli.model, cli.images, &cli.output) {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }

    if cli.images == ImageMode::Zip {
        println!("OCR output written to {}", cli.output.with_extension("zip").display());
    } else {
        println!("OCR markdown written to {}", cli.output.display());
    }
}
