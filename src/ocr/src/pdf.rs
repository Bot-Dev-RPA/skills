//! Reads PDF pages as images through `Windows.Data.Pdf`, which is built into Windows.
//! Rendering covers scanned and digital PDFs alike, with no PDF library to ship.

use std::path::Path;

use windows::core::{HRESULT, HSTRING};
use windows::Data::Pdf::{PdfDocument, PdfPageRenderOptions};
use windows::Graphics::Imaging::BitmapEncoder;
use windows::Storage::StorageFile;
use windows::Storage::Streams::{DataReader, InMemoryRandomAccessStream};

use crate::error::OcrError;
use crate::types::{OcrImage, MAX_DIM, MIN_DIM};

/// `PdfPage.Size` reports device-independent pixels, at 96 per inch.
const DIPS_PER_INCH: f32 = 96.0;

/// What `PdfDocument` returns for a PDF that needs a password.
const ERROR_WRONG_PASSWORD: HRESULT = HRESULT(0x8007_052B_u32 as i32);

/// True when the path has a `.pdf` extension, in any letter case.
pub fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

pub struct PdfFile {
    document: PdfDocument,
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
        Ok(Self { document })
    }

    pub fn page_count(&self) -> Result<u32, OcrError> {
        Ok(self.document.PageCount()?)
    }

    /// Renders one page, numbered from 1, at the given resolution.
    pub fn render_page(&self, number: u32, dpi: u32) -> Result<OcrImage, OcrError> {
        let page = self.document.GetPage(number - 1)?;
        let size = page.Size()?;
        let (width, height) = render_size(size.Width, size.Height, dpi);

        let options = PdfPageRenderOptions::new()?;
        options.SetDestinationWidth(width)?;
        options.SetDestinationHeight(height)?;
        // BMP avoids a slow PNG encode and decode of every page.
        options.SetBitmapEncoderId(BitmapEncoder::BmpEncoderId()?)?;

        let stream = InMemoryRandomAccessStream::new()?;
        page.RenderWithOptionsToStreamAsync(&stream, &options)?.join()?;

        let length = stream.Size()? as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(length)?.join()?;
        let mut bytes = vec![0u8; length as usize];
        reader.ReadBytes(&mut bytes)?;

        OcrImage::from_bytes(&bytes)
    }
}

/// Pixel size for a page at `dpi`, scaled down when needed to fit what the engine accepts.
fn render_size(width_dips: f32, height_dips: f32, dpi: u32) -> (u32, u32) {
    let mut scale = dpi as f32 / DIPS_PER_INCH;
    let longest = width_dips.max(height_dips) * scale;
    if longest > MAX_DIM as f32 {
        scale *= MAX_DIM as f32 / longest;
    }

    let pixels = |dips: f32| ((dips * scale).round() as u32).clamp(MIN_DIM, MAX_DIM);
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
        assert_eq!((width, height), (MAX_DIM, MAX_DIM / 2));
    }

    #[test]
    fn render_size_raises_a_tiny_page_to_the_engine_minimum() {
        assert_eq!(render_size(10.0, 10.0, 96), (MIN_DIM, MIN_DIM));
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
