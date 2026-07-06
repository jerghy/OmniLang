//! Timeline emission: orchestrates multi-element animations by element ID.
use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    /// Emit a timeline definition JS (orchestrates animations by element ID).
    pub(super) fn emit_timeline_js(&mut self, timeline: &TimelineIntent) {
        let saved_len = self.mount_js.len();
        self.mount_js
            .push_str(&format!("/// ── @timeline {} ──\n", timeline.name));

        let mut cursor_delay = 0.0;
        let mut prev_item_start = 0.0;
        let mut pair_base_cursor = 0.0;
        let mut pair_base_prev_start = 0.0;
        let mut pair_from_delay = 0.0;
        let mut use_pair_base = false;
        let mut labels: Vec<(String, f64)> = Vec::new();
        let mut child_idx: usize = 0;

        for (i, item) in timeline.items.iter().enumerate() {
            let item_label = format!("tl_{}_item_{}", timeline.name, i);

            match &item.kind {
                TimelineItemKindIR::Direction {
                    dir,
                    properties,
                    over,
                    ease,
                    delay,
                    color_space: _,
                    ref start,
                    ref path,
                    ref path_rotate,
                } => {
                    let (from_props, to_props) = match dir {
                        DirectionIR::To => {
                            let to: Vec<String> = properties
                                .iter()
                                .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                                .collect();
                            (String::new(), to.join(", "))
                        }
                        DirectionIR::FromTo => {
                            let mut from: Vec<String> = Vec::new();
                            let mut to: Vec<String> = Vec::new();
                            let mut is_to = false;
                            for (k, v) in properties.iter() {
                                if is_to {
                                    to.push(format!("\"{}\": {}", k, self.expr_to_js(v)));
                                } else {
                                    from.push(format!("\"{}\": {}", k, self.expr_to_js(v)));
                                }
                                is_to = !is_to;
                            }
                            (from.join(", "), to.join(", "))
                        }
                    };

                    let base_delay = delay.as_ref().and_then(|d| {
                        if let ExprIR::LiteralExpr { value } = d {
                            value.as_f64()
                        } else {
                            None
                        }
                    });

                    let (ref_cursor, ref_prev_start) = if use_pair_base {
                        (pair_base_cursor, pair_base_prev_start)
                    } else {
                        (cursor_delay, prev_item_start)
                    };

                    let mut parse_ms = |s: &str| -> f64 {
                        let trimmed = s.trim_end_matches("ms").trim();
                        match trimmed.parse::<f64>() {
                            Ok(v) => v,
                            Err(_) => {
                                self.warnings.push(format!(
                                    "Invalid timeline timing value '{}', using 0.0",
                                    s
                                ));
                                0.0
                            }
                        }
                    };

                    let has_explicit_start = start.is_some();
                    let item_delay = if let Some(d) = base_delay {
                        d
                    } else if use_pair_base && has_explicit_start {
                        pair_from_delay + 0.3
                    } else if let Some(ref start_mode) = start {
                        let st = start_mode.as_str();
                        if st == "with-prev" || st == "withPrev" {
                            ref_prev_start
                        } else if st.starts_with("overlap(") {
                            let inner = st.trim_start_matches("overlap(").trim_end_matches(')');
                            let n = parse_ms(inner);
                            (ref_cursor - n).max(0.0)
                        } else if st.starts_with("after-prev(") || st.starts_with("afterPrev(") {
                            let inner = st
                                .trim_start_matches("after-prev(")
                                .trim_start_matches("afterPrev(")
                                .trim_end_matches(')');
                            let n = parse_ms(inner);
                            ref_cursor + n
                        } else if st.starts_with("with-offset(") || st.starts_with("withOffset(") {
                            let inner = st
                                .trim_start_matches("with-offset(")
                                .trim_start_matches("withOffset(")
                                .trim_end_matches(')');
                            let n = parse_ms(inner);
                            ref_prev_start + n
                        } else if st.starts_with("absolute(") {
                            let inner = st.trim_start_matches("absolute(").trim_end_matches(')');
                            parse_ms(inner)
                        } else {
                            ref_cursor
                        }
                    } else {
                        if use_pair_base {
                            pair_from_delay + 0.3
                        } else {
                            cursor_delay
                        }
                    };

                    if !use_pair_base {
                        pair_base_cursor = cursor_delay;
                        pair_base_prev_start = prev_item_start;
                        pair_from_delay = item_delay;
                        use_pair_base = true;
                    } else {
                        use_pair_base = false;
                    }

                    let over_ms = if let ExprIR::LiteralExpr { value } = over {
                        match value.as_f64() {
                            Some(d) if d > 0.0 => d,
                            Some(_) => {
                                self.warnings.push(
                                    "Timeline 'over' duration must be positive, using default 300ms"
                                        .into(),
                                );
                                300.0
                            }
                            None => {
                                self.warnings.push(
                                    "Timeline 'over' duration not a valid number, using default 300ms"
                                        .into(),
                                );
                                300.0
                            }
                        }
                    } else {
                        300.0
                    };

                    let path_config = path
                        .as_ref()
                        .map(|p| {
                            let mut path_parts =
                                vec![format!("\"path\": \"{}\"", p.replace('"', "\\\""))];
                            if let Some(pr) = path_rotate {
                                path_parts.push(format!("\"pathRotate\": {}", pr));
                            }
                            path_parts.join(",\n\x20\x20\x20\x20")
                        })
                        .unwrap_or_default();

                    let ci = child_idx;
                    child_idx += 1;

                    self.mount_js.push_str(&format!(
                        "__timeouts.push(setTimeout(function() {{\n\
                         \x20 const el = __children[{ci}];\n\
                         \x20 if (el) {{\n\
                         \x20\x20 const config = {{\n\
                         \x20\x20\x20 {from}\
                         \x20\x20\x20 {to}\
                         \x20\x20\x20 \"over\": {over},\n\
                         \x20\x20\x20 \"easing\": {easing}{path_comma}{path}\n\
                         \x20\x20 }};\n\
                         \x20\x20 const player = new AnimationPlayer(el, config);\n\
                         \x20\x20 __omni_animations.set(\"{label}\", player);\n\
                         \x20\x20 player.play();\n\
                         \x20 }}\n\
                         }}, {delay}));\n",
                        ci = ci,
                        from = if from_props.is_empty() {
                            String::new()
                        } else {
                            format!("\"from\": {{{}}},\n", from_props)
                        },
                        to = if to_props.is_empty() {
                            String::new()
                        } else {
                            format!("\"to\": {{{}}},\n", to_props)
                        },
                        over = over_ms,
                        easing = ease
                            .as_ref()
                            .map(|e| self.easing_to_js(e))
                            .unwrap_or_else(|| "\"easeOut\"".into()),
                        path_comma = if path_config.is_empty() {
                            ""
                        } else {
                            ",\n\x20\x20\x20\x20"
                        },
                        path = path_config,
                        delay = item_delay,
                        label = item_label,
                    ));

                    prev_item_start = item_delay;
                    cursor_delay = item_delay + over_ms;
                }
                TimelineItemKindIR::Set(props) => {
                    let props_str: Vec<String> = props
                        .iter()
                        .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                        .collect();
                    let ci = child_idx;
                    child_idx += 1;
                    self.mount_js.push_str(&format!(
                        "__timeouts.push(setTimeout(function() {{\n\
                         \x20 const el = __children[{ci}];\n\
                         \x20 if (el) el.style.cssText = Object.entries({{{props}}}).map(([k,v]) => k + ':' + v).join(';');\n\
                         }}, {delay}));\n",
                        ci = ci,
                        props = props_str.join(", "),
                        delay = cursor_delay
                    ));
                    prev_item_start = cursor_delay;
                }
                TimelineItemKindIR::MotionRef(motion_name) => {
                    self.mount_js.push_str(&format!(
                        "__timeouts.push(setTimeout(function() {{\n\
                         \x20 if (!el) return;\n\
                         \x20 el.querySelectorAll(\"[data-nid]\").forEach(function(child) {{\n\
                         \x20\x20 const player = __omni_motion_{motion}(child, {{}});\n\
                         \x20\x20 __omni_animations.set(child.getAttribute('data-nid') + '_{motion}', player);\n\
                         \x20\x20 player.play();\n\
                         \x20 }});\n\
                         }}, {delay}));\n",
                        motion = motion_name,
                        delay = cursor_delay
                    ));
                    prev_item_start = cursor_delay;
                }
                TimelineItemKindIR::Stagger(motion_expr, timing, _direction) => {
                    let motion_name = match &**motion_expr {
                        MotionExprIR::Ref(name) => name.clone(),
                        MotionExprIR::Inline(_) => String::new(),
                    };
                    let stagger_delay = if let ExprIR::LiteralExpr { value } = &timing.each {
                        match value.as_f64() {
                            Some(d) if d > 0.0 => d,
                            Some(_) => {
                                self.warnings.push(
                                    "Stagger 'each' delay must be positive, using default 50ms"
                                        .into(),
                                );
                                50.0
                            }
                            None => {
                                self.warnings.push(
                                    "Stagger 'each' delay not a valid number, using default 50ms"
                                        .into(),
                                );
                                50.0
                            }
                        }
                    } else {
                        50.0
                    };
                    if !motion_name.is_empty() {
                        self.mount_js.push_str(&format!(
                            "__timeouts.push(setTimeout(function() {{\n\
                             \x20 if (!el) return;\n\
                             \x20 el.querySelectorAll(\"[data-nid]\").forEach(function(child, idx) {{\n\
                             \x20\x20 __timeouts.push(setTimeout(function() {{\n\
                             \x20\x20\x20 const player = __omni_motion_{motion}(child, {{}});\n\
                             \x20\x20\x20 __omni_animations.set(child.getAttribute('data-nid') + '_{motion}', player);\n\
                             \x20\x20\x20 player.play();\n\
                             \x20\x20 }}, idx * {stagger}));\n\
                             \x20 }});\n\
                             }}, {delay}));\n",
                            motion = motion_name,
                            stagger = stagger_delay,
                            delay = cursor_delay
                        ));
                    }
                    prev_item_start = cursor_delay;
                }
                TimelineItemKindIR::Label(ref name, ref on_enter) => {
                    labels.push((name.clone(), cursor_delay));
                    if let Some(ref expr) = on_enter {
                        let callback_js = self.expr_to_js(expr);
                        self.mount_js.push_str(&format!(
                            "__timeouts.push(setTimeout(function() {{ {} }}, {}));\n",
                            callback_js, cursor_delay
                        ));
                    }
                }
            }

            match &item.connector {
                ConnectorIR::None => {
                    cursor_delay = 0.0;
                }
                ConnectorIR::Then => {}
                ConnectorIR::With => {}
            }
        }

        if !labels.is_empty() {
            let labels_str: Vec<String> = labels
                .iter()
                .map(|(name, pos)| format!("\"{}\": {}", name, pos))
                .collect();
            self.mount_js.push_str(&format!(
                "__omni_timeline_labels.set(\"{}\", {{{}}});\n",
                timeline.name,
                labels_str.join(", ")
            ));
        }

        let timeline_js = self.mount_js.split_off(saved_len);
        self.js.push_str(&format!(
            "function __omni_motion_{name}(el, config) {{\n\
             \x20 var __children = el ? Array.from(el.querySelectorAll('[data-nid]')) : [];\n\
             \x20 var __childIdx = 0;\n\
             \x20 var __timeouts = [];\n{content}\
             \x20 return {{\n\
             \x20\x20 name: \"{name}\",\n\
             \x20\x20 type: \"timeline\",\n\
             \x20\x20 kill: function() {{\n\
             \x20\x20\x20 __timeouts.forEach(function(t) {{ clearTimeout(t); }});\n\
             \x20\x20\x20 // Also cancel any running WAAPI animations on children\n\
             \x20\x20\x20 if (el) el.querySelectorAll('[data-nid]').forEach(function(c) {{\n\
             \x20\x20\x20\x20 c.getAnimations().forEach(function(a) {{ a.cancel(); }});\n\
             \x20\x20\x20 }});\n\
             \x20\x20 }}\n\
             \x20 }};\n\
             }};\n",
            name = timeline.name,
            content = timeline_js
        ));
        self.mount_js
            .push_str(&format!("__omni_motion_{}(null, {{}});\n", timeline.name));
    }
}
