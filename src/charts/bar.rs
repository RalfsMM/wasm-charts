use crate::data::{DataPoint, Points};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;

pub struct BarChartHandle {
    canvas: Rc<web_sys::HtmlCanvasElement>,
    running: Rc<RefCell<bool>>,
    mouse_closure: Closure<dyn FnMut(web_sys::MouseEvent)>,
    click_closure: Closure<dyn FnMut(web_sys::MouseEvent)>,
    chart_label: String,
    text_w: f64,
    points: Vec<DataPoint>,
}
//Apstadina kada chartu un taa listeners
impl BarChartHandle {
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

//globalais mainigais, stack ar visiem chartiem drilldownaa, kur last ir aktivais
thread_local! {
    static CHART_STACK: RefCell<Vec<BarChartHandle>> = RefCell::new(Vec::new());
}

//Konfiguracija Bar chartam, geo
#[derive(Clone, Deserialize)]
pub struct BarConfig {
    pub label_column: usize,
    pub value_column: Vec<usize>,
    pub vert: bool,
    pub sort_asc: bool,
    pub sort_desc: bool,
    pub sub_color:bool,
    pub sub_stack:bool,
    pub start_x: f64,
    pub start_y: f64,
    pub height: f64,
    pub width: f64,
    pub font_size: f64,
    pub font: String,
    pub text_color: String,
    pub step_fraction: f64,//nosaka cik biezi iedalas. 1.0, katras 10 mervienibas, 0.5 katras 5, 0.1 katru mervienibu. Var ari jebkadus skaitlus bet sitie ir standarts.
}
//Traits taa lai parsingaa var izmantot visu chartu configus
impl Points for BarConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> Vec<usize> {
        self.value_column.clone()
    }
}

//Noklusejuma vertibas, pirma kollona key, otra value
impl Default for BarConfig {
    fn default() -> Self {
        BarConfig { label_column: 0, value_column: vec![1], vert: true, sort_asc: false, sort_desc: false, sub_color: true, sub_stack:true, start_x:50.0, start_y:50.0, height: 300.0, width: 400.0,  font_size: 10.0, font: String::from("10px sans-serif"), text_color: String::from("black"), step_fraction: 1.0}
    }
}

impl BarConfig {
    pub fn from_json(json: JsValue) -> Result<Self, JsValue> {
        let cfg: BarConfig = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid BarConfig JSON: {e}")))?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<(), JsValue> {
        let err = |m: &str| Err(JsValue::from_str(m));
        if self.sort_asc && self.sort_desc {
            return err("sort_asc and sort_desc cannot both be true");
        }
        if self.value_column.is_empty() {
            return err("value_column must contain at least one column index");
        }
        if self.width <= 0.0 || self.height <= 0.0 {
            return err("width and height must be positive");
        }
        // if !(0.0..=1.0).contains(&self.step_fraction) || self.step_fraction == 0.0 {
        //     return err("step_fraction must be in (0, 1]");
        // }
        Ok(())
    }
}

pub struct Bar{
    pub label: String,
    pub value: f64,
    pub width: f64,
    pub height: f64,
    pub start_x: f64,
    pub start_y: f64,
    pub sub_points: Option<Vec<DataPoint>>,
    pub part: f64,
}

pub struct BarChart{
    pub bars: Vec<Bar>,
    pub top_lines: Vec<f64>,
    pub bottom_lines: Vec<f64>,
    pub exponent: f64,
}

fn palette_color(index: usize) -> String {
    let colors = [
        "#FFCE56", "#4BC0C0", "#9966FF","#FF6384", "#36A2EB",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()].to_string()
}

pub fn push_chart(handle: BarChartHandle) {
    CHART_STACK.with(|stack| stack.borrow_mut().push(handle));
}

