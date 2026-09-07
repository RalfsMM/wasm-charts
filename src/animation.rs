use wasm_bindgen::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

/// Starts a requestAnimationFrame loop that calls `tick` every frame
/// with elapsed milliseconds since the loop started. Runs indefinitely.
pub fn start_loop(mut tick: impl FnMut(f64) + 'static) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let performance = window.performance().ok_or_else(|| JsValue::from_str("no performance"))?;
    let start_time = performance.now();

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        let now = web_sys::window().unwrap().performance().unwrap().now();
        tick(now - start_time);

        web_sys::window()
            .unwrap()
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }));

    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}