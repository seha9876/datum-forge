//! CSV escaping and source-file reading helpers.

use super::*;
use std::collections::HashMap;

pub(super) fn csv_escape_field(value: &str) -> String {
    // CSVの特殊文字を含む値だけダブルクォートで包み、内部のクォートは2つにします。
    if value.contains(|ch| matches!(ch, ',' | '"' | '\r' | '\n')) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone)]
pub(super) struct CsvSourceRow {
    pub(super) row_number: usize,
    pub(super) values: HashMap<String, String>,
}

pub(super) fn read_csv_import_source(
    input_path: &str,
) -> Result<(Vec<String>, Vec<CsvSourceRow>), DbError> {
    let mut reader = ::csv::ReaderBuilder::new().from_path(input_path)?;
    let headers = reader
        .headers()?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for (row_index, record) in reader.records().enumerate() {
        let record = record?;
        let values = headers
            .iter()
            .enumerate()
            .map(|(index, header)| {
                (
                    header.clone(),
                    record.get(index).unwrap_or_default().to_string(),
                )
            })
            .collect::<HashMap<_, _>>();
        rows.push(CsvSourceRow {
            row_number: row_index + 2,
            values,
        });
    }
    Ok((headers, rows))
}
