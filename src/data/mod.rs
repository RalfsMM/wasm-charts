pub mod table;

#[derive(Clone)]
pub struct DataPoint {
    pub label: String,
    pub value: f64,
    pub sub_points: Option<Vec<DataPoint>>
}

pub trait Points{
    fn label_col(&self) -> usize;
    fn value_col(&self) -> Vec<usize>;
}

pub fn extract_points(table: &table::Table, config: &impl Points) -> Vec<DataPoint> {
    let label_col = config.label_col();
    let value_col = config.value_col();

    table
        .rows
        .iter()
        .enumerate()
        .map(|(row_index, _row)|DataPoint { 
            label: table::cell_as_string(table, row_index, label_col),
            value: table::value(table, row_index, value_col.clone()),
            sub_points: if value_col.len() > 1 {
                Some(value_col.iter().map(|&col_index| DataPoint {
                    label: table.columns[col_index].clone(),
                    value: table::cell_as_f64(table, row_index, col_index),
                    sub_points: None
                }).collect())
            } else {
                None
            }
        } 
        )
        .collect()
}