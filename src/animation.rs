use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

pub fn start_loop(mut tick: impl FnMut(f64) -> bool + 'static) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let performance = window.performance().ok_or_else(|| JsValue::from_str("no performance"))?;
    let start_time = performance.now();

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        let now = web_sys::window().unwrap().performance().unwrap().now();
        let should_continue = tick(now - start_time);
        if should_continue {
            web_sys::window()
                .unwrap()
                .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .unwrap();
        }
        // if false: don't reschedule, loop just ends here
    }));

    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}