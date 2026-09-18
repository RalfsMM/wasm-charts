use crate::data::{DataPoint, Points};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;

//Konfiguracija Piechartam, geo
#[derive(Clone, Copy)]
pub struct PieConfig {
    pub label_column: usize,
    pub value_column: usize,
    pub cx: f64,
    pub cy: f64,
    pub outer_r: f64,
    pub inner_r_diff: f64,
    pub default_font_size: f64,
    pub default_font: &'static str,
    pub middle_font_size: f64,
    pub middle_font: &'static str,
    pub text_color: &'static str,
    pub hover_title_color: &'static str,

}
//Traits taa lai parsingaa var izmantot visu chartu configus
impl Points for PieConfig {
    fn label_col(&self) -> usize {
        self.label_column
    }
    fn value_col(&self) -> usize {
        self.value_column
    }
}
//Noklusejuma vertibas, pirma kollona key, otra value
impl Default for PieConfig {
    fn default() -> Self {
        PieConfig { label_column: 0, value_column: 1, cx: 200.0, cy: 200.0, outer_r: 100.0, inner_r_diff: 20.0, default_font_size: 10.0, default_font: "10px sans-serif", middle_font_size: 54.0, middle_font: "54px sans-serif", text_color: "black", hover_title_color: "blue" }
    }
}

//sektors, ar lenkiem un apaksvertibam
pub struct PieSlice {
    pub label: String,
    pub angle_start: f64,
    pub angle_end: f64,
    pub color: String,
    pub percent: f64,
    pub sub_points: Option<Vec<DataPoint>>, // optional sub-points for drill-down
}

//funkcija kas nosaka sektoru kraasas atkariba no index
fn palette_color(index: usize) -> String {
    let colors = [
        "#FF6384", "#36A2EB", "#FFCE56", "#4BC0C0", "#9966FF",
        "#FF9F40", "#E7E9ED", "#76B041", "#F7464A", "#46BFBD",
    ];
    colors[index % colors.len()].to_string()
}

//Info par veselu chart, domats prieks drilldown, jo tad no vienas tabulas ir vairaki charti, so te info par tiem
pub struct PieChartHandle {
    canvas: web_sys::HtmlCanvasElement,
    running: Rc<RefCell<bool>>,
    mouse_closure: Closure<dyn FnMut(web_sys::MouseEvent)>,
    click_closure: Closure<dyn FnMut(web_sys::MouseEvent)>,
    chart_label: String,
    text_w: f64,
    points: Vec<DataPoint>,
    fraction: f64,
}
//Apstadina kada chartu un taa listeners
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

//globalais mainigais, stack ar visiem chartiem drilldownaa, kur last ir aktivais
thread_local! {
    static CHART_STACK: RefCell<Vec<PieChartHandle>> = RefCell::new(Vec::new());
}

//funkcija prieks lib.rs lai initial chartu ieliktu stekaa
pub fn push_chart(handle: PieChartHandle) {
    CHART_STACK.with(|stack| stack.borrow_mut().push(handle));
}

//Iztukso steku, prieks lib.rs
pub fn stop_all_drilldowns() {
    CHART_STACK.with(|stack| {
        while let Some(handle) = stack.borrow_mut().pop() {
            handle.stop();
        }
    });
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
    let mut others: PieSlice = PieSlice { label: "Others".to_string(), angle_start: -std::f64::consts::PI / 2.0, angle_end: 0.0, color: "#808080".to_string(), percent: 0.0, sub_points: None };

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
            Some(PieSlice { label: p.label, angle_start: start, angle_end: angle, color: palette_color(i), percent, sub_points: None })
        })
        .collect();

    if others.percent > 0.0 {
        slices.insert(0, others);//others pieliek saakumaa
    }
    Ok(slices)
}

