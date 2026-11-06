---
name: ocr
description: Extract text from scanned PDFs and images with free offline OCR. Use when the user asks to read, OCR or extract text from a scanned PDF, a PDF with no text layer, an image, a screenshot, a photo of a document, or the clipboard, or when a PDF text tool returns nothing. Reads a whole multi-page PDF in one call and can return line and word positions as JSON. Windows 11 only.
argument-hint: <pdf-or-image-path> | --clipboard | - (stdin)
allowed-tools: Bash
---

# ocr

Offline OCR for scanned PDFs and images, using Windows Snipping Tool's engine.
It needs no API keys or network, and has no per-page charge.

## Tool

```
EXE="${CLAUDE_SKILL_DIR}/bin/oneocr.exe"
```

Run via Bash: `"${CLAUDE_SKILL_DIR}/bin/oneocr.exe" <args>`

## How It Works

- **Images** go straight to the OCR engine.
- **PDFs** are rendered page by page through `Windows.Data.Pdf`, which is built into Windows, and each page then goes through the same engine.
  This reads scanned PDFs with no text layer and digital PDFs alike.
  A file counts as a PDF when its extension is `.pdf`, in any letter case.
- **The engine loads once per call**, so pass a whole PDF or many files in one call, not one call per page.

## First Run

The engine files (~110MB) are auto-copied from Windows Snipping Tool on first use. Requires Windows 11 with Snipping Tool 11.2409+.

If engine files are already available elsewhere, pass `--engine-dir <path>` or set `ONEOCR_ENGINE_DIR`.

## Inputs

- `<file>` -- one or more PDF or image files (PDF, PNG, JPEG, BMP, GIF, TIFF, WebP)
- `--clipboard` / `-c` -- read image from clipboard
- `-` -- read image from stdin

## Output Formats

- `--format text` (default) -- plain extracted text
- `--format json` / `-f json` -- structured JSON with bounding boxes and confidence
- `--format lines` / `-f lines` -- per-line with index and confidence
- `--format spaced` / `-f spaced` -- reconstructs horizontal spacing for column-aligned text
- `--format table` / `-f table` -- detects tabular layouts, outputs as Markdown

For a PDF, the text formats print each page in order and label it on stderr, for example `--- scan.pdf page 3 ---`.

For a PDF, `-f json` prints one document.
Each page's `width` and `height` give the rendered size in pixels that its bounding boxes refer to:

```json
{
  "pages": [
    {
      "page": 1,
      "width": 2550,
      "height": 3300,
      "text": "Hello World!",
      "text_angle": 0.09,
      "lines": [
        {
          "text": "Hello World!",
          "bounding_box": { "x1": 10.0, "y1": 20.0, "x2": 200.0, "y2": 20.0, "x3": 200.0, "y3": 50.0, "x4": 10.0, "y4": 50.0 },
          "words": [{ "text": "Hello", "confidence": 0.99, "bounding_box": { } }]
        }
      ]
    }
  ]
}
```

## Other Flags

- `--dpi <72-1200>` -- PDF render resolution, 300 by default
- `--copy` -- copy extracted text to clipboard
- `--engine-dir <path>` -- path to engine directory
- `--setup` -- force a re-copy of the engine files from Snipping Tool

## Examples

```bash
"$EXE" scan.pdf
"$EXE" scan.pdf -f json > scan.json
"$EXE" scan.pdf --dpi 200
"$EXE" photo.png
"$EXE" screenshot.png -f json
"$EXE" --clipboard
"$EXE" document.png --copy
"$EXE" scan1.png scan2.jpg scan.pdf
cat image.png | "$EXE" -
"$EXE" table.png -f table
```

## Parsing natural language from $ARGUMENTS

| User says | Command |
|---|---|
| `ocr scan.pdf` | `scan.pdf` |
| `read this scanned pdf` | `<file>.pdf` |
| `pdf text extraction found nothing` | `<file>.pdf` |
| `ocr screenshot.png` | `screenshot.png` |
| `read text from image.png` | `image.png` |
| `ocr clipboard` | `--clipboard` |
| `extract text as json` | `<file> -f json` |
| `ocr this table` | `<file> -f table` |

## Notes

- A 206-page scanned PDF took about 2 minutes at 300 DPI and about 94 seconds at 200 DPI on a 16-core PC
- PDF JSON runs to about 80 KB per page, so write long PDFs to a file with `> scan.json` and read what you need from there
- Password-protected PDFs are not supported, and the tool reports them as an error
- Any failed page fails the whole run, so a successful run has read every page
- Supports images and rendered pages from 50x50 to 10,000x10,000 pixels, and very large PDF pages are scaled down to fit
- Engine files are cached next to the executable after first run
- The engine files are Microsoft's, so they are copied from the local Snipping Tool and never shipped with this skill
- The engine is an undocumented part of Snipping Tool, so Microsoft does not support this use and a Windows update could break it
- For structured data extraction, prefer `-f json` for per-word bounding boxes
- For tabular content, use `-f table` for Markdown output
- If the user provides a screenshot path, just OCR it directly
