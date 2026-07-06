// ═══════════════════════════════════════════════════════════
// Motion / Animation Helpers (React)
// ═══════════════════════════════════════════════════════════

use std::collections::{HashMap, HashSet};

use crate::codegen::react::expr;
use crate::codegen::CodeWriter;
use crate::ir::*;

/// Emit React hooks for @enter motion triggers found in the view tree.
///
/// Scans for @enter triggers and emits:
/// 1. A `_M` constant mapping motion names → `[keyframes[], options{}]` tuples
/// 2. A `useEffect` that reads `data-motion-name` from each element,
///    looks up the motion definition, and calls `el.animate(kf, opt)`
///
/// `motion_bodies` maps motion name → its MotionBodyIR, from both component
/// and module-level @motion definitions.
pub(super) fn emit_motion_effects_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    motion_bodies: &HashMap<String, MotionBodyIR>,
    signal_names: &HashSet<String>,
) {
    // Collect all unique motion names referenced by @enter triggers in this tree
    let motion_refs = collect_enter_motion_refs(node);
    if motion_refs.is_empty() {
        return;
    }

    // Filter to only those we have definitions for
    let defined_refs: HashSet<&String> = motion_refs
        .iter()
        .filter(|name| motion_bodies.contains_key(name.as_str()))
        .collect();
    if defined_refs.is_empty() {
        return;
    }

    // ── Emit the MOTION_DEFS constant ──
    w.emit("// -- Motion definitions --");
    w.emit("const _M={");
    w.indent();

    let mut sorted: Vec<&String> = defined_refs.into_iter().collect();
    sorted.sort();
    for name in &sorted {
        if let Some(body) = motion_bodies.get(name.as_str()) {
            let def_js = format_motion_to_waapi(body);
            w.emit(&format!("{}:{},", name, def_js));
        }
    }
    w.dedent();
    w.emit("};");
    w.emit("");

    // ── Emit useEffect that applies animations ──
    w.emit("useEffect(()=>{");
    w.indent();
    w.emit("const els=document.querySelectorAll('[data-motion-enter]');");
    w.emit("els.forEach(el=>{");
    w.indent();
    w.emit("const n=el.getAttribute('data-motion-name');");
    w.emit("if(n&&_M[n]){");
    w.indent();
    w.emit("const[kf,opt,setup]=_M[n];");
    w.emit("if(setup)setup(el);");
    w.emit("el.animate(kf,opt);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    // Include signal names in dep array so @enter re-triggers on state changes
    if signal_names.is_empty() {
        w.emit("},[]);");
    } else {
        let mut sorted: Vec<&String> = signal_names.iter().collect();
        sorted.sort();
        let deps: Vec<&str> = sorted.iter().map(|s| s.as_str()).collect();
        let deps_str = deps.join(", ");
        w.emit(&format!("}},[{}]);", deps_str));
    }
}

/// ── Helpers ────────────────────────────────────────────────────────────

/// Collect all unique motion names referenced by @enter triggers in a view tree.
fn collect_enter_motion_refs(node: &ViewTree) -> HashSet<String> {
    let mut refs = HashSet::new();
    collect_enter_motion_refs_inner(node, &mut refs);
    refs
}

fn collect_enter_motion_refs_inner(node: &ViewTree, refs: &mut HashSet<String>) {
    match node {
        ViewTree::Element(el) => {
            for trigger in &el.motion_triggers {
                if matches!(trigger.kind, TriggerKind::Enter) {
                    if let Some(ref name) = trigger.motion_ref {
                        refs.insert(name.clone());
                    }
                }
            }
            for child in &el.children {
                collect_enter_motion_refs_inner(child, refs);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(body) = &branch.body {
                    collect_enter_motion_refs_inner(body, refs);
                }
            }
            if let Some(fb) = &cond.fallback {
                collect_enter_motion_refs_inner(fb, refs);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(body) = &list.body {
                collect_enter_motion_refs_inner(body, refs);
            }
            if let Some(empty) = &list.empty_body {
                collect_enter_motion_refs_inner(empty, refs);
            }
        }
        _ => {}
    }
}

/// Format a full MotionBodyIR into a JS tuple `[keyframes[], options{}, setupFn?]`.
fn format_motion_to_waapi(body: &MotionBodyIR) -> String {
    match body {
        MotionBodyIR::Keyframe(kf) => {
            let keyframes = format_keyframes(kf);
            let options = format_playback_options(&kf.over, &kf.ease, &kf.playback);
            // Path setup: if motion has a path, emit a setup function
            if let Some(path) = &kf.path {
                let rot = kf.path_rotate.unwrap_or(false);
                format!("[{},{} ,(el)=>{{el.style.offsetPath='path(\"{}\")';{}}}]",
                    keyframes, options, path,
                    if rot { "el.style.offsetRotate='auto';" } else { "" }
                )
            } else {
                format!("[{},{}]", keyframes, options)
            }
        }
        MotionBodyIR::Timeline(_) => {
            // Timeline animations — basic fallback for now
            "[[{opacity:0,offset:0},{opacity:1,offset:1}],{duration:300,easing:\"ease-out\",fill:\"both\"}]".to_string()
        }
        MotionBodyIR::CallbackOnly(_) => {
            "[[{opacity:0,offset:0},{opacity:1,offset:1}],{duration:300,easing:\"ease-out\",fill:\"both\"}]".to_string()
        }
    }
}

/// Format keyframe stops into a WAAPI-compatible keyframes array string.
///   `[{opacity:0,transform:"translateX(-20px)",offset:0}, ...]`
fn format_keyframes(kf: &KeyframeIR) -> String {
    let mut items: Vec<String> = Vec::new();

    if kf.stops.len() == 1 {
        let stop = &kf.stops[0];
        if stop.position >= 0.5 {
            // to-only: synthetic 0% keyframe (start from current state)
            items.push("{offset:0}".to_string());
            items.push(format_stop(stop, true));
        } else {
            // from-only: synthetic 100% keyframe (return to current state)
            items.push(format_stop(stop, true));
            items.push("{offset:1}".to_string());
        }
        return format!("[{}]", items.join(","));
    }

    // Sort stops by position
    let mut sorted_stops = kf.stops.clone();
    sorted_stops.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for stop in &sorted_stops {
        items.push(format_stop(stop, kf.stops.len() <= 2));
    }

    format!("[{}]", items.join(","))
}

/// Format a single keyframe stop into a JS keyframe object like `{opacity:0,transform:"...",offset:0}`.
fn format_stop(stop: &KeyframeStopIR, is_binary: bool) -> String {
    let mut css_props: Vec<String> = Vec::new(); // name:value
    let mut transform_parts: Vec<String> = Vec::new(); // translateX(...) etc
    let mut filter_parts: Vec<String> = Vec::new(); // blur(...) etc

    for (prop, value) in &stop.properties {
        let val_str = expr::format_expr_ir_tsx(value);
        match prop.as_str() {
            // Transform shortcuts → combined into one `transform` string
            "x" => transform_parts.push(format!("translateX({}px)", val_str)),
            "y" => transform_parts.push(format!("translateY({}px)", val_str)),
            "rotate" => transform_parts.push(format!("rotate({}deg)", val_str)),
            "rotateX" => transform_parts.push(format!("rotateX({}deg)", val_str)),
            "rotateY" => transform_parts.push(format!("rotateY({}deg)", val_str)),
            "rotateZ" => transform_parts.push(format!("rotateZ({}deg)", val_str)),
            "scale" => transform_parts.push(format!("scale({})", val_str)),
            "scaleX" => transform_parts.push(format!("scaleX({})", val_str)),
            "scaleY" => transform_parts.push(format!("scaleY({})", val_str)),
            "skewX" => transform_parts.push(format!("skewX({}deg)", val_str)),
            "skewY" => transform_parts.push(format!("skewY({}deg)", val_str)),
            // Filter shortcuts
            "blur" => filter_parts.push(format!("blur({}px)", val_str)),
            "brightness" => filter_parts.push(format!("brightness({})", val_str)),
            "contrast" => filter_parts.push(format!("contrast({})", val_str)),
            "saturate" => filter_parts.push(format!("saturate({})", val_str)),
            "hueRotate" => filter_parts.push(format!("hue-rotate({}deg)", val_str)),
            "offsetDistance" => {
                // WAAPI requires a string like "0%" or "100%".
                // Omni uses 0..=1 range; convert to percentage.
                if let Ok(n) = val_str.parse::<f64>() {
                    css_props.push(format!("offsetDistance:\"{}%\"", n * 100.0));
                } else {
                    css_props.push(format!("offsetDistance:{}", val_str));
                }
            }
            // Everything else passes through as a camelCase CSS property
            _ => {
                if is_length_prop(prop) && is_numeric_str(&val_str) {
                    css_props.push(format!("{}:\"{}px\"", prop, val_str));
                } else {
                    css_props.push(format!("{}:{}", prop, val_str));
                }
            }
        }
    }

    // Build combined keyframe
    let mut parts: Vec<String> = Vec::new();
    parts.extend(css_props);
    if !transform_parts.is_empty() {
        parts.push(format!("transform:\"{}\"", transform_parts.join(" ")));
    }
    if !filter_parts.is_empty() {
        parts.push(format!("filter:\"{}\"", filter_parts.join(" ")));
    }
    if !is_binary {
        // Always include explicit offset for multi-stop keyframes
        parts.push(format!("offset:{}", stop.position));
    }

    format!("{{{}}}", parts.join(","))
}

/// Properties that need `px` suffix in WAAPI keyframes when value is numeric.
fn is_length_prop(prop: &str) -> bool {
    matches!(
        prop,
        "width"
            | "height"
            | "margin"
            | "padding"
            | "borderRadius"
            | "borderWidth"
            | "marginTop"
            | "marginRight"
            | "marginBottom"
            | "marginLeft"
            | "paddingTop"
            | "paddingRight"
            | "paddingBottom"
            | "paddingLeft"
            | "borderTopLeftRadius"
            | "borderTopRightRadius"
            | "borderBottomLeftRadius"
            | "borderBottomRightRadius"
            | "gap"
            | "rowGap"
            | "columnGap"
            | "maxWidth"
            | "minWidth"
            | "maxHeight"
            | "minHeight"
            | "top"
            | "left"
            | "right"
            | "bottom"
            | "fontSize"
            | "letterSpacing"
            | "wordSpacing"
    )
}

/// Check if a formatted JS value string is a bare number (not quoted).
fn is_numeric_str(s: &str) -> bool {
    !s.starts_with('"')
        && !s.starts_with('\'')
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-' || c == '+')
}

/// Format playback options into a WAAPI-compatible options object.
///   `{duration:400,easing:"ease-out",fill:"forwards"}`
fn format_playback_options(
    over: &ExprIR,
    ease: &Option<EasingIR>,
    playback: &PlaybackIR,
) -> String {
    let mut opts: Vec<String> = Vec::new();

    // Duration
    let dur_str = expr::format_expr_ir_tsx(over);
    opts.push(format!("duration:{}", dur_str));

    // Easing
    if let Some(easing) = ease {
        opts.push(format!("easing:\"{}\"", translate_easing(easing)));
    }

    // Fill mode
    if let Some(fill) = &playback.fill {
        let s = match fill {
            FillValue::None => "none",
            FillValue::Forwards => "forwards",
            FillValue::Backwards => "backwards",
            FillValue::Both => "both",
        };
        opts.push(format!("fill:\"{}\"", s));
    }

    // Direction
    if let Some(dir) = &playback.direction {
        let s = match dir {
            DirectionValue::Normal => "normal",
            DirectionValue::Reverse => "reverse",
            DirectionValue::Alternate => "alternate",
            DirectionValue::AlternateReverse => "alternate-reverse",
        };
        opts.push(format!("direction:\"{}\"", s));
    }

    // Loop count
    if let Some(loop_val) = &playback.loop_count {
        match loop_val {
            LoopValue::Infinite => opts.push("iterations:Infinity".into()),
            LoopValue::Count(n) => opts.push(format!("iterations:{}", n)),
        }
    }

    // Delay
    if let Some(delay) = &playback.delay {
        let d_str = expr::format_expr_ir_tsx(delay);
        opts.push(format!("delay:{}", d_str));
    }

    format!("{{{}}}", opts.join(","))
}

/// Translate an Omni easing IR to a WAAPI-compatible easing string.
fn translate_easing(ease: &EasingIR) -> String {
    match ease {
        EasingIR::Named(name) => translate_named_easing(name),
        EasingIR::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2)
        }
        EasingIR::Spring {
            tension,
            friction,
            velocity,
        } => {
            // Spring → approximate cubic-bezier based on parameters
            let _ = velocity;
            let ratio = if *tension > 0.0 {
                *friction / *tension
            } else {
                1.0
            };
            if ratio < 0.3 {
                // Underdamped (bouncy)
                let overshoot = (1.0 - ratio * 2.0).min(0.3);
                format!(
                    "cubic-bezier(0.250,{},0.450,{})",
                    0.460 + overshoot,
                    0.940 + overshoot
                )
            } else {
                "cubic-bezier(0.250,0.460,0.450,0.940)".to_string()
            }
        }
        EasingIR::Back {
            intensity,
            direction,
        } => {
            let dir = direction.as_deref().unwrap_or("Out");
            let i = intensity.min(2.0).max(0.5);
            match dir {
                "In" => format!("cubic-bezier({},-{},0.735,0.045)", 0.600, 0.28 * i),
                "InOut" => format!(
                    "cubic-bezier({},-{},0.265,{})",
                    0.680,
                    0.55 * i,
                    (1.55 * i).min(1.8)
                ),
                _ => format!(
                    "cubic-bezier(0.175,{},0.320,{})",
                    0.885 * i.min(1.5),
                    (1.275 * i.min(1.5)).min(2.0)
                ),
            }
        }
        EasingIR::Elastic {
            amplitude,
            period,
            direction,
        } => {
            // Elastic → approximate cubic-bezier
            let _ = amplitude;
            let _ = period;
            let dir = direction.as_deref().unwrap_or("Out");
            match dir {
                "In" => "cubic-bezier(0.600,0.040,0.980,0.335)".to_string(),
                "InOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)".to_string(),
                _ => "cubic-bezier(0.250,0.460,0.450,0.940)".to_string(),
            }
        }
        EasingIR::Steps { count, direction } => {
            format!("steps({},{})", count, direction)
        }
    }
}

