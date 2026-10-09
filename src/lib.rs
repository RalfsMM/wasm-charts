// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use serde::de::value;
use wasm_bindgen::prelude::*;
use charts::pie;
use charts::bar;
use std::rc::Rc;


#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {
    pie::stop_all_drilldowns();

    let table = data::table::parse(table_json)?;
    let config=
    if let Some(conf)=config_json{
        pie::PieConfig::from_json(conf)?
    }else{
        pie::PieConfig::default()
    };
    let points = data::extract_points(&table, &config);
    let handle = pie::render_interactive_pie(canvas_id, points, String::from("piechart"), 100.0, &config)?;

    pie::push_chart(handle);
    Ok(())
}

#[wasm_bindgen]
pub fn render_bar_chart(canvas_id: &str, table_json: JsValue, config_json: Option<JsValue>) -> Result<(), JsValue> {

    let table = data::table::parse(table_json)?;
    let config=Rc::new(
    if let Some(conf)=config_json{
        bar::BarConfig::from_json(conf)?
    }else{
        bar::BarConfig::default()
    });

    for i in &config.value_column{
        if *i>=table.columns.len(){
            return Err(JsValue::from_str(&format!("value column index {} out of bounds for table with {} columns", i, table.columns.len())));
        }
    }

    let key_label=Rc::new(table.columns[config.label_column].clone());

    //japamaina velak
    let mut value_labels: Vec<String>=vec![];
    for i in &config.value_column {
        value_labels.push(table.columns[*i].clone());
    };
    let value_labels=Rc::new(value_labels);

    let points = data::extract_points(&table, &(*config).clone());
    let handle = bar::render_bar_chart(canvas_id, points, String::from("barchart"), config, value_labels, key_label)?;

    bar::push_chart(handle);
    Ok(())
}
