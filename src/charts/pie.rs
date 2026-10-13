use crate::data::{DataPoint, Points};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CHARTS: RefCell<HashMap<String, PieChartController>> = RefCell::new(HashMap::new());
}
pub fn push_chart(canvas_id: &str, ctrl: PieChartController){
    CHARTS.with(|c| c.borrow_mut().insert(canvas_id.to_string(), ctrl));
}
pub fn destroy_chart(canvas_id: &str) {
    CHARTS.with(|c| c.borrow_mut().remove(canvas_id));
}

struct Level { label: String, text_w: f64, fraction:f64, points: Vec<DataPoint> }

//glaba radiusa animacijas datus sektoram
#[derive(Clone, Copy)]
struct SliceAnimRadius {
    start_r: f64,
    target_r: f64,
    start_time: f64,
}

//glaba lenka animacijas datus sektoram
#[derive(Clone, Copy)]
struct SliceAnimAngle {
    start_a: f64,
    target_a: f64,
    order: usize,
    start_time: f64,
    duration_ms: f64,
}

pub struct ChartState {
    canvas: web_sys::HtmlCanvasElement,
    config: PieConfig,
    history: Vec<Level>,
    slices: Vec<PieSlice>,
    radii: Vec<f64>,
    angles: Vec<[f64; 2]>,
    anim_angles: Vec<SliceAnimAngle>,
    anim_radii: Vec<SliceAnimRadius>,
    hover_slice: Option<usize>,
    hover_title: Option<usize>,
    current_order:usize,
    dirty: bool,
    context: web_sys::CanvasRenderingContext2d,
}

impl ChartState{
    pub fn on_click(&mut self, mx:f64, my: f64)-> Result<(), wasm_bindgen::JsValue>{
        if let Some(title_index) = self.hit_test_titles(mx, my) {
            if title_index < self.history.len() - 1 {
                self.history.truncate(title_index + 1);
                self.current_level()?;
            }
        }else if let Some(i) = self.hit_test_slice(mx, my) {
            let slice=&self.slices[i];
            if let Some(subs) = &slice.sub_points {
                self.history.push(Level {
                    label: slice.label.clone(),
                    text_w: self.measure(&slice.label)?,
                    fraction:slice.percent,
                    points: subs.clone(),
                });
                self.current_level()?;
            }
        }
        Ok(())
    }
    pub fn current_level(&mut self)->Result<(), wasm_bindgen::JsValue>{
        self.hover_slice=None;
        self.hover_title=None;
        let current_level = self.history.last().unwrap();

        self.slices=compute_pie_slices(current_level.points.clone(), current_level.fraction)?;
        self.radii=self.slices.iter().map(|_|self.config.outer_r).collect();
        self.angles=self.slices.iter().map(|a|[a.angle_start, a.angle_end]).collect();
        self.anim_angles=self.slices.iter().enumerate()
            .map(|(i, slice)| SliceAnimAngle {
                start_a: slice.angle_start,
                target_a: slice.angle_end,
                start_time: now_ms(),
                order: i,
                duration_ms: self.config.entrance_dur/self.slices.len() as f64,
            }).collect();
        self.anim_radii=self.slices.iter()
            .map(|_| SliceAnimRadius { start_r: self.config.outer_r, target_r: self.config.outer_r, start_time: now_ms()})
            .collect();
        self.current_order=0;
        self.dirty=true;

        Ok(())
    }

    fn on_hover(&mut self, mx: f64, my: f64)-> Result<(), wasm_bindgen::JsValue>{
        let title_hit=self.hit_test_titles(mx, my);
        if title_hit != self.hover_title{
            self.hover_title = title_hit;
            self.dirty = true;
            return Ok(());
        }
        let slice_hit=self.hit_test_slice(mx, my);
        if slice_hit != self.hover_slice{
            self.hover_slice = slice_hit;
            self.dirty = true;
            let now = now_ms();
            for (i, anim) in self.anim_radii.iter_mut().enumerate() {
                let new_target = 
                    if slice_hit == Some(i) { 
                        self.config.outer_r + self.config.inner_r_diff/2.0 
                    } else { 
                        self.config.outer_r 
                    };
                let changed = anim.target_r != new_target;
                if changed {
                    anim.start_r = current_radius(anim);
                    anim.target_r = new_target;
                    anim.start_time = now;
                }
            }
            return Ok(());
        }
        self.dirty=false;
        Ok(())
    }

