// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use wasm_bindgen::prelude::*;
use charts::pie;


#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue) -> Result<(), JsValue> {
    pie::stop_all_drilldowns();

    let table = data::table::parse(table_json)?;
    let config = pie::PieConfig::default();
    //let config = pie::PieConfig{ label_column: 0, value_column: 1, cx: 400.0, cy: 400.0, outer_r: 200.0, inner_r_diff: 40.0, default_font_size: 20.0, default_font: "20px sans-serif", middle_font_size: 108.0, middle_font: "108px sans-serif", text_color: "black", hover_title_color: "blue" };
    let points = data::extract_points(&table, &config);
    let handle = pie::render_interactive_pie(canvas_id, points, String::from("piechart"), 100.0, config)?;

    pie::push_chart(handle);
    Ok(())
}