fn translate_named_easing(name: &str) -> String {
    match name {
        // CSS standard keywords
        "linear" => "linear",
        "ease" => "ease",
        "easeIn" | "ease-in" => "ease-in",
        "easeOut" | "ease-out" => "ease-out",
        "easeInOut" | "ease-in-out" => "ease-in-out",
        // GSAP power1 (quadratic)
        "power1In" => "cubic-bezier(0.550,0.085,0.680,0.530)",
        "power1Out" => "cubic-bezier(0.250,0.460,0.450,0.940)",
        "power1InOut" => "cubic-bezier(0.455,0.030,0.515,0.955)",
        // GSAP power2 (cubic)
        "power2In" => "cubic-bezier(0.550,0.055,0.675,0.190)",
        "power2Out" => "cubic-bezier(0.215,0.610,0.355,1.000)",
        "power2InOut" => "cubic-bezier(0.645,0.045,0.355,1.000)",
        // GSAP power3 (quartic)
        "power3In" => "cubic-bezier(0.895,0.030,0.685,0.220)",
        "power3Out" => "cubic-bezier(0.165,0.840,0.440,1.000)",
        "power3InOut" => "cubic-bezier(0.770,0.000,0.175,1.000)",
        // GSAP power4 (quintic)
        "power4In" => "cubic-bezier(0.755,0.050,0.855,0.060)",
        "power4Out" => "cubic-bezier(0.230,1.000,0.320,1.000)",
        "power4InOut" => "cubic-bezier(0.860,0.000,0.070,1.000)",
        // Back easings (support overshoot in WAAPI)
        "backIn" => "cubic-bezier(0.600,-0.280,0.735,0.045)",
        "backOut" => "cubic-bezier(0.175,0.885,0.320,1.275)",
        "backInOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)",
        // Bounce → cubic-bezier approximations
        "bounceIn" => "cubic-bezier(0.215,0.610,0.355,1.000)",
        "bounceOut" => "cubic-bezier(0.215,0.610,0.355,1.000)",
        "bounceInOut" => "cubic-bezier(0.645,0.045,0.355,1.000)",
        // Elastic → cubic-bezier approximations
        "elasticIn" => "cubic-bezier(0.600,0.040,0.980,0.335)",
        "elasticOut" => "cubic-bezier(0.250,0.460,0.450,0.940)",
        "elasticInOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)",
        // Spring → generic bouncy
        "spring" => "cubic-bezier(0.250,0.460,0.450,0.940)",
        // Fallback
        _ => "ease-out",
    }
    .to_string()
}