pub fn compute_vertical_bars( mut points: Vec<DataPoint>, config: &BarConfig) -> Result<BarChart, JsValue>{
    if config.sort_asc{
        points.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
    }
    if config.sort_desc{
        points.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    }

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
    let avg;
    if min_value==0.0 && max_value==0.0{
        avg=1.0;
    }else{
        avg = total_value/point_count;
    }
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
            let subpoints = if p.sub_points.is_some() && !p.sub_points.as_ref().unwrap().is_empty() {
                p.sub_points
            } else {
                None
            };

            Bar{label: p.label, value: p.value, width: width, height: height, start_x: x, start_y: start_y, sub_points: subpoints, part:part}
        })
        .collect();
    let mut bottom_lines=vec![];
    let mut top_lines=vec![];
    let mut i=0.0;
    while i<=step_amount_pos{
        let line_y=start_y-i*step_line_px;
        let line_val = step*i;
        top_lines.push(line_y);
        top_lines.push(line_val);
        i+=1.0;
    }
    i=1.0;
    while i<=step_amount_neg{
        let line_y=start_y+i*step_line_px;
        let line_val = step*(-1.0)*i;
        bottom_lines.push(line_y);
        bottom_lines.push(line_val);
        i+=1.0;
    }

    let res= BarChart {bars: bars,bottom_lines: bottom_lines,top_lines: top_lines, exponent:exponent};
    Ok(res)
}

pub fn compute_horizontal_bars(mut points: Vec<DataPoint>, config: &BarConfig) -> Result<BarChart, JsValue>{
    if config.sort_asc{
        points.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
    }    
    if config.sort_desc{
        points.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
    }
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
    let avg;
    if min_value==0.0 && max_value==0.0{
        avg=1.0;
    }else{
        avg = total_value/point_count;
    }
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
    let step_line_px=config.width/(step_amount_pos+ step_amount_neg);

    //cik pikseli ir viena datu mervieniba
    let one_px=step_line_px/step;

    
    let start_x;
    if min_value==0.0{
        start_x =config.start_x;
    }else{
        start_x=config.start_x + step_amount_neg*step_line_px;
    }

    
    let bars:Vec<Bar> = points.into_iter().enumerate().map(|(i, p)|{
            let part = config.height/point_count;
            let height= 0.6*part;
            let width= one_px*p.value;
            let y =(i as f64 )* part + part*0.2+ config.start_y;
            let subpoints = if p.sub_points.is_some() && !p.sub_points.as_ref().unwrap().is_empty() {
                p.sub_points
            } else {
                None
            };


            Bar{label: p.label, value: p.value, width: height, height: width, start_x: start_x, start_y: y, sub_points: subpoints, part:part}
        })
        .collect();
    let mut bottom_lines=vec![];
    let mut top_lines=vec![];
    let mut i=0.0;
    while i<=step_amount_pos{
        let line_x=start_x+i*step_line_px;
        let line_val = step*i;
        top_lines.push(line_x);
        top_lines.push(line_val);
        i+=1.0;
    }
    i=1.0;
    while i<=step_amount_neg{
        let line_x=start_x-i*step_line_px;
        let line_val = step*(-1.0)*i;
        bottom_lines.push(line_x);
        bottom_lines.push(line_val);
        i+=1.0;
    }

    let res= BarChart {bars: bars,bottom_lines: bottom_lines,top_lines: top_lines, exponent:exponent};
    Ok(res)
}

