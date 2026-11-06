//! PDF input tests. The fixture is image-only with no text layer, like a scanned batch:
//! page 1 "Hello World", page 2 blank, page 3 several lines, page 4 digits.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SCANNED_PDF: &str = "scanned_four_pages.pdf";

fn oneocr() -> Command {
    Command::new(env!("CARGO_BIN_EXE_oneocr"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn assert_ok(out: &Output) {
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
}

fn run_json(pdf: &Path, extra: &[&str]) -> serde_json::Value {
    let out = oneocr().arg(pdf).args(["-f", "json"]).args(extra).output().unwrap();
    assert_ok(&out);
    serde_json::from_slice(&out.stdout).expect("stdout should be one JSON document")
}

/// A temporary directory removed on drop, even if the test panics.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn text_format_reads_every_page_in_order() {
    let out = oneocr().arg(fixture(SCANNED_PDF)).output().unwrap();
    assert_ok(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let hello = stdout.find("Hello").expect("page 1 text");
    let digits = stdout.find("12345").expect("page 4 text");
    assert!(hello < digits, "pages out of order: {stdout}");
}

#[test]
fn text_format_labels_each_page_on_stderr() {
    let out = oneocr().arg(fixture(SCANNED_PDF)).output().unwrap();
    assert_ok(&out);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for number in 1..=4 {
        assert!(stderr.contains(&format!("page {number} ---")), "missing page {number}: {stderr}");
    }
}

#[test]
fn json_is_one_document_with_numbered_pages() {
    let json = run_json(&fixture(SCANNED_PDF), &[]);
    let pages = json["pages"].as_array().expect("pages array");
    assert_eq!(pages.len(), 4);
    for (index, page) in pages.iter().enumerate() {
        assert_eq!(page["page"], index as u64 + 1);
    }
    assert!(pages[0]["text"].as_str().unwrap().contains("Hello"));
    assert!(pages[3]["text"].as_str().unwrap().contains("12345"));
}

#[test]
fn json_line_boxes_fit_the_reported_page_size() {
    let json = run_json(&fixture(SCANNED_PDF), &[]);
    let page = &json["pages"][0];
    let width = page["width"].as_f64().expect("page width");
    let height = page["height"].as_f64().expect("page height");
    let lines = page["lines"].as_array().unwrap();
    assert!(!lines.is_empty());
    for line in lines {
        let bbox = &line["bounding_box"];
        for corner in ["x1", "x2", "x3", "x4"] {
            let x = bbox[corner].as_f64().unwrap();
            assert!((0.0..=width).contains(&x), "{corner}={x} outside width {width}");
        }
        for corner in ["y1", "y2", "y3", "y4"] {
            let y = bbox[corner].as_f64().unwrap();
            assert!((0.0..=height).contains(&y), "{corner}={y} outside height {height}");
        }
    }
}

#[test]
fn blank_page_is_kept_with_no_lines() {
    let json = run_json(&fixture(SCANNED_PDF), &[]);
    let blank = &json["pages"][1];
    assert_eq!(blank["page"], 2);
    assert_eq!(blank["text"], "");
    assert!(blank["lines"].as_array().unwrap().is_empty());
}

#[test]
fn dpi_sets_the_rendered_page_size() {
    let at_150 = run_json(&fixture(SCANNED_PDF), &["--dpi", "150"]);
    let at_300 = run_json(&fixture(SCANNED_PDF), &["--dpi", "300"]);
    let width_150 = at_150["pages"][0]["width"].as_f64().unwrap();
    let width_300 = at_300["pages"][0]["width"].as_f64().unwrap();
    assert!((width_300 - 2.0 * width_150).abs() <= 2.0, "{width_150} and {width_300}");
    assert!(at_150["pages"][0]["text"].as_str().unwrap().contains("Hello"));
}

#[test]
fn dpi_outside_the_supported_range_is_rejected() {
    for dpi in ["0", "71", "1201"] {
        let out = oneocr().arg(fixture(SCANNED_PDF)).args(["--dpi", dpi]).output().unwrap();
        assert!(!out.status.success(), "dpi {dpi} should be rejected");
    }
}

#[test]
fn uppercase_extension_is_treated_as_pdf() {
    let dir = TempDir::new("oneocr_test_pdf_upper");
    let copy = dir.0.join("SCAN.PDF");
    std::fs::copy(fixture(SCANNED_PDF), &copy).unwrap();
    let json = run_json(&copy, &[]);
    assert_eq!(json["pages"].as_array().unwrap().len(), 4);
}

#[test]
fn relative_path_with_spaces_works() {
    let dir = TempDir::new("oneocr_test_pdf_relative");
    std::fs::copy(fixture(SCANNED_PDF), dir.0.join("my scan.pdf")).unwrap();
    let out = oneocr()
        .current_dir(&dir.0)
        .args(["my scan.pdf", "-f", "json"])
        .output()
        .unwrap();
    assert_ok(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["pages"].as_array().unwrap().len(), 4);
}

#[test]
fn file_that_is_not_a_pdf_fails_and_names_the_file() {
    let dir = TempDir::new("oneocr_test_pdf_corrupt");
    let bad = dir.0.join("broken.pdf");
    std::fs::write(&bad, b"this is not a pdf").unwrap();
    let out = oneocr().arg(&bad).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("broken.pdf"));
}

#[test]
fn password_protected_pdf_fails_with_a_clear_message() {
    let out = oneocr().arg(fixture("password_protected.pdf")).output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("password_protected.pdf"), "{stderr}");
    assert!(stderr.contains("password-protected PDFs are not supported"), "{stderr}");
}

#[test]
fn missing_pdf_fails_and_names_the_file() {
    let out = oneocr().arg("no_such_scan_12345.pdf").output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no_such_scan_12345.pdf"));
}

#[test]
fn pdf_and_image_can_be_given_together() {
    let out = oneocr()
        .arg(fixture(SCANNED_PDF))
        .arg(fixture("numbers.png"))
        .output()
        .unwrap();
    assert_ok(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Hello") && stdout.contains("12345"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("numbers.png"));
}

#[test]
fn image_json_output_is_unchanged() {
    let out = oneocr().arg(fixture("hello.png")).args(["-f", "json"]).output().unwrap();
    assert_ok(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["text"].is_string());
    assert!(json.get("pages").is_none());
}
