# R4 File Size Reduction — Design Spec

> **Baseline:** 2026-06-29 (SHA: `67a9151`), 30 source files, 18 files > 800 lines
> **Goal:** All `.rs` files ≤ 800 lines (code, excluding `#[cfg(test)]` blocks)
> **Strategy:** Batch by module family; extract tests first; split by concern

---

## Batch 1: Codegen Targets (8 files)

### Pattern: Extract tests first, then split code

Each codegen target becomes a directory. The `#[cfg(test)] mod tests` block
moves to `tests.rs` in every case. This alone fixes two targets.

### rust.rs (1812 total, 787 code) → `rust/`

```
src/codegen/rust/
├── mod.rs       # RustCodegen, Codegen impl, generate (~500 lines)
├── expr.rs      # format_expr_ir_rust, generate_statement_rust (~287 lines)
└── tests.rs     # all tests (~1025 lines)
```

**Why:** Code is already <800. Only tests need extraction.

### dart.rs (1748 total, 769 code) → `dart/`

```
src/codegen/dart/
├── mod.rs       # DartCodegen, Codegen impl, generate (~450 lines)
├── expr.rs      # format_expr_ir_dart, generate_statement_dart, dart_type (~319 lines)
└── tests.rs     # all tests (~979 lines)
```

**Why:** Code is already <800. Only tests need extraction.

### javascript.rs (2220 total, 911 code) → `javascript/`

```
src/codegen/javascript/
├── mod.rs       # JavaScriptCodegen, Codegen impl, generate (~300 lines)
├── expr.rs      # format_expr_ir_js, generate_statement_js, format_json_value_js (~350 lines)
├── utils.rs     # js_type, binary_op_to_js, field_extract_js, emit_event_object_js (~261 lines)
└── tests.rs     # all tests (~1309 lines)
```

### dioxus.rs (1885 total, 1059 code) → `dioxus/`

```
src/codegen/dioxus/
├── mod.rs       # DioxusCodegen, Codegen impl, gen_component_rs, gen_store_rs (~450 lines)
├── view.rs      # gen_view_node_rsx, motion/trigger helpers (~350 lines)
├── expr.rs      # format_expr_ir_dioxus, generate_statement_dioxus (~259 lines)
└── tests.rs     # all tests (~826 lines)
```

### vue.rs (1824 total, 1002 code) → `vue/`

```
src/codegen/vue/
├── mod.rs       # VueCodegen, Codegen impl, gen_component_vue, gen_store_vue (~450 lines)
├── template.rs  # gen_template, gen_view_node_template (~352 lines)
├── expr.rs      # format_expr_ir_vue, generate_statement_vue (~200 lines)
└── tests.rs     # all tests (~822 lines)
```

### react.rs (2897 total, 1955 code) → `react/`

```
src/codegen/react/
├── mod.rs       # ReactCodegen, Codegen impl, gen_component_tsx, gen_store_hook (~550 lines)
├── view.rs      # gen_view_node_tsx, all motion/trigger effects_tsx fns (~700 lines)
├── expr.rs      # format_expr_ir_tsx, generate_statement_tsx, format_inline_handler (~350 lines)
├── utils.rs     # capitalize_first, format_json_value_tsx, binary_op_to_tsx, to_css_prop,
│                #   to_jsx_attr, to_react_event, field_extract_react, emit_event_object_react (~355 lines)
└── tests.rs     # all tests (~942 lines)
```

### flutter.rs (3665 total, 2515 code) → `flutter/`

```
src/codegen/flutter/
├── mod.rs       # FlutterCodegen, Codegen impl, gen_component_dart, gen_store_dart (~500 lines)
├── view.rs      # format_view_tree, format_element, format_conditional, format_list_rendering (~520 lines)
├── expr.rs      # format_expr_ir_flutter, format_json_value_flutter, binary_op_to_flutter (~400 lines)
├── stmt.rs      # generate_statement_flutter, format_inline_handler_flutter (~200 lines)
├── motion.rs    # All motion/trigger helpers + InView/Scroll/Drag widget classes (~700 lines)
├── utils.rs     # tag_to_flutter_widget, event_to_flutter_callback, prop_to_flutter_param_name,
│                #   capitalize_first, extract_keyframe_properties, format_easing_flutter, etc. (~195 lines)
└── tests.rs     # all tests (~1150 lines)
```

### omniview.rs (5084 total, 5065 code) → `omniview/`

This is the hardest file. 5065 lines of code with almost no tests. The single
function `emit_list_rendering_html` is ~1214 lines alone. Strategy:

