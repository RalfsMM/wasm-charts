use crate::data::{DataPoint, Points};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CHARTS: RefCell<HashMap<String, BarChartController>> = RefCell::new(HashMap::new());
}
pub fn push_chart(canvas_id: &str, ctrl: BarChartController){
    CHARTS.with(|c| c.borrow_mut().insert(canvas_id.to_string(), ctrl));
}
pub fn destroy_chart(canvas_id: &str) {
    CHARTS.with(|c| c.borrow_mut().remove(canvas_id));
}

struct Level { label: String, text_w: f64, points: Vec<DataPoint> }

struct ChartState {
    canvas: web_sys::HtmlCanvasElement,
    config: BarConfig,
    history: Vec<Level>,
    layout: BarChart,
    anim_start: f64,
    hover_bar: Option<usize>,
    hover_title: Option<usize>,
    dirty: bool,
    value_labels: Vec<String>,
    key_label: String,
    context: web_sys::CanvasRenderingContext2d,
}

impl ChartState{
    pub fn on_click(&mut self, mx: f64, my: f64) ->Result<(), wasm_bindgen::JsValue>{
        if let Some(title_index) = self.hit_test_titles(mx, my) {
            if title_index < self.history.len() - 1 {
                self.history.truncate(title_index + 1);

                self.current_level()?;
            }
        }else if let Some(i) = self.hit_test_bar(mx, my) {
            let bar=&self.layout.bars[i];
            if let Some(subs) = &bar.sub_points {
                self.history.push(Level {
                    label: bar.label.clone(),
                    text_w: self.measure(&bar.label)?,
                    points: subs.clone(),
                });
                self.current_level()?;
            }
        }
        Ok(())
    }
    pub fn current_level(&mut self)->Result<(), wasm_bindgen::JsValue>{
        self.hover_bar=None;
        self.hover_title=None;
        let current_level = self.history.last().unwrap();
        self.layout = compute_bars(current_level.points.clone(), &self.config);

        self.anim_start = now_ms();
        self.dirty = true;
        Ok(())
    }

    pub fn animating(&self, now: f64) -> bool {
        now - self.anim_start < 500.0
    }

    pub fn current_progress(&self, now: f64)->f64{
        let elapsed = now - self.anim_start;
        (elapsed / 500.0).clamp(0.0, 1.0)
    }
    pub fn hit_test_bar(&self, mx: f64, my: f64)->Option<usize>{
        let geo= &self.config;
        let bars=&self.layout.bars;
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
    fn hit_test_titles(&self, mx: f64, my: f64) -> Option<usize> {
        let geo=&self.config;

        if my<geo.start_y - 0.1*geo.height - geo.font_size||my>geo.start_y - 0.1*geo.height||mx<geo.start_x{
            return None;
        }

        let charts=&self.history;
        let mut indent=0.0;
        for (i,chart) in charts.iter().enumerate() {
            indent += chart.text_w+ geo.font_size/3.0;
            if mx< geo.start_x + indent{
                return Some(i);
            }
        }
        None
    }

    fn measure(&self, text: &str) -> Result<f64, JsValue> {
        self.context.set_font(&self.config.font);
        Ok(self.context.measure_text(text)?.width())
    }
}

#[wasm_bindgen]
pub struct BarChartController {
    state: Rc<RefCell<ChartState>>,
    mouse: Closure<dyn FnMut(web_sys::MouseEvent)>,
    click: Closure<dyn FnMut(web_sys::MouseEvent)>
}

//Konfiguracija Bar chartam, geo
#[derive(Clone, Deserialize)]
#[serde(default)]
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
    pub hover_color: String,
    pub bar_color: String,
    pub line_color: String
}
//Traits taa lai parsingaa var izmantot visu chartu configus
impl Points for BarConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> &[usize] {
        &self.value_column
    }
}

//Noklusejuma vertibas, pirma kollona key, otra value
impl Default for BarConfig {
    fn default() -> Self {
        BarConfig { label_column: 0, value_column: vec![1], vert: true, sort_asc: false, sort_desc: false, sub_color: true, sub_stack:true, start_x:50.0, start_y:50.0, height: 300.0, width: 400.0,  font_size: 10.0, font: String::from("10px sans-serif"), text_color: String::from("black"), hover_color: String::from("red"),bar_color: String::from("black"),line_color: String::from("gray")}
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
        Ok(())
    }
}