// ══════════════════════════════════════════════════════════════════════════
// Below: unchanged helpers for @driven-by, @in-view, @scroll, @drag
// ══════════════════════════════════════════════════════════════════════════

/// Emit React useEffect hooks for @driven-by external progress control.
pub(super) fn emit_driven_by_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_driven_by(node) {
        return;
    }

    w.emit("// @driven-by bindings: external progress control via signal");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-driven-by]');");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const drivenVal = el.getAttribute('data-driven-by');");
    w.emit("const progress = Math.max(0, Math.min(1, Number(drivenVal)));");
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => {");
    w.indent();
    w.emit("if (a.playState !== 'finished') {");
    w.indent();
    w.emit("const duration = a.effect?.getTiming()?.duration || 300;");
    w.emit("a.currentTime = progress * duration;");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
}

/// Check if any element in the view tree has @driven-by.
pub(super) fn has_driven_by(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el.driven_by.is_some() {
                return true;
            }
            el.children.iter().any(|c| has_driven_by(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().map_or(false, |body| has_driven_by(body)))
                || cond.fallback.as_ref().map_or(false, |fb| has_driven_by(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().map_or(false, |body| has_driven_by(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_driven_by(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @in-view triggers using IntersectionObserver.
pub(super) fn emit_inview_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_inview_trigger(node) {
        return;
    }

    w.emit("// @in-view: IntersectionObserver for entrance animations");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-inview]');");
    w.emit("const observers = [];");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const threshold = parseFloat(el.getAttribute('data-inview-threshold') || '0');");
    w.emit("const observer = new IntersectionObserver((entries) => {");
    w.indent();
    w.emit("entries.forEach(entry => {");
    w.indent();
    w.emit("if (entry.isIntersecting) {");
    w.indent();
    w.emit("el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 300, easing: 'ease-out', fill: 'both' });");
    w.emit("observer.disconnect();");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("}, { threshold });");
    w.emit("observer.observe(el);");
    w.emit("observers.push(observer);");
    w.dedent();
    w.emit("});");
    w.emit("return () => observers.forEach(o => o.disconnect());");
    w.dedent();
    w.emit("}, []);");
}

/// Recursively check if view tree has any @in-view triggers.
pub(super) fn has_inview_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::InView))
            {
                return true;
            }
            el.children.iter().any(|c| has_inview_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .map_or(false, |body| has_inview_trigger(body))
            }) || cond
                .fallback
                .as_ref()
                .map_or(false, |fb| has_inview_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_inview_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_inview_trigger(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @scroll triggers using scroll-driven animation progress.
pub(super) fn emit_scroll_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_scroll_trigger(node) {
        return;
    }

    w.emit("// @scroll: scroll-driven animation progress");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-scroll]');");
    w.emit("if (!els.length) return;");
    w.emit("function parseScrollTrigger(cfg, el, vh) {");
    w.indent();
    w.emit("if (!cfg || typeof cfg !== 'string') return 0;");
    w.emit("const parts = cfg.split(' ');");
    w.emit("if (parts.length < 2) return 0;");
    w.emit("const edge = parts[0];");
    w.emit("const offset = parts[1];");
    w.emit("const elRect = el.getBoundingClientRect();");
    w.emit("let elPos;");
    w.emit("if (edge === 'top') elPos = elRect.top;");
    w.emit("else if (edge === 'center') elPos = elRect.top + elRect.height / 2;");
    w.emit("else if (edge === 'bottom') elPos = elRect.bottom;");
    w.emit("else elPos = elRect.top;");
    w.emit("if (offset.endsWith('%')) {");
    w.indent();
    w.emit("return window.scrollY + elPos - vh * (parseFloat(offset) / 100);");
    w.dedent();
    w.emit("} else if (offset === 'bottom') {");
    w.indent();
    w.emit("return window.scrollY + elPos - vh;");
    w.dedent();
    w.emit("} else if (offset === 'top') {");
    w.indent();
    w.emit("return window.scrollY + elPos;");
    w.dedent();
    w.emit("}");
    w.emit("return window.scrollY + elPos - vh * 0.5;");
    w.dedent();
    w.emit("}");
    w.emit("function computeProgress(el) {");
    w.indent();
    w.emit("const rect = el.getBoundingClientRect();");
    w.emit("const vh = window.innerHeight;");
    w.emit("const startCfg = el.getAttribute('data-scroll-start') || 'top 80%';");
    w.emit("const endCfg = el.getAttribute('data-scroll-end') || 'bottom 20%';");
    w.emit("const startPos = parseScrollTrigger(startCfg, el, vh);");
    w.emit("const endPos = parseScrollTrigger(endCfg, el, vh);");
    w.emit("const raw = (window.scrollY - startPos) / (endPos - startPos);");
    w.emit("return Math.max(0, Math.min(1, raw));");
    w.dedent();
    w.emit("}");
    w.emit("function scrubValue(el, target) {");
    w.indent();
    w.emit("const delay = parseFloat(el.getAttribute('data-scroll-scrub') || '0');");
    w.emit("if (delay > 0) {");
    w.indent();
    w.emit("return (scrubCache.get(el) || 0) + (target - (scrubCache.get(el) || 0)) * Math.min(1, 16.67 / (delay * 1000));");
    w.dedent();
    w.emit("}");
    w.emit("return target;");
    w.dedent();
    w.emit("}");
    w.emit("const scrubCache = new Map();");
    w.emit("const handleScroll = () => {");
    w.indent();
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const target = computeProgress(el);");
    w.emit("const scrubbed = scrubValue(el, target);");
    w.emit("scrubCache.set(el, scrubbed);");
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => {");
    w.indent();
    w.emit("const duration = a.effect?.getTiming()?.duration || 300;");
    w.emit("a.currentTime = scrubbed * duration;");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("};");
    w.emit("window.addEventListener('scroll', handleScroll, { passive: true });");
    w.emit("handleScroll();");
    w.emit("return () => window.removeEventListener('scroll', handleScroll);");
    w.dedent();
    w.emit("}, []);");
}

/// Recursively check if view tree has any @scroll triggers.
pub(super) fn has_scroll_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el
                .motion_triggers
                .iter()
                .any(|t| matches!(t.kind, TriggerKind::Scroll))
            {
                return true;
            }
            el.children.iter().any(|c| has_scroll_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches.iter().any(|b| {
                b.body
                    .as_ref()
                    .map_or(false, |body| has_scroll_trigger(body))
            }) || cond
                .fallback
                .as_ref()
                .map_or(false, |fb| has_scroll_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_scroll_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_scroll_trigger(empty))
        }
        _ => false,
    }
}

/// Recursively check if view tree has any @drag-horizontal or @drag-vertical triggers.
pub(super) fn has_drag_trigger(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            if el.motion_triggers.iter().any(|t| {
                matches!(
                    t.kind,
                    TriggerKind::DragHorizontal | TriggerKind::DragVertical
                )
            }) {
                return true;
            }
            el.children.iter().any(|c| has_drag_trigger(c))
        }
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().map_or(false, |body| has_drag_trigger(body)))
                || cond
                    .fallback
                    .as_ref()
                    .map_or(false, |fb| has_drag_trigger(fb))
        }
        ViewTree::ListRendering(list) => {
            list.body
                .as_ref()
                .map_or(false, |body| has_drag_trigger(body))
                || list
                    .empty_body
                    .as_ref()
                    .map_or(false, |empty| has_drag_trigger(empty))
        }
        _ => false,
    }
}