    //aprekina lenki prieks animacijas
    fn current_angle(&self, anim: &SliceAnimAngle) -> f64 {
        let order = self.current_order;
        if anim.order < order {
            anim.target_a
        } else if anim.order > order {
            anim.start_a + 0.04
        } else {
            let elapsed = now_ms() - anim.start_time;
            let t = (elapsed / anim.duration_ms).clamp(0.0, 1.0);
            anim.start_a + (anim.target_a - anim.start_a) * t
        }
    }
    //aprekina kopejo lenki prieks animacijas
    fn current_angle_progress(&self, anim: &SliceAnimAngle)-> f64{
        let now_ms=now_ms();
        let elapsed = now_ms - anim.start_time;
        (elapsed / anim.duration_ms).clamp(0.0, 1.0)
    }
    //parbauda vai padotas koordinatas ir uz kada slice, un atgriez uz kura ja ir.
    pub fn hit_test_slice(&self, mx: f64, my: f64) -> Option<usize> {
        let geo= &self.config;
        let slices=&self.slices;
        let dx = mx - geo.cx;
        let dy = my - geo.cy;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < (geo.outer_r - geo.inner_r_diff)|| dist > geo.outer_r + geo.inner_r_diff/2.0 {
            return None;
        }

        let mut angle = dy.atan2(dx);
        if angle < -std::f64::consts::PI / 2.0 {
            angle += std::f64::consts::TAU;
        }

        slices.iter().position(|s| angle >= s.angle_start && angle < s.angle_end)
    }
    //parbauda vai padotas koordinatas ir uz kada title, un atgriez uz kura ja ir
    pub fn hit_test_titles(&self, mx: f64, my: f64) -> Option<usize> {
        let geo= &self.config;
        if my<geo.cy - 1.6*geo.outer_r - geo.font_size||my>geo.cy - 1.6*geo.outer_r||mx<geo.cx - 1.6*geo.outer_r{
            return None;
        }

        let charts=&self.history;
        let mut indent=0.0;
        for (i,chart) in charts.iter().enumerate() {
            indent += chart.text_w+ geo.font_size/3.0;
            if mx< geo.cx - 1.6*geo.outer_r + indent{
                return Some(i);
            }
        }
        None
    }
    fn measure(&self, text: &str) -> Result<f64, JsValue> {
        self.context.set_font(&self.config.font);
        Ok(self.context.measure_text(text)?.width())
    }
    pub fn animating_angles(&self, now: f64) -> bool {
        true
        // now - self.anim_angles[0].start_time < (self.config.entrance_dur+(self.anim_angles.len()as f64)*15.0)
    }
    pub fn animating_radii(&self, now: f64)->bool{
        true
        // let mut res=false;
        // for a in self.anim_radii.iter(){
        //     res= now- a.start_time>200.0;
        // }
        // res
    }
}

//Konfiguracija Piechartam, geo
#[derive(Clone, Deserialize)]
pub struct PieConfig {
    pub label_column: usize,
    pub value_column: Vec<usize>,
    pub cx: f64,
    pub cy: f64,
    pub outer_r: f64,
    pub inner_r_diff: f64,
    pub font_size: f64,
    pub font: String,
    pub middle_font_size: f64,
    pub middle_font: String,
    pub text_color: String,
    pub hover_title_color: String,
    pub entrance_dur: f64,
}

//Traits taa lai parsingaa var izmantot visu chartu configus
impl Points for PieConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> &[usize] {
        &self.value_column
    }
}