pub struct Bar{
    pub label: String,
    pub value: f64,
    pub thickness: f64,
    pub length: f64,
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
    pub value_pixels: f64,
}

fn palette_color(index: usize) -> &'static str {
    let colors = [
        "#FFCE56", "#4BC0C0", "#9966FF","#FF6384", "#36A2EB",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()]
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

pub fn compute_bars( mut points: Vec<DataPoint>, config: &BarConfig) -> BarChart{
    if config.sort_asc{
        points.sort_by(|a, b| a.value.total_cmp(&b.value));
    }
    if config.sort_desc{
        points.sort_by(|a, b| b.value.total_cmp(&a.value));
    }

    let mut max_value = 0.0;
    let mut min_value =0.0;
    points.iter().for_each(|point| {
        if let Some(subpoints)=&point.sub_points{
            subpoints.iter().for_each(|p| {
                if p.value<min_value{
                    min_value=p.value;
                }
                if p.value>max_value{
                    max_value=p.value;
                }
            });
        }
        if point.value<min_value{
            min_value=point.value;
        }
        if point.value>max_value{
            max_value=point.value;
        }
    });

    let point_count = points.len() as f64;
    let max_abs_value=max_value.abs().max(min_value.abs());
    

    //kada desmit pakape ir lielakajam zinatniska pierakstaa skaitla modulim no datiem
    let exponent = if max_abs_value==0.0{
        1.0
    }else{
        max_abs_value.abs().log10().floor()
    };

    let step_fract= max_abs_value/10.0_f64.powf(exponent+1.0);

    let step =
    if step_fract<0.15 {
        0.1*10.0_f64.powf(exponent)
    }else if step_fract<0.5{
        0.20*10.0_f64.powf(exponent)
    }else if step_fract<0.75{
        0.5*10.0_f64.powf(exponent)
    }else{
        1.0*10.0_f64.powf(exponent)
    };
    

    //cik iedalas bus charta uz pozitivo virzienu
    let step_amount_pos =(max_value/step).ceil()+2.0;
    //cik iedalas bus charta uz negativo virzienu
    let step_amount_neg=
    if min_value != 0.0 {
        (min_value.abs()/step).ceil()
    }else{
        0.0
    };

    //iedalu linijas atstarpes
    let step_line_px=
        if config.vert{
            config.height/(step_amount_pos+ step_amount_neg)
        }else{
            config.width/(step_amount_pos+ step_amount_neg)
        };

    //cik pikseli ir viena datu mervieniba
    let one_px=step_line_px/step;

    let start=
    if config.vert{
        if min_value==0.0{
            config.start_y+config.height
        }else{
            config.start_y+config.height-step_amount_neg*step_line_px
        }
    }else{
        if min_value==0.0{
            config.start_x
        }else{
            config.start_x + step_amount_neg*step_line_px
        }
    };

    
    let bars:Vec<Bar> = points.into_iter().enumerate().map(|(i, p)|{
            let part = if config.vert{
                config.width/point_count
            }else{
                config.height/point_count
            };
            let thickness= 0.6*part;
            let length= one_px*p.value;
            let x =(i as f64 )* part + part*0.2+ config.start_x;
            let y =(i as f64 )* part + part*0.2+ config.start_y;

            let sub_points = if p.sub_points.is_some() && !p.sub_points.as_ref().unwrap().is_empty() {
                p.sub_points
            } else {
                None
            };
            if config.vert{
                Bar{label: p.label, value: p.value, thickness, length, start_x: x, start_y: start, sub_points, part}
            }else{
                Bar{label: p.label, value: p.value, thickness, length, start_x: start, start_y: y, sub_points, part}
            }
        })
        .collect();
    let mut bottom_lines=vec![];
    let mut top_lines=vec![];
    let k=if config.vert{-1.0}else{1.0};
    let mut i=0.0;
    while i<=step_amount_pos{
        let line=start+k*i*step_line_px;
        let line_val = step*i;
        top_lines.push(line);
        top_lines.push(line_val);
        i+=1.0;
    }
    i=1.0;
    while i<=step_amount_neg{
        let line=start-k*i*step_line_px;
        let line_val = -step*i;
        bottom_lines.push(line);
        bottom_lines.push(line_val);
        i+=1.0;
    }
    BarChart {bars, bottom_lines, top_lines, exponent, value_pixels: one_px}
}