```
src/codegen/omniview/
├── mod.rs        # OmniViewGenerator struct fields, generate(), compile_files() (~300 lines)
├── runtime.rs    # emit_js_header, emit_signal_runtime, emit_animation_runtime (~140 lines)
├── store.rs      # emit_store, generate_component, emit_signal_decl, emit_computed_decl,
│                 #   emit_function, emit_function_named (~500 lines)
├── stmt.rs       # emit_statement, emit_mutation (~290 lines)
├── view.rs       # emit_view, emit_dom_and_bindings, emit_style (~120 lines)
├── editor.rs     # emit_editor_bridge, emit_conditional_html (~290 lines)
├── list.rs       # emit_list_rendering_html — part 1: init/state setup (~600 lines)
├── list_render.rs # emit_list_rendering_html — part 2: DOM gen/event binding (~614 lines)
├── motion.rs     # emit_motion_js, emit_states_js, format_transform_value (~460 lines)
├── motion_triggers.rs # emit_motion_triggers_for_element (~658 lines)
├── timeline.rs   # emit_timeline_js (~718 lines)
└── utils.rs      # serde_value_to_js, map_tag, map_event, emit_event_object,
                  #   collect_for_events, map_binary_op, to_kebab, to_css_property,
                  #   escape_html, wrap_debug_html (~200 lines)
```

**Note on `emit_list_rendering_html` split:** This function is a large match/if-else
tree generating JS template strings for @for list rendering. The split point is
at the boundary between "state variable initialization" and "DOM node generation."
Both halves remain private methods on OmniViewGenerator, called sequentially from
the original `emit_list_rendering_html` which becomes a thin dispatcher in `list.rs`.

---

## Batch 2: IR Module (ir/mod.rs, 3576 code)

```
src/ir/
├── mod.rs          # Re-exports (pub use types::*; pub use lower_ast::*;),
│                   #   IRProgram::new(), all_components(), all_stores(), etc. (~100 lines)
├── types.rs        # All IR type definitions: ExprIR, StatementIR, ViewTree, ComponentIR,
│                   #   StoreIR, MotionIntent, TimelineIR, StatesIR, HeroIntent,
│                   #   EventDataKind, EventFieldDef, CallbackBodyIR, etc. (~1050 lines)
├── lower_ast.rs    # AST→IR lowering: lower(), lower_component(), lower_view_node(),
│                   #   lower_stmt(), lower_expr(), try_lower_mutation(),
│                   #   EventFieldCollector, etc. (~2500 lines)
├── lower_logic.rs  # Already exists (147 lines) — pure function/store lowering
└── logic.rs        # Already exists (92 lines) — logic IR types
```

**`lower_ast.rs` at 2500 lines is still over 800.** This is tracked as follow-up
work: it can be further split by lowering phase (lower_component, lower_view,
lower_stmt) in a future session. The immediate win is separating types from logic
and getting the IR module organized.

---

## Batch 3: Semantic (lints.rs 2025 code + mod.rs 2942 code)

### lints.rs → `lints/` directory

```
src/semantic/lints/
├── mod.rs               # LintRule trait, LintSeverity, LintDiagnostic, run_all(), all_rules() (~120 lines)
├── helpers.rs           # AST name reference collectors, Store/Prop/Computed validation helpers (~350 lines)
├── undefined_ref.rs     # UndefinedRef rule + tests (~250 lines)
├── type_mismatch.rs     # TypeMismatch rule + tests (~280 lines)
├── store_field.rs       # StoreFieldDirectMutation rule + tests (~285 lines)
├── prop_immutability.rs # PropImmutability rule + tests (~250 lines)
├── computed_purity.rs   # ComputedPurity rule + tests (~250 lines)
├── missing_return.rs    # MissingReturn rule + tests (~250 lines)
├── dead_code.rs         # DeadCode rule + tests (~360 lines)
├── unused_import.rs     # UnusedImport rule + tests (~280 lines)
├── invalid_event_field.rs  # InvalidEventField rule + tests (~280 lines)
└── store_action_event.rs   # StoreActionEventData rule + tests (~280 lines)
```

Each rule file contains: rule struct, `impl LintRule`, `#[cfg(test)] mod tests`.
Tests stay co-located with their rule since each file remains well under 800 lines.

### semantic/mod.rs → split by pass