//Noklusejuma vertibas, pirma kollona key, otra value
impl Default for PieConfig {
    fn default() -> Self {
        PieConfig { label_column: 0, value_column: vec![1], cx: 200.0, cy: 200.0, outer_r: 100.0, inner_r_diff: 20.0, font_size: 10.0, font: String::from("10px sans-serif"), middle_font_size: 54.0, middle_font: String::from("54px sans-serif"), text_color: String::from("black"), hover_title_color: String::from("blue"), entrance_dur:500.0 }
    }
}

impl PieConfig {
    pub fn from_json(json: JsValue) -> Result<Self, JsValue> {
        let cfg: PieConfig = serde_wasm_bindgen::from_value(json)
            .map_err(|e| JsValue::from_str(&format!("Invalid PieConfig JSON: {e}")))?;
        cfg.validate()?;
        Ok(cfg)
    }

    //todo izveidot pareizu validaciju
    fn validate(&self) -> Result<(), JsValue> {
        let err = |m: &str| Err(JsValue::from_str(m));
        if self.value_column.is_empty() {
            return err("value_column must contain at least one column index");
        }
        if self.cx <= 0.0 || self.cy <= 0.0 {
            return err("center coordinates must be positive");
        }
        if self.outer_r <= 0.0 {
            return err("outer radius must be positive");
        }
        if self.inner_r_diff <= 0.0 || self.inner_r_diff >= self.outer_r {
            return err("inner radius difference must be positive and less than outer radius");
        }
        Ok(())
    }
}

//sektors, ar lenkiem un apaksvertibam
#[derive(Clone)]
pub struct PieSlice {
    pub label: String,
    pub angle_start: f64,
    pub angle_end: f64,
    pub color: &'static str,
    pub percent: f64,
    pub sub_points: Option<Vec<DataPoint>>, // optional sub-points for drill-down
}

//funkcija kas nosaka sektoru kraasas atkariba no index
fn palette_color(index: usize) -> &'static str {
    let colors = [
        "#FF6384", "#36A2EB", "#FFCE56", "#4BC0C0", "#9966FF",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()]
}

//Info par veselu chart, domats prieks drilldown, jo tad no vienas tabulas ir vairaki charti, so te info par tiem
#[wasm_bindgen]
pub struct PieChartController {
    state: Rc<RefCell<ChartState>>,
    mouse: Closure<dyn FnMut(web_sys::MouseEvent)>,
    click: Closure<dyn FnMut(web_sys::MouseEvent)>
}

//Dabuu datapoints, sorto un partaisa sektoros ar aprekinatiem lenkiem, utt
pub fn compute_pie_slices(mut points: Vec<DataPoint>, fraction: f64) -> Result<Vec<PieSlice>, JsValue> {
    if points.iter().any(|p| p.value < 0.0) {
        return Err(JsValue::from_str("negative values are not allowed in pie chart"));
    }
    points.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));

    let total: f64 = points.iter().map(|p| p.value).sum();
    let point_count = points.len() as f64;
    let mut angle = -std::f64::consts::PI / 2.0;

    //mazakie slices salikti kopaa, kaa others
    let mut others: PieSlice = PieSlice { label: "Others".to_string(), angle_start: -std::f64::consts::PI / 2.0, angle_end: 0.0, color: "#808080", percent: 0.0, sub_points: None };

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
                percent = (p.value / total) * fraction;//reiz superslice procenti
            }else{
                sweep = (1.0 / point_count) * std::f64::consts::TAU;// ja visas values ir 0 tad visi sektori vienadi
                percent = 0.0;
            }

            let start = angle;
            angle += sweep;

            if (percent > 0.0) && (others.percent + percent < 0.1 * fraction) {//sektoru pievieno others, ja tas atbilst siem nosacijumiem
                others.angle_end= angle;               
                others.percent += percent;
                if others.sub_points.is_none() {
                    others.sub_points = Some(vec![]);
                }
                others.sub_points.as_mut().unwrap().push(p);
                return None;
            }
            let subpoints = if p.sub_points.is_some() && !p.sub_points.as_ref().unwrap().is_empty() {
                p.sub_points
            } else {
                None
            };

            Some(PieSlice { label: p.label, angle_start: start, angle_end: angle, color: palette_color(i), percent, sub_points: subpoints })
        })
        .collect();

    if others.percent > 0.0 {
        slices.insert(0, others);//others pieliek saakumaa
    }
    Ok(slices)
}

