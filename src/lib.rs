// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use wasm_bindgen::prelude::*;
use charts::pie;
use charts::bar;
use charts::time;


#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {
    pie::destroy_chart(canvas_id);
    let table = data::table::parse(table_json)?;
    let config=
    if let Some(conf)=config_json{
        pie::PieConfig::from_json(conf)?
    }else{
        pie::PieConfig::default()
    };
    let points = data::extract_points(&table, &config);
    let handle = pie::render_pie_chart(canvas_id, points, String::from("piechart"), config)?;
    pie::push_chart(canvas_id, handle);

    Ok(())
}

#[wasm_bindgen]
pub fn render_bar_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {
    bar::destroy_chart(canvas_id);
    let table = data::table::parse(table_json)?;
    let config=
    if let Some(conf)=config_json{
        bar::BarConfig::from_json(conf)?
    }else{
        bar::BarConfig::default()
    };

    for i in &config.value_column{
        if *i>=table.columns.len(){
            return Err(JsValue::from_str(&format!("value column index {} out of bounds for table with {} columns", i, table.columns.len())));
        }
    }

    let key_label=table.columns[config.label_column].clone();

    let mut value_labels: Vec<String>=vec![];
    for i in &config.value_column {
        value_labels.push(table.columns[*i].clone());
    };

    let points = data::extract_points(&table, &config);
    let barchart =bar::render_bar_chart(canvas_id, points, String::from("barchart"), config, value_labels, key_label)?;
    bar::push_chart(canvas_id, barchart);

    Ok(())
}

#[wasm_bindgen]
pub fn render_time_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {
    time::destroy_chart(canvas_id);
    let table = data::table::parse(table_json)?;
    let config=
    if let Some(conf)=config_json{
        time::TimeConfig::from_json(conf)?
    }else{
        time::TimeConfig::default()
    };

    if config.label_column >= table.columns.len() {
        return Err(JsValue::from_str(&format!("label column index {} out of bounds for table with {} columns", config.label_column, table.columns.len())));
    }
    for i in &config.value_column{
        if *i>=table.columns.len(){
            return Err(JsValue::from_str(&format!("value column index {} out of bounds for table with {} columns", i, table.columns.len())));
        }
    }

    let key_label=table.columns[config.label_column].clone();

    let mut value_labels: Vec<String>=vec![];
    for i in &config.value_column {
        value_labels.push(table.columns[*i].clone());
    };

    let points = data::extract_points(&table, &config);
    let timechart =time::render_time_chart(canvas_id, points, String::from("timechart"), config, value_labels, key_label)?;
    time::push_chart(canvas_id, timechart);

    Ok(())
}