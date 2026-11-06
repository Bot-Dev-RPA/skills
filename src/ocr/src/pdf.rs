//! Reads PDF pages as images through `Windows.Data.Pdf`, which is built into Windows.
//! Rendering covers scanned and digital PDFs alike, with no PDF library to ship.

use std::io::Cursor;
use std::path::Path;

use windows::core::{HRESULT, HSTRING};
use windows::Data::Pdf::{PdfDocument, PdfPage, PdfPageRenderOptions};
use windows::Graphics::Imaging::BitmapEncoder;
use windows::Storage::StorageFile;
use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

use crate::error::OcrError;
use crate::types::{OcrImage, MAX_DIM, MIN_DIM};

/// `PdfPage.Size` reports device-independent pixels, at 96 per inch.
const DIPS_PER_INCH: f32 = 96.0;

/// Size of the test render that measures display scaling.
const PROBE_DIPS: u32 = 1000;

/// Rendering rounds to whole device-independent pixels, so targets stay this far inside the engine's limits.
const ROUNDING_MARGIN: u32 = 2;

/// What `PdfDocument` returns for a PDF that needs a password.
const ERROR_WRONG_PASSWORD: HRESULT = HRESULT(0x8007_052B_u32 as i32);

/// True when the path has a `.pdf` extension, in any letter case.
pub fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

pub struct PdfFile {
    document: PdfDocument,
    pixels_per_dip: f32,
}

impl PdfFile {
    pub fn open(path: &Path) -> Result<Self, OcrError> {
        // StorageFile takes absolute paths only, and rejects the \\?\ form that canonicalize returns.
        let absolute = std::path::absolute(path).map_err(|e| OcrError::PdfPath {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(absolute.as_os_str()))?.join()?;
        let document = PdfDocument::LoadFromFileAsync(&file)?.join().map_err(|e| {
            if e.code() == ERROR_WRONG_PASSWORD {
                OcrError::PdfPasswordProtected
            } else {
                OcrError::Pdf(e)
            }
        })?;
        let pixels_per_dip = measure_pixels_per_dip(&document)?;
        Ok(Self {
            document,
            pixels_per_dip,
        })
    }

    pub fn page_count(&self) -> Result<u32, OcrError> {
        Ok(self.document.PageCount()?)
    }

    /// Renders one page, numbered from 1, at the given resolution.
    pub fn render_page(&self, number: u32, dpi: u32) -> Result<OcrImage, OcrError> {
        let page = self.document.GetPage(number - 1)?;
        let size = page.Size()?;
        let (width, height) = render_size(size.Width, size.Height, dpi);
        let bytes = render_bitmap(
            &page,
            dips_for_pixels(width, self.pixels_per_dip),
            dips_for_pixels(height, self.pixels_per_dip),
        )?;
        OcrImage::from_bytes(&bytes)
    }
}

/// Render sizes are in device-independent pixels, so the pixels they produce depend on the
/// machine's display scaling. A small test render measures that factor, which keeps `--dpi`
/// true on any machine.
fn measure_pixels_per_dip(document: &PdfDocument) -> Result<f32, OcrError> {
    if document.PageCount()? == 0 {
        return Ok(1.0);
    }

    let bytes = render_bitmap(&document.GetPage(0)?, PROBE_DIPS, PROBE_DIPS)?;
    let (width, _) = image::ImageReader::with_format(Cursor::new(&bytes), image::ImageFormat::Bmp)
        .into_dimensions()?;
    Ok(width as f32 / PROBE_DIPS as f32)
}

/// Renders a page to BMP bytes, because BMP avoids a slow PNG encode and decode of every page.
fn render_bitmap(page: &PdfPage, width_dips: u32, height_dips: u32) -> Result<Vec<u8>, OcrError> {
    let options = PdfPageRenderOptions::new()?;
    options.SetDestinationWidth(width_dips)?;
    options.SetDestinationHeight(height_dips)?;
    options.SetBitmapEncoderId(BitmapEncoder::BmpEncoderId()?)?;

    let stream = InMemoryRandomAccessStream::new()?;
    page.RenderWithOptionsToStreamAsync(&stream, &options)?.join()?;

    let length = stream.Size()? as u32;
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    reader.LoadAsync(length)?.join()?;
    let mut bytes = vec![0u8; length as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

fn dips_for_pixels(pixels: u32, pixels_per_dip: f32) -> u32 {
    (pixels as f32 / pixels_per_dip).round() as u32
}

/// Pixel size for a page at `dpi`, scaled down when needed to fit what the engine accepts.
fn render_size(width_dips: f32, height_dips: f32, dpi: u32) -> (u32, u32) {
    let mut scale = dpi as f32 / DIPS_PER_INCH;
    let (fewest, most) = (MIN_DIM + ROUNDING_MARGIN, MAX_DIM - ROUNDING_MARGIN);
    let longest = width_dips.max(height_dips) * scale;
    if longest > most as f32 {
        scale *= most as f32 / longest;
    }

    let pixels = |dips: f32| ((dips * scale).round() as u32).clamp(fewest, most);
    (pixels(width_dips), pixels(height_dips))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LETTER_WIDTH: f32 = 816.0;
    const LETTER_HEIGHT: f32 = 1056.0;

    #[test]
    fn render_size_scales_from_96_per_inch() {
        assert_eq!(render_size(LETTER_WIDTH, LETTER_HEIGHT, 96), (816, 1056));
        assert_eq!(render_size(LETTER_WIDTH, LETTER_HEIGHT, 300), (2550, 3300));
    }

    #[test]
    fn render_size_fits_a_huge_page_within_the_engine_limit() {
        let (width, height) = render_size(10_000.0, 5_000.0, 300);
        assert_eq!((width, height), (MAX_DIM - ROUNDING_MARGIN, (MAX_DIM - ROUNDING_MARGIN) / 2));
    }

    #[test]
    fn render_size_raises_a_tiny_page_to_the_engine_minimum() {
        let smallest = MIN_DIM + ROUNDING_MARGIN;
        assert_eq!(render_size(10.0, 10.0, 96), (smallest, smallest));
    }

    #[test]
    fn dips_for_pixels_undoes_display_scaling() {
        assert_eq!(dips_for_pixels(2550, 1.0), 2550);
        assert_eq!(dips_for_pixels(2550, 1.4), 1821);
        assert_eq!(dips_for_pixels(2550, 2.0), 1275);
    }

    #[test]
    fn scaled_size_stays_within_the_engine_limits_after_rounding() {
        for scale in [1.0_f32, 1.25, 1.4, 1.5, 1.75, 2.0, 2.25, 3.0] {
            for target in [MIN_DIM + ROUNDING_MARGIN, MAX_DIM - ROUNDING_MARGIN] {
                let produced = (dips_for_pixels(target, scale) as f32 * scale).round() as u32;
                assert!((MIN_DIM..=MAX_DIM).contains(&produced), "{target} at scale {scale} gave {produced}");
            }
        }
    }

    #[test]
    fn pdf_extension_is_recognized_in_any_case() {
        assert!(is_pdf_path(Path::new("scan.pdf")));
        assert!(is_pdf_path(Path::new("SCAN.PDF")));
        assert!(is_pdf_path(Path::new("C:\\folder name\\scan.Pdf")));
    }

    #[test]
    fn other_paths_are_not_pdfs() {
        assert!(!is_pdf_path(Path::new("scan.png")));
        assert!(!is_pdf_path(Path::new("pdf")));
        assert!(!is_pdf_path(Path::new("-")));
    }
}