pub fn draw_bar_chart( canvas: &web_sys::HtmlCanvasElement, bars: &[Bar], geo: &BarConfig, top_lines: &Vec<f64>, bottom_lines: &Vec<f64>, exponent: f64, lenghts: &[f64], bar_hover: &Option<usize>, title_hover: &Option<usize>, value_labels: &Vec<String>, key_label: &String)->Result<(), JsValue>{
    let context = crate::canvas::get_context(canvas)?;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    let default_size = geo.font_size;
    let font=geo.font.as_str();
    let text_color =geo.text_color.as_str();
    let all_val_lab= value_labels.join("  ");


    context.set_fill_style_str(text_color);
    context.set_font(font);
    context.set_text_align("right");
    context.set_text_baseline("bottom");

    
    CHART_STACK.with(|stack| {
        let charts = stack.borrow();
        let mut indent= 0.0;
        for (i, chart) in charts.iter().enumerate() {
            if *title_hover == Some(i) {
                context.set_fill_style_str("red");
            } else {
                context.set_fill_style_str(text_color);
            }
            let label;
            if indent>0.0{
                label = format!("/{}", chart.chart_label);
                indent +=default_size/3.0;
            }else{
                label = chart.chart_label.clone();
            }
            indent += chart.text_w;
            context
                .fill_text(&label, geo.start_x + indent, geo.start_y - 0.1*geo.height)
                .map_err(|_| JsValue::from_str("failed to draw chart label"))?;
        }
        Ok::<(), JsValue>(())
    })?;
    context.set_fill_style_str(text_color);

    if geo.vert{
        context.set_text_align("left");
        context.set_text_baseline("bottom");
        context.fill_text(&all_val_lab, geo.start_x, geo.start_y-default_size)?;
        context.fill_text(key_label, geo.start_x+geo.width, geo.start_y+geo.height)?;
        context.set_text_baseline("top");
        
        if geo.sub_color && value_labels.len()>1{
             for (i,lab) in value_labels.iter().enumerate(){
                context.set_fill_style_str(&palette_color(i));
                context.fill_rect(geo.start_x + geo.width+ geo.font_size, geo.start_y+2.0*geo.font_size*(i as f64), geo.font_size, geo.font_size);
                context.set_fill_style_str(text_color);
                context.fill_text(lab, geo.start_x+geo.width + 3.0*geo.font_size, geo.start_y+2.0*geo.font_size*(i as f64))?;
             }
        }

        context.begin_path();
        context.set_fill_style_str("gray");
        context.set_text_baseline("middle");
        context.set_text_align("right");
        context.set_line_width(0.5);

        let mut i =2;
        while i<top_lines.len(){
            context.move_to(geo.start_x, top_lines[i]);
            context.line_to(geo.start_x+geo.width, top_lines[i]);
            let exp;
            if exponent>0.0{
                exp=0;
            }else{
                exp=-exponent as usize;
            }
            let label=format!("{:.exp$}  ",top_lines[i+1]);
            context.fill_text(&label, geo.start_x, top_lines[i])?;
            i+=2;
        }
        i =0;
        while i<bottom_lines.len(){
            context.move_to(geo.start_x, bottom_lines[i]);
            context.line_to(geo.start_x+geo.width, bottom_lines[i]);
            let exp;
            if exponent>0.0{
                exp=0;
            }else{
                exp=-exponent as usize;
            }
            let label=format!("{:.exp$}  ",bottom_lines[i+1]);
            context.fill_text(&label, geo.start_x, bottom_lines[i])?;
            i+=2;
        }
        context.stroke();
        context.set_fill_style_str("black");

        for (i, bar) in bars.iter().enumerate(){
            if *bar_hover==Some(i){
                context.set_fill_style_str("red");
                context.set_global_alpha(0.1);
                context.fill_rect(bar.start_x-bar.part*0.2, geo.start_y, bar.part, geo.height);
                context.set_global_alpha(1.0);
                context.fill_rect(bar.start_x, bar.start_y, bar.width, -lenghts[i]);
            }else{
                if bar.sub_points.is_some(){
                    if geo.sub_stack{
                        if geo.sub_color{
			    let mut last_y_pos=bar.start_y;
			    let mut last_y_neg=bar.start_y;
			    for (j, p) in bar.sub_points.clone().unwrap().iter().enumerate(){
			        context.set_fill_style_str(&palette_color(j));
			        if p.value>0.0{
			            context.fill_rect(bar.start_x, last_y_pos, bar.width, -lenghts[i]/bar.value*p.value);
			            last_y_pos-=lenghts[i]/bar.value*p.value;
			        }else{
			            context.fill_rect(bar.start_x, last_y_neg, bar.width, -lenghts[i]/bar.value*p.value);
			            last_y_neg-=lenghts[i]/bar.value*p.value;
			        }

			    }
                        }else{
                            context.fill_rect(bar.start_x, bar.start_y, bar.width, -lenghts[i]);
                        }
                    }else{
                        if geo.sub_color{
                            let mut last_x=bar.start_x;
                            let width=bar.width/(bar.sub_points.clone().unwrap().len() as f64);
                            for (j, p) in bar.sub_points.clone().unwrap().iter().enumerate(){
                                context.set_fill_style_str(&palette_color(j));
                                context.fill_rect(last_x, bar.start_y,  width, -lenghts[i]/bar.value*p.value);
                                last_x+=width;
                            }
                        }else{
                            let mut last_x=bar.start_x;
                            let width=bar.width/(bar.sub_points.clone().unwrap().len() as f64);
                            for p in bar.sub_points.clone().unwrap(){
                                context.fill_rect(last_x, bar.start_y,  width, -lenghts[i]/bar.value*p.value);
                                last_x+=width;
                            }
                        }
                    }
                }else{
                    if geo.sub_color && CHART_STACK.with(|stack| stack.borrow().len()>1){
                        context.set_fill_style_str(&palette_color(i));
                    }
                    context.fill_rect(bar.start_x, bar.start_y, bar.width, -lenghts[i]);
                }
                context.set_fill_style_str("black");
            }

            
            // let value=format!("{}",bar.value);
            // if lenghts[i]>0.0{
            //     context.set_text_baseline("bottom");
            // }else{
            //     context.set_text_baseline("top");
            // }
            // context.set_text_align("middle");
            // context.fill_text(&value, bar.start_x+bar.width/1.2, bar.start_y -lenghts[i])?;
            context.save();
            context.translate(bar.start_x+(bar.width/2.0), geo.start_y + geo.height + 0.3*geo.font_size)?;// move origin to where the text should be
            context.rotate(-std::f64::consts::PI / 2.0)?;
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.fill_text(&bar.label, 0.0, 0.0)?;
            context.restore();
            context.set_fill_style_str("black");
        }

        context.begin_path();
        context.set_line_width(1.0);
        context.set_text_baseline("middle");
        context.set_text_align("right");
        context.move_to(geo.start_x, top_lines[0]);
        context.line_to(geo.start_x+geo.width, top_lines[0]);
        context.fill_text("0  ", geo.start_x, top_lines[0])?;
        context.stroke();

    }else{
        context.set_text_align("left");
        context.set_text_baseline("top");
        context.fill_text(key_label, geo.start_x, geo.start_y+geo.height)?;

        for (i,lab) in value_labels.iter().enumerate(){
            let mut gap=geo.font_size;
            if geo.sub_color && value_labels.len()>1{
                context.set_fill_style_str(&palette_color(i));
                context.fill_rect(geo.start_x + geo.width+ geo.font_size, geo.start_y+2.0*geo.font_size*(i as f64), geo.font_size, geo.font_size);
                gap *= 3.0;
            }
            context.set_fill_style_str(text_color);
            context.fill_text(lab, geo.start_x+geo.width + gap, geo.start_y+2.0*geo.font_size*(i as f64))?;
        }
            
        context.begin_path();
        context.set_fill_style_str("gray");
        context.set_text_baseline("bottom");
        context.set_text_align("center");
        context.set_line_width(0.5);

        let mut i =2;
        while i<top_lines.len(){
            context.move_to( top_lines[i],geo.start_y);
            context.line_to(top_lines[i], geo.start_y+geo.height);
            let exp;
            if exponent>0.0{
                exp=0;
            }else{
                exp=-exponent as usize;
            }
            let label=format!("{:.exp$}",top_lines[i+1]);

            context.save();
            context.translate(top_lines[i], geo.start_y- geo.font_size*0.3)?;// move origin to where the text should be
            context.rotate(std::f64::consts::PI / 2.0)?;
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.fill_text(&label, 0.0, 0.0)?;
            context.restore();
            i+=2;

        }
        i=0;
        while i<bottom_lines.len(){
            context.move_to( bottom_lines[i], geo.start_y);
            context.line_to(bottom_lines[i], geo.start_y+geo.height);
            let exp;
            if exponent>0.0{
                exp=0;
            }else{
                exp=-exponent as usize;
            }
            let label=format!("{:.exp$}",bottom_lines[i+1]);

            context.save();
            context.translate(bottom_lines[i], geo.start_y- geo.font_size*0.3)?;// move origin to where the text should be
            context.rotate(std::f64::consts::PI / 2.0)?;
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.fill_text(&label, 0.0, 0.0)?;
            context.restore();
            i+=2;
        }
        context.stroke();

        context.set_fill_style_str(text_color);
        for (i, bar) in bars.iter().enumerate(){
            if *bar_hover==Some(i){
                context.set_fill_style_str("red");
                context.set_global_alpha(0.1);
                context.fill_rect(geo.start_x,bar.start_y-bar.part*0.2, geo.width, bar.part);
                context.set_global_alpha(1.0);
                context.fill_rect(bar.start_x, bar.start_y, lenghts[i], bar.width);
            }else{
                if bar.sub_points.is_some(){
                    if geo.sub_stack{
                        if geo.sub_color{
                            let mut last_x_pos=bar.start_x;
                            let mut last_x_neg=bar.start_x;
                            for (j, p) in bar.sub_points.clone().unwrap().iter().enumerate(){
                                context.set_fill_style_str(&palette_color(j));
                                if p.value>0.0{
                                    context.fill_rect(last_x_pos,bar.start_y, lenghts[i]/bar.value*p.value,bar.width,);
                                    last_x_pos+=lenghts[i]/bar.value*p.value;
                                }else{
                                    context.fill_rect(last_x_neg, bar.start_y, lenghts[i]/bar.value*p.value, bar.width);
                                    last_x_neg+=lenghts[i]/bar.value*p.value;
                                }
                            }
                        }else{
                            context.fill_rect(bar.start_x, bar.start_y, lenghts[i], bar.width);
                        }
                    }else{
                        if geo.sub_color{
                            let mut last_y=bar.start_y;
                            let height=bar.width/(bar.sub_points.clone().unwrap().len() as f64);
                            for (j, p) in bar.sub_points.clone().unwrap().iter().enumerate(){
                                context.set_fill_style_str(&palette_color(j));
                                context.fill_rect(bar.start_x, last_y,  lenghts[i]/bar.value*p.value, height);
                                last_y+=height;
                            }
                        }else{
                            let mut last_y=bar.start_y;
                            let height=bar.width/(bar.sub_points.clone().unwrap().len() as f64);
                            for p in bar.sub_points.clone().unwrap(){
                                context.fill_rect(bar.start_x, last_y, lenghts[i]/bar.value*p.value, height);
                                last_y+=height;
                            }
                        }
                    }
                }else{
                    if geo.sub_color && CHART_STACK.with(|stack| stack.borrow().len()>1){
                        context.set_fill_style_str(&palette_color(i));
                    }
                    context.fill_rect(bar.start_x, bar.start_y, lenghts[i], bar.width);
                }
                context.set_fill_style_str("black");
            }
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.fill_text(&bar.label, geo.start_x-geo.font_size*0.3, bar.start_y)?;
            context.set_fill_style_str("black");
        }
        context.begin_path();
        context.set_line_width(1.0);
        context.move_to( top_lines[0], geo.start_y);
        context.line_to( top_lines[0],geo.start_y+geo.height);
        context.stroke();
        context.save();
        context.translate(top_lines[0], geo.start_y- geo.font_size*0.3)?;// move origin to where the text should be
        context.rotate(std::f64::consts::PI / 2.0)?;
        context.set_text_align("right");
        context.set_text_baseline("middle");
        context.fill_text("0", 0.0, 0.0)?;
        context.restore();

    }           
    Ok(())
}