//uzzime pie atbilstosi lenkiem, radiusiem utt. So izmanto independently no pie stavokla
pub fn draw_pie(state:&ChartState) -> Result<(), JsValue> {
    let context = &state.context;
    let canvas =&state.canvas;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    
    let geo=&state.config;
    let title_hover=state.hover_title;
    let slices=&state.slices;
    let radii=&state.radii;
    let angles=&state.angles;
    let slice_hover=state.hover_slice;

    let default_size = geo.font_size;
    let default_font=geo.font.as_str();
    let middle_size = geo.middle_font_size;
    let middle_font =geo.middle_font.as_str();
    let text_color =geo.text_color.as_str();
    let hover_title_color = geo.hover_title_color.as_str();

    context.set_font(default_font);
    context.set_fill_style_str(text_color);
    context.set_text_align("right");

    let charts = &state.history;
    let mut indent= 0.0;
    for (i, chart) in charts.iter().enumerate() {
        if title_hover == Some(i) {
            context.set_fill_style_str(hover_title_color);
        } else {
            context.set_fill_style_str(text_color);
        }
        let label;
        if indent>0.0{
            label = format!("/{}", chart.label);
            indent +=default_size/3.0;
        }else{
            label = chart.label.clone();
        }
        indent += chart.text_w;
        context
            .fill_text(&label, geo.cx - 1.6*geo.outer_r + indent, geo.cy - 1.6*geo.outer_r)
            .map_err(|_| JsValue::from_str("failed to draw chart label"))?;
    }


    for (i, slice) in slices.iter().enumerate() {
        let outer_r = radii[i];
        let inner_r = outer_r -geo.inner_r_diff;
        let start_angle=angles[i][0];
        let end_angle= angles[i][1];

        context.begin_path();
        context.move_to(geo.cx, geo.cy);
        context
            .arc(geo.cx, geo.cy, outer_r, start_angle + 0.02, end_angle - 0.02)
            .map_err(|_| JsValue::from_str("failed to draw arc"))?;
        context
            .arc_with_anticlockwise(geo.cx, geo.cy, inner_r, end_angle - 0.02, start_angle + 0.02, true)
            .map_err(|_| JsValue::from_str("failed to draw arc"))?;
        context.close_path();
        context.set_fill_style_str(slice.color);
        context.fill();

        if start_angle != end_angle -0.04 {
            context.set_fill_style_str(text_color);
            context.set_font(default_font);

            let mid_angle = (start_angle + end_angle) / 2.0;
            let label_x=
            if !(std::f64::consts::PI / 2.0..=std::f64::consts::PI * 3.0 / 2.0).contains(&mid_angle) {
                context.set_text_align("left");
                outer_r + geo.cx + geo.outer_r/5.0
            } else {
                context.set_text_align("right");
                geo.cx - outer_r - geo.outer_r/5.0
            };
            let label_y = geo.cy + outer_r * mid_angle.sin();
            context
                .fill_text(&slice.label, label_x, label_y)
                .map_err(|_| JsValue::from_str("failed to draw text"))?;
        }
        if slice_hover == Some(i) {
            context.set_font(middle_font);
            context.set_text_align("center");
            context.set_fill_style_str(text_color);
            let opacity = ((outer_r - geo.outer_r) / (geo.inner_r_diff / 2.0)).clamp(0.0, 1.0);
            context.set_global_alpha(opacity);
            let percent = format!("{:.1}%", slice.percent);
            context
                .fill_text(&percent, geo.cx, geo.cy+ middle_size/3.0)
                .map_err(|_| JsValue::from_str("failed to draw text"))?;
            context.set_global_alpha(1.0);
        }
    }
    //context.begin_path();
    //context.move_to(0.0,(canvas.height() as f64)/2);
    //context.line_to(canvas.width() as f64,(canvas.height() as f64)/2);
    //context.move_to((canvas.width() as f64)/2,0.0);
    //context.line_to((canvas.width() as f64)/2, canvas.height() as f64);
    //context.stroke();
    
    Ok(())
}

