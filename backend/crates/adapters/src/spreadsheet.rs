//! Reading and writing CSV and XLSX tables.
//!
//! Values are handled as text end to end. XLSX numeric cells are converted to
//! decimal text, and the writer always emits text, so amounts are not subjected
//! to Excel's binary float representation.

use calamine::{open_workbook_auto, Data, Reader};
use networth_application::Table;
use rust_xlsxwriter::Workbook;

/// The supported spreadsheet encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadsheetFormat {
    /// Comma-separated values.
    Csv,
    /// Office Open XML workbook.
    Xlsx,
}

impl SpreadsheetFormat {
    /// Parse a format name (`csv`, `xlsx`, `excel`).
    pub fn from_name(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "xlsx" | "excel" => Some(Self::Xlsx),
            _ => None,
        }
    }

    /// Parse a format from a `Content-Type` header.
    pub fn from_content_type(value: &str) -> Option<Self> {
        let value = value
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        match value.as_str() {
            "text/csv" => Some(Self::Csv),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some(Self::Xlsx),
            _ => None,
        }
    }

    /// The response content type.
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Csv => "text/csv; charset=utf-8",
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        }
    }

    /// The file extension.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
        }
    }
}

/// Errors raised while reading or writing a spreadsheet.
#[derive(Debug, thiserror::Error)]
pub enum SpreadsheetError {
    /// The file could not be read or parsed.
    #[error("could not read the spreadsheet: {0}")]
    Read(String),
    /// The file could not be written.
    #[error("could not write the spreadsheet: {0}")]
    Write(String),
    /// The file has no header row.
    #[error("the spreadsheet has no header row")]
    Empty,
}

/// Read a table from CSV or XLSX bytes.
pub fn read_table(bytes: &[u8], format: SpreadsheetFormat) -> Result<Table, SpreadsheetError> {
    match format {
        SpreadsheetFormat::Csv => read_csv(bytes),
        SpreadsheetFormat::Xlsx => read_xlsx(bytes),
    }
}

/// Write a table to CSV or XLSX bytes.
pub fn write_table(table: &Table, format: SpreadsheetFormat) -> Result<Vec<u8>, SpreadsheetError> {
    match format {
        SpreadsheetFormat::Csv => write_csv(table),
        SpreadsheetFormat::Xlsx => write_xlsx(table),
    }
}

fn read_csv(bytes: &[u8]) -> Result<Table, SpreadsheetError> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);

    let headers: Vec<String> = reader
        .headers()
        .map_err(|error| SpreadsheetError::Read(error.to_string()))?
        .iter()
        .map(|header| header.trim().to_string())
        .collect();

    if headers.is_empty() {
        return Err(SpreadsheetError::Empty);
    }

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|error| SpreadsheetError::Read(error.to_string()))?;
        let mut row: Vec<String> = record.iter().map(|cell| cell.trim().to_string()).collect();
        row.resize(headers.len(), String::new());
        rows.push(row);
    }

    Ok(Table { headers, rows })
}

fn read_xlsx(bytes: &[u8]) -> Result<Table, SpreadsheetError> {
    let temp = tempfile::NamedTempFile::new()
        .map_err(|error| SpreadsheetError::Read(error.to_string()))?;
    std::fs::write(temp.path(), bytes)
        .map_err(|error| SpreadsheetError::Read(error.to_string()))?;

    let mut workbook = open_workbook_auto(temp.path())
        .map_err(|error| SpreadsheetError::Read(error.to_string()))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or(SpreadsheetError::Empty)?;

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|error| SpreadsheetError::Read(error.to_string()))?;

    let mut iter = range.rows();
    let headers: Vec<String> = iter
        .next()
        .ok_or(SpreadsheetError::Empty)?
        .iter()
        .map(cell_to_string)
        .collect();

    if headers.is_empty() {
        return Err(SpreadsheetError::Empty);
    }

    let mut rows = Vec::new();
    for row in iter {
        let mut values: Vec<String> = row.iter().map(cell_to_string).collect();
        values.resize(headers.len(), String::new());
        if values.iter().all(|value| value.is_empty()) {
            continue;
        }
        rows.push(values);
    }

    Ok(Table { headers, rows })
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => float_to_decimal(*value),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        _ => String::new(),
    }
}

/// Render an Excel numeric cell as decimal text without trailing zeros.
fn float_to_decimal(value: f64) -> String {
    if !value.is_finite() {
        return String::new();
    }
    let mut text = format!("{value:.15}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    text
}

fn write_csv(table: &Table) -> Result<Vec<u8>, SpreadsheetError> {
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer
        .write_record(&table.headers)
        .map_err(|error| SpreadsheetError::Write(error.to_string()))?;
    for row in &table.rows {
        writer
            .write_record(row)
            .map_err(|error| SpreadsheetError::Write(error.to_string()))?;
    }
    writer
        .flush()
        .map_err(|error| SpreadsheetError::Write(error.to_string()))?;
    writer
        .into_inner()
        .map_err(|error| SpreadsheetError::Write(error.to_string()))
}

fn write_xlsx(table: &Table) -> Result<Vec<u8>, SpreadsheetError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    for (column, header) in table.headers.iter().enumerate() {
        worksheet
            .write_string(0, column as u16, header)
            .map_err(|error| SpreadsheetError::Write(error.to_string()))?;
    }

    for (row_index, row) in table.rows.iter().enumerate() {
        for (column, value) in row.iter().enumerate() {
            // Always write text: this keeps decimals exact and avoids Excel's
            // float representation.
            worksheet
                .write_string((row_index + 1) as u32, column as u16, value)
                .map_err(|error| SpreadsheetError::Write(error.to_string()))?;
        }
    }

    workbook
        .save_to_buffer()
        .map_err(|error| SpreadsheetError::Write(error.to_string()))
}