pub fn hit_test_bar(bars: &[Bar], mx:f64, my: f64, geo: &BarConfig)->Option<usize>{
    if geo.vert{
        for (i, bar) in bars.iter().enumerate(){
            if
                mx>(bar.start_x-bar.part*0.2)
                && 
                mx<(bar.start_x+bar.part*0.8)
                &&
                my>(geo.start_y)
            {
                return Some(i);
            }
        }
    }else{
        for (i, bar) in bars.iter().enumerate(){
            if
                my>(bar.start_y-bar.part*0.2)
                && 
                my<(bar.start_y+bar.part*0.8)
                &&
                mx>geo.start_x
            {
                return Some(i);
            }
        }
    }
    None
}

//parbauda vai padotas koordinatas ir uz kada title, un atgriez uz kura ja ir
fn hit_test_titles(mx: f64, my: f64, geo: &BarConfig) -> Option<usize> {
    if my<geo.start_y - 0.1*geo.height - geo.font_size||my>geo.start_y - 0.1*geo.height||mx<geo.start_x{
        return None;
    }
    CHART_STACK.with(|stack|{
        let charts=stack.borrow();
        let mut indent=0.0;
        for (i,chart) in charts.iter().enumerate() {
            indent += chart.text_w+ geo.font_size/3.0;
            if mx< geo.start_x + indent{
                return Some(i);
            }
        }
        None
    })
}

