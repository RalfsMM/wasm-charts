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
    pub step_fraction: f64,//nosaka cik biezi iedalas. 1.0, katras 10 mervienibas, 0.5 katras 5, 0.1 katru mervienibu. Var ari jebkadus skaitlus bet sitie ir standarts.
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
    pub exponent: f64,
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

            Bar{label: p.label, value: p.value, width: width, height: height, start_x: x, start_y: start_y}
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

pub fn draw_bar_chart( canvas: &web_sys::HtmlCanvasElement, bars: &[Bar], geo: &BarConfig, top_lines: &Vec<f64>, bottom_lines: &Vec<f64>, exponent: f64, heights: &[f64], bar_hover: &Option<usize>, label_x: &String, label_y: &String)->Result<(), JsValue>{
    let context = crate::canvas::get_context(canvas)?;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    let default_size = geo.default_font_size;
    let default_font=geo.default_font;
    let text_color =geo.text_color;
    context.set_font(default_font);
    context.set_text_align("right");
    context.set_text_baseline("bottom");
    context.fill_text(label_x, geo.start_x, geo.start_y-default_size/1.5)?;
    context.set_text_align("left");
    context.set_text_baseline("top");
    context.fill_text(label_y, geo.start_x+geo.width, geo.start_y+geo.height)?;

    context.begin_path();
    context.set_fill_style_str("gray");
    context.set_text_baseline("middle");
    context.set_text_align("right");

    context.begin_path();
    let mut i =0;
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

    for (i, bar) in bars.iter().enumerate(){
        if *bar_hover==Some(i){
            context.set_fill_style_str("red");
        }else{
            context.set_fill_style_str("black");
        }
        context.fill_rect(bar.start_x, bar.start_y, bar.width, -heights[i]);
        context.save();
        context.translate(bar.start_x+(bar.width/2.0), geo.start_y + geo.height + 0.3*geo.default_font_size)?;            // move origin to where the text should be
        context.rotate(-std::f64::consts::PI / 2.0)?;
        context.set_text_baseline("middle");
        context.fill_text(&bar.label, 0.0, 0.0)?;
        context.restore();
    }
            
    Ok(())
}

pub fn hit_test_bar(bars: &[Bar], mx:f64, my: f64)->Option<usize>{
    for (i, bar) in bars.iter().enumerate(){
        if  (
                mx>bar.start_x 
                && 
                mx<(bar.start_x+bar.width)
            )
            &&
            (
                (
                    bar.height>0.0 
                    && 
                    my<bar.start_y 
                    && 
                    my>(bar.start_y-bar.height)
                )
                ||
                (
                    bar.height<0.0 
                    && 
                    my>bar.start_y
                    && 
                    my<(bar.start_y-bar.height)
                )
            )
        {
            return Some(i);
        }
    }
    None
}

pub fn render_bar_chart(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, config: BarConfig, label_x: String, label_y: String) -> Result<(), JsValue> {
    let barchart= compute_vertical_bars(points, config)?;

    let bars = Rc::new(barchart.bars);
    let canvas = Rc::new(crate::canvas::get_canvas(canvas_id)?);
    let geo=Rc::new(config);
    let toplines=Rc::new(barchart.top_lines);
    let bottomlines=Rc::new(barchart.bottom_lines);

    let now_ms = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .ok_or_else(|| JsValue::from_str("no performance"))?;

    let hover_bar: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
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

    let mouse_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_mouse.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();


        //parbauda vai hover uz bar
        let bar_hit = hit_test_bar(&bars_for_mouse, mx, my);
        let mut bar_hover_ref = hover_bar_for_mouse.borrow_mut();     
        if *bar_hover_ref != bar_hit{
            *bar_hover_ref = bar_hit;
        }

    });
    canvas.add_event_listener_with_callback("mousemove", mouse_closure.as_ref().unchecked_ref())?;
    mouse_closure.forget();

    //animacijas loop, kas visu laiku(vai ari kad izmainas) zime chart ar atbilstosa frame vertibam
    let canvas_for_loop = canvas.clone();
    let geo_for_loop =geo.clone();
    let toplines_for_loop=toplines.clone();
    let bottomlines_for_loop=bottomlines.clone();
    let bars_for_loop = bars.clone();
    let anim_for_loop = anim_state.clone();
    let hover_bar_for_loop = hover_bar.clone();

    crate::animation::start_loop(move |_elapsed_ms| {
        let now = web_sys::window().unwrap().performance().unwrap().now();
        let heights: Vec<f64> = anim_for_loop.borrow().iter().map(|a| current_height(a, now)).collect();
        let hover_bar_value = *hover_bar_for_loop.borrow();
        let _ = draw_bar_chart(&canvas_for_loop, &bars_for_loop, &geo_for_loop, &toplines_for_loop, &bottomlines_for_loop, barchart.exponent, &heights, &hover_bar_value, &label_x, &label_y);
        true
    })?;

    

    Ok(())
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