// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use wasm_bindgen::prelude::*;
use charts::pie;

use std::cell::RefCell;

#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue) -> Result<(), JsValue> {
    pie::stop_all_drilldowns();

    let table = data::table::parse(table_json)?;
    let points = data::extract_points(&table, &pie::PieConfig::default());
    let handle = pie::render_interactive_pie(canvas_id, points)?;

    pie::push_chart(handle);
    Ok(())
}
