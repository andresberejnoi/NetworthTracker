//! Encrypted local document store, content detection, parsing, and audit.

use std::fs;
use std::io::{Cursor, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use calamine::{open_workbook_from_rs, Data, Reader};
use flate2::read::ZlibDecoder;
use networth_domain::{Currency, Money, NewBalance, Quantity};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Iso8601;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::data_exchange::Table;
use crate::migration::{
    MigrationApplyRequest, MigrationMode, MigrationService, MigrationSourceKind,
};
use crate::ports::{
    DocumentAuditEvent, DocumentRepository, NewDocument, RepositoryError, StoredDocument,
};

/// The parser version recorded on parsed documents.
pub const PARSER_VERSION: &str = "generic-1";

/// Detected file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    /// CSV text.
    Csv,
    /// Office Open XML workbook.
    Xlsx,
    /// Generic XML.
    Xml,
    /// OFX/QFX.
    Ofx,
    /// PDF.
    Pdf,
    /// Content that no adapter recognizes.
    Unknown,
}

impl DocumentFormat {
    /// Stable identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentFormat::Csv => "csv",
            DocumentFormat::Xlsx => "xlsx",
            DocumentFormat::Xml => "xml",
            DocumentFormat::Ofx => "ofx",
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::Unknown => "unknown",
        }
    }
}

/// A parsed document ready for preview or mapping.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentPreview {
    /// Document metadata.
    pub document: StoredDocument,
    /// Detected format.
    pub format: String,
    /// Detected institution.
    pub institution: Option<String>,
    /// Detected document type.
    pub document_type: Option<String>,
    /// Classification confidence.
    pub confidence: Option<f64>,
    /// Parsed table, if any.
    pub table: Option<Table>,
    /// Parse error, if any.
    pub error: Option<String>,
    /// Audit trail.
    pub audit: Vec<DocumentAuditEvent>,
}

/// Errors raised by the document store.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// Invalid input or unsupported content.
    #[error("{0}")]
    Validation(String),
    /// The document does not exist.
    #[error("{0}")]
    NotFound(String),
    /// Persistence failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    /// Filesystem or encryption failed.
    #[error("{0}")]
    Storage(String),
    /// Migration failed.
    #[error(transparent)]
    Migration(#[from] crate::migration::MigrationError),
}

/// A parser adapter selected by content detection.
pub trait DocumentParser: Send + Sync {
    /// Format handled by this adapter.
    fn format(&self) -> DocumentFormat;
    /// Parse plaintext bytes into a table.
    fn parse(&self, bytes: &[u8]) -> Result<Table, String>;
}

struct CsvParser;
struct XlsxParser;
struct XmlParser;
struct OfxParser;
struct PdfParser;

impl DocumentParser for CsvParser {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Csv
    }

    fn parse(&self, bytes: &[u8]) -> Result<Table, String> {
        let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(text.as_bytes());
        let headers = reader
            .headers()
            .map_err(|error| error.to_string())?
            .iter()
            .map(|header| header.trim().to_string())
            .collect::<Vec<_>>();
        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record.map_err(|error| error.to_string())?;
            rows.push(
                record
                    .iter()
                    .map(|value| value.trim().to_string())
                    .collect(),
            );
        }
        Ok(Table { headers, rows })
    }
}

impl DocumentParser for XlsxParser {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Xlsx
    }

    fn parse(&self, bytes: &[u8]) -> Result<Table, String> {
        let cursor = Cursor::new(bytes.to_vec());
        let mut workbook: calamine::Xlsx<_> =
            open_workbook_from_rs(cursor).map_err(|error| format!("invalid XLSX: {error}"))?;
        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| "XLSX has no worksheets".to_string())?
            .map_err(|error| error.to_string())?;
        let mut rows = range.rows();
        let headers = rows
            .next()
            .map(|row| row.iter().map(cell_text).collect())
            .unwrap_or_default();
        let values = rows
            .map(|row| row.iter().map(cell_text).collect())
            .collect();
        Ok(Table {
            headers,
            rows: values,
        })
    }
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => value.to_string(),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) | Data::DurationIso(value) => value.clone(),
        Data::Error(value) => format!("{value:?}"),
    }
}

