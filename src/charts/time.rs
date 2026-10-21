use crate::data::{DataPoint, Points};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use jiff::civil::DateTime;

thread_local! {
    static CHARTS: RefCell<HashMap<String, TimeChartController>> = RefCell::new(HashMap::new());
}
pub fn push_chart(canvas_id: &str, ctrl: TimeChartController){
    CHARTS.with(|c| c.borrow_mut().insert(canvas_id.to_string(), ctrl));
}
pub fn destroy_chart(canvas_id: &str) {
    CHARTS.with(|c| c.borrow_mut().remove(canvas_id));
}

pub struct TimeChartController {
    state: Rc<RefCell<ChartState>>,
    mouse: Closure<dyn FnMut(web_sys::MouseEvent)>,
    click: Closure<dyn FnMut(web_sys::MouseEvent)>
}

struct Level { label: String, text_w: f64, interval_start: i64, interval_end: i64, time_step:i64 }

struct ChartState {
    canvas: web_sys::HtmlCanvasElement,
    config: TimeConfig,
    history: Vec<Level>,
    layout: TimeChart,
    anim_start: f64,
    hover_bar: Option<usize>,
    hover_title: Option<usize>,
    points: Vec<DataPoint>,
    dirty: bool,
    value_labels: Vec<String>,
    key_label: String,
    context: web_sys::CanvasRenderingContext2d,
}

