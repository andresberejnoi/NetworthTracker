//! Parser and classification adapter tests.

use networth_application::{classify_document, parse_document, DocumentFormat, Table};
use rust_xlsxwriter::Workbook;

#[test]
fn detects_by_content_not_extension() {
    let bytes = b"record_type,account_name\naccount,Checking\n";
    let (format, table) = parse_document(bytes, "misleading.xlsx");
    assert_eq!(format, DocumentFormat::Csv);
    assert_eq!(table.unwrap().headers[0], "record_type");

    let (format, _) = parse_document(b"%PDF-1.4\n", "statement.csv");
    assert_eq!(format, DocumentFormat::Pdf);
}

#[test]
fn parses_xlsx_csv_xml_ofx_and_pdf_adapters() {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.write_string(0, 0, "record_type").unwrap();
    sheet.write_string(0, 1, "account_name").unwrap();
    sheet.write_string(1, 0, "account").unwrap();
    sheet.write_string(1, 1, "Checking").unwrap();
    let xlsx = workbook.save_to_buffer().unwrap();
    let (format, table) = parse_document(&xlsx, "statement.xlsx");
    assert_eq!(format, DocumentFormat::Xlsx);
    assert_eq!(table.unwrap().rows[0][1], "Checking");

    let (format, table) = parse_document(
        b"<root><account_name>Checking</account_name><amount>10.00</amount></root>",
        "generic.xml",
    );
    assert_eq!(format, DocumentFormat::Xml);
    let table = table.unwrap();
    assert!(table.headers.contains(&"account_name".to_string()));
    assert!(table.rows[0].contains(&"10.00".to_string()));

    let ofx = b"<OFX><BANKMSGSRSV1><STMTTRNRS><STMTRS><BANKTRANLIST><STMTTRN>\
<TRNAMT>10.00</TRNAMT><DTPOSTED>20260101</DTPOSTED></STMTTRN>\
</BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1></OFX>";
    let (format, table) = parse_document(ofx, "activity.qfx");
    assert_eq!(format, DocumentFormat::Ofx);
    let table = table.unwrap();
    assert!(table.headers.contains(&"trnamt".to_string()));

    let pdf = b"%PDF-1.4\n1 0 obj\n<<>>\nstream\n(Fidelity statement total 10.00) Tj\nendstream\nendobj\n%%EOF";
    let (format, table) = parse_document(pdf, "fidelity.pdf");
    assert_eq!(format, DocumentFormat::Pdf);
    assert!(table.unwrap().rows[0][0].contains("Fidelity"));
}

#[test]
fn classifies_known_institutions_and_types() {
    let table = Table {
        headers: vec!["amount".to_string(), "date".to_string()],
        rows: vec![vec!["10.00".to_string(), "2026-01-01".to_string()]],
    };
    let classification =
        classify_document("fidelity-holdings.csv", "Fidelity holdings 2026", &table);
    assert_eq!(classification.institution, "Fidelity");
    assert_eq!(classification.document_type, "holdings");
    assert!(classification.confidence >= 0.8);

    let classification = classify_document("coinbase.csv", "Coinbase activity", &table);
    assert_eq!(classification.institution, "Coinbase");
    assert_eq!(classification.document_type, "transactions");

    let classification = classify_document("mystery.bin", "nothing", &table);
    assert_eq!(classification.institution, "Generic");
    assert!(classification.confidence < 0.5);
}
