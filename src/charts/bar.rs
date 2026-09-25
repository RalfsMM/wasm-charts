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
    pub step_fraction: f64,
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
        BarConfig { label_column: 0, value_column: 1, vert: true, sort: false, start_x:50.0, start_y:50.0, height: 300.0, width: 400.0,  default_font_size: 10.0, default_font: "10px sans-serif", text_color: "black", step_fraction: 1.0}
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

pub struct BarChart{
    pub bars: Vec<Bar>,
    pub top_lines: Vec<f64>,
    pub bottom_lines: Vec<f64>,
}

pub fn compute_vertical_bars(points: Vec<DataPoint>, config: BarConfig) -> Result<BarChart, JsValue>{
    let mut total_value=0.0;
    let mut max_value = 0.0;
    let mut min_value =0.0;
    points.iter().for_each(|p| {
        total_value+=p.value;
        if p.value<min_value{
            min_value=p.value;
        }
        if p.value>max_value{
            max_value=p.value;
        }
    });
    //kkadu checku ja viss ir 0
    let point_count = points.len() as f64;
    let avg = total_value/point_count;
    //kada desmit pakape ir average zinatniska pierakstaa skaitlim no datiem
    let exponent = avg.abs().log10().floor();
    //let exponent = max_value.abs().log10().floor();


    //cik mervienibas viena iedala, aprekina ar configa dalskaitli un 10 exponent pakaapee
    let step = config.step_fraction * 10.0_f64.powf(exponent);
    //cik iedalas bus charta uz pozitivo virzienu
    let step_amount_pos =(max_value/step).ceil()+2.0;
    //cik iedalas bus charta uz negativo virzienu
    let step_amount_neg;
    if min_value != 0.0 {
        step_amount_neg =(min_value.abs()/step).ceil();
    }else{
        step_amount_neg=0.0;
    }
    //iedalu linijas atstarpes
    let step_line_px=config.height/(step_amount_pos+ step_amount_neg);

    //cik pikseli ir viena datu mervieniba
    let one_px=step_line_px/step;

    
    let start_y;
    if min_value==0.0{
        start_y =config.start_y+config.height;
    }else{
        start_y=config.start_y+config.height-step_amount_neg*step_line_px;
    }

    
    let bars:Vec<Bar> = points.into_iter().enumerate().map(|(i, p)|{
            let part = config.width/point_count;
            let width= 0.6*part;
            let height= one_px*p.value;
            let x =(i as f64 )* part + part*0.2+ config.start_x;

            Bar{label: p.label, value: p.value, width: width, height: height, start_x: x, start_y: start_y}
        })
        .collect();
    let mut bottom_lines=vec![];
    let mut top_lines=vec![];
    let mut i=0.0;
    while i<=step_amount_pos{
        let line_y=start_y-i*step_line_px;
        top_lines.push(line_y);
        i+=1.0;
    }
    i=1.0;
    while i<=step_amount_neg{
        let line_y=start_y+i*step_line_px;
        bottom_lines.push(line_y);
        i+=1.0;
    }

    let res= BarChart {bars: bars,bottom_lines: bottom_lines,top_lines: top_lines};
    Ok(res)
}

pub fn draw_bar_chart( canvas: &web_sys::HtmlCanvasElement, bars: &[Bar], geo: BarConfig, top_lines: Vec<f64>, bottom_lines: Vec<f64>)->Result<(), JsValue>{
    let context = crate::canvas::get_context(canvas)?;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    let default_size = geo.default_font_size;
    let default_font=geo.default_font;
    let text_color =geo.text_color;
    context.set_font(default_font);
    //context.set_fill_style_str(text_color);

    context.begin_path();
    context.set_fill_style_str("gray");
    context.fill_rect(geo.start_x, geo.start_y, geo.width, geo.height);
    context.set_fill_style_str("black");

    context.begin_path();
    for line in top_lines{
        context.move_to(geo.start_x, line);
        context.line_to(geo.start_x+geo.width, line);
    }
    for line in bottom_lines{
        context.move_to(geo.start_x, line);
        context.line_to(geo.start_x+geo.width, line);
    }
    context.stroke();

    for bar in bars{
        context.fill_rect(bar.start_x, bar.start_y, bar.width, -bar.height);
        context.rotate(std::f64::consts::PI/2.0)?;
        context.fill_text(&bar.label, bar.start_x, geo.start_y + geo.height)?;
        context.rotate(-std::f64::consts::PI/2.0)?;
    }
    
    Ok(())
}

pub fn render_bar_chart(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, config: BarConfig) -> Result<(), JsValue> {
    let barchart= compute_vertical_bars(points, config)?;
    let bars=barchart.bars;
    let toplines=barchart.top_lines;
    let bottomlines=barchart.bottom_lines;
    let canvas = crate::canvas::get_canvas(canvas_id)?;
    let geo=config;

    draw_bar_chart(&canvas, &bars, geo, toplines, bottomlines)?;

    Ok(())
}