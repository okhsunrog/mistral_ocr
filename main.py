import argparse
import base64
import os
import sys
from pathlib import Path

from mistralai import Mistral
from mistralai.models.sdkerror import SDKError


def encode_pdf(pdf_path: Path) -> str:
    """Return the base64-encoded contents of the PDF at *pdf_path*."""
    try:
        data = pdf_path.read_bytes()
    except FileNotFoundError as exc:
        raise FileNotFoundError(f"PDF not found: {pdf_path}") from exc
    return base64.b64encode(data).decode("utf-8")


def write_markdown(output_path: Path, response) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_lines = []
    for page in response.pages:
        output_lines.append(f"# Page {page.index + 1}\n\n")
        output_lines.append(page.markdown.rstrip() + "\n\n")
    output_path.write_text("".join(output_lines))


def run_ocr(pdf_path: Path, model: str, include_images: bool, output_path: Path) -> str:
    api_key = os.getenv("MISTRAL_API_KEY")
    if not api_key:
        raise RuntimeError("MISTRAL_API_KEY environment variable is not set")

    base64_pdf = encode_pdf(pdf_path)

    client = Mistral(api_key=api_key)
    document = {
        "type": "document_url",
        "document_url": f"data:application/pdf;base64,{base64_pdf}",
        "document_name": pdf_path.name,
    }

    kwargs = {"model": model, "document": document}
    if include_images:
        kwargs["include_image_base64"] = True

    try:
        response = client.ocr.process(**kwargs)
    except SDKError as err:
        raise RuntimeError(f"OCR request failed: {err}") from err

    write_markdown(output_path, response)
    return str(output_path)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Mistral OCR on a PDF")
    parser.add_argument(
        "--pdf",
        type=Path,
        default=Path("../AXP2101_no_watermark.pdf"),
        help="Path to the PDF file to process",
    )
    parser.add_argument(
        "--model",
        default="mistral-ocr-latest",
        help="Mistral OCR model name",
    )
    parser.add_argument(
        "--include-images",
        action="store_true",
        help="Include extracted images (base64) in the response",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("ocr_output.md"),
        help="Where to write the markdown output",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    try:
        output_file = run_ocr(args.pdf, args.model, args.include_images, args.output)
    except Exception as exc:  # noqa: BLE001
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)

    print(f"OCR markdown written to {output_file}")


if __name__ == "__main__":
    main()
