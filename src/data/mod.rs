pub mod table;

#[derive(Clone)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
}

pub trait Points{
    fn label_col(&self) -> i32;
    fn value_col(&self) -> i32;
}

pub fn extract_points(table: &table::Table, config: &impl Points) -> Vec<DataPoint> {
    let label_col = config.label_col() as usize;
    let value_col = config.value_col() as usize;

    table
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, _row)| DataPoint {
            label: table::cell_as_string(table, row_index, label_col),
            value: table::cell_as_f64(table, row_index, value_col),
        })
        .collect()
}