```
src/semantic/
├── mod.rs          # SemanticResult, analyze() entry point, SemanticAnalyzer::new() (~100 lines)
├── collect.rs      # Pass 1: collect symbols (modules, fns, components, stores, types) (~350 lines)
├── resolve.rs      # Pass 2: resolve references + view tree resolution (~600 lines)
├── validate.rs     # Pass 3-4: dependency collection + validation
│                   #   (type checking, store refs, motion validation) (~900 lines)
├── motion.rs       # Standalone motion validation helpers:
│                   #   collect_element_ids, is_valid_easing_name, is_keyword_value,
│                   #   has_enter_leave_animation, contains_velocity_ref,
│                   #   parse_duration_ms, collect_timeline_refs (~350 lines)
├── helpers.rs      # type_to_string, is_non_interpolatable, TypeAnnotation impl (~100 lines)
├── symbols.rs      # Already exists
├── cycles.rs       # Already exists
└── lints/          # Directory as above
```

**`validate.rs` at ~900 lines is borderline.** If it exceeds 800 after extraction,
split into `validate_types.rs` (type checking) and `validate_motion.rs` (motion
validation). Exact line counts will be confirmed during implementation.

---

## Batch 4: Parser (top_level.rs + view.rs + expr.rs + ast.rs)

### top_level.rs (2085 code) → `top_level/` directory

```
src/parser/top_level/
├── mod.rs          # parse_top_level_item() dispatcher (~100 lines)
├── component.rs    # parse_component_def() (~400 lines)
├── function.rs     # parse_function_def(), parse_action_def(), parse_lifecycle() (~350 lines)
├── store.rs        # parse_store_def() (~150 lines)
├── struct_def.rs   # parse_struct_def(), parse_tagged_union_def(), parse_type_alias() (~250 lines)
├── module.rs       # parse_module(), parse_import(), parse_export() (~350 lines)
└── motion.rs       # parse_motion_block(), parse_timeline_block(), parse_position_string() (~535 lines)
```

### view.rs (1993 code) → `view/` directory

```
src/parser/view/
├── mod.rs          # parse_view_node() main body, parse_element() (~700 lines)
├── directives.rs   # @if, @for, @show directives, @enter/@hover motion trigger parsing (~700 lines)
└── style.rs        # style_value_to_part(), CSS function arg parsing, dimension helpers (~593 lines)
```

### expr.rs (1034 code) → `expr/` directory

Only ~200 lines over 800. The cleanest split is extracting motion/easing parsers
(~276 lines) since they're a distinct concern called from `top_level/motion.rs`:

```
src/parser/expr/
├── mod.rs          # Pratt parser core: parse_expr, parse_expr_bp, parse_prefix,
│                   #   infix_bp, parse_infix, parse_primary, parse_ident_expr,
│                   #   parse_call_args, parse_assign, parse_match_*, etc. (~758 lines)
└── motion.rs       # parse_easing, parse_motion_expr, parse_elastic_params,
│                   #   parse_stagger_call, parse_number_literal (~276 lines)
```

### ast.rs (975 code) → `ast/` directory

All type definitions. Natural split by domain — OmniView types and Motion types
each form a cohesive sub-module:

```
src/parser/ast/
├── mod.rs          # Base types (TypeAnnotation, Expr, Literal, BinaryOpKind, Stmt,
│                   #   Block, MatchArm), Top-level decls (Param, FunctionDef, StructDef,
│                   #   TaggedUnionDef, TypeAlias), Visibility, Import/Export,
│                   #   Module, Program (~500 lines)
├── view.rs         # OmniView-specific: ComponentDef, PropDecl, SignalDecl, ComputedDecl,
│                   #   EventDecl, RefDecl, LifecycleHook, ActionDef, ViewBlock, ViewNode,
│                   #   Element, ElementProp, EventHandler, ClassValue, IfDirective,
│                   #   ForDirective, StyleRule, StyleValue, StyleValuePart (~210 lines)
└── motion.rs       # Motion/Animation: MotionDef, TimelineDef, MotionBody, KeyframeBody,
│                   #   KeyframeStop, TimelineBody, TimelineItem, DirectionItem, Direction,
│                   #   Connector, PositionValue, CallbackBody, PlaybackProp, LoopValue,
│                   #   DirectionValue, FillValue, InterruptValue, WhenValue, EasingExpr,
│                   #   StaggerCall, MotionTrigger, TriggerKind, StateDef, HeroDef
│                   #   + Store & Theme types (~310 lines)
```

---

## Batch 5: Remaining (stdlib.rs + lib.rs)

### lsp.rs — DELETE

`lsp.rs` (1634 lines) is dead code: the file exists but is never declared as a
module in `lib.rs` and is not compiled. **Delete it.** This removes one file
from the >800 list immediately, no splitting needed.

### stdlib.rs (1211 code) → `stdlib/` directory

