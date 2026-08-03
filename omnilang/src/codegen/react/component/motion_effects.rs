// ═══════════════════════════════════════════════════════════
// Motion effect codegen: per-ref native effects for all 8 triggers
// ═══════════════════════════════════════════════════════════

use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;

use crate::codegen::react::motion;

/// Emit all motion effect blocks:
///   1. `_M` motion definition table (named @motion defs, WAAPI keyframes)
///      — at MODULE scope when `table_hoisted` (task 2.6: the table is a
///      compile-time constant and was re-allocated per render), else
///      in-function (a referenced motion carries render-time values).
///   2. Per motion element: useCallback handlers for @hover/@press and
///      useEffect blocks for @enter/@focus/@in-view/@scroll/@drag-*
///
/// `store_field_renames` (task 2.10, perf M1) resolve `Store.field` reads
/// in motion values and trigger control dimensions to slice locals — only
/// consulted for the in-function table and the per-element effects, both
/// inside the component where the slice locals are in scope.
///
/// All effects are keyed on element refs (user `@ref` or synthesized
/// `__el_N`) — no data-* attributes, no document.querySelectorAll, no
/// MutationObserver.
pub(super) fn gen_motion_effects(
    w: &mut CodeWriter,
    view: &ViewTree,
    motion_map: &FxHashMap<String, MotionBodyIR>,
    component_map: &FxHashMap<&str, &ComponentIR>,
    table_hoisted: bool,
    store_field_renames: &FxHashMap<String, String>,
) {
    // Per-element @enter/@hover/@press/@focus/@in-view/@scroll/@drag effects
    motion::emit_motion_effects_tsx(
        w,
        view,
        motion_map,
        component_map,
        table_hoisted,
        store_field_renames,
    );
}