pub fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

//aprekina radiusu prieks animacijas
fn current_radius(anim: &SliceAnimRadius) -> f64 {
    let now_ms=now_ms();
    let elapsed = now_ms - anim.start_time;
    let t = (elapsed / 200.0).clamp(0.0, 1.0);
    let eased=1.0 - (1.0 - t).powi(3);
    anim.start_r + (anim.target_r - anim.start_r) * eased
}
//uzzime pie ar animacijam un palaiz mousemove un mousedown listeners, un handlo izmainas
pub fn render_pie_chart(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, config: PieConfig) -> Result<PieChartController, JsValue> {
    let slices = compute_pie_slices(points.clone(), 100.0)?;
    let slices_clone=slices.clone();
    let canvas = crate::canvas::get_canvas(canvas_id)?;
    let context=crate::canvas::get_context(&canvas)?;
    context.set_font(&config.font);
    let width=context.measure_text(&chart_label)?.width();
    let now_ms = now_ms();
    let config_clone=config.clone();

    let state= Rc::new(RefCell::new(ChartState {
        canvas: canvas.clone(),
        config,
        history: vec![Level { label: chart_label.clone(), text_w:width, fraction: 100.0 , points }],
        slices,
        radii: slices_clone.iter().map(|_|config_clone.outer_r).collect(),
        angles: slices_clone.iter().map(|a|[a.angle_start, a.angle_end]).collect(),
        anim_angles: slices_clone.iter().enumerate()
            .map(|(i, slice)| SliceAnimAngle {
                start_a: slice.angle_start,
                target_a: slice.angle_end,
                start_time: now_ms,
                order: i,
                duration_ms: config_clone.entrance_dur/slices_clone.len() as f64,
            }).collect(),
        anim_radii: slices_clone.iter()
            .map(|_| SliceAnimRadius { start_r: config_clone.outer_r, target_r: config_clone.outer_r, start_time: now_ms})
            .collect(),
        hover_slice: None,
        hover_title: None,
        current_order:0,
        dirty: true,
        context,
    }));

    //mousemove listener un handler
    let state_for_mouse=state.clone();

    let mouse_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = state_for_mouse.borrow().canvas.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();
        let _  = state_for_mouse.borrow_mut().on_hover(mx, my);
    });
    canvas.add_event_listener_with_callback("mousemove", mouse_closure.as_ref().unchecked_ref())?;

    //mousedown listener un handler
    let state_for_click=state.clone();
    
    let click_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = state_for_click.borrow().canvas.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();
        let _ = state_for_click.borrow_mut().on_click(mx,my);
    });
    canvas.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())?;

    let weak = Rc::downgrade(&state);
    crate::animation::start_loop(move |_| {
        let Some(state) = weak.upgrade() else { return false };
        let mut st= state.borrow_mut();
        let now = web_sys::window().unwrap().performance().unwrap().now();
        let animating =st.animating_angles(now)||st.animating_radii(now);
        if !st.dirty && !animating { return true; }

        let now = web_sys::window().unwrap().performance().unwrap().now();//now_ms();
        st.radii= st.anim_radii.iter().map(current_radius).collect();
        st.angles= st.anim_angles.iter().map(|a| [a.start_a, st.current_angle(a)]).collect();
        let order = st.current_order;
        let all_settled_for_current = st.anim_angles.iter()
            .filter(|a| a.order == order)
            .all(|a| st.current_angle_progress(a) >= 1.0);
        if all_settled_for_current {
            let new_order = order + 1;
            st.current_order = new_order;

            for anim in st.anim_angles.iter_mut() {
                if anim.order == new_order {
                    anim.start_time = now;
                }
            }
        }
        let _ = draw_pie(&st);

        st.dirty = animating;
        true
    })?;

    Ok(PieChartController { state, mouse: mouse_closure, click: click_closure})
}