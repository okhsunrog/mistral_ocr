use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tracing::info;
use zip::write::SimpleFileOptions;

const API_URL: &str = "https://api.mistral.ai/v1/ocr";

const MODEL: &str = "mistral-ocr-latest";

pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp"];
pub const CONVERTIBLE_EXTENSIONS: &[&str] = &[
    "doc", "docx", "odt", "rtf", "txt", "html", "htm", "pptx", "ppt", "odp", "xlsx", "xls", "ods",
    "csv", "epub",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImageMode {
    None,
    Separate,
    Inline,
    Zip,
}

/// RAII guard that removes a temp file on drop.
struct TempCleanup(PathBuf);
impl Drop for TempCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[derive(Serialize)]
struct OcrRequest {
    model: String,
    document: Document,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_image_base64: Option<bool>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Document {
    #[serde(rename = "document_url")]
    DocumentUrl {
        document_url: String,
        document_name: String,
    },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: String },
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

fn mime_for_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn find_libreoffice() -> Result<PathBuf> {
    for name in &["libreoffice", "soffice"] {
        if let Ok(output) = Command::new("which").arg(name).output()
            && output.status.success()
        {
            return Ok(PathBuf::from(name));
        }
        if let Ok(output) = Command::new("where").arg(name).output()
            && output.status.success()
        {
            return Ok(PathBuf::from(name));
        }
    }

    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            "/opt/homebrew/bin/soffice",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ]
    } else {
        &["/usr/bin/libreoffice", "/usr/bin/soffice"]
    };

    for path in candidates {
        if Path::new(path).exists() {
            return Ok(PathBuf::from(path));
        }
    }

    bail!(
        "LibreOffice not found. Install it from https://www.libreoffice.org/\n\
         LibreOffice is only needed for office document conversion (docx, odt, pptx, etc.).\n\
         PDF and image files work without it."
    )
}

fn convert_to_pdf(input_path: &Path) -> Result<PathBuf> {
    let lo_bin = find_libreoffice()?;
    let temp_dir = std::env::temp_dir().join("mistral_ocr");
    fs::create_dir_all(&temp_dir)?;

    let output = Command::new(&lo_bin)
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(&temp_dir)
        .arg(input_path)
        .output()
        .with_context(|| format!("Failed to run LibreOffice at {}", lo_bin.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("libreoffice conversion failed: {stderr}");
    }

    let stem = input_path.file_stem().context("Input file has no stem")?;
    let pdf_path = temp_dir.join(format!("{}.pdf", stem.to_string_lossy()));

    if !pdf_path.exists() {
        bail!(
            "libreoffice did not produce expected PDF at {}",
            pdf_path.display()
        );
    }

    Ok(pdf_path)
}

fn encode_file(path: &Path) -> Result<String> {
    let data = fs::read(path).with_context(|| format!("File not found: {}", path.display()))?;
    Ok(BASE64.encode(&data))
}

pub fn run_ocr(
    input_path: &Path,
    image_mode: ImageMode,
    output_path: &Path,
    api_key: &str,
) -> Result<()> {
    let ext = input_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let temp_pdf: Option<PathBuf>;
    let effective_path;
    if CONVERTIBLE_EXTENSIONS.contains(&ext.as_str()) {
        info!("Converting .{ext} to PDF via LibreOffice...");
        temp_pdf = Some(convert_to_pdf(input_path)?);
        effective_path = temp_pdf.as_deref().unwrap().to_path_buf();
    } else {
        temp_pdf = None;
        effective_path = input_path.to_path_buf();
    }

    let _cleanup = temp_pdf.as_ref().map(|p| TempCleanup(p.clone()));

    let effective_ext = effective_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    info!("Encoding file...");
    let b64 = encode_file(&effective_path)?;

    let document = if effective_ext == "pdf" {
        let file_name = input_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Document::DocumentUrl {
            document_url: format!("data:application/pdf;base64,{b64}"),
            document_name: file_name,
        }
    } else if IMAGE_EXTENSIONS.contains(&effective_ext.as_str()) {
        let mime = mime_for_ext(&effective_ext);
        Document::ImageUrl {
            image_url: format!("data:{mime};base64,{b64}"),
        }
    } else {
        bail!(
            "Unsupported file type: .{ext} (expected pdf, image, or document: docx, odt, pptx, xlsx, etc.)"
        );
    };

    let include_image_base64 = match image_mode {
        ImageMode::None => None,
        ImageMode::Separate | ImageMode::Inline | ImageMode::Zip => Some(true),
    };

    let request = OcrRequest {
        model: MODEL.to_string(),
        document,
        include_image_base64,
    };

    info!("Sending OCR request to Mistral API...");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .context("Failed to build HTTP client")?;
    let response = client
        .post(API_URL)
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .context("OCR request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("OCR request failed (HTTP {status}): {body}");
    }

    info!("Processing response...");
    let ocr: OcrResponse = response.json().context("Failed to parse OCR response")?;
    write_markdown(output_path, &ocr, image_mode)?;

    if image_mode == ImageMode::Zip {
        info!(
            "Done! Output written to {}",
            output_path.with_extension("zip").display()
        );
    } else {
        info!("Done! Output written to {}", output_path.display());
    }
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

    let mut zip_images: Vec<(String, Vec<u8>)> = Vec::new();
    let images_subdir = "images";

    let mut output = String::new();
    let multi_page = response.pages.len() > 1;

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
                            let img_ext = Path::new(id)
                                .extension()
                                .map(|e| e.to_string_lossy().to_lowercase())
                                .unwrap_or_else(|| "jpeg".to_string());
                            let mime = mime_for_ext(&img_ext);
                            format!("data:{mime};base64,{b64_data}")
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

        if multi_page {
            output.push_str(&format!("# Page {}\n\n", page.index + 1));
        }
        output.push_str(&md);
        output.push_str("\n\n");
    }

    if image_mode == ImageMode::Zip {
        let zip_path = output_path.with_extension("zip");
        let file = fs::File::create(&zip_path).context("Failed to create zip file")?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

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
