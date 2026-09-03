use wasm_bindgen::JsValue;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

pub fn parse(table_json: JsValue) -> Result<Table, JsValue> {
    serde_wasm_bindgen::from_value(table_json)
        .map_err(|e| JsValue::from_str(&format!("failed to parse table: {e}")))
}

/// Helper: read a single cell as a string, regardless of its underlying
/// JSON type (string, number, bool, etc). Falls back to an empty string
/// if the cell is missing or null, rather than panicking.
pub fn cell_as_string(table: &Table, row: usize, col: usize) -> String {
    table
        .rows
        .get(row)
        .and_then(|r| r.get(col))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

/// Helper: read a single cell as an f64, defaulting to 0.0 if it's
/// missing or not numeric. Used by chart modules pulling out the
/// "value" column.
pub fn cell_as_f64(table: &Table, row: usize, col: usize) -> f64 {
    table
        .rows
        .get(row)
        .and_then(|r| r.get(col))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_as_f64_reads_numeric_value() {
        let table = Table {
            columns: vec!["category".into(), "value".into()],
            rows: vec![vec![Value::String("A".into()), Value::from(42.0)]],
        };
        assert_eq!(cell_as_f64(&table, 0, 1), 42.0);
    }

    #[test]
    fn cell_as_f64_defaults_on_missing_row() {
        let table = Table { columns: vec![], rows: vec![] };
        assert_eq!(cell_as_f64(&table, 5, 0), 0.0);
    }
}