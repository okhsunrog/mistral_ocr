# Mistral OCR

Convert PDF, image, and document files into Markdown using [Mistral AI's OCR API](https://docs.mistral.ai/capabilities/document_ai/basic_ocr). Written in Rust.

Available as a **CLI tool** (`mistral_ocr`) and a **GUI app** (`mistral_ocr_gui`).

## Features

- Supports PDF, images (jpg, png, gif, bmp, tiff, webp), and office documents (docx, odt, pptx, xlsx, etc.)
- **PDF and image files require no external dependencies** — just the binary and an API key
- Office documents (docx, odt, pptx, etc.) are automatically converted to PDF via LibreOffice
- Multiple image handling modes:
  - **separate** — save images as files in a `_images/` directory
  - **inline** — embed images as base64 data URIs (single self-contained `.md` file)
  - **zip** — bundle markdown + images into a single `.zip` archive
- Cross-platform: works on Linux, macOS, and Windows

## Installation

### Prerequisites

- A [Mistral API key](https://console.mistral.ai/)
- [LibreOffice](https://www.libreoffice.org/) — **only** needed if you process office documents (docx, odt, pptx, xlsx, etc.). Not required for PDF or image files.

### CLI (pre-built binary)

```bash
cargo binstall mistral_ocr
```

### CLI (from source)

```bash
cargo install mistral_ocr
```

### GUI (pre-built binary)

```bash
cargo binstall mistral_ocr_gui
```

### GUI (from source)

```bash
cargo install mistral_ocr_gui
```

## Configuration

Set your Mistral API key as an environment variable (the GUI also accepts it in the UI):

```bash
export MISTRAL_API_KEY='your-api-key-here'
```

## CLI Usage

### Basic usage (text only)

```bash
mistral_ocr document.pdf
```

### Process an image

```bash
mistral_ocr photo.png
```

### Process a Word document

```bash
mistral_ocr report.docx --images inline
```

### Extract images as separate files

```bash
mistral_ocr document.pdf --images separate
```

### Single self-contained markdown file

```bash
mistral_ocr document.pdf --images inline
```

### Bundle everything into a zip

```bash
mistral_ocr document.pdf --images zip
```

### All options

```
<INPUT>               Path to the input file (required)
--images <MODE>       How to handle images: none, separate, inline, zip (default: none)
--output <PATH>       Where to write the output (default: ocr_output.md)
```

### Supported file types

| Type | Extensions | Requires LibreOffice? |
|------|-----------|:---------------------:|
| PDF | pdf | No |
| Images | jpg, jpeg, png, gif, bmp, tiff, webp | No |
| Documents | doc, docx, odt, rtf, txt, html, pptx, ppt, odp, xlsx, xls, ods, csv, epub | Yes |

## Project Structure

This is a Cargo workspace with two crates:

- **`mistral_ocr`** — library + CLI binary (published on [crates.io](https://crates.io/crates/mistral_ocr))
- **`mistral_ocr_gui`** — GUI binary using egui

## License

MIT — see [LICENSE](LICENSE).
