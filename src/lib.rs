// wasm-pack build --target web
mod canvas;
mod data;
mod charts;
mod animation;

use wasm_bindgen::prelude::*;
use charts::pie;

use std::cell::RefCell;
thread_local! {
    static CURRENT_CHART: RefCell<Option<pie::PieChartHandle>> = RefCell::new(None);
}

#[wasm_bindgen]
pub fn render_pie_chart(canvas_id: &str, table_json: JsValue) -> Result<(), JsValue> {
    CURRENT_CHART.with(|c| {
        if let Some(old) = c.borrow_mut().take() {
            old.stop(); // kill the previous chart's loop + listeners
        }
    });

    let table = data::table::parse(table_json)?;
    let points = data::extract_points(&table, &pie::PieConfig::default());
    let handle = pie::render_interactive_pie(canvas_id, points)?;

    CURRENT_CHART.with(|c| *c.borrow_mut() = Some(handle));
    Ok(())
}