impl DocumentParser for XmlParser {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Xml
    }

    fn parse(&self, bytes: &[u8]) -> Result<Table, String> {
        parse_tag_table(bytes, false)
    }
}

impl DocumentParser for OfxParser {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Ofx
    }

    fn parse(&self, bytes: &[u8]) -> Result<Table, String> {
        parse_tag_table(bytes, true)
    }
}

impl DocumentParser for PdfParser {
    fn format(&self) -> DocumentFormat {
        DocumentFormat::Pdf
    }

    fn parse(&self, bytes: &[u8]) -> Result<Table, String> {
        let text = extract_pdf_text(bytes)?;
        let rows = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| vec![line.trim().to_string()])
            .collect();
        Ok(Table {
            headers: vec!["text".to_string()],
            rows,
        })
    }
}

fn parse_tag_table(bytes: &[u8], ofx: bool) -> Result<Table, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut headers = Vec::new();
    let mut values = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = text[cursor..].find('<') {
        let start = cursor + start;
        let Some(end) = text[start..].find('>') else {
            break;
        };
        let end = start + end;
        let tag = text[start + 1..end].trim();
        if tag.starts_with('/') || tag.starts_with('?') || tag.starts_with('!') || tag.is_empty() {
            cursor = end + 1;
            continue;
        }
        let name = tag.split_whitespace().next().unwrap_or("");
        let close = format!("</{name}>");
        if let Some(close_index) = text[end + 1..].find(&close) {
            let close_index = end + 1 + close_index;
            let value = text[end + 1..close_index].trim();
            if !value.contains('<') {
                headers.push(name.to_ascii_lowercase());
                values.push(value.to_string());
                cursor = close_index + close.len();
            } else {
                // Nested element: keep scanning its children instead of jumping
                // over the whole outer element.
                cursor = end + 1;
            }
        } else {
            cursor = end + 1;
        }
    }
    if headers.is_empty() {
        return Err(if ofx {
            "no OFX values found".to_string()
        } else {
            "no XML values found".to_string()
        });
    }
    Ok(Table {
        headers,
        rows: vec![values],
    })
}

