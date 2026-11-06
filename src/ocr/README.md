# oneocr - source

Offline OCR CLI for images and PDFs, using Windows Snipping Tool's engine via FFI.
PDF pages are rendered through `Windows.Data.Pdf`, which is built into Windows.
This is the tool behind the `ocr` skill, and the crate and binary keep the name `oneocr`.

## Build

Requires Rust 1.88+ and Windows 11 with Snipping Tool 11.2409+.

```bash
cd src/ocr
cargo build --release
cp target/release/oneocr.exe ../../skills/ocr/bin/
```

## Engine Files

The OCR engine files (~110MB) are auto-copied from Snipping Tool on first run. To use a custom location:

```bash
oneocr --engine-dir /path/to/engine image.png
# or
ONEOCR_ENGINE_DIR=/path/to/engine oneocr image.png
```

## Requirements

- Windows 11 with Snipping Tool 11.2409+
- Rust 1.88+ (build only)