pub fn render_bar_chart(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, conf: &BarConfig, value_labels: Vec<String>, key_label: String) -> Result<BarChartHandle, JsValue> {
    let barchart;
    let config=conf.clone();
    if config.vert{
        barchart= compute_vertical_bars(points.clone(), &config)?;
    }else{
        barchart=compute_horizontal_bars(points.clone(), &config)?;
    }

    let bars = Rc::new(barchart.bars);
    let canvas = Rc::new(crate::canvas::get_canvas(canvas_id)?);
    let config=Rc::new(config);
    let toplines=Rc::new(barchart.top_lines);
    let bottomlines=Rc::new(barchart.bottom_lines);
    let running = Rc::new(RefCell::new(true));
    let value_labels = Rc::new(value_labels);
    let key_label = Rc::new(key_label);

    let now_ms = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .ok_or_else(|| JsValue::from_str("no performance"))?;

    let hover_bar: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let hover_title: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let anim_state: Rc<RefCell<Vec<BarAnimState>>> = Rc::new(RefCell::new(
        bars
            .iter()
            .map(|bar| BarAnimState { start_h: 0.0, target_h: bar.height, start_time: now_ms })
            .collect(),
    ));

    //mousemove listener un handler
    let canvas_for_mouse = canvas.clone();
    let bars_for_mouse = bars.clone();
    let hover_bar_for_mouse = hover_bar.clone();
    let hover_title_for_mouse = hover_title.clone();
    let config_for_mouse=config.clone();

    let mouse_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_mouse.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();


        //parbauda vai hover uz bar
        let bar_hit = hit_test_bar(&bars_for_mouse, mx, my, &config_for_mouse);
        let mut bar_hover_ref = hover_bar_for_mouse.borrow_mut();     
        if *bar_hover_ref != bar_hit{
            *bar_hover_ref = bar_hit;
        }

        //parbauda vai hover uz title
        let title_hit = hit_test_titles(mx, my, &config_for_mouse);
        let mut title_hover_ref = hover_title_for_mouse.borrow_mut();
        if *title_hover_ref != title_hit {
            *title_hover_ref = title_hit;
        }

    });
    canvas.add_event_listener_with_callback("mousemove", mouse_closure.as_ref().unchecked_ref())?;

    //mousedown listener un handler
    let canvas_for_click = canvas.clone();
    let bars_for_click = bars.clone();
    let config_for_click = config.clone();
    let running_for_click = running.clone();
    let value_labels_for_click = value_labels.clone();
    let key_label_for_click = key_label.clone();

    let click_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_click.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();
        
        //parbauda vai click uz bar
        let bar_hit = hit_test_bar(&bars_for_click,  mx, my, &config_for_click);
        if bar_hit.is_some() {
            for ( i, bar) in bars_for_click.iter().enumerate() {
                if Some(i) == bar_hit {
                    if bar.sub_points.is_some() {
                        *running_for_click.borrow_mut() = false;

                        CHART_STACK.with(|stack| {
                            if let Some(old_handle) = stack.borrow_mut().last() {
                                old_handle.stop();
                            }
                        });

                        if let Ok(new_handle) = render_bar_chart(canvas_for_click.id().as_str(), bar.sub_points.clone().unwrap(), bar.label.clone(), &config_for_click, (*value_labels_for_click).clone(), (*key_label_for_click).clone()) {
                            CHART_STACK.with(|stack| stack.borrow_mut().push(new_handle));
                        }

                        break;
                    }
                }
            }
        }

        //parbauda vai click uz title
        let title_hit = hit_test_titles(mx, my, &config_for_click);
        if let Some(i) = title_hit {
            CHART_STACK.with(|stack| {
                let mut charts = stack.borrow_mut(); // single mutable borrow, used for everything below
                *running_for_click.borrow_mut() = false;
                let points = charts[i].points.clone();
                let label = charts[i].chart_label.clone();

                while charts.len() > i {
                    if let Some(old_chart) = charts.pop() {
                        old_chart.stop();//kinda redundant, jo tikai augsejais stackaa var but running
                    }
                }
                drop(charts);
                if let Ok(new_handle) = render_bar_chart(canvas_for_click.id().as_str(), points, label,  &config_for_click, (*value_labels_for_click).clone(), (*key_label_for_click).clone()) {
                    CHART_STACK.with(|stack| stack.borrow_mut().push(new_handle));
                }
            });
        }
    });
    canvas.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())?;

    //animacijas loop, kas visu laiku(vai ari kad izmainas) zime chart ar atbilstosa frame vertibam
    let canvas_for_loop = canvas.clone();
    let config_for_loop =config.clone();
    let toplines_for_loop=toplines.clone();
    let bottomlines_for_loop=bottomlines.clone();
    let bars_for_loop = bars.clone();
    let anim_for_loop = anim_state.clone();
    let hover_bar_for_loop = hover_bar.clone();
    let hover_title_for_loop = hover_title.clone();
    let running_for_loop = running.clone();
    let value_labels_for_loop = value_labels.clone();
    let key_label_for_loop = key_label.clone();

    crate::animation::start_loop(move |_elapsed_ms| {
        if !*running_for_loop.borrow() {
            return false;
        }
        let now = web_sys::window().unwrap().performance().unwrap().now();
        let lenghts: Vec<f64> = anim_for_loop.borrow().iter().map(|a| current_height(a, now)).collect();
        let hover_bar_value = *hover_bar_for_loop.borrow();
        let hover_title_value = *hover_title_for_loop.borrow();
        let _ = draw_bar_chart(&canvas_for_loop, &bars_for_loop, &config_for_loop, &toplines_for_loop, &bottomlines_for_loop, barchart.exponent, &lenghts, &hover_bar_value, &hover_title_value, &value_labels_for_loop, &key_label_for_loop);
        true
    })?;

    //izmera cik gars ir title
    let context = crate::canvas::get_context(&canvas)?;
    let textmetrics = context.measure_text(chart_label.as_str());
    let text_w =textmetrics.unwrap().width();//chatins piedavaja map_err seit, es nez

    Ok(BarChartHandle { canvas, running, mouse_closure, click_closure, chart_label, text_w, points })
}

pub fn current_height(anim: &BarAnimState, now_ms: f64)->f64{
    let elapsed = now_ms - anim.start_time;
    let t = (elapsed / 500.0).clamp(0.0, 1.0);
    anim.start_h + (anim.target_h - anim.start_h) * t
}
pub struct BarAnimState{
    start_h:f64,
    target_h:f64,
    start_time: f64,
}