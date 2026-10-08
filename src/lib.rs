// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use wasm_bindgen::prelude::*;
use charts::pie;
use charts::bar;


#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {
    pie::stop_all_drilldowns();

    let table = data::table::parse(table_json)?;
    let config;
    if config_json.is_none(){
        config = pie::PieConfig::default();
    }else{
        config= pie::PieConfig::from_json(config_json.unwrap())?;
    }
    let points = data::extract_points(&table, &config);
    let handle = pie::render_interactive_pie(canvas_id, points, String::from("piechart"), 100.0, &config)?;

    pie::push_chart(handle);
    Ok(())
}

#[wasm_bindgen]
pub fn render_bar_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {

    let table = data::table::parse(table_json)?;
    let config;
    if config_json.is_none(){
        config = bar::BarConfig::default();
    }else{
        config= bar::BarConfig::from_json(config_json.unwrap())?;
    }
    for i in &config.value_column{
        if *i>=table.columns.len(){
            return Err(JsValue::from_str(&format!("value column index {} out of bounds for table with {} columns", i, table.columns.len())));
        }
    }

    let key_label=table.columns[config.label_column].clone();

    //japamaina velak
    let mut value_labels: Vec<String>=vec![];
    for i in &config.value_column {
        value_labels.push(table.columns[*i].clone());
    };

    let points = data::extract_points(&table, &config);
    let handle = bar::render_bar_chart(canvas_id, points, String::from("barchart"), &config, value_labels, key_label)?;

    bar::push_chart(handle);
    Ok(())
}
