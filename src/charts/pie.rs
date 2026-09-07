use crate::data::Points;
use crate::data::DataPoint;
use wasm_bindgen::prelude::*;
use crate::canvas::{get_canvas, get_context};
use std::rc::Rc;
use std::cell::RefCell;

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

// charts/pie.rs
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

pub struct PieSlice { label: String, angle_start: f64, angle_end: f64, color: String, percent: f64 }

pub fn compute_pie_slices(points: Vec<DataPoint>) -> Result<Vec<PieSlice>, wasm_bindgen::JsValue> {

    if points.iter().any(|p| p.value < 0.0) {
    return Err(wasm_bindgen::JsValue::from_str("negative values are not allowed in pie chart"));
    }
    
    let total: f64 = points.iter().map(|p| p.value).sum();
    let mut angle = 0.0;
    let pointcount = points.len() as f64;

    Ok(points.into_iter().enumerate().map(|(i, p)| {
        let sweep = if total > 0.0 { (p.value / total) * std::f64::consts::TAU } else { (1.0 / pointcount) * std::f64::consts::TAU };
        let start = angle;
        angle += sweep;
        let percent = if total > 0.0 { (p.value / total) * 100.0 } else { 0.0 };
        PieSlice { label: p.label, angle_start: start, angle_end: angle, color: palette_color(i), percent }
    }).collect())
}

pub fn draw_pie(
    canvas: &web_sys::HtmlCanvasElement,
    slices: &[PieSlice],
    geo: &PieGeometry,
    hovered: Option<usize>,
) -> Result<(), JsValue> {
    let context = crate::canvas::get_context(canvas)?;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    for (i, slice) in slices.iter().enumerate() {
        // pop the hovered slice's outer radius out slightly
        let outer_r = if hovered == Some(i) { geo.outer_r + 10.0 } else { geo.outer_r };
        let inner_r = if hovered == Some(i) { geo.inner_r + 10.0 } else { geo.inner_r };

        context.begin_path();
        context.move_to(geo.cx, geo.cy);
        context.arc(geo.cx, geo.cy, outer_r, slice.angle_start + 0.02, slice.angle_end - 0.02)
            .map_err(|_| JsValue::from_str("failed to draw arc"))?;
        context.arc_with_anticlockwise(geo.cx, geo.cy, inner_r, slice.angle_end - 0.02, slice.angle_start + 0.02, true)
            .map_err(|_| JsValue::from_str("failed to draw arc"))?;
        context.close_path();
        context.set_fill_style_str(&slice.color);
        context.fill();

        context.set_fill_style_str("black");
        let mid_angle = (slice.angle_start + slice.angle_end) / 2.0;
        let label_x;
        if mid_angle < std::f64::consts::PI / 2.0 || mid_angle > std::f64::consts::PI * 3.0 / 2.0 {
            context.set_text_align("left");
            label_x = geo.cx + 120.0;
        } else {
            context.set_text_align("right");
            label_x = geo.cx - 120.0;
        }
        let label_y = geo.cy + outer_r * mid_angle.sin();
        context.fill_text(&slice.label, label_x, label_y)
            .map_err(|_| JsValue::from_str("failed to draw text"))?;
    }
    Ok(())
}

fn palette_color(index: usize) -> String {
    let colors = [
        "#FF6384", "#36A2EB", "#FFCE56", "#4BC0C0", "#9966FF",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()].to_string()
}

pub fn hit_test(slices: &[PieSlice], geo: &PieGeometry, mx: f64, my: f64) -> Option<usize> {
    let dx = mx - geo.cx;
    let dy = my - geo.cy;
    let dist = (dx * dx + dy * dy).sqrt();

    if dist < geo.inner_r || dist > geo.outer_r {
        return None;
    }

    let mut angle = dy.atan2(dx);
    if angle < 0.0 {
        angle += std::f64::consts::TAU;
    }

    slices.iter().position(|s| angle >= s.angle_start && angle < s.angle_end)
}

pub fn render_interactive_pie(canvas_id: &str, slices: Vec<PieSlice>) -> Result<(), JsValue> {
    let canvas = crate::canvas::get_canvas(canvas_id)?;
    let geo = PieGeometry::default();

    let slices = Rc::new(slices);
    let hover = Rc::new(RefCell::new(None::<usize>));

    // initial draw
    draw_pie(&canvas, &slices, &geo, *hover.borrow())?;

    let canvas_clone = canvas.clone();
    let slices_clone = slices.clone();
    let hover_clone = hover.clone();
    let geo_clone = PieGeometry { ..geo };

    let closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_clone.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();

        let hit = hit_test(&slices_clone, &geo_clone, mx, my);
        let changed = *hover_clone.borrow() != hit;
        if changed {
            *hover_clone.borrow_mut() = hit;
            let _ = draw_pie(&canvas_clone, &slices_clone, &geo_clone, hit);
        }
    });

    canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}