```
src/stdlib/
├── mod.rs          # StdlibFn type, lookup(), module_available(), available_targets_for(),
│                   #   static registry (~120 lines)
├── http.rs         # codegen_http_send, codegen_http_get, codegen_http_post (~160 lines)
├── json.rs         # codegen_json_parse, codegen_json_stringify (~70 lines)
├── storage.rs      # codegen_storage_get, codegen_storage_set (~70 lines)
├── math.rs         # codegen_math_abs/min/max/ceil/floor/round/random/sqrt/pow (~240 lines)
├── string.rs       # codegen_string_length/concat/split/join/replace/substring/contains/
│                   #   to_upper/to_lower/trim/starts_with/ends_with (~310 lines)
├── array.rs        # codegen_array_length/push/pop/map/filter/find (~200 lines)
├── console.rs      # codegen_console_log/warn/error (~70 lines)
└── tests.rs        # all tests (~348 lines)
```

### lib.rs (970 code) → extract WASM + FFI + validation

Actual breakdown: WASM bindings (~72 lines, `#[cfg(feature = "wasm")]`), C FFI (~270 lines,
`extern "C"` + `#[no_mangle]`), multi-file validation (~436 lines), main pipeline (~192 lines).

```
src/
├── lib.rs          # Crate docs, public re-exports, module declarations,
│                   #   CompileResult, compile(), OmniViewResult, compile_omniview(),
│                   #   compile_to_target_json(), compile_multi_to_target() (~450 lines)
├── wasm.rs         # WASM exports: compile_to_omniview, compile_omniview_component,
│                   #   compile_to_ir, version — behind #[cfg(feature = "wasm")] (~72 lines)
├── ffi.rs          # C FFI: rust_to_c_string, omni_free_string, omni_version,
│                   #   omni_analyse, omni_compile_multi_to_target (~270 lines)
└── validate.rs     # validate_multi(), validate_file_consistency(),
│                   #   build_pub_symbols(), validate_cross_file(),
│                   #   validate_named_import(), CrossFileSymbolKind (~436 lines)
```

**Note:** `lib.rs` declares `#[cfg(feature = "wasm")] pub mod wasm;` so WASM bindings
remain conditionally compiled.

---

## Final Tally

| Metric | Before | After |
|--------|--------|-------|
| Source files | 30 | ~76 |
| Files > 800 lines (code) | 18 | **1** (lower_ast.rs, tracked for follow-up) |
| Largest file | omniview.rs (5084) | lower_ast.rs (~2500, down from mod.rs 3576) |

**Remaining >800:** `lower_ast.rs` at ~2500 lines. This is a 30% reduction from the
original `ir/mod.rs` (3576 lines). It can be further split by lowering phase
(`lower_component.rs`, `lower_view.rs`, `lower_stmt.rs`) in a follow-up — the
immediate win is separating IR types (now in `types.rs`, ~1050 lines) from
lowering logic.

## Visibility Changes Required

When splitting a single `.rs` file into a directory with sub-modules, private
`fn` items that are called across the new file boundaries must become
`pub(super)` so sibling modules can access them.

**Audit needed per batch:**
- **Codegen:** All `fn emit_*`, `fn format_*`, `fn gen_*` helper methods on the
  generator struct cross-reference each other. They become `pub(super)` methods
  on the same `impl` block (split across files via inherent `impl` blocks).
- **IR:** `lower()` calls `lower_component()`, `lower_view_node()`, etc. These
  become `pub(super)` free functions in `lower_ast.rs`.
- **Semantic:** Analyzer passes share `SemanticAnalyzer` fields. Split as
  `pub(super)` methods on the same struct.
- **Parser:** Each `parse_*()` function in `top_level/` sub-modules is called
  from the dispatcher in `mod.rs`. These become `pub(super)`.
- **stdlib:** Standalone `codegen_*()` functions become `pub(super)`.

No public API changes. `pub(crate)` items stay `pub(crate)`. The only change is
private `fn` → `pub(super) fn` for items accessed across sibling module
boundaries. `cargo clippy` will flag any unnecessarily-public items afterward
so we can tighten them back.

### Implementation Order

Batches are ordered by impact and dependency:
1. **Codegen** (8 files → ~50 files) — biggest win, no structural dependencies
2. **IR** (1 file → 3 files) — foundational, codegen depends on these types
3. **Semantic** (2 files → ~16 files) — lints depend on IR types
4. **Parser** (4 files → ~18 files) — feeds into semantic
5. **Remaining** (3 files → ~18 files) — leaf modules

Each batch should be committed and tested independently before moving to the next.
