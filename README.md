# Mistral OCR

A command-line tool to convert PDF, image, and document files into Markdown using [Mistral AI's OCR API](https://docs.mistral.ai/capabilities/document_ai/basic_ocr). Written in Rust.

## Features

- Supports PDF, images (jpg, png, gif, bmp, tiff, webp), and office documents (docx, odt, pptx, xlsx, etc.)
- Office documents are automatically converted to PDF via LibreOffice
- Multiple image handling modes:
  - **separate** — save images as files in a `_images/` directory
  - **inline** — embed images as base64 data URIs (single self-contained `.md` file)
  - **zip** — bundle markdown + images into a single `.zip` archive
- Single static binary, no runtime dependencies

## Installation

### Prerequisites

- A [Mistral API key](https://console.mistral.ai/)
- [LibreOffice](https://www.libreoffice.org/) (only needed for office document conversion)

### Pre-built binary (recommended)

```bash
cargo binstall mistral_ocr
```

### From source

```bash
cargo install mistral_ocr
```

## Configuration

Set your Mistral API key as an environment variable:

```bash
export MISTRAL_API_KEY='your-api-key-here'
```

## Usage

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
--model <MODEL>       Mistral OCR model name (default: mistral-ocr-latest)
--images <MODE>       How to handle images: none, separate, inline, zip (default: none)
--output <PATH>       Where to write the output (default: ocr_output.md)
```

### Supported file types

| Type | Extensions |
|------|-----------|
| PDF | pdf |
| Images | jpg, jpeg, png, gif, bmp, tiff, webp |
| Documents (via LibreOffice) | doc, docx, odt, rtf, txt, html, pptx, ppt, odp, xlsx, xls, ods, csv, epub |

## License

MIT — see [LICENSE](LICENSE).
