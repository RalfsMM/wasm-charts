use crate::data::Points;
use crate::data::DataPoint;
use wasm_bindgen::prelude::*;

pub struct PieConfig {
    pub label_column: i32,
    pub value_column: i32,
}
impl Points for PieConfig {
    fn label_col(&self) -> i32 {
        self.label_column
    }
    fn value_col(&self) -> i32 {
        self.value_column
    }
}

impl Default for PieConfig {
    fn default() -> Self {
        PieConfig { label_column: 0, value_column: 1 }
    }
}

pub struct PieSlice { label: String, angle_start: f64, angle_end: f64, color: String, percent: f64 }

pub fn compute_pie_slices(points: Vec<DataPoint>) -> Result<Vec<PieSlice>, wasm_bindgen::JsValue> {

    let mut negative = false;
    let p2=points.clone();
    p2.into_iter().for_each(|p| {
        if p.value < 0.0 {
            negative = true;
        }
    });

    if negative {
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

pub fn draw_pie(canvas_id: &str, slices: &[PieSlice]) -> Result<(), wasm_bindgen::JsValue> {
    let document = web_sys::window()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("no window"))?
        .document()
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("canvas not found"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| wasm_bindgen::JsValue::from_str("not a canvas element"))?;
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("failed to get 2D context"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .map_err(|_| wasm_bindgen::JsValue::from_str("failed to cast to CanvasRenderingContext2d"))?;

    let mut start_angle = 0.0;
    for slice in slices {
        let fraction = (slice.angle_end - slice.angle_start) / std::f64::consts::TAU;
        let end_angle = start_angle + fraction * std::f64::consts::TAU;
        context.begin_path();
        context.move_to(200.0, 200.0); // center of the pie
        context.arc(200.0, 200.0, 100.0, start_angle + 0.02, end_angle -0.02)
            .map_err(|_| wasm_bindgen::JsValue::from_str("failed to draw arc"))?;
        context.arc_with_anticlockwise(200.0, 200.0, 80.0, end_angle -0.02,start_angle + 0.02,  true)
            .map_err(|_| wasm_bindgen::JsValue::from_str("failed to draw arc"))?;
        context.close_path();
        context.set_fill_style_str(&slice.color);
        context.fill();
        context.set_fill_style_str("black");
        let mid_angle = (start_angle + end_angle) / 2.0;
        //let percentage = format!("{:.1}%", slice.percent);
        //let percent_x = 150.0 + 80.0 * mid_angle.cos();
        //let percent_y = 150.0 + 80.0 * mid_angle.sin();
        //context.fill_text(&percentage, percent_x, percent_y)
        //    .map_err(|_| wasm_bindgen::JsValue::from_str("failed to draw text"))?;
        let label_x;
        if (mid_angle < std::f64::consts::PI/2.0) || (mid_angle > std::f64::consts::PI*3.0/2.0) {
            context.set_text_align("left");
            label_x= 310.0;
        } else {
            label_x= 90.0;
            context.set_text_align("right");
        }
        let label_y = 200.0 + 100.0 * mid_angle.sin();
        context.fill_text(&slice.label, label_x, label_y)
            .map_err(|_| wasm_bindgen::JsValue::from_str("failed to draw text"))?;
        start_angle = end_angle;
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