impl ChartState{
    pub fn on_click(&mut self, mx: f64, my: f64) ->Result<(), wasm_bindgen::JsValue>{
        // if let Some(title_index) = self.hit_test_titles(mx, my) {
        //     if title_index < self.history.len() - 1 {
        //         self.history.truncate(title_index + 1);

        //         self.current_level()?;
        //     }
        // }else if let Some(i) = self.hit_test_bar(mx, my) {
        //     let bar=&self.layout.bars[i];
        //     if let Some(subs) = &bar.sub_points {
        //         self.history.push(Level {
        //             label: bar.label.clone(),
        //             text_w: self.measure(&bar.label)?,
        //             points: subs.clone(),
        //         });
        //         self.current_level()?;
        //     }
        // }
        Ok(())
    }
    pub fn current_level(&mut self)->Result<(), wasm_bindgen::JsValue>{
        // self.hover_bar=None;
        // self.hover_title=None;
        // let current_level = self.history.last().unwrap();
        // self.layout = compute_bars(current_level.points.clone(), &self.config);

        // self.anim_start = now_ms();
        // self.dirty = true;
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
        for (i, bar) in bars.iter().enumerate(){
            if
                mx>(bar.start_x)
                && 
                mx<(bar.start_x+bar.thickness)
                &&
                my>(geo.start_y)
            {
                return Some(i);
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

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct TimeConfig {
    pub label_column: usize,
    pub value_column: Vec<usize>,
    pub interval_start: i64, //no kada laika
    pub interval_end: i64, //lidz kuram laikam
    pub time_step: i64, //uz kādu laika posmu ir viens bar
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
    pub line_color: String,
    pub time_zone: String,

}
impl Points for TimeConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> &[usize] {
        &self.value_column
    }
}

//Noklusejuma vertibas, pirma kollona key, otra value
impl Default for TimeConfig {
    fn default() -> Self {
        TimeConfig { label_column: 0, value_column: vec![1], interval_start:0, interval_end:0,time_step:0, sub_color: true, sub_stack:true, start_x:50.0, start_y:50.0, height: 300.0, width: 400.0,  font_size: 10.0, font: String::from("10px sans-serif"), text_color: String::from("black"), hover_color: String::from("red"),bar_color: String::from("black"),line_color: String::from("gray"), time_zone:String::from("Europe/Riga")}
    }
}

impl TimeConfig {
    pub fn from_json(json: JsValue) -> Result<Self, JsValue> {
        let cfg: TimeConfig = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid TimeConfig JSON: {e}")))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), JsValue> {
        let err = |m: &str| Err(JsValue::from_str(m));
        if self.value_column.is_empty() {
            return err("value_column must contain at least one column index");
        }
        if self.width <= 0.0 || self.height <= 0.0 {
            return err("width and height must be positive");
        }
        if self.interval_end <= self.interval_start {
            return err("interval_end must be greater than interval_start");
        }
        if self.time_step <= 0 {
            return err("time_step must be positive");
        }
        Ok(())
    }
}

pub struct Bar{
    pub label: String,
    pub values: Vec<f64>,
    pub thickness: f64,
    pub length: f64,
    pub start_x: f64,
    pub start_y: f64,
}

pub struct TimePoint{
    time:i64,
    values:Vec<f64>,
}
pub struct TimeChart{
    pub bars: Vec<Bar>,
    pub top_lines: Vec<f64>,
    pub bottom_lines: Vec<f64>,
    pub exponent: f64,
    pub value_pixels: f64,
}

fn local_to_unix_ms(s: &str, tz: &str) -> Result<i64, jiff::Error> {
    let dt: DateTime = s.parse()?;
    Ok(dt.in_tz(tz)?.timestamp().as_millisecond())
}

pub fn compute_bars(data_points: &[DataPoint], config: &TimeConfig, interval_start:i64, interval_end:i64, time_step:i64)->Result<TimeChart, JsValue>{
    if interval_end <= interval_start {
        return Err(JsValue::from_str("interval_end must be greater than interval_start"));
    }
    if time_step <= 0 {
        return Err(JsValue::from_str("time_step must be positive"));
    }
    let duration = interval_end
        .checked_sub(interval_start)
        .ok_or_else(|| JsValue::from_str("time interval is too large"))?;

    let bucket_count = duration / time_step + i64::from(duration % time_step != 0);

    let bucket_count = usize::try_from(bucket_count)
        .map_err(|_| JsValue::from_str("time interval contains too many buckets"))?;
    
    let series_count = data_points
        .iter()
        .filter_map(|p| p.sub_points.as_ref().map(|s| s.len()))
        .max()
        .unwrap_or(0);

    let mut buckets = vec![vec![0.0; series_count]; bucket_count];

    for p in data_points {
        let Some(subs) = p.sub_points.as_ref() else { continue };
        let Ok(time) = local_to_unix_ms(&p.label, &config.time_zone) else { continue };
        if time < interval_start || time >= interval_end {
            continue;
        }
        let idx = ((time - interval_start) / time_step) as usize;
        for (acc, s) in buckets[idx].iter_mut().zip(subs) {
            *acc += s.value;
        }
    }

    let mut max_value = 0.0;
    let mut min_value = 0.0;

    let points: Vec<TimePoint> = buckets
        .into_iter()
        .enumerate()
        .map(|(i, values)|{
            let sum=values.iter().sum::<f64>();
            let min =values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max =values.iter().fold(f64::INFINITY, |a, &b| a.max(b));
            if sum>max_value{
                max_value=sum;
            }
            if sum<min_value{
                min_value=sum;
            }
            if max>max_value{
                max_value=max;
            }            
            if min<min_value{
                min_value=min;
            }
            TimePoint {time: interval_start + i as i64 * time_step, values }
        }).collect();


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
    let step_line_px=config.height/(step_amount_pos+ step_amount_neg);


    //cik pikseli ir viena datu mervieniba
    let one_px=step_line_px/step;

    let start_y=
        if min_value==0.0{
            config.start_y+config.height
        }else{
            config.start_y+config.height-step_amount_neg*step_line_px
        };

    
    let bars:Vec<Bar> = points.into_iter().enumerate().map(|(i, p)|{
            let thickness =config.width/point_count;
            let length= one_px*p.values.iter().sum::<f64>();
            let x =(i as f64 )*thickness+ config.start_x;

            let label=p.time.to_string();

            Bar{label: label, values: p.values, thickness, length, start_x: x, start_y: start_y}

        })
        .collect();
    
    let mut bottom_lines=vec![];
    let mut top_lines=vec![];
    let mut i=0.0;
    while i<=step_amount_pos{
        let line=start_y-i*step_line_px;
        let line_val = step*i;
        top_lines.push(line);
        top_lines.push(line_val);
        i+=1.0;
    }
    i=1.0;
    while i<=step_amount_neg{
        let line=start_y+i*step_line_px;
        let line_val = -step*i;
        bottom_lines.push(line);
        bottom_lines.push(line_val);
        i+=1.0;
    }
    Ok(TimeChart {bars, bottom_lines, top_lines, exponent, value_pixels: one_px})
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn palette_color(index: usize) -> &'static str {
    let colors = [
        "#FFCE56", "#4BC0C0", "#9966FF","#FF6384", "#36A2EB",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()]
}

fn draw_time_chart(state:&ChartState)->Result<(),JsValue>{
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

    let progress=state.current_progress(now_ms());

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
            context.fill_rect(bar.start_x, geo.start_y, bar.thickness, geo.height);
            context.set_global_alpha(1.0);
            context.fill_rect(bar.start_x, bar.start_y, bar.thickness, -progress*bar.length);
        }else{
            let values=&bar.values;
            if values.len()>1{
                if geo.sub_stack{
                    if geo.sub_color{
                        let mut last_y_pos=bar.start_y;
                        let mut last_y_neg=bar.start_y;
                        for (j, p) in values.iter().enumerate(){
                            context.set_fill_style_str(palette_color(j));
                            if *p>0.0{
                                context.fill_rect(bar.start_x, last_y_pos, bar.thickness, -progress*one_px*p);
                                last_y_pos-=progress*one_px*p;
                            }else{
                                context.fill_rect(bar.start_x, last_y_neg, bar.thickness, -progress*one_px*p);
                                last_y_neg-=progress*one_px*p;
                            }

                        }
            
                    }else{
                        context.fill_rect(bar.start_x, bar.start_y, bar.thickness, -progress*bar.length);
                    }
                }else{
                    if geo.sub_color{
                        let mut last_x=bar.start_x;
                        let thickness=bar.thickness/(values.len() as f64);
                        for (j, p) in values.iter().enumerate(){
                            context.set_fill_style_str(palette_color(j));
                            context.fill_rect(last_x, bar.start_y,  thickness, -progress*one_px*p);
                            last_x+=thickness;
                        }
                    }else{
                        let mut last_x=bar.start_x;
                        let thickness=bar.thickness/(values.len() as f64);
                        for p in values{
                            context.fill_rect(last_x, bar.start_y,  thickness, -progress*one_px*p);
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

        context.save();
        context.translate(bar.start_x+(bar.thickness/2.0), geo.start_y + geo.height + 0.3*geo.font_size)?;// move origin to where the text should be
        context.rotate(-std::f64::consts::PI / 2.0)?;
        context.set_text_align("right");
        context.set_text_baseline("middle");
        context.fill_text(&bar.label, 0.0, 0.0)?;
        context.restore();
        context.set_fill_style_str(bar_color);
    }

    Ok(())
}

pub fn render_time_chart(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, config: TimeConfig, value_labels: Vec<String>, key_label: String)->Result<TimeChartController, JsValue>{
    config.validate()?;
    let timechart = compute_bars(&points, &config, config.interval_start, config.interval_end, config.time_step)?;
    let canvas=crate::canvas::get_canvas(canvas_id)?;
    let context=crate::canvas::get_context(&canvas)?;
    context.set_font(&config.font);
    let width=context.measure_text(&chart_label)?.width();

    let state = Rc::new(RefCell::new(ChartState {
        canvas: canvas.clone(),
        points,
        history: vec![Level { label: chart_label.clone(), text_w:width, interval_start: config.interval_start, interval_end: config.interval_end, time_step: config.time_step }],
        config,
        layout: timechart,
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
        let _ = draw_time_chart(&st);
        st.dirty = animating;
        true
    })?;

    Ok(TimeChartController {state, mouse:mouse_closure, click: click_closure})
}