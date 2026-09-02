use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;
use std::f64::consts::PI;

#[wasm_bindgen]
pub fn animate_rectangle(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no global window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("canvas not found")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "element is not a canvas")?;
    let context = canvas
        .get_context("2d")?
        .ok_or("no 2d context")?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| "failed to cast to CanvasRenderingContext2d")?;

    let canvas_w = canvas.width() as f64;
    let canvas_h = canvas.height() as f64;

    // This is the self-referencing closure pattern: the callback needs to be able
    // to schedule itself again next frame, but a Rust closure can't normally refer
    // to itself while it's being defined. Rc<RefCell<Option<Closure>>> works around
    // that — we create an empty slot, define the closure with a clone of the Rc
    // pointing to that slot, then fill the slot in after the closure exists.
let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
let g = f.clone();

let start_time = window.performance().ok_or("no performance")?.now();

*g.borrow_mut() = Some(Closure::new(move || {
    let now = web_sys::window().unwrap().performance().unwrap().now();
    let elapsed_ms = now - start_time;

    let cycle_ms = 2000.0;
    let progress = (elapsed_ms % cycle_ms) / cycle_ms;
    let scale = 0.5 + 0.5 * ((progress * 2.0 * PI).sin() * 0.5 + 0.5);

    context.clear_rect(0.0, 0.0, canvas_w, canvas_h);

    let base_w = 100.0;
    let base_h = 80.0;
    let w = base_w * scale;
    let h = base_h * scale;

    let cx = canvas_w / 2.0;
    let cy = canvas_h / 2.0;
    let x = cx - w / 2.0;
    let y = cy - h / 2.0;

    context.set_fill_style_str("red");
    context.fill_rect(x, y, w, h);

    web_sys::window()
        .unwrap()
        .request_animation_frame(
            f.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
        )
        .unwrap();
}));

window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;

    // Kick off the first frame.
    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;

    Ok(())
}