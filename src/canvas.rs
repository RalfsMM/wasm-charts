use wasm_bindgen::{JsCast, JsValue};

pub fn get_canvas(canvas_id: &str) -> Result<web_sys::HtmlCanvasElement, JsValue> {
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str("canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("not a canvas element"))
}

pub fn get_context(canvas: &web_sys::HtmlCanvasElement) -> Result<web_sys::CanvasRenderingContext2d, JsValue> {
    canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("failed to get 2D context"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| JsValue::from_str("failed to cast to CanvasRenderingContext2d"))
}