fn extract_pdf_text(bytes: &[u8]) -> Result<String, String> {
    let mut text = String::new();
    let mut cursor = 0usize;
    while let Some(start) = find_bytes(&bytes[cursor..], b"stream") {
        let start = cursor + start + b"stream".len();
        let start = if bytes.get(start) == Some(&b'\r') {
            start
                + if bytes.get(start + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                }
        } else if bytes.get(start) == Some(&b'\n') {
            start + 1
        } else {
            start
        };
        let Some(end) = find_bytes(&bytes[start..], b"endstream") else {
            break;
        };
        let raw = &bytes[start..start + end];
        let mut decoded = String::new();
        let mut decoder = ZlibDecoder::new(raw);
        if decoder.read_to_string(&mut decoded).is_err() {
            decoded = String::from_utf8_lossy(raw).to_string();
        }
        text.push_str(&extract_pdf_strings(&decoded));
        text.push('\n');
        cursor = start + end + b"endstream".len();
    }
    if text.trim().is_empty() {
        return Err(
            "no extractable PDF text (scanned/compressed PDFs are retained unparsed)".to_string(),
        );
    }
    Ok(text)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn extract_pdf_strings(content: &str) -> String {
    let mut result = String::new();
    let mut chars = content.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '(' {
            continue;
        }
        let mut value = String::new();
        let mut escape = false;
        for next in chars.by_ref() {
            if escape {
                value.push(match next {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escape = false;
            } else if next == '\\' {
                escape = true;
            } else if next == ')' {
                break;
            } else {
                value.push(next);
            }
        }
        if !value.trim().is_empty() {
            result.push_str(&value);
            result.push(' ');
        }
    }
    if result.is_empty() {
        result
    } else {
        result.trim().to_string()
    }
}

fn detect_format(bytes: &[u8], name: &str) -> DocumentFormat {
    if bytes.starts_with(b"%PDF") {
        return DocumentFormat::Pdf;
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return DocumentFormat::Xlsx;
    }
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with("<OFX") || text.to_ascii_lowercase().contains("<ofx>") {
        return DocumentFormat::Ofx;
    }
    if trimmed.starts_with('<') {
        return DocumentFormat::Xml;
    }
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".csv") || trimmed.contains(',') || trimmed.contains('\t') {
        return DocumentFormat::Csv;
    }
    DocumentFormat::Unknown
}

/// Detect the format and parse a document using the matching adapter.
///
/// The extension is ignored; magic bytes and text sniffing decide the format.
pub fn parse_document(bytes: &[u8], name: &str) -> (DocumentFormat, Result<Table, String>) {
    let format = detect_format(bytes, name);
    let result = match parser_for(format) {
        Some(parser) => parser.parse(bytes),
        None => Err("unsupported document format".to_string()),
    };
    (format, result)
}

/// Classify a parsed document by institution and type.
pub fn classify_document(name: &str, text: &str, table: &Table) -> Classification {
    classify(name, text, table)
}

fn parser_for(format: DocumentFormat) -> Option<Box<dyn DocumentParser>> {
    match format {
        DocumentFormat::Csv => Some(Box::new(CsvParser)),
        DocumentFormat::Xlsx => Some(Box::new(XlsxParser)),
        DocumentFormat::Xml => Some(Box::new(XmlParser)),
        DocumentFormat::Ofx => Some(Box::new(OfxParser)),
        DocumentFormat::Pdf => Some(Box::new(PdfParser)),
        DocumentFormat::Unknown => None,
    }
}

/// Classification heuristics used before user confirmation.
#[derive(Debug, Clone, Serialize)]
pub struct Classification {
    /// Institution name.
    pub institution: String,
    /// Document type.
    pub document_type: String,
    /// Confidence in 0..1.
    pub confidence: f64,
    /// Statement period, if a year was found.
    pub period: Option<String>,
}

fn classify(name: &str, text: &str, table: &Table) -> Classification {
    let haystack = format!("{} {}", name, text).to_ascii_lowercase();
    let (institution, institution_confidence): (String, f64) = if haystack.contains("fidelity") {
        ("Fidelity".to_string(), 0.95)
    } else if haystack.contains("schwab") {
        ("Charles Schwab".to_string(), 0.95)
    } else if haystack.contains("coinbase") {
        ("Coinbase".to_string(), 0.95)
    } else if haystack.contains("binance") {
        ("Binance".to_string(), 0.95)
    } else {
        ("Generic".to_string(), 0.25)
    };

    let headers = table
        .headers
        .iter()
        .map(|header| header.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    let (document_type, type_confidence) =
        if haystack.contains("1099") || haystack.contains("tax form") {
            ("tax_form", 0.9)
        } else if haystack.contains("transaction") || haystack.contains("activity") {
            ("transactions", 0.85)
        } else if haystack.contains("holding") || haystack.contains("position") {
            ("holdings", 0.85)
        } else if haystack.contains("balance") {
            ("balances", 0.8)
        } else if haystack.contains("statement") {
            ("statement", 0.75)
        } else if headers.contains("symbol") || headers.contains("quantity") {
            ("holdings", 0.6)
        } else if headers.contains("amount") || headers.contains("date") {
            ("transactions", 0.5)
        } else {
            ("other", 0.3)
        };

    let period = (1900..=2200)
        .rev()
        .find(|year| haystack.contains(&year.to_string()))
        .map(|year| year.to_string());

    Classification {
        institution,
        document_type: document_type.to_string(),
        confidence: institution_confidence.min(type_confidence),
        period,
    }
}

/// The encrypted document store and use cases.
pub struct DocumentService {
    repo: Arc<dyn DocumentRepository>,
    migrations: Arc<MigrationService>,
    dir: PathBuf,
    key: [u8; 32],
    max_bytes: usize,
    #[allow(dead_code)]
    key_version: String,
}

impl DocumentService {
    /// Build a service from an explicit directory and 32-byte key.
    pub fn new(
        repo: Arc<dyn DocumentRepository>,
        migrations: Arc<MigrationService>,
        dir: PathBuf,
        key: [u8; 32],
        max_bytes: usize,
    ) -> Self {
        Self {
            repo,
            migrations,
            dir,
            key,
            max_bytes,
            key_version: "1".to_string(),
        }
    }

    /// Build from `DOCUMENT_DIR` and `DOCUMENT_ENCRYPTION_KEY`.
    pub fn from_env(
        repo: Arc<dyn DocumentRepository>,
        migrations: Arc<MigrationService>,
    ) -> Result<Self, DocumentError> {
        let dir = PathBuf::from(
            std::env::var("DOCUMENT_DIR").unwrap_or_else(|_| "./documents".to_string()),
        );
        let encoded = std::env::var("DOCUMENT_ENCRYPTION_KEY").map_err(|_| {
            DocumentError::Validation(
                "DOCUMENT_ENCRYPTION_KEY is required for the encrypted document store".to_string(),
            )
        })?;
        let key = BASE64.decode(encoded.trim()).map_err(|error| {
            DocumentError::Validation(format!("invalid DOCUMENT_ENCRYPTION_KEY: {error}"))
        })?;
        let key: [u8; 32] = key.try_into().map_err(|_| {
            DocumentError::Validation(
                "DOCUMENT_ENCRYPTION_KEY must be 32 bytes (base64)".to_string(),
            )
        })?;
        let max_bytes = std::env::var("DOCUMENT_MAX_BYTES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(20 * 1024 * 1024);
        Ok(Self::new(repo, migrations, dir, key, max_bytes))
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new_from_slice(&self.key).expect("32-byte key")
    }

    fn path_for(&self, sha256: &str) -> PathBuf {
        self.dir.join(format!("{sha256}.enc"))
    }

    /// Upload plaintext bytes, deduplicating by SHA-256.
    pub async fn upload(
        &self,
        user_id: Option<Uuid>,
        original_name: &str,
        mime_type: &str,
        bytes: &[u8],
        tags: Vec<String>,
    ) -> Result<StoredDocument, DocumentError> {
        if bytes.is_empty() {
            return Err(DocumentError::Validation("the file is empty".to_string()));
        }
        if bytes.len() > self.max_bytes {
            return Err(DocumentError::Validation(format!(
                "file is larger than the {} byte limit",
                self.max_bytes
            )));
        }
        let sha256 = hex_digest(bytes);
        if let Some(mut existing) = self.repo.find_by_sha256(&sha256).await? {
            let mut merged = existing.tags.clone();
            for tag in tags {
                if !merged.iter().any(|current| current == &tag) {
                    merged.push(tag);
                }
            }
            if merged != existing.tags {
                existing = self
                    .repo
                    .update_tags(existing.id, &merged)
                    .await?
                    .unwrap_or(existing);
            }
            self.repo
                .record_audit(
                    existing.id,
                    user_id,
                    "upload",
                    &serde_json::json!({
                        "deduplicated": true,
                        "original_name": original_name,
                    })
                    .to_string(),
                )
                .await?;
            return Ok(existing);
        }

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let ciphertext = self
            .cipher()
            .encrypt(Nonce::from_slice(&nonce_bytes), bytes)
            .map_err(|_| DocumentError::Storage("document encryption failed".to_string()))?;
        fs::create_dir_all(&self.dir)
            .map_err(|error| DocumentError::Storage(format!("create document dir: {error}")))?;
        let path = self.path_for(&sha256);
        if !path.exists() {
            let temp = path.with_extension("tmp");
            {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .mode(0o600)
                    .open(&temp)
                    .map_err(|error| DocumentError::Storage(format!("write document: {error}")))?;
                use std::io::Write;
                file.write_all(&ciphertext)
                    .map_err(|error| DocumentError::Storage(format!("write document: {error}")))?;
            }
            fs::rename(&temp, &path)
                .map_err(|error| DocumentError::Storage(format!("finalize document: {error}")))?;
        }

        let format = detect_format(bytes, original_name);
        let parser = parser_for(format);
        let (table, parse_error) = match parser {
            Some(parser) => match parser.parse(bytes) {
                Ok(table) => (Some(table), None),
                Err(error) => (None, Some(error)),
            },
            None => (None, Some("unsupported document format".to_string())),
        };
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_string();
        let classification = classify(
            original_name,
            &text,
            table.as_ref().unwrap_or(&Table {
                headers: Vec::new(),
                rows: Vec::new(),
            }),
        );
        let status = if table.is_some() {
            "parsed"
        } else {
            "unparsed"
        };
        let nonce = BASE64.encode(nonce_bytes);
        let document = self
            .repo
            .insert(&NewDocument {
                id: Uuid::new_v4(),
                sha256,
                original_name: original_name.to_string(),
                mime_type: mime_type.to_string(),
                byte_size: bytes.len() as i64,
                uploader_user_id: user_id,
                tags,
                institution: Some(classification.institution.clone()),
                document_type: Some(classification.document_type.clone()),
                classification_confidence: Some(classification.confidence),
                parser_version: Some(PARSER_VERSION.to_string()),
                status: status.to_string(),
                period: classification.period,
                parse_error,
                nonce,
                key_version: self.key_version.clone(),
            })
            .await?;
        self.repo
            .record_audit(document.id, user_id, "upload", r#"{"deduplicated":false}"#)
            .await?;
        self.repo
            .record_audit(
                document.id,
                user_id,
                "classify",
                &serde_json::json!({
                    "institution": classification.institution,
                    "document_type": classification.document_type,
                    "confidence": classification.confidence,
                })
                .to_string(),
            )
            .await?;
        Ok(document)
    }

    /// List documents.
    pub async fn list(&self, include_deleted: bool) -> Result<Vec<StoredDocument>, DocumentError> {
        Ok(self.repo.list(include_deleted).await?)
    }

    /// Find one document.
    pub async fn get(&self, id: Uuid) -> Result<StoredDocument, DocumentError> {
        self.repo
            .find(id)
            .await?
            .ok_or_else(|| DocumentError::NotFound(format!("document not found: {id}")))
    }

    /// Decrypt and return the original bytes.
    pub async fn download(
        &self,
        id: Uuid,
        user_id: Option<Uuid>,
    ) -> Result<(StoredDocument, Vec<u8>), DocumentError> {
        let document = self.get(id).await?;
        let nonce = BASE64
            .decode(&document.nonce)
            .map_err(|error| DocumentError::Storage(format!("invalid nonce: {error}")))?;
        let ciphertext = fs::read(self.path_for(&document.sha256))
            .map_err(|error| DocumentError::Storage(format!("read document: {error}")))?;
        let plaintext = self
            .cipher()
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|_| DocumentError::Storage("document decryption failed".to_string()))?;
        self.repo
            .record_audit(id, user_id, "download", "{}")
            .await?;
        Ok((document, plaintext))
    }

    /// Replace tags.
    pub async fn set_tags(
        &self,
        id: Uuid,
        user_id: Option<Uuid>,
        tags: Vec<String>,
    ) -> Result<StoredDocument, DocumentError> {
        let document = self
            .repo
            .update_tags(id, &tags)
            .await?
            .ok_or_else(|| DocumentError::NotFound(format!("document not found: {id}")))?;
        self.repo
            .record_audit(id, user_id, "classify", r#"{"tags_updated":true}"#)
            .await?;
        Ok(document)
    }

    /// Confirm or override classification.
    pub async fn confirm_classification(
        &self,
        id: Uuid,
        user_id: Option<Uuid>,
        institution: Option<String>,
        document_type: Option<String>,
        confidence: Option<f64>,
        period: Option<String>,
    ) -> Result<StoredDocument, DocumentError> {
        let document = self
            .repo
            .update_classification(
                id,
                institution.as_deref(),
                document_type.as_deref(),
                confidence,
                "confirmed",
                Some(PARSER_VERSION),
                period.as_deref(),
                None,
            )
            .await?
            .ok_or_else(|| DocumentError::NotFound(format!("document not found: {id}")))?;
        self.repo
            .record_audit(
                id,
                user_id,
                "classify",
                &serde_json::json!({
                    "institution": institution,
                    "document_type": document_type,
                    "confirmed": true,
                })
                .to_string(),
            )
            .await?;
        Ok(document)
    }

    /// Soft-delete metadata; the encrypted file remains for cleanup/audit.
    pub async fn soft_delete(&self, id: Uuid, user_id: Option<Uuid>) -> Result<(), DocumentError> {
        if !self.repo.soft_delete(id).await? {
            return Err(DocumentError::NotFound(format!("document not found: {id}")));
        }
        self.repo.record_audit(id, user_id, "delete", "{}").await?;
        Ok(())
    }

    /// Parse and classify the document for preview.
    pub async fn preview(&self, id: Uuid) -> Result<DocumentPreview, DocumentError> {
        let (document, bytes) = self.download(id, None).await?;
        let format = detect_format(&bytes, &document.original_name);
        let parser = parser_for(format);
        let table = parser.as_ref().map(|parser| parser.parse(&bytes));
        let (table, error) = match table {
            Some(Ok(table)) => (Some(table), None),
            Some(Err(error)) => (None, Some(error)),
            None => (None, Some("unsupported document format".to_string())),
        };
        let audit = self.repo.list_audit(id).await?;
        let classification = table.as_ref().map(|table| {
            classify(
                &document.original_name,
                &String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]),
                table,
            )
        });
        Ok(DocumentPreview {
            format: format.as_str().to_string(),
            institution: classification
                .as_ref()
                .map(|value| value.institution.clone())
                .or(document.institution.clone()),
            document_type: classification
                .as_ref()
                .map(|value| value.document_type.clone())
                .or(document.document_type.clone()),
            confidence: classification
                .as_ref()
                .map(|value| value.confidence)
                .or(document.classification_confidence),
            table,
            error,
            audit,
            document,
        })
    }

    /// List audit events for a document.
    pub async fn audit_events(&self, id: Uuid) -> Result<Vec<DocumentAuditEvent>, DocumentError> {
        Ok(self.repo.list_audit(id).await?)
    }

    /// Import the document through the guided migration pipeline.
    pub async fn import(
        &self,
        id: Uuid,
        user_id: Option<Uuid>,
        mapping: crate::migration::MigrationMapping,
        mode: MigrationMode,
        backup_confirmed: bool,
    ) -> Result<crate::migration::MigrationReport, DocumentError> {
        let (document, bytes) = self.download(id, user_id).await?;
        let format = detect_format(&bytes, &document.original_name);
        let parser = parser_for(format)
            .ok_or_else(|| DocumentError::Validation("unsupported document format".to_string()))?;
        let table = parser.parse(&bytes).map_err(DocumentError::Validation)?;
        let mut writer = csv::Writer::from_writer(Vec::new());
        writer
            .write_record(&table.headers)
            .map_err(|error| DocumentError::Validation(error.to_string()))?;
        for row in &table.rows {
            writer
                .write_record(row)
                .map_err(|error| DocumentError::Validation(error.to_string()))?;
        }
        let content = String::from_utf8(
            writer
                .into_inner()
                .map_err(|error| DocumentError::Validation(error.to_string()))?,
        )
        .map_err(|error| DocumentError::Validation(error.to_string()))?;
        let report = self
            .migrations
            .apply(&MigrationApplyRequest {
                request: crate::migration::MigrationRequest {
                    source: MigrationSourceKind::GenericCsv,
                    mapping,
                    content,
                },
                mode,
                backup_confirmed,
            })
            .await?;
        self.repo
            .record_provenance(id, report.migration_id, "migration", report.migration_id)
            .await?;
        self.repo
            .record_audit(
                id,
                user_id,
                "import",
                &serde_json::json!({
                    "migration_id": report.migration_id,
                    "accounts_created": report.accounts_created,
                    "balances_saved": report.balances_saved,
                })
                .to_string(),
            )
            .await?;
        Ok(report)
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Parse a balance amount from a document table for future adapters.
pub fn parse_money(value: &str, currency: &Currency) -> Result<Money, String> {
    Money::parse(value, currency.as_str()).map_err(|error| error.to_string())
}

/// Parse a quantity from a document table for future adapters.
pub fn parse_quantity(value: &str) -> Result<Quantity, String> {
    Quantity::parse(value).map_err(|error| error.to_string())
}

/// Parse a document date for future adapters.
pub fn parse_date(value: &str) -> Result<Date, String> {
    Date::parse(value, &Iso8601::DATE).map_err(|error| error.to_string())
}

/// Build a balance for a document adapter.
pub fn document_balance(
    account_id: Uuid,
    date: Date,
    amount: &str,
    currency: &str,
) -> Result<NewBalance, String> {
    let currency = Currency::new(currency).map_err(|error| error.to_string())?;
    let amount = parse_money(amount, &currency)?;
    Ok(NewBalance {
        account_id,
        as_of: date,
        amount,
        source: networth_domain::BalanceSource::Import,
    })
}

#[allow(dead_code)]
fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

#[allow(dead_code)]
fn path_exists(path: &Path) -> bool {
    path.exists()
}
