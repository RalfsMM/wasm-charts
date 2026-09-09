use crate::data::{DataPoint, Points};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;

// ---------- Config ----------

pub struct PieConfig {
    pub label_column: usize,
    pub value_column: usize,
}

impl Points for PieConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> usize {
        self.value_column
    }
}

impl Default for PieConfig {
    fn default() -> Self {
        PieConfig { label_column: 0, value_column: 1 }
    }
}

// ---------- Geometry ----------

#[derive(Clone, Copy)]
pub struct PieGeometry {
    pub cx: f64,
    pub cy: f64,
    pub outer_r: f64,
    pub inner_r: f64,
}
impl Default for PieGeometry {
    fn default() -> Self {
        PieGeometry { cx: 200.0, cy: 200.0, outer_r: 100.0, inner_r: 80.0 }
    }
}

// ---------- Data ----------

pub struct PieSlice {
    pub label: String,
    pub angle_start: f64,
    pub angle_end: f64,
    pub color: String,
    pub percent: f64,
    pub sub_points: Option<Vec<DataPoint>>, // optional sub-points for drill-down
}

fn palette_color(index: usize) -> String {
    let colors = [
        "#FF6384", "#36A2EB", "#FFCE56", "#4BC0C0", "#9966FF",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()].to_string()
}

pub struct PieChartHandle {
    canvas: web_sys::HtmlCanvasElement,
    running: Rc<RefCell<bool>>,
    mouse_closure: Closure<dyn FnMut(web_sys::MouseEvent)>,
    click_closure: Closure<dyn FnMut(web_sys::MouseEvent)>,
}

impl PieChartHandle {
    pub fn stop(&self) {
        *self.running.borrow_mut() = false;
        let _ = self.canvas.remove_event_listener_with_callback(
            "mousemove", self.mouse_closure.as_ref().unchecked_ref(),
        );
        let _ = self.canvas.remove_event_listener_with_callback(
            "click", self.click_closure.as_ref().unchecked_ref(),
        );
    }
}

pub fn compute_pie_slices(mut points: Vec<DataPoint>) -> Result<Vec<PieSlice>, JsValue> {
    if points.iter().any(|p| p.value < 0.0) {
        return Err(JsValue::from_str("negative values are not allowed in pie chart"));
    }

    points.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));

    let total: f64 = points.iter().map(|p| p.value).sum();
    let point_count = points.len() as f64;
    let mut angle = -std::f64::consts::PI / 2.0;
    let mut others: PieSlice = PieSlice { label: "Others".to_string(), angle_start: 0.0, angle_end: std::f64::consts::PI * 3.0 / 2.0, color: "#808080".to_string(), percent: 0.0, sub_points: None };

    let mut slices: Vec<PieSlice> = points
        .into_iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let sweep;
            let percent;
            if total > 0.0 {
                if p.value == 0.0 {
                    return None; // skip zero-value slices
                }
                sweep = (p.value / total) * std::f64::consts::TAU;
                percent = (p.value / total) * 100.0;
            }else{
                sweep = (1.0 / point_count) * std::f64::consts::TAU;
                percent = 0.0;
            }

            let start = angle;
            angle += sweep;

            if percent > 0.0 && percent < 3.0 && others.percent + percent < 10.0 {
                if others.percent == 0.0 {
                    others.angle_start = start;
                }
                others.percent += percent;
                if others.sub_points.is_none() {
                    others.sub_points = Some(vec![]);
                }
                others.sub_points.as_mut().unwrap().push(p);
                return None;
            }
            Some(PieSlice { label: p.label, angle_start: start, angle_end: angle, color: palette_color(i), percent, sub_points: None })
        })
        .collect();
    if others.percent > 0.0 {
        slices.push(others);
    }
    Ok(slices)
}



// ---------- Drawing (pure function of state) ----------

