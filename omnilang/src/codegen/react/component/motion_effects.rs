// ═══════════════════════════════════════════════════════════
// Motion effect codegen: @enter/@hover/@press/@focus/@driven-by/scroll/drag
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;

use crate::codegen::react::motion;

/// Emit all five motion-related effect blocks:
///   1. @enter / @hover / @press / @focus  (animation triggers)
///   2. @driven-by                         (external progress control)
///   3. @in-view                           (IntersectionObserver)
///   4. @scroll                            (scroll-driven progress)
///   5. @drag-horizontal / @drag-vertical  (gesture-driven progress)
pub(super) fn gen_motion_effects(w: &mut CodeWriter, view: &ViewTree, ir: &IRProgram) {
    // Build complete motion definition map (component-level + module-level)
    let mut motion_map: FxHashMap<String, MotionBodyIR> = FxHashMap::default();
    for m in ir.modules.values().flat_map(|m| &m.motions) {
        motion_map.insert(m.name.clone(), m.body.clone());
    }

    // 1. @enter / @hover / @press / @focus animation triggers
    motion::emit_motion_effects_tsx(w, view, &motion_map);

    // 2. @driven-by effects — reactive hooks for external progress control
    motion::emit_driven_by_effects_tsx(w, view);

    // 3. @in-view effects — IntersectionObserver for entrance animations
    motion::emit_inview_effects_tsx(w, view);

    // 4. @scroll effects — scroll-driven animation progress
    motion::emit_scroll_effects_tsx(w, view);

    // 5. @drag-horizontal / @drag-vertical effects — gesture-driven animation
    motion::emit_drag_effects_tsx(w, view);
}
