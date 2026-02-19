# Mistral OCR

A command-line tool to convert PDF documents into Markdown using [Mistral AI's OCR API](https://docs.mistral.ai/capabilities/document_ai/basic_ocr). Written in Rust.

## Features

- Convert PDF files to clean Markdown format
- Multiple image handling modes:
  - **separate** — save images as files in a `_images/` directory
  - **inline** — embed images as base64 data URIs (single self-contained `.md` file)
  - **zip** — bundle markdown + images into a single `.zip` archive
- Single static binary, no runtime dependencies

## Installation

### Prerequisites

- [Rust toolchain](https://rustup.rs/)
- A [Mistral API key](https://console.mistral.ai/)

### Build

```bash
git clone https://github.com/okhsunrog/mistral_ocr.git
cd mistral_ocr
cargo build --release
```

The binary will be at `target/release/mistral_ocr`.

## Configuration

Set your Mistral API key as an environment variable:

```bash
export MISTRAL_API_KEY='your-api-key-here'
```

## Usage

### Basic usage (text only)

```bash
mistral_ocr --pdf document.pdf
```

### Extract images as separate files

```bash
mistral_ocr --pdf document.pdf --images separate
```

### Single self-contained markdown file

```bash
mistral_ocr --pdf document.pdf --images inline
```

### Bundle everything into a zip

```bash
mistral_ocr --pdf document.pdf --images zip
```

### All options

```
--pdf <PATH>          Path to the PDF file to process (required)
--model <MODEL>       Mistral OCR model name (default: mistral-ocr-latest)
--images <MODE>       How to handle images: none, separate, inline, zip (default: none)
--output <PATH>       Where to write the output (default: ocr_output.md)
```

## License

MIT — see [LICENSE](LICENSE).
