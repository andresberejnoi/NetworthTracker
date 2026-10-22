//! Convert the CSV sample spreadsheets to XLSX using the app's own writer.
//!
//! Run from `backend/`:
//! `cargo run -p networth-adapters --example make_sample_xlsx -- ../samples`

use std::fs;

use networth_adapters::{read_table, write_table, SpreadsheetFormat};

fn main() {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../samples".to_string());

    let entries = fs::read_dir(&directory).expect("read samples directory");
    for entry in entries {
        let path = entry.expect("directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("csv") {
            continue;
        }

        let bytes = fs::read(&path).expect("read csv");
        let table = read_table(&bytes, SpreadsheetFormat::Csv).expect("parse csv");
        let xlsx = write_table(&table, SpreadsheetFormat::Xlsx).expect("write xlsx");

        let output = path.with_extension("xlsx");
        fs::write(&output, xlsx).expect("write xlsx file");
        println!("wrote {}", output.display());
    }
}
