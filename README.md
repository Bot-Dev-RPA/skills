# skills

A collection of [Claude Code](https://claude.ai/claude-code) skills.

## Install

Add this marketplace once:

```
/plugin marketplace add Bot-Dev-RPA/skills
```

Then install the skills you want:

```
/plugin install winctl@skills
/plugin install screenshot@skills
/plugin install ocr@skills
```

To update installed skills later:

```
/plugin update
```

## Skills

### [winctl](skills/winctl/)

Resize, move, snap, maximize, minimize, center, pin, and list desktop windows using Win32 APIs.

```
/winctl Chrome 1920x1080
/winctl snap VS Code left
/winctl maximize Chrome
/winctl list
```

| Command | Example |
|---|---|
| Resize | `/winctl Chrome 800x600` |
| Move | `/winctl Chrome at 0,0` |
| Snap | `/winctl snap Chrome left` |
| Center | `/winctl center Firefox 1280x720` |
| Maximize | `/winctl maximize Chrome` |
| Minimize | `/winctl minimize Chrome` |
| Restore | `/winctl restore Chrome` |
| Pin on top | `/winctl pin Notepad on top` |
| Unpin | `/winctl unpin Notepad` |
| Move to monitor | `/winctl move Chrome to monitor 2` |
| Window info | `/winctl info Chrome` |
| List windows | `/winctl list` |
| Monitor info | `/winctl monitors` |
| Wait for window | `/winctl wait-for Notepad` |

### [screenshot](skills/screenshot/)

Capture screenshots of windows, screens, monitors, or regions. Outputs PNG files.

```
/screenshot Chrome
/screenshot --screen
/screenshot monitor 2
```

| Command | Example |
|---|---|
| Capture window | `/screenshot Chrome` |
| Capture by PID | `/screenshot --pid 1234` |
| Full screen | `/screenshot --screen` |
| Specific monitor | `/screenshot monitor 2` |
| Region | `/screenshot region 0,0,800,600` |
| Custom output path | `/screenshot Chrome --output grab.png` |

### [ocr](skills/ocr/)

Free offline OCR for scanned PDFs, images, screenshots, or clipboard. Uses Windows Snipping Tool's engine -- no API keys, network, or per-page charges.
It reads a whole PDF in one call, including scanned PDFs with no text layer. The tool itself is `oneocr.exe`, built from `src/ocr`.

```
/ocr scan.pdf
/ocr photo.png
/ocr --clipboard
/ocr scan.pdf -f json
/ocr table.png -f table
```

| Command | Example |
|---|---|
| OCR a PDF | `/ocr scan.pdf` |
| PDF render resolution | `/ocr scan.pdf --dpi 200` |
| OCR an image | `/ocr photo.png` |
| Multiple files | `/ocr scan.pdf photo.png` |
| From clipboard | `/ocr --clipboard` |
| From stdin | `cat image.png \| oneocr -` |
| Copy result | `/ocr photo.png --copy` |
| JSON with bounding boxes | `/ocr scan.pdf -f json` |
| Lines with confidence | `/ocr image.png -f lines` |
| Spaced columns | `/ocr image.png -f spaced` |
| Table detection | `/ocr table.png -f table` |

This skill was called `oneocr` before it read PDFs. If you installed it under that name, switch with:

```
/plugin uninstall oneocr@skills
/plugin install ocr@skills
```

All skills are Windows only. winctl and screenshot require Windows 10/11. ocr requires Windows 11 with Snipping Tool 11.2409+.

## Contributing

Each skill has two directories:
- `skills/<name>/` -- distribution (SKILL.md + pre-built binary)
- `src/<name>/` -- source code and build instructions

Pre-built binaries are committed to git so users don't need build tools. See `src/<name>/README.md` for build instructions.