pub fn draw_pie( canvas: &web_sys::HtmlCanvasElement, slices: &[PieSlice], geo: &PieGeometry, radii: &[f64], angles: Vec<&[f64]>) -> Result<(), JsValue> {
    let context = crate::canvas::get_context(canvas)?;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    for (i, slice) in slices.iter().enumerate() {
        let outer_r = radii[i];
        let inner_r = outer_r -20.0;
        let start_angle;
        let end_angle;

        if angles.len()>0{
            start_angle = angles[i][0];
            end_angle = angles[i][1];
        }else{
            start_angle = slice.angle_start;
            end_angle = slice.angle_end;
        }

        context.begin_path();
        context.move_to(geo.cx, geo.cy);
        context
            .arc(geo.cx, geo.cy, outer_r, start_angle + 0.02, end_angle - 0.02)
            .map_err(|_| JsValue::from_str("failed to draw arc"))?;
        context
            .arc_with_anticlockwise(geo.cx, geo.cy, inner_r, end_angle - 0.02, start_angle + 0.02, true)
            .map_err(|_| JsValue::from_str("failed to draw arc"))?;
        context.close_path();
        context.set_fill_style_str(&slice.color);
        context.fill();

        context.set_fill_style_str("black");
        let mid_angle = (start_angle + end_angle) / 2.0;
        let label_x;
        if mid_angle < std::f64::consts::PI / 2.0 || mid_angle > std::f64::consts::PI * 3.0 / 2.0 {
            context.set_text_align("left");
            label_x = outer_r + geo.cx + 20.0;
        } else {
            context.set_text_align("right");
            label_x = geo.cx - outer_r - 20.0;
        }
        let label_y = geo.cy + outer_r * mid_angle.sin();
        context
            .fill_text(&slice.label, label_x, label_y)
            .map_err(|_| JsValue::from_str("failed to draw text"))?;
    }
    Ok(())
}

// ---------- Hit-testing ----------

fn hit_test(slices: &[PieSlice], geo: &PieGeometry, mx: f64, my: f64) -> Option<usize> {
    let dx = mx - geo.cx;
    let dy = my - geo.cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < geo.inner_r || dist > geo.outer_r + 10.0 {
        return None;
    }

    let mut angle = dy.atan2(dx);
    if angle < -std::f64::consts::PI / 2.0 {
        angle += std::f64::consts::TAU; // wrap into the same -PI/2..3*PI/2 frame the slices use
    }

    slices.iter().position(|s| angle >= s.angle_start && angle < s.angle_end)
}

// ---------- Wiring: mouse events + animation loop ----------

