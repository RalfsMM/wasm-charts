// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use wasm_bindgen::prelude::*;
use charts::pie;
use charts::bar;


#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue) -> Result<(), JsValue> {
    pie::stop_all_drilldowns();

    let table = data::table::parse(table_json)?;
    let config = pie::PieConfig::default();
    //let config = pie::PieConfig{ label_column: 0, value_column: vec![1], cx: 400.0, cy: 400.0, outer_r: 200.0, inner_r_diff: 40.0, default_font_size: 20.0, default_font: "20px sans-serif", middle_font_size: 108.0, middle_font: "108px sans-serif", text_color: "black", hover_title_color: "blue" };
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