fn draw_bar_chart( state:&ChartState, now: f64 )->Result<(), JsValue>{
    let context = &state.context;
    let canvas =&state.canvas;
    let geo =&state.config;
    let value_labels=&state.value_labels;
    let title_hover=state.hover_title;
    let key_label=&state.key_label;
    let bar_hover=state.hover_bar;
    let top_lines= &state.layout.top_lines;
    let bottom_lines=&state.layout.bottom_lines;
    let exponent =state.layout.exponent;
    let bars=&state.layout.bars;
    let one_px=state.layout.value_pixels;

    let progress=state.current_progress(now);

    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    let font_size = geo.font_size;
    let font=geo.font.as_str();
    let text_color =geo.text_color.as_str();
    let hover_color= geo.hover_color.as_str();
    let bar_color = geo.bar_color.as_str();
    let line_color=geo.line_color.as_str();
    let all_val_lab= value_labels.join("  ");


    context.set_fill_style_str(text_color);
    context.set_font(font);
    context.set_text_align("right");
    context.set_text_baseline("bottom");

    let charts = &state.history;
    let mut indent= 0.0;
    for (i, chart) in charts.iter().enumerate() {
        if title_hover == Some(i) {
            context.set_fill_style_str(hover_color);
        } else {
            context.set_fill_style_str(text_color);
        }
        let label;
        if indent>0.0{
            label = format!("/{}", chart.label);
            indent +=font_size/3.0;
        }else{
            label = chart.label.clone();
        }
        indent += chart.text_w;
        context
            .fill_text(&label, geo.start_x + indent, geo.start_y - 0.1*geo.height)
            .map_err(|_| JsValue::from_str("failed to draw chart label"))?;
    }
    context.set_fill_style_str(text_color);
    context.set_text_align("left");

    if geo.vert{
        context.set_text_baseline("bottom");
        context.fill_text(&all_val_lab, geo.start_x, geo.start_y-font_size)?;
        context.fill_text(key_label, geo.start_x+geo.width, geo.start_y+geo.height)?;
        context.set_text_baseline("top");
        
        if geo.sub_color && value_labels.len()>1{
             for (i,lab) in value_labels.iter().enumerate(){
                context.set_fill_style_str(palette_color(i));
                context.fill_rect(geo.start_x + geo.width+ geo.font_size, geo.start_y+2.0*geo.font_size*(i as f64), geo.font_size, geo.font_size);
                context.set_fill_style_str(text_color);
                context.fill_text(lab, geo.start_x+geo.width + 3.0*geo.font_size, geo.start_y+2.0*geo.font_size*(i as f64))?;
             }
        }

        context.begin_path();
        context.set_fill_style_str(line_color);
        context.set_text_baseline("middle");
        context.set_text_align("right");
        context.set_line_width(0.5);

        let mut i =0;
        while i<top_lines.len(){
            context.move_to(geo.start_x, top_lines[i]);
            context.line_to(geo.start_x+geo.width, top_lines[i]);
            let exp=
            if exponent>0.0{
                0
            }else if exponent ==0.0{
                1
            }else{
                -exponent as usize
            };
            let label=format!("{:.exp$}  ",top_lines[i+1]);
            context.fill_text(&label, geo.start_x, top_lines[i])?;
            i+=2;
        }
        i =0;
        while i<bottom_lines.len(){
            context.move_to(geo.start_x, bottom_lines[i]);
            context.line_to(geo.start_x+geo.width, bottom_lines[i]);
            let exp=
            if exponent>0.0{
                0
            }else if exponent ==0.0{
                1
            }else{
                -exponent as usize
            };
            let label=format!("{:.exp$}  ",bottom_lines[i+1]);
            context.fill_text(&label, geo.start_x, bottom_lines[i])?;
            i+=2;
        }
        context.stroke();
        context.set_fill_style_str(bar_color);

        for (i, bar) in bars.iter().enumerate(){
            if bar_hover==Some(i){
                context.set_fill_style_str(hover_color);
                context.set_global_alpha(0.1);
                context.fill_rect(bar.start_x-bar.part*0.2, geo.start_y, bar.part, geo.height);
                context.set_global_alpha(1.0);
                context.fill_rect(bar.start_x, bar.start_y, bar.thickness, -progress*bar.length);
            }else{
                if let Some(subs)=&bar.sub_points{
                    if geo.sub_stack{
                        if geo.sub_color{
                            let mut last_y_pos=bar.start_y;
                            let mut last_y_neg=bar.start_y;
                            for (j, p) in subs.iter().enumerate(){
                                context.set_fill_style_str(palette_color(j));
                                if p.value>0.0{
                                    context.fill_rect(bar.start_x, last_y_pos, bar.thickness, -progress*one_px*p.value);
                                    last_y_pos-=progress*one_px*p.value;
                                }else{
                                    context.fill_rect(bar.start_x, last_y_neg, bar.thickness, -progress*one_px*p.value);
                                    last_y_neg-=progress*one_px*p.value;
                                }

                            }
                
                        }else{
                            context.fill_rect(bar.start_x, bar.start_y, bar.thickness, -progress*bar.length);
                        }
                    }else{
                        if geo.sub_color{
                            let mut last_x=bar.start_x;
                            let thickness=bar.thickness/(subs.len() as f64);
                            for (j, p) in subs.iter().enumerate(){
                                context.set_fill_style_str(palette_color(j));
                                context.fill_rect(last_x, bar.start_y,  thickness, -progress*one_px*p.value);
                                last_x+=thickness;
                            }
                        }else{
                            let mut last_x=bar.start_x;
                            let thickness=bar.thickness/(subs.len() as f64);
                            for p in subs{
                                context.fill_rect(last_x, bar.start_y,  thickness, -progress*one_px*p.value);
                                last_x+=thickness;
                            }
                        }
                    }
                }else{
                    if geo.sub_color && state.history.len()>1{
                        context.set_fill_style_str(palette_color(i));
                    }
                    context.fill_rect(bar.start_x, bar.start_y, bar.thickness, -progress*bar.length);
                }
                context.set_fill_style_str(text_color);
            }

            
            // let value=format!("{}",bar.value);
            // if lengths[i]>0.0{
            //     context.set_text_baseline("bottom");
            // }else{
            //     context.set_text_baseline("top");
            // }
            // context.set_text_align("middle");
            // context.fill_text(&value, bar.start_x+bar.width/1.2, bar.start_y -lengths[i])?;

            context.save();
            context.translate(bar.start_x+(bar.thickness/2.0), geo.start_y + geo.height + 0.3*geo.font_size)?;// move origin to where the text should be
            context.rotate(-std::f64::consts::PI / 2.0)?;
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.fill_text(&bar.label, 0.0, 0.0)?;
            context.restore();
            context.set_fill_style_str(bar_color);
        }

    }else{
        context.set_text_baseline("top");
        context.fill_text(key_label, geo.start_x, geo.start_y+geo.height)?;

        for (i,lab) in value_labels.iter().enumerate(){
            let mut gap=geo.font_size;
            if geo.sub_color && value_labels.len()>1{
                context.set_fill_style_str(palette_color(i));
                context.fill_rect(geo.start_x + geo.width+ geo.font_size, geo.start_y+2.0*geo.font_size*(i as f64), geo.font_size, geo.font_size);
                gap *= 3.0;
            }
            context.set_fill_style_str(text_color);
            context.fill_text(lab, geo.start_x+geo.width + gap, geo.start_y+2.0*geo.font_size*(i as f64))?;
        }
            
        context.begin_path();
        context.set_fill_style_str(line_color);
        context.set_text_baseline("bottom");
        context.set_text_align("center");
        context.set_line_width(0.5);

        let mut i =0;
        while i<top_lines.len(){
            context.move_to( top_lines[i],geo.start_y);
            context.line_to(top_lines[i], geo.start_y+geo.height);
            let exp=
            if exponent>0.0{
                0
            }else if exponent ==0.0{
                1
            }else{
                -exponent as usize
            };
            let label=format!("{:.exp$} ",top_lines[i+1]);

            context.save();
            context.translate(top_lines[i], geo.start_y)?;
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
            let exp=
            if exponent>0.0{
                0
            }else if exponent ==0.0{
                1    
            }else{
                -exponent as usize
            };
            let label=format!("{:.exp$} ",bottom_lines[i+1]);

            context.save();
            context.translate(bottom_lines[i], geo.start_y)?;
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
            if bar_hover==Some(i){
                context.set_fill_style_str(hover_color);
                context.set_global_alpha(0.1);
                context.fill_rect(geo.start_x,bar.start_y-bar.part*0.2, geo.width, bar.part);
                context.set_global_alpha(1.0);
                context.fill_rect(bar.start_x, bar.start_y, progress*bar.length, bar.thickness);
            }else{
                if let Some(subs) = &bar.sub_points{
                    if geo.sub_stack{
                        if geo.sub_color{
                            let mut last_x_pos=bar.start_x;
                            let mut last_x_neg=bar.start_x;
                            for (j, p) in subs.iter().enumerate(){
                                context.set_fill_style_str(palette_color(j));
                                if p.value>0.0{
                                    context.fill_rect(last_x_pos,bar.start_y, progress*one_px*p.value,bar.thickness,);
                                    last_x_pos+=progress*one_px*p.value
                                }else{
                                    context.fill_rect(last_x_neg, bar.start_y, progress*one_px*p.value, bar.thickness);
                                    last_x_neg+=progress*one_px*p.value;
                                }
                            }
                        }else{
                            context.fill_rect(bar.start_x, bar.start_y, progress*bar.length, bar.thickness);
                        }
                    }else{
                        if geo.sub_color{
                            let mut last_y=bar.start_y;
                            let thickness=bar.thickness/(subs.len() as f64);
                            for (j, p) in subs.iter().enumerate(){
                                context.set_fill_style_str(palette_color(j));
                                context.fill_rect(bar.start_x, last_y,  progress*one_px*p.value, thickness);
                                last_y+=thickness;
                            }
                        }else{
                            let mut last_y=bar.start_y;
                            let thickness=bar.thickness/(subs.len() as f64);
                            for p in subs{
                                context.fill_rect(bar.start_x, last_y, progress*p.value*p.value, thickness);
                                last_y+=thickness;
                            }
                        }
                    }
                }else{
                    if geo.sub_color && state.history.len()>1{
                        context.set_fill_style_str(palette_color(i));
                    }
                    context.fill_rect(bar.start_x, bar.start_y, progress*bar.length, bar.thickness);
                }
                context.set_fill_style_str(text_color);
            }
            context.set_text_align("right");
            context.set_text_baseline("middle");
            context.fill_text(&bar.label, geo.start_x-geo.font_size*0.3, bar.start_y+(bar.thickness/2.0))?;
            context.set_fill_style_str(bar_color);
        }

    }           
    Ok(())
}

pub fn render_bar_chart(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, config: BarConfig, value_labels: Vec<String>, key_label: String) -> Result<BarChartController, JsValue> {
    let barchart=compute_bars(points.clone(), &config);
    let canvas=crate::canvas::get_canvas(canvas_id)?;
    let context=crate::canvas::get_context(&canvas)?;
    context.set_font(&config.font);
    let width=context.measure_text(&chart_label)?.width();
    
    let state = Rc::new(RefCell::new(ChartState {
        canvas: canvas.clone(),
        config,
        history: vec![Level { label: chart_label.clone(), text_w:width, points }],
        layout: barchart,
        anim_start: web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .ok_or_else(|| JsValue::from_str("no performance"))?,
        hover_bar: None,
        hover_title: None,
        dirty: true,
        value_labels,
        key_label,
        context,
    }));


    let state_for_mouse = state.clone();

    let mouse_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = state_for_mouse.borrow().canvas.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();

        //parbauda vai hover uz bar
        let mut st = state_for_mouse.borrow_mut();
        let bar_hit = st.hit_test_bar(mx, my);
        if st.hover_bar != bar_hit {
            st.hover_bar = bar_hit;
            st.dirty = true;
        }

        //parbauda vai hover uz title
        let title_hit = st.hit_test_titles(mx, my);
        if st.hover_title != title_hit {
            st.hover_title = title_hit;
            st.dirty = true;
        }

    });
    canvas.add_event_listener_with_callback("mousemove", mouse_closure.as_ref().unchecked_ref())?;

    //mousedown listener un handler
    let state_for_click = state.clone();

    let click_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = state_for_click.borrow().canvas.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();
        let _ = state_for_click.borrow_mut().on_click(mx, my);
    });
    canvas.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())?;

    let weak = Rc::downgrade(&state);
    crate::animation::start_loop(move |_| {
        let Some(state) = weak.upgrade() else { return false };
        let mut st = state.borrow_mut();
        let now = now_ms();
        let animating = st.animating(now);
        if !st.dirty && !animating { return true; }
        let _ = draw_bar_chart(&st, now);
        st.dirty = animating;
        true
    })?;

    Ok(BarChartController {state, mouse: mouse_closure, click: click_closure})
}