pub fn render_interactive_pie(canvas_id: &str, points: Vec<DataPoint>) -> Result<PieChartHandle, JsValue> {
    let slices = compute_pie_slices(points)?;
    let canvas = crate::canvas::get_canvas(canvas_id)?;
    let geo = PieGeometry::default();
    let running = Rc::new(RefCell::new(true));

    let now_ms = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .ok_or_else(|| JsValue::from_str("no performance"))?;

    let slices = Rc::new(slices);
    let hover: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let anim_state: Rc<RefCell<Vec<SliceAnimRadius>>> = Rc::new(RefCell::new(
        slices
            .iter()
            .map(|_| SliceAnimRadius { start_r: geo.outer_r, target_r: geo.outer_r, start_time: now_ms })
            .collect(),
    ));

    // --- initial draw, nothing hovered yet ---
    let initial_radii: Vec<f64> = anim_state.borrow().iter().map(|a| a.target_r).collect();
    new_pie_animation(&canvas, &slices, &geo, &initial_radii)?;

    // --- mousemove: updates hover + sets new animation targets ---
    let canvas_for_mouse = canvas.clone();
    let slices_for_mouse = slices.clone();
    let hover_for_mouse = hover.clone();
    let anim_for_mouse = anim_state.clone();
    let geo_for_mouse = geo;

    let mouse_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_mouse.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();

        let hit = hit_test(&slices_for_mouse, &geo_for_mouse, mx, my);
        let mut hover_ref = hover_for_mouse.borrow_mut();

        if *hover_ref != hit {
            *hover_ref = hit;

            let now = web_sys::window().unwrap().performance().unwrap().now();
            let mut anims = anim_for_mouse.borrow_mut();
            for (i, anim) in anims.iter_mut().enumerate() {
                let new_target = if hit == Some(i) { geo_for_mouse.outer_r + 10.0 } else { geo_for_mouse.outer_r };

                let changed = anim.target_r != new_target;

                if changed {
                    let cur = current_radius(anim, now);
                    anim.start_r = cur;
                    anim.target_r = new_target;
                    anim.start_time = now; 
                }
            }
        }
    });

    canvas.add_event_listener_with_callback("mousemove", mouse_closure.as_ref().unchecked_ref())?;

    // --- click listener

    let canvas_for_click = canvas.clone();
    let slices_for_click = slices.clone();
    let geo_for_click = geo;
    let running_for_click = running.clone();
    
    let click_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_click.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();

        let hit = hit_test(&slices_for_click, &geo_for_click, mx, my);
        if hit.is_some() {
            for ( i, slice) in slices_for_click.iter().enumerate() {
                if Some(i) == hit {
                    if slice.sub_points.is_some() {
                        let _ = render_interactive_pie(canvas_for_click.id().as_str(), slice.sub_points.clone().unwrap());
                        break;
                    }
                }
            }
        }
    });

    canvas.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())?;

    // --- animation loop: redraws every frame using current eased radii ---
    let canvas_for_loop = canvas.clone();
    let slices_for_loop = slices.clone();
    let anim_for_loop = anim_state.clone();
    let running_for_loop = running.clone();

    crate::animation::start_loop(move |_elapsed_ms| {
        if !*running_for_loop.borrow() {
            return false;
        }
        let now = web_sys::window().unwrap().performance().unwrap().now();
        let radii: Vec<f64> = anim_for_loop.borrow().iter().map(|a| current_radius(a, now)).collect();
        let _ = draw_pie(&canvas_for_loop, &slices_for_loop, &geo, &radii, vec![]);
        true
    })?;

    Ok(PieChartHandle { canvas, running, mouse_closure, click_closure })
}

// ---------- Per-slice animation state (elapsed-time + easing) ----------

#[derive(Clone, Copy)]
struct SliceAnimRadius {
    start_r: f64,    // radius at the moment the target last changed
    target_r: f64,   // radius we're easing toward
    start_time: f64, // performance.now() timestamp when target last changed
}

#[derive(Clone, Copy)]
struct SliceAnimAngle {
    start_a: f64,    // angle at the moment the target last changed
    target_a: f64,   // angle we're easing toward
    start_time: f64, // performance.now() timestamp when target last changed
}

const ANIM_DURATION_MS: f64 = 200.0;

/// Cubic ease-out: fast start, slow finish. `t` is 0.0..1.0 progress.
fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Given a slice's animation state and the current time, compute the
/// radius to actually draw this frame.
fn current_radius(anim: &SliceAnimRadius, now_ms: f64) -> f64 {
    let elapsed = now_ms - anim.start_time;
    let t = (elapsed / ANIM_DURATION_MS).clamp(0.0, 1.0);
    let eased = ease_out_cubic(t);
    anim.start_r + (anim.target_r - anim.start_r) * eased
}

fn current_angle(anim: &SliceAnimAngle, now_ms: f64) -> f64 {
    //let elapsed = now_ms - anim.start_time;
    //let t = (elapsed / ANIM_DURATION_MS).clamp(0.0, 1.0);
    //let eased = ease_out_cubic(t);
    //anim.start_r + (anim.target_r - anim.start_r) * eased
    0.0
}


fn new_pie_animation(canvas: &web_sys::HtmlCanvasElement, slices: &[PieSlice], geo: &PieGeometry, radii: &[f64]) -> Result<(), JsValue> {

    draw_pie(&canvas, &slices, &geo, &radii, vec![])?;

    Ok(())
}