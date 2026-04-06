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

### [oneocr](skills/oneocr/)

Offline OCR to extract text from images, screenshots, or clipboard. Uses Windows Snipping Tool's engine -- no API keys or network required.

```
/oneocr photo.png
/oneocr --clipboard
/oneocr document.png -f json
/oneocr table.png -f table
```

| Command | Example |
|---|---|
| OCR a file | `/oneocr photo.png` |
| Multiple files | `/oneocr scan1.png scan2.jpg` |
| From clipboard | `/oneocr --clipboard` |
| From stdin | `cat image.png \| oneocr -` |
| Copy result | `/oneocr photo.png --copy` |
| JSON with bounding boxes | `/oneocr image.png -f json` |
| Lines with confidence | `/oneocr image.png -f lines` |
| Spaced columns | `/oneocr image.png -f spaced` |
| Table detection | `/oneocr table.png -f table` |

All skills are Windows only. winctl and screenshot require Windows 10/11. oneocr requires Windows 11 with Snipping Tool 11.2409+.

## Contributing

Each skill has two directories:
- `skills/<name>/` -- distribution (SKILL.md + pre-built binary)
- `src/<name>/` -- source code and build instructions

Pre-built binaries are committed to git so users don't need build tools. See `src/<name>/README.md` for build instructions.
