// ═══════════════════════════════════════════════════════════
// Motion effect codegen: per-ref native effects for all 8 triggers
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;

use crate::codegen::react::motion;

/// Emit all motion effect blocks:
///   1. `_M` motion definition table (named @motion defs, WAAPI keyframes)
///   2. Per motion element: useCallback handlers for @hover/@press and
///      useEffect blocks for @enter/@focus/@in-view/@scroll/@drag-*
///
/// All effects are keyed on element refs (user `@ref` or synthesized
/// `__el_N`) — no data-* attributes, no document.querySelectorAll, no
/// MutationObserver.
pub(super) fn gen_motion_effects(
    w: &mut CodeWriter,
    view: &ViewTree,
    ir: &IRProgram,
    component_map: &FxHashMap<&str, &ComponentIR>,
) {
    // Build complete motion definition map (module-level + component-level
    // motions; inline bodies are hoisted to named motions by lowering)
    let mut motion_map: FxHashMap<String, MotionBodyIR> = FxHashMap::default();
    for m in ir.modules.values().flat_map(|m| &m.motions) {
        motion_map.insert(m.name.clone(), m.body.clone());
    }

    // Per-element @enter/@hover/@press/@focus/@in-view/@scroll/@drag effects
    motion::emit_motion_effects_tsx(w, view, &motion_map, component_map);
}