//uzzime pie atbilstosi lenkiem, radiusiem utt. So izmanto independently no pie stavokla
pub fn draw_pie( canvas: &web_sys::HtmlCanvasElement, slices: &[PieSlice], geo: &PieConfig, radii: &[f64], angles: &Vec<[f64; 2]>, title_hover: &Option<usize>, slice_hover: &Option<usize>) -> Result<(), JsValue> {
    let context = crate::canvas::get_context(canvas)?;
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    
    let default_size = geo.default_font_size;
    let default_font=geo.default_font;
    let middle_size = geo.middle_font_size;
    let middle_font =geo.middle_font;
    let text_color =geo.text_color;
    let hover_title_color = geo.hover_title_color;

    context.set_font(default_font);
    context.set_fill_style_str(text_color);
    context.set_text_align("right");
    CHART_STACK.with(|stack| {
        let charts = stack.borrow();
        let mut indent= 0.0;
        for (i, chart) in charts.iter().enumerate() {
            if *title_hover == Some(i) {
                context.set_fill_style_str(hover_title_color);
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
                .fill_text(&label, geo.cx - 1.6*geo.outer_r + indent, geo.cy - 1.6*geo.outer_r)
                .map_err(|_| JsValue::from_str("failed to draw chart label"))?;
        }
        Ok::<(), JsValue>(())
    })?;


    for (i, slice) in slices.iter().enumerate() {
        let outer_r = radii[i];
        let inner_r = outer_r -geo.inner_r_diff;
        let start_angle;
        let end_angle;

        start_angle = angles[i][0];
        end_angle = angles[i][1];

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

        if start_angle != end_angle -0.04 {
            context.set_fill_style_str(text_color);
            context.set_font(default_font);

            let mid_angle = (start_angle + end_angle) / 2.0;
            let label_x;
            if mid_angle < std::f64::consts::PI / 2.0 || mid_angle > std::f64::consts::PI * 3.0 / 2.0 {
                context.set_text_align("left");
                label_x = outer_r + geo.cx + geo.outer_r/5.0;
            } else {
                context.set_text_align("right");
                label_x = geo.cx - outer_r - geo.outer_r/5.0;
            }
            let label_y = geo.cy + outer_r * mid_angle.sin();
            context
                .fill_text(&slice.label, label_x, label_y)
                .map_err(|_| JsValue::from_str("failed to draw text"))?;
        }
        if *slice_hover == Some(i) {
            context.set_font(middle_font);
            context.set_text_align("center");
            context.set_fill_style_str(text_color);
            let percent = format!("{:.1}%", slice.percent);
            context
                .fill_text(&percent, geo.cx, geo.cy+ middle_size/3.0)
                .map_err(|_| JsValue::from_str("failed to draw text"))?;
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

//parbauda vai padotas koordinatas ir uz kada slice, un atgriez uz kura ja ir.
fn hit_test_slices(slices: &[PieSlice], geo: &PieConfig, mx: f64, my: f64) -> Option<usize> {
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
fn hit_test_titles(mx: f64, my: f64, geo: &PieConfig) -> Option<usize> {
    if my<geo.cy - 1.6*geo.outer_r - geo.default_font_size||my>geo.cy - 1.6*geo.outer_r||mx<geo.cx - 1.6*geo.outer_r{
        return None;
    }
    CHART_STACK.with(|stack|{
        let charts=stack.borrow();
        let mut indent=0.0;
        for (i,chart) in charts.iter().enumerate() {
            indent += chart.text_w+ geo.default_font_size/3.0;
            if mx< geo.cx - 1.6*geo.outer_r + indent{
                return Some(i);
            }
        }
        None
    })
}

//uzzime pie ar animacijam un palaiz mousemove un mousedown listeners, un handlo izmainas
pub fn render_interactive_pie(canvas_id: &str, points: Vec<DataPoint>, chart_label: String, fraction: f64, config: PieConfig) -> Result<PieChartHandle, JsValue> {
    let slices = compute_pie_slices(points.clone(), fraction)?;
    let canvas = crate::canvas::get_canvas(canvas_id)?;
    //let geo = PieConfig::default();
    let geo =config;
    let running = Rc::new(RefCell::new(true));

    let now_ms = web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .ok_or_else(|| JsValue::from_str("no performance"))?;

    let slices = Rc::new(slices);
    let hover_slice: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let hover_title: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let anim_state: Rc<RefCell<Vec<SliceAnimRadius>>> = Rc::new(RefCell::new(
        slices
            .iter()
            .map(|_| SliceAnimRadius { start_r: geo.outer_r, target_r: geo.outer_r, start_time: now_ms })
            .collect(),
    ));
    
    const TOTAL_ENTRANCE_DURATION_MS: f64 = 500.0; // the whole circle always takes this long
    let slice_count = slices.len() as f64;
    let per_slice_duration = TOTAL_ENTRANCE_DURATION_MS / slice_count;

    let angle_anim_state: Rc<RefCell<Vec<SliceAnimAngle>>> = Rc::new(RefCell::new(
        slices
            .iter()
            .enumerate()
            .map(|(i, slice)| SliceAnimAngle {
                start_a: slice.angle_start,
                target_a: slice.angle_end,
                start_time: now_ms,
                order: i,
                duration_ms: per_slice_duration,
            })
            .collect(),
    ));
    let current_order: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));

    //mousemove listener un handler
    let canvas_for_mouse = canvas.clone();
    let slices_for_mouse = slices.clone();
    let hover_slice_for_mouse = hover_slice.clone();
    let hover_title_for_mouse = hover_title.clone();
    let anim_for_mouse = anim_state.clone();
    let geo_for_mouse = geo;

    let mouse_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_mouse.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();

        //parbauda vai hover uz slice
        let slice_hit = hit_test_slices(&slices_for_mouse, &geo_for_mouse, mx, my);
        let mut slice_hover_ref = hover_slice_for_mouse.borrow_mut();

        if *slice_hover_ref != slice_hit {
            *slice_hover_ref = slice_hit;

            let now = web_sys::window().unwrap().performance().unwrap().now();
            let mut anims = anim_for_mouse.borrow_mut();
            for (i, anim) in anims.iter_mut().enumerate() {
                let new_target = if slice_hit == Some(i) { geo_for_mouse.outer_r + geo_for_mouse.inner_r_diff/2.0 } else { geo_for_mouse.outer_r };

                let changed = anim.target_r != new_target;

                if changed {
                    let cur = current_radius(anim, now);
                    anim.start_r = cur;
                    anim.target_r = new_target;
                    anim.start_time = now; 
                }
            }
        }

        //parbauda vai hover uz title
        let title_hit = hit_test_titles(mx, my, &geo_for_mouse);
        let mut title_hover_ref = hover_title_for_mouse.borrow_mut();
        if *title_hover_ref != title_hit {
            *title_hover_ref = title_hit;
        }
    });
    canvas.add_event_listener_with_callback("mousemove", mouse_closure.as_ref().unchecked_ref())?;

    //mousedown listener un handler
    let canvas_for_click = canvas.clone();
    let slices_for_click = slices.clone();
    let geo_for_click = geo;
    let running_for_click = running.clone();
    
    let click_closure = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |event: web_sys::MouseEvent| {
        let rect = canvas_for_click.get_bounding_client_rect();
        let mx = event.client_x() as f64 - rect.left();
        let my = event.client_y() as f64 - rect.top();

        //parbauda vai click uz slice
        let slice_hit = hit_test_slices(&slices_for_click, &geo_for_click, mx, my);
        if slice_hit.is_some() {
            for ( i, slice) in slices_for_click.iter().enumerate() {
                if Some(i) == slice_hit {
                    if slice.sub_points.is_some() {
                        *running_for_click.borrow_mut() = false;

                        CHART_STACK.with(|stack| {
                            if let Some(old_handle) = stack.borrow_mut().last() {
                                old_handle.stop();
                            }
                        });

                        if let Ok(new_handle) = render_interactive_pie(canvas_for_click.id().as_str(), slice.sub_points.clone().unwrap(), slice.label.clone(), slice.percent, geo_for_click) {
                            CHART_STACK.with(|stack| stack.borrow_mut().push(new_handle));
                        }

                        break;
                    }
                }
            }
        }

        //parbauda vai click uz title
        let title_hit = hit_test_titles(mx, my, &geo_for_click);
        if let Some(i) = title_hit {
            CHART_STACK.with(|stack| {
                let mut charts = stack.borrow_mut(); // single mutable borrow, used for everything below
                *running_for_click.borrow_mut() = false;
                let points = charts[i].points.clone();
                let label = charts[i].chart_label.clone();
                let fract = charts[i].fraction.clone();

                while charts.len() > i {
                    if let Some(old_chart) = charts.pop() {
                        old_chart.stop();//kinda redundant, jo tikai augsejais stackaa var but running
                    }
                }
                drop(charts);
                if let Ok(new_handle) = render_interactive_pie(canvas_for_click.id().as_str(), points, label, fract, geo_for_click) {
                    CHART_STACK.with(|stack| stack.borrow_mut().push(new_handle));
                }
            });
        }

    });
    canvas.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())?;

    //animacijas loop, kas visu laiku(vai ari kad izmainas) zime pie ar atbilstosa frame lenku un radiusu vertibam
    let canvas_for_loop = canvas.clone();
    let slices_for_loop = slices.clone();
    let anim_for_loop = anim_state.clone();
    let anim_angle_for_loop = angle_anim_state.clone();
    let running_for_loop = running.clone();
    let hover_title_for_loop = hover_title.clone();
    let hover_slice_for_loop = hover_slice.clone();

    crate::animation::start_loop(move |_elapsed_ms| {
        if !*running_for_loop.borrow() {
            return false;
        }
        let now = web_sys::window().unwrap().performance().unwrap().now();
        let radii: Vec<f64> = anim_for_loop.borrow().iter().map(|a| current_radius(a, now)).collect();
        
        let angles: Vec<[f64; 2]> = anim_angle_for_loop.borrow().iter().map(|a| [a.start_a, current_angle(a, now, &current_order)]).collect();
        let order = *current_order.borrow();
        let all_settled_for_current = anim_angle_for_loop.borrow().iter()
            .filter(|a| a.order == order)
            .all(|a| current_angle_progress(a, now) >= 1.0);
        if all_settled_for_current {
            let new_order = order + 1;
            *current_order.borrow_mut() = new_order;

            for anim in anim_angle_for_loop.borrow_mut().iter_mut() {
                if anim.order == new_order {
                    anim.start_time = now;
                }
            }
        }

        let hover_title_value = *hover_title_for_loop.borrow();
        let hover_slice_value = *hover_slice_for_loop.borrow();
        let _ = draw_pie(&canvas_for_loop, &slices_for_loop, &geo, &radii, &angles, &hover_title_value, &hover_slice_value);
        true
    })?;

    //izmera cik gars ir title
    let context = crate::canvas::get_context(&canvas)?;
    let textmetrics = context.measure_text(chart_label.as_str());
    let text_w =textmetrics.unwrap().width();//chatins piedavaja map_err seit, es nez

    Ok(PieChartHandle { canvas, running, mouse_closure, click_closure, chart_label, text_w, points, fraction })
}

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

//easing algoritms hoveram uz slices
fn ease_out_cubic(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

//aprekina radiusu prieks animacijas
fn current_radius(anim: &SliceAnimRadius, now_ms: f64) -> f64 {
    let elapsed = now_ms - anim.start_time;
    let t = (elapsed / 200.0).clamp(0.0, 1.0);
    let eased = ease_out_cubic(t);
    anim.start_r + (anim.target_r - anim.start_r) * eased
}

//aprekina kopejo lenki prieks animacijas
fn current_angle_progress(anim: &SliceAnimAngle, now_ms: f64)-> f64{
    let elapsed = now_ms - anim.start_time;
    let t = (elapsed / anim.duration_ms).clamp(0.0, 1.0);
    t
}

//aprekina lenki prieks animacijas
fn current_angle(anim: &SliceAnimAngle, now_ms: f64, current_order: &Rc<RefCell<usize>>) -> f64 {
    let order = *current_order.borrow();
    if anim.order < order {
        anim.target_a
    } else if anim.order > order {
        anim.start_a + 0.04
    } else {
        let elapsed = now_ms - anim.start_time;
        let t = (elapsed / anim.duration_ms).clamp(0.0, 1.0);
        anim.start_a + (anim.target_a - anim.start_a) * t
    }
}
