//wasm-pack build --target web
mod canvas;
mod data;
mod charts;

use wasm_bindgen::prelude::*;
use charts::pie;

#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue) -> Result<(), JsValue> {
    let table = data::table::parse(table_json)?;
    let points = data::extract_points(&table, &pie::PieConfig::default());
    let slices = match pie::compute_pie_slices(points) {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    pie::draw_pie(canvas_id, &slices)
}