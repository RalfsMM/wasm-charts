use wasm_bindgen::JsValue;
use serde::Deserialize;
use serde_json::Value;
use chrono::{DateTime, NaiveDate, NaiveDateTime};

#[derive(Deserialize, Debug)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

pub fn parse(table_json: JsValue) -> Result<Table, JsValue> {
    serde_wasm_bindgen::from_value(table_json)
        .map_err(|e| JsValue::from_str(&format!("failed to parse table: {e}")))
}

pub fn value(table: &Table, row: usize, col: Vec<usize>)->f64{
    let mut res=0.0;
    for i in col{
        res+=cell_as_f64(table, row, i);
    }
    res
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


/// Helper: read a single cell as a date/time. Accepts strings in common
/// formats (RFC 3339, ISO 8601, `YYYY-MM-DD`, `DD.MM.YYYY`, ...) and numeric
/// Unix timestamps (seconds or milliseconds). Returns `None` if the cell is
/// missing, null, or can't be interpreted as a date, rather than panicking.
pub fn cell_as_date(table: &Table, row: usize, col: usize) -> Option<NaiveDateTime> {
    match table.rows.get(row)?.get(col)? {
        Value::String(s) => parse_date_str(s.trim()),
        Value::Number(n) => n.as_f64().and_then(from_timestamp),
        _ => None,
    }
}

/// Treat large values as milliseconds, smaller ones as seconds.
/// 1e11 seconds is ~year 5138, so anything above it is almost certainly ms.
fn from_timestamp(ts: f64) -> Option<NaiveDateTime> {
    if !ts.is_finite() {
        return None;
    }
    let millis = if ts.abs() >= 1e11 { ts } else { ts * 1000.0 };
    DateTime::from_timestamp_millis(millis as i64).map(|dt| dt.naive_utc())
}

fn parse_date_str(s: &str) -> Option<NaiveDateTime> {
    if s.is_empty() {
        return None;
    }

    // Full RFC 3339 with offset, e.g. "2024-03-15T10:30:00+02:00" (normalized to UTC)
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.naive_utc());
    }

    // Date + time without offset. `%.f` also accepts a missing fraction.
    const DATETIME_FORMATS: &[&str] = &[
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
        "%d.%m.%Y %H:%M:%S",
        "%d.%m.%Y %H:%M",
    ];
    for fmt in DATETIME_FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(dt);
        }
    }

    // Date only -> midnight
    const DATE_FORMATS: &[&str] = &["%Y-%m-%d", "%Y/%m/%d", "%d.%m.%Y"];
    for fmt in DATE_FORMATS {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return d.and_hms_opt(0, 0, 0);
        }
    }

    // Numeric timestamp stored as a string, e.g. "1710498600"
    s.parse::<f64>().ok().and_then(from_timestamp)
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
