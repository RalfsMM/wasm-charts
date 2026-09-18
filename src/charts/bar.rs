use crate::data::{DataPoint, Points};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;

//Konfiguracija Bar chartam, geo
#[derive(Clone, Copy)]
pub struct BarConfig {
    pub label_column: usize,
    pub value_column: usize,
    pub vert: bool,
    pub sort: bool,
    pub start_x: f64,
    pub start_y: f64,
    pub height: f64,
    pub width: f64,
    pub default_font_size: f64,
    pub default_font: &'static str,
    pub text_color: &'static str,

}
//Traits taa lai parsingaa var izmantot visu chartu configus
impl Points for BarConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> usize {
        self.value_column
    }
}

//Noklusejuma vertibas, pirma kollona key, otra value
impl Default for BarConfig {
    fn default() -> Self {
        BarConfig { label_column: 0, value_column: 1, vert: true, sort: false, start_x:0.0, start_y:0.0, height: 400.0, width: 600.0,  default_font_size: 10.0, default_font: "10px sans-serif", text_color: "black"}
    }
}

pub struct Bar{
    pub label: String,
    pub value: f64,
    pub width: f64,
    pub height: f64,
    pub start_x: f64,
    pub start_y: f64
}

pub fn compute_bars(points: Vec<DataPoint>, config: BarConfig) -> Result<Vec<Bar>, JsValue>{
    let mut total_value=0.0;
    let mut min_value =0.0;
    points.iter().for_each(|p| {
        total_value+=p.value;
        if p.value<min_value{
            min_value=p.value;
        }
    });
    let point_count = points.len() as f64;
    Ok(
        points.into_iter().enumerate().map(|(i, p)|{
            let mut width= 0.0;
            let mut height= 0.0;
            if config.vert{
                width= 0.8*(config.width/point_count);
            }else{
                height= 0.8*(config.height/point_count);
            }
            let x =0.0;
            let y=0.0;

            Bar{label: p.label, value: p.value, width: width, height: height, start_x: x, start_y: y}
        })
        .collect()
    )
}