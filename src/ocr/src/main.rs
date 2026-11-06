use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};

use oneocr::{
    is_pdf_path, resolve_engine_dir, setup_engine, to_spaced_text, to_table_text, OcrEngine,
    OcrImage, OcrResult, PdfFile, PdfOcrResult, PdfPageResult,
};

#[derive(Parser)]
#[command(name = "oneocr")]
#[command(about = "Extract text from images and PDFs using Windows Snipping Tool's OCR engine")]
struct Cli {
    /// Image or PDF file(s) to process. Use '-' for an image on stdin.
    files: Vec<PathBuf>,

    /// Read image from clipboard
    #[arg(long, short)]
    clipboard: bool,

    /// Copy extracted text to clipboard
    #[arg(long)]
    copy: bool,

    /// Output format
    #[arg(long, short, default_value = "text", value_enum)]
    format: OutputFormat,

    /// Path to engine directory containing oneocr.dll and model
    #[arg(long, env = "ONEOCR_ENGINE_DIR")]
    engine_dir: Option<PathBuf>,

    /// Force re-copy of engine files from Snipping Tool
    #[arg(long)]
    setup: bool,

    /// Resolution used to render PDF pages
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u32).range(72..=1200))]
    dpi: u32,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    /// Plain text (default)
    Text,
    /// Structured JSON with bounding boxes and confidence
    Json,
    /// Line-by-line with confidence scores
    Lines,
    /// Reconstructed spacing from bounding boxes
    Spaced,
    /// Detect and output tables as markdown
    Table,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.setup {
        force_setup(cli.engine_dir.as_deref())?;
        if cli.files.is_empty() && !cli.clipboard {
            return Ok(());
        }
    }

    if cli.files.is_empty() && !cli.clipboard {
        bail!("provide image or PDF file(s), --clipboard, or '-' for stdin");
    }

    let engine_dir =
        resolve_engine_dir(cli.engine_dir.as_deref()).context("engine directory resolution")?;
    let engine = OcrEngine::new(&engine_dir).context("engine initialization")?;

    let mut all_text = Vec::new();
    let multiple = (cli.files.len() + cli.clipboard as usize) > 1;

    if cli.clipboard {
        let image = image_from_clipboard().context("reading clipboard")?;
        let result = engine.recognize(&image).context("OCR failed")?;
        print_result(&result, &cli.format, if multiple { Some("clipboard") } else { None });
        all_text.push(result.text);
    }

    for path in &cli.files {
        if is_pdf_path(path) {
            process_pdf(&engine, path, &cli, multiple, &mut all_text)?;
            continue;
        }

        let image = if path.as_os_str() == "-" {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .context("reading stdin")?;
            OcrImage::from_bytes(&buf).context("decoding stdin image")?
        } else {
            OcrImage::open(path)
                .with_context(|| format!("loading {}", path.display()))?
        };

        let result = engine.recognize(&image).context("OCR failed")?;
        let header = if multiple {
            Some(path.display().to_string())
        } else {
            None
        };
        print_result(&result, &cli.format, header.as_deref());
        all_text.push(result.text);
    }

    if cli.copy {
        let combined = all_text.join("\n");
        let mut cb = arboard::Clipboard::new().context("clipboard init")?;
        cb.set_text(&combined).context("clipboard write")?;
        eprintln!("(copied to clipboard)");
    }

    Ok(())
}

fn process_pdf(
    engine: &OcrEngine,
    path: &Path,
    cli: &Cli,
    multiple: bool,
    all_text: &mut Vec<String>,
) -> Result<()> {
    let name = path.display();
    let pdf = PdfFile::open(path).with_context(|| format!("loading {name}"))?;
    let page_count = pdf.page_count().with_context(|| format!("loading {name}"))?;
    let as_json = matches!(cli.format, OutputFormat::Json);

    let mut pages = Vec::new();
    for page in 1..=page_count {
        let image = pdf
            .render_page(page, cli.dpi)
            .with_context(|| format!("rendering page {page} of {name}"))?;
        let result = engine
            .recognize(&image)
            .with_context(|| format!("OCR failed on page {page} of {name}"))?;
        all_text.push(result.text.clone());

        if as_json {
            pages.push(PdfPageResult {
                page,
                width: image.width(),
                height: image.height(),
                result,
            });
        } else {
            print_result(&result, &cli.format, Some(&format!("{name} page {page}")));
        }
    }

    if as_json {
        if multiple {
            eprintln!("--- {name} ---");
        }
        println!("{}", serde_json::to_string_pretty(&PdfOcrResult { pages })?);
    }

    Ok(())
}

fn print_result(result: &OcrResult, format: &OutputFormat, header: Option<&str>) {
    if let Some(name) = header {
        eprintln!("--- {} ---", name);
    }

    match format {
        OutputFormat::Text => {
            println!("{}", result.text);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(result).unwrap());
        }
        OutputFormat::Lines => {
            for (i, line) in result.lines.iter().enumerate() {
                let avg_conf = if line.words.is_empty() {
                    0.0
                } else {
                    line.words.iter().map(|w| w.confidence).sum::<f32>()
                        / line.words.len() as f32
                };
                println!("[{:02}] ({:.2}) {}", i, avg_conf, line.text);
            }
            if let Some(angle) = result.text_angle {
                eprintln!("angle: {:.2}", angle);
            }
        }
        OutputFormat::Spaced => {
            println!("{}", to_spaced_text(result));
        }
        OutputFormat::Table => {
            println!("{}", to_table_text(result));
        }
    }
}

fn image_from_clipboard() -> Result<OcrImage> {
    let mut cb = arboard::Clipboard::new().context("clipboard init")?;
    let img = cb.get_image().context("no image in clipboard")?;
    OcrImage::from_rgba(img.width as u32, img.height as u32, img.bytes.into_owned())
        .context("converting clipboard image")
}

fn force_setup(engine_dir: Option<&std::path::Path>) -> Result<()> {
    let target = engine_dir
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("engine")))
        })
        .unwrap_or_else(|| std::path::PathBuf::from("engine"));

    setup_engine(&target, true).context("engine setup")?;
    Ok(())
}
