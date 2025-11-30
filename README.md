# Mistral OCR

A command-line tool to convert PDF documents into Markdown using [Mistral AI's OCR API](https://docs.mistral.ai/capabilities/document_ai/basic_ocr).

## Features

- Convert PDF files to clean Markdown format
- Extract text with high accuracy using Mistral AI's vision models
- Optional image extraction (base64 encoded)
- Simple command-line interface
- Page-by-page output organization

## Installation

### Prerequisites

- Python 3.12 or higher
- A [Mistral API key](https://console.mistral.ai/)

### Using uv (recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/mistral_ocr.git
cd mistral_ocr

# Install with uv
uv sync
```

### Using pip

```bash
# Clone the repository
git clone https://github.com/yourusername/mistral_ocr.git
cd mistral_ocr

# Install dependencies
pip install mistralai
```

## Configuration

Set your Mistral API key as an environment variable:

```bash
export MISTRAL_API_KEY='your-api-key-here'
```

Or add it to your `.bashrc`, `.zshrc`, or `.env` file for persistence.

## Usage

### Basic usage

```bash
uv run python main.py --pdf path/to/your/document.pdf
```

### With custom output path

```bash
uv run python main.py --pdf input.pdf --output results/output.md
```

### Include extracted images

```bash
uv run python main.py --pdf input.pdf --include-images
```

### Using a specific model

```bash
uv run python main.py --pdf input.pdf --model mistral-ocr-latest
```

### Command-line options

```
--pdf PATH              Path to the PDF file to process (default: ../AXP2101_no_watermark.pdf)
--model MODEL           Mistral OCR model name (default: mistral-ocr-latest)
--include-images        Include extracted images (base64) in the response
--output PATH           Where to write the markdown output (default: ocr_output.md)
```

## How It Works

1. The tool reads your PDF file and encodes it as base64
2. Sends the encoded PDF to Mistral AI's OCR API
3. Receives structured markdown output, page by page
4. Writes the formatted markdown to your specified output file

## Error Handling

The tool will display clear error messages if:
- PDF file is not found
- MISTRAL_API_KEY is not set
- API request fails
- Permission issues with output directory

## Contributing

Contributions are welcome! Feel free to:
- Report bugs
- Suggest new features
- Submit pull requests

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Mistral AI](https://mistral.ai/) OCR API
- Uses the official [mistralai Python SDK](https://github.com/mistralai/client-python)