/// Emit React useEffect hooks for @drag-horizontal / @drag-vertical triggers using pointer events.
pub(super) fn emit_drag_effects_tsx(w: &mut CodeWriter, node: &ViewTree) {
    if !has_drag_trigger(node) {
        return;
    }

    w.emit("// @drag-horizontal / @drag-vertical: gesture-driven animation progress");
    w.emit("useEffect(() => {");
    w.indent();
    w.emit("const els = document.querySelectorAll('[data-motion-drag]');");
    w.emit("if (!els.length) return;");
    w.emit("");
    w.emit("function parseConstraint(cfg) {");
    w.indent();
    w.emit("if (!cfg) return { min: -Infinity, max: Infinity };");
    w.emit("if (cfg === 'parent') {");
    w.indent();
    w.emit("const parent = el.parentElement;");
    w.emit("if (!parent) return { min: -Infinity, max: Infinity };");
    w.emit("const elRect = el.getBoundingClientRect();");
    w.emit("const parentRect = parent.getBoundingClientRect();");
    w.emit("return { min: 0, max: parentRect.width - elRect.width };");
    w.dedent();
    w.emit("}");
    w.emit("const parts = cfg.split(' ');");
    w.emit("let min = -Infinity, max = Infinity;");
    w.emit("parts.forEach(p => {");
    w.indent();
    w.emit("const [key, val] = p.split(':');");
    w.emit("if (key === 'left' || key === 'top') min = parseFloat(val) || 0;");
    w.emit("if (key === 'right' || key === 'bottom') max = parseFloat(val) || Infinity;");
    w.dedent();
    w.emit("});");
    w.emit("return { min, max };");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("const axis = el.getAttribute('data-motion-drag') || 'x';");
    w.emit("const constraints = el.getAttribute('data-drag-constraints') || '';");
    w.emit("const elastic = parseFloat(el.getAttribute('data-drag-elastic') || '0');");
    w.emit("const threshold = parseFloat(el.getAttribute('data-drag-threshold') || '0');");
    w.emit("const velocityMax = parseFloat(el.getAttribute('data-drag-velocity-max') || '2000');");
    w.emit("const constraint = parseConstraint(constraints);");
    w.emit("const maxExtent = Math.abs(constraint.max - constraint.min) || 1;");
    w.emit("let startPos = 0, currentDelta = 0, isDragging = false;");
    w.emit("let lastMoveTime = 0, lastMovePos = 0, velocity = 0;");
    w.emit("");
    w.emit("const onPointerDown = (e) => {");
    w.indent();
    w.emit("isDragging = true;");
    w.emit("startPos = axis === 'x' ? e.clientX : e.clientY;");
    w.emit("currentDelta = 0;");
    w.emit("el.setPointerCapture(e.pointerId);");
    w.dedent();
    w.emit("};");
    w.emit("");
    w.emit("const onPointerMove = (e) => {");
    w.indent();
    w.emit("if (!isDragging) return;");
    w.emit("const now = performance.now();");
    w.emit("const raw = (axis === 'x' ? e.clientX : e.clientY) - startPos;");
    w.emit("if (lastMoveTime > 0) {");
    w.indent();
    w.emit("const dt = now - lastMoveTime;");
    w.emit("const dp = Math.abs(raw - lastMovePos);");
    w.emit("if (dt > 0) velocity = Math.min(velocityMax, dp / dt * 1000);");
    w.dedent();
    w.emit("}");
    w.emit("lastMoveTime = now;");
    w.emit("lastMovePos = raw;");
    w.emit("let constrained = raw;");
    w.emit("if (raw < constraint.min) {");
    w.indent();
    w.emit("constrained = constraint.min - (constraint.min - raw) * (1 - elastic);");
    w.dedent();
    w.emit("} else if (raw > constraint.max) {");
    w.indent();
    w.emit("constrained = constraint.max + (raw - constraint.max) * (1 - elastic);");
    w.dedent();
    w.emit("}");
    w.emit("currentDelta = raw;");
    w.emit("const progress = Math.min(1, Math.abs(constrained) / maxExtent);");
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => {");
    w.indent();
    w.emit("const duration = a.effect?.getTiming()?.duration || 300;");
    w.emit("a.currentTime = progress * duration;");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("};");
    w.emit("");
    w.emit("const onPointerUp = (e) => {");
    w.indent();
    w.emit("isDragging = false;");
    w.emit("el.releasePointerCapture(e.pointerId);");
    w.emit("const progress = Math.abs(currentDelta) / maxExtent;");
    w.emit("if (progress >= threshold && maxExtent > 0) {");
    w.indent();
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => a.play());");
    w.dedent();
    w.emit("} else {");
    w.indent();
    w.emit("const anims = el.getAnimations();");
    w.emit("anims.forEach(a => a.reverse());");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("};");
    w.emit("");
    w.emit("el.addEventListener('pointerdown', onPointerDown);");
    w.emit("el.addEventListener('pointermove', onPointerMove);");
    w.emit("el.addEventListener('pointerup', onPointerUp);");
    w.emit("el.addEventListener('pointercancel', onPointerUp);");
    w.emit("");
    w.emit("// Cleanup");
    w.emit("el._dragCleanup = () => {");
    w.indent();
    w.emit("el.removeEventListener('pointerdown', onPointerDown);");
    w.emit("el.removeEventListener('pointermove', onPointerMove);");
    w.emit("el.removeEventListener('pointerup', onPointerUp);");
    w.emit("el.removeEventListener('pointercancel', onPointerUp);");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("return () => {");
    w.indent();
    w.emit("els.forEach(el => {");
    w.indent();
    w.emit("if (el._dragCleanup) el._dragCleanup();");
    w.dedent();
    w.emit("});");
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("}, []);");
}
