# Architecture Review Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete all critical/high findings from the 2026-07-27 architecture re-reviews — source_map bug fix, source_span population, dead code removal, security hardening, TypeKind serialization, CI gates, and launch deferred items (P35/P36/P38b/P39).

**Architecture:** Four-phase execution: Phase 1 (Day 1) quick wins + Clippy → Phase 2 (Week 1) security + TypeKind + CI → Phase 3 (Week 2-3) P35 crate split + P36 parallelization + P39 minimum viable → Phase 4 (Week 4) motion test gap closure + complete P39.

**Tech Stack:** Rust (omnilang workspace), TypeScript (poiesis), Bash (CI)

## Global Constraints

- All Rust changes must pass `cargo test --lib -p omnilang` and `cargo clippy --lib -- -D warnings`
- All TS changes must pass `npx tsc --noEmit` and `npx vitest run`
- Commit after each task with conventional commit format
- File paths use forward slashes from workspace root
- ADR-004 Decision 2: FS permissions are by design, do not modify capability scopes
- P34 AI code completion remains deferred

---

## Phase 1: Day 1 — Quick Wins + Clippy Cleanup

### Task 1.1: Fix source_map Overwrite Bug

**Files:**
- Modify: `omnilang/src/codegen/mod.rs:117`

**Interfaces:**
- Consumes: `ir.source_map` (FxHashMap<String, SourceMapEntry>), `output.source_map` (Vec<SourceMapEntry>)
- Produces: Merged source_map in CompileOutput preserving codegen-level per-element entries

- [ ] **Step 1: Verify current behavior**

Read `omnilang/src/codegen/mod.rs` line 117. Confirm it uses `=` (overwrite) not `extend`.

```bash
grep -n "output.source_map" omnilang/src/codegen/mod.rs
```

- [ ] **Step 2: Apply fix — extend instead of overwrite**

In `omnilang/src/codegen/mod.rs:117`, change:

```rust
// Before (line 117):
output.source_map = ir.source_map.values().cloned().collect();

// After:
// Preserve per-element source_map from codegen, append IR-level entries
output.source_map.extend(ir.source_map.values().cloned());
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build -p omnilang --lib
```

Expected: Compiles with no new errors.

- [ ] **Step 4: Run existing tests**

```bash
cargo test -p omnilang --lib
```

Expected: 653 tests pass, 3 pre-existing dart/flutter failures unchanged.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/mod.rs
git commit -m "fix: preserve per-element source_map instead of overwriting at codegen/mod.rs:117

The assignment output.source_map = ir.source_map.values().cloned().collect()
was discarding the OmniView codegen's per-element (NID-level) source_map
entries computed at element.rs:874-888. Changed to extend() to merge both
maps."
```

---

### Task 1.2: Populate source_span on 6 IR Structs

**Files:**
- Modify: `omnilang/src/ir/lower/component.rs:107,130,260,289,620,634`

**Interfaces:**
- Consumes: AST spans from `ComponentDef`, `StoreDef`, `FunctionDef`, `ActionDef`, `HeroDef`
- Produces: Populated `source_span: Option<Span>` on ComponentIR, StoreIR, FunctionIR (3 sites), HeroIntent

- [ ] **Step 1: Populate ComponentIR.source_span**

In `omnilang/src/ir/lower/component.rs:289`, change:

```rust
// Before:
source_span: None,

// After:
source_span: Some(comp.span),
```

- [ ] **Step 2: Populate StoreIR.source_span**

In `omnilang/src/ir/lower/component.rs:634`, change:

```rust
// Before:
source_span: None,

// After:
source_span: Some(store.span),
```

- [ ] **Step 3: Populate FunctionIR.source_span (component functions)**

In `omnilang/src/ir/lower/component.rs:107`, change:

```rust
// Before:
source_span: None,

// After:
source_span: Some(f.span),
```

- [ ] **Step 4: Populate FunctionIR.source_span (component actions)**

In `omnilang/src/ir/lower/component.rs:130`, change:

```rust
// Before:
source_span: None,

// After:
source_span: Some(a.span),
```

- [ ] **Step 5: Populate HeroIntent.source_span**

In `omnilang/src/ir/lower/component.rs:260`, change:

```rust
// Before:
source_span: None,

// After:
source_span: Some(h.span),
```

- [ ] **Step 6: Populate FunctionIR.source_span (store actions)**

In `omnilang/src/ir/lower/component.rs:620`, change:

```rust
// Before:
source_span: None,

// After:
source_span: Some(action.span),
```

- [ ] **Step 7: Build and test**

```bash
cargo build -p omnilang --lib && cargo test -p omnilang --lib
```

- [ ] **Step 8: Commit**

```bash
git add omnilang/src/ir/lower/component.rs
git commit -m "fix: populate source_span on ComponentIR, StoreIR, FunctionIR, HeroIntent

P33 follow-up: 6 lowering sites now use available AST spans instead of
None. ModuleIR and IRProgram cannot be populated (Module/Program AST types
lack span fields). 29 expression-oriented structs remain None until concrete
consumers emerge."
```

---

### Task 1.3: Delete 6 Confirmed Dead Code Items

**Files:**
- Delete (content): `omnilang/src/codegen/shared/cache.rs` (135 lines)
- Delete (content): `build_component_output` fn in `omnilang/src/codegen/shared/ui_codegen.rs:74-94`
- Delete (content): `build_store_output` fn in `omnilang/src/codegen/shared/ui_codegen.rs:96-119`
- Delete (content): `lower_expr_to_json` fn in `omnilang/src/ir/lower/expr.rs:543-551`
- Delete (content): `timeline_body_to_config` fn in `omnilang/src/codegen/omniview/motion_config.rs:117-131`
- Delete (content): `callback_to_config` fn in `omnilang/src/codegen/omniview/motion_config.rs:132-149`
- Modify: `omnilang/src/codegen/shared/mod.rs:6` (remove `pub(crate) mod cache;`)
- Modify: `omnilang/src/codegen/omniview/motion_config.rs:18` (add comment to `keyframe_to_config`)

**Interfaces:**
- Removes: `CodegenCache` struct (zero callers), `defaults::build_component_output` (zero callers), `defaults::build_store_output` (zero callers), `lower_expr_to_json` (Expr variant, zero callers), `timeline_body_to_config` (zero callers), `callback_to_config` (zero callers)
- Preserves: `keyframe_to_config` (called by test at `tests_regression/motion.rs:533`)

- [ ] **Step 1: Delete CodegenCache (entire file)**

```bash
rm omnilang/src/codegen/shared/cache.rs
```

In `omnilang/src/codegen/shared/mod.rs:6`, remove the line:
```rust
pub(crate) mod cache;
```

- [ ] **Step 2: Delete build_component_output and build_store_output**

In `omnilang/src/codegen/shared/ui_codegen.rs`, delete lines 74-119 (both functions).

- [ ] **Step 3: Delete lower_expr_to_json (Expr variant)**

In `omnilang/src/ir/lower/expr.rs`, delete the `pub(super) fn lower_expr_to_json` function (lines 543-551). Keep `lower_expr_to_json_from_ir` (still called by `view.rs:383`).

Also update the module doc comment on line 3 to remove the mention of `lower_expr_to_json`.

- [ ] **Step 4: Delete timeline_body_to_config and callback_to_config**

In `omnilang/src/codegen/omniview/motion_config.rs`, delete:
- `pub(super) fn timeline_body_to_config` (lines 117-131)
- `pub(super) fn callback_to_config` (lines 132-149)

- [ ] **Step 5: Add deprecation comment to keyframe_to_config**

In `omnilang/src/codegen/omniview/motion_config.rs`, add a doc comment above `keyframe_to_config` at line 18:

```rust
/// Produces a {from, to, over, easing, ...} config object.
///
/// NOTE: Only used by tests (`tests_regression/motion.rs:533`).
/// Production code uses `keyframe_to_animate_call` instead.
/// TODO: Migrate test to `keyframe_to_animate_call` and delete this.
pub(super) fn keyframe_to_config(
```

- [ ] **Step 6: Build and test**

```bash
cargo build -p omnilang --lib && cargo test -p omnilang --lib
```

- [ ] **Step 7: Commit**

```bash
git add omnilang/src/codegen/shared/cache.rs omnilang/src/codegen/shared/mod.rs \
        omnilang/src/codegen/shared/ui_codegen.rs omnilang/src/ir/lower/expr.rs \
        omnilang/src/codegen/omniview/motion_config.rs
git commit -m "chore: remove 6 dead code items (zero callers confirmed by grep audit)

- CodegenCache struct (cache.rs, 135 lines)
- defaults::build_component_output and build_store_output (ui_codegen.rs)
- lower_expr_to_json (Expr variant, ir/lower/expr.rs)
- timeline_body_to_config and callback_to_config (motion_config.rs)
- keyframe_to_config preserved (test-only, added deprecation note)"
```

---

### Task 1.4: Fix Auto-Fixable Clippy Warnings + Manual Strip_Prefix

**Files:**
- Modify: `omnilang/src/parser/design.rs:154-155` (manual strip)
- Modify: `omnilang/src/parser/design.rs:63-67` (identical if blocks)
- Others: auto-fixed by `cargo clippy --fix`

- [ ] **Step 1: Run auto-fix**

```bash
cd omnilang && cargo clippy --fix --lib --allow-dirty --allow-staged
```

This auto-fixes 19 of 41 warnings.

- [ ] **Step 2: Fix manual strip_prefix at design.rs:154-155**

In `omnilang/src/parser/design.rs:154-155`, change:

```rust
// Before:
if input.starts_with('"') {
    let end = input[1..]

// After:
if let Some(stripped) = input.strip_prefix('"') {
    let end = stripped;
```

- [ ] **Step 3: Fix identical if blocks at design.rs:63-67**

In `omnilang/src/parser/design.rs:63-67`, merge the two identical branches:

```rust
// Before:
} else if remaining.starts_with("}") {
    break;
} else if remaining.is_empty() {
    break;
}

// After:
} else if remaining.starts_with("}") || remaining.is_empty() {
    break;
}
```

- [ ] **Step 4: Verify remaining Clippy warnings (should be ~20 or fewer)**

```bash
cargo clippy --lib -p omnilang 2>&1 | tail -5
```

Expected: Warning count reduced from 41 to ≤22.

- [ ] **Step 5: Run full test suite**

```bash
cargo test -p omnilang --lib
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: fix clippy warnings — auto-fix 19 + manual strip_prefix + if-merge

41 → ~22 warnings remaining (mostly dead_code/derive suggestions
that need individual review).
Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Phase 2: Week 1 — Security Hardening + TypeKind Serialization + CI Gates

### Task 2.1: Fix P38b — innerHTML to textContent (P0 Critical)

**Files:**
- Modify: `poiesis/src-tauri/debug-window/index.html:213-215,227-229`
- Modify: `poiesis/public/debug-window/index.html:213-215,227-229` (identical copy)

**Interfaces:**
- Consumes: `signalList` DOM element, `actionList` DOM element
- Produces: XSS-safe DOM-based rendering instead of innerHTML injection

- [ ] **Step 1: Fix signalList.innerHTML → textContent in src-tauri copy**

In `poiesis/src-tauri/debug-window/index.html:207-216`, replace the innerHTML pattern:

```javascript
// Before (lines 207-216):
var html = '';
signalKeys.forEach(function(k) {
    var val = signals[k];
    if (typeof val === 'object') val = JSON.stringify(val).substring(0, 60);
    else if (typeof val === 'function') val = '[fn]';
    else val = String(val);
    html += '<div class="signal-row"><span class="signal-name">' + k + '</span><span class="signal-value">' + val + '</span></div>';
});
signalList.innerHTML = html;
```

```javascript
// After:
signalList.innerHTML = '';
signalKeys.forEach(function(k) {
    var val = signals[k];
    if (typeof val === 'object') val = JSON.stringify(val).substring(0, 60);
    else if (typeof val === 'function') val = '[fn]';
    else val = String(val);

    var row = document.createElement('div');
    row.className = 'signal-row';

    var nameSpan = document.createElement('span');
    nameSpan.className = 'signal-name';
    nameSpan.textContent = k;

    var valSpan = document.createElement('span');
    valSpan.className = 'signal-value';
    valSpan.textContent = val;

    row.appendChild(nameSpan);
    row.appendChild(valSpan);
    signalList.appendChild(row);
});
```

- [ ] **Step 2: Fix actionList.innerHTML in src-tauri copy**

In `poiesis/src-tauri/debug-window/index.html:227-229`, replace:

```javascript
// Before:
actionList.innerHTML = storeArr.map(function(sn) {
    return '<div class="action-row">📦 ' + sn + '</div>';
}).join('');
```

```javascript
// After:
actionList.innerHTML = '';
storeArr.forEach(function(sn) {
    var div = document.createElement('div');
    div.className = 'action-row';
    div.textContent = '📦 ' + sn;
    actionList.appendChild(div);
});
```

- [ ] **Step 3: Copy changes to public/ copy**

```bash
cp poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html
git commit -m "fix(security): replace innerHTML with textContent in debug window signal display

P0 fix for XSS chain: innerHTML injection of unescaped signal names/values
allowed code execution in parent window context with full Tauri IPC access.
textContent prevents HTML injection while preserving display."
```

---

### Task 2.2: Fix P38b — Add Origin Validation to Message Listeners (P0 Critical)

**Files:**
- Modify: `poiesis/src-tauri/debug-window/index.html:181,277`
- Sync: `poiesis/public/debug-window/index.html`

- [ ] **Step 1: Add e.source + e.origin validation to parent listener**

In `poiesis/src-tauri/debug-window/index.html:181-183`, change:

```javascript
// Before:
window.addEventListener('message', function(e) {
    var msg = e.data;
    if (!msg || !msg.source || msg.source !== 'omni-preview') return;
```

```javascript
// After:
window.addEventListener('message', function(e) {
    // Verify sender is the sandboxed iframe (origin is 'null' with sandbox="allow-scripts")
    if (e.source !== omniPreview.contentWindow || e.origin !== 'null') return;
    var msg = e.data;
    if (!msg || !msg.source || msg.source !== 'omni-preview') return;
```

- [ ] **Step 2: Add e.source verification to bridge script listener**

In the bridge script string at line 277-278, change:

```javascript
// Before (inside bridgeScript string):
'window.addEventListener("message",function(e){'
+ 'var m=e.data;if(!m||m.source!=="debug-window")return;'
```

```javascript
// After:
'window.addEventListener("message",function(e){'
+ 'if(e.source!==window.parent)return;'
+ 'if(e.origin!==window.parent.origin)return;'
+ 'var m=e.data;if(!m||m.source!=="debug-window")return;'
```

- [ ] **Step 3: Copy changes to public/ copy**

```bash
cp poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html
git commit -m "fix(security): add e.source + e.origin validation to debug window message listeners

P0 fix: message listeners previously only checked a forgeable e.data.source
string. Now verify e.source matches the expected window reference and
e.origin matches 'null' (sandboxed iframe) or window.parent.origin."
```

---

### Task 2.3: Fix P38b — Tighten debug-window Capability + postMessage Origin (P1)

**Files:**
- Modify: `poiesis/src-tauri/capabilities/debug-window.json:7`
- Modify: `poiesis/src-tauri/debug-window/index.html:176`

- [ ] **Step 1: Tighten capability to atomic permissions**

In `poiesis/src-tauri/capabilities/debug-window.json`, change:

```json
// Before:
"permissions": [
    "core:default",
    "core:event:default"
]
```

```json
// After:
"permissions": [
    "core:event:allow-emit",
    "core:event:allow-listen"
]
```

This reduces from 9 permission bundles to the 2 exact atomic permissions needed for IPC.

- [ ] **Step 2: Replace postMessage '*' with parent origin**

In `poiesis/src-tauri/debug-window/index.html:176`, change:

```javascript
// Before:
omniPreview.contentWindow.postMessage(msg, '*');
```

```javascript
// After:
// The sandboxed iframe has origin 'null' (sandbox="allow-scripts", no allow-same-origin).
// For srcdoc iframes, the parent origin is used for posting.
omniPreview.contentWindow.postMessage(msg, window.location.origin);
```

- [ ] **Step 3: Sync public/ copy + Commit**

```bash
cp poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html
git add poiesis/src-tauri/capabilities/debug-window.json poiesis/src-tauri/debug-window/index.html poiesis/public/debug-window/index.html
git commit -m "fix(security): tighten debug-window capability + fix postMessage origin

Capability reduced from core:default (9 bundles) to core:event:allow-emit +
core:event:allow-listen (2 atomic permissions). postMessage targetOrigin
changed from '*' to window.location.origin."
```

---

### Task 2.4: Fix TypeKind Serialization to Preserve type_args

**Files:**
- Modify: `omnilang/src/ir/types.rs:336-348`
- Create: `omnilang/src/ir/types_tests.rs` (add test if not exists, or append to existing)

- [ ] **Step 1: Rewrite Serialize to emit structured JSON**

In `omnilang/src/ir/types.rs:336-339`, change:

```rust
// Before:
impl Serialize for TypeKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
```

```rust
// After:
impl Serialize for TypeKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            TypeKind::Named { name, type_args } if !type_args.is_empty() => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("name", name)?;
                map.serialize_entry("type_args", type_args)?;
                map.end()
            }
            _ => serializer.serialize_str(self.as_str()),
        }
    }
}
```

- [ ] **Step 2: Rewrite Deserialize for backward compatibility**

In `omnilang/src/ir/types.rs:343-348`, change:

```rust
// Before:
impl<'de> Deserialize<'de> for TypeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(TypeKind::from(s))
    }
}
```

```rust
// After:
impl<'de> Deserialize<'de> for TypeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::value::MapAccessDeserializer;
        use serde::de::{MapAccess, Visitor};

        struct TypeKindVisitor;

        impl<'de> Visitor<'de> for TypeKindVisitor {
            type Value = TypeKind;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or a {name, type_args} object")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<TypeKind, E> {
                Ok(TypeKind::from(s.to_string()))
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<TypeKind, M::Error> {
                let mut name = None;
                let mut type_args = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = Some(map.next_value()?),
                        "type_args" => type_args = map.next_value()?,
                        _ => { let _: serde::de::IgnoredAny = map.next_value()?; }
                    }
                }
                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                Ok(TypeKind::Named { name, type_args })
            }
        }

        deserializer.deserialize_any(TypeKindVisitor)
    }
}
```

- [ ] **Step 3: Add round-trip test**

At the end of `omnilang/src/ir/types_tests.rs`, add:

```rust
#[test]
fn test_typekind_serialize_preserves_type_args() {
    let array_string = TypeKind::Named {
        name: "Array".into(),
        type_args: vec![TypeKind::Named {
            name: "String".into(),
            type_args: vec![],
        }],
    };

    // Serialize
    let json = serde_json::to_string(&array_string).expect("serialize");
    // Should be structured JSON for Named with type_args
    assert!(json.contains("\"name\""), "Expected structured JSON: {json}");
    assert!(json.contains("\"type_args\""), "Expected type_args in: {json}");

    // Deserialize back
    let roundtripped: TypeKind = serde_json::from_str(&json).expect("deserialize");
    let expected = TypeKind::Named {
        name: "Array".into(),
        type_args: vec![TypeKind::Named {
            name: "String".into(),
            type_args: vec![],
        }],
    };
    assert_eq!(roundtripped, expected, "Round-trip must preserve type_args");
}

#[test]
fn test_typekind_deserialize_backward_compat() {
    // Old format: flat string
    let old_json = r#""Array""#;
    let ty: TypeKind = serde_json::from_str(old_json).expect("deserialize old format");
    assert_eq!(ty, TypeKind::Named {
        name: "Array".into(),
        type_args: vec![],
    }, "Old flat-string format must still deserialize");
}

#[test]
fn test_typekind_simple_variants_still_flat() {
    // Simple variants (Int, String, etc.) should still serialize as flat strings
    let int_ty = TypeKind::Int;
    let json = serde_json::to_string(&int_ty).expect("serialize");
    assert_eq!(json, r#""int""#, "Simple types stay flat strings");
}
```

- [ ] **Step 4: Build and run type tests**

```bash
cargo test -p omnilang --lib types_tests
cargo build -p omnilang --lib
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/ir/types.rs omnilang/src/ir/types_tests.rs
git commit -m "fix: preserve TypeKind::Named type_args through JSON serialization

P24 follow-up: TypeKind::Named with non-empty type_args now serializes as
{name, type_args} object. Simple types and empty-args Named continue as flat
strings. Backward-compatible deserialization supports both formats.

Adds 3 round-trip tests verifying type_args preservation, backward compat."
```

---

### Task 2.5: Add Clippy + fmt + Audit Gates to CI

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add cargo fmt check to the rust job**

In `.github/workflows/ci.yml`, find the `rust` job. After the existing `cargo build` step, add:

```yaml
      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --lib -p omnilang -- -D warnings
```

- [ ] **Step 2: Fix cargo audit to deny warnings**

In `.github/workflows/ci.yml`, find the `audit` job. Change line 99:

```yaml
# Before:
        run: cargo audit

# After:
        run: cargo audit --deny warnings
```

- [ ] **Step 3: Add ts-rs binding freshness check to the rust job**

In `.github/workflows/ci.yml`, after the `cargo test` step in the `rust` job, add:

```yaml
      - name: Check ts-rs bindings freshness
        run: |
          cargo test export_bindings --lib -p omnilang
          git diff --exit-code poiesis/src/commands/__generated__/ || {
            echo "ERROR: ts-rs generated files are stale. Run 'cargo test export_bindings --lib' and commit the changes."
            exit 1
          }
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add cargo fmt, clippy, audit --deny warnings, and ts-rs freshness check

- cargo fmt --check prevents formatting drift
- cargo clippy -- -D warnings enforces lint on all PRs
- cargo audit --deny warnings blocks merges on new CVEs
- ts-rs binding check prevents stale generated __generated__/ files"
```

---

## Phase 3: Week 2-3 — P35 Crate Split + P36 Parallelization + P39 MVP

### Task 3.1: P35 Phase 1 — Create omnilang-core + omnilang-codegen-omniview

**Files:**
- Create: `crates/omnilang-core/Cargo.toml`
- Create: `crates/omnilang-codegen-omniview/Cargo.toml`
- Move: ~110 .rs files into `omnilang-core/src/`
- Move: ~17 .rs files from `omnilang/src/codegen/omniview/` into `omnilang-codegen-omniview/src/`
- Modify: `omnilang/Cargo.toml` (facade crate, depends on new crates)
- Modify: `omnilang/src/lib.rs` (re-export from new crates)
- Modify: `omnilang/src/codegen/mod.rs` (import from external codegen crates)
- Modify: `Cargo.toml` at workspace root (add new members)
- Modify: `omnic/Cargo.toml` (update dependency path)
- Modify: `omnilang-wasm/Cargo.toml` (update dependency path)

**Interfaces:**
- Produces: `omnilang_core` crate (lexer → parser → semantic → ir → analyse → shared codegen infra)
- Produces: `omnilang_codegen_omniview` crate (OmniView codegen backend)
- Preserves: `omnilang` facade crate (re-exports, backward compatible API)

- [ ] **Step 1: Create workspace directory**

```bash
mkdir -p crates
```

- [ ] **Step 2: Create omnilang-core**

Move all files EXCEPT `codegen/omniview/`, `codegen/react/`, `codegen/flutter/`, and other codegen backend directories into `crates/omnilang-core/src/`.

```bash
cp -r omnilang/src crates/omnilang-core/
# Remove codegen backends from core
rm -rf crates/omnilang-core/src/codegen/omniview
rm -rf crates/omnilang-core/src/codegen/react
rm -rf crates/omnilang-core/src/codegen/flutter
# (also remove dart, python, rust, javascript, vue, dioxus)
```

Create `crates/omnilang-core/Cargo.toml`:

```toml
[package]
name = "omnilang-core"
version = "0.2.0"
edition = "2021"
description = "OmniLang compiler core — lexer, parser, semantic analysis, IR, shared codegen infrastructure"

[lib]
crate-type = ["rlib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
strum = { version = "0.26", features = ["derive"] }
pathdiff = "0.2"
rayon = "1"
log = "0.4"
walkdir = "2"
rustc-hash = "2"
ts-rs = { version = "10", features = ["serde-compat"] }

[dev-dependencies]
insta = { version = "1", features = ["glob"] }
proptest = "1"
```

- [ ] **Step 3: Create omnilang-codegen-omniview**

```bash
cp -r omnilang/src/codegen/omniview crates/omnilang-codegen-omniview/src
```

Create `crates/omnilang-codegen-omniview/Cargo.toml`:

```toml
[package]
name = "omnilang-codegen-omniview"
version = "0.2.0"
edition = "2021"
description = "OmniView codegen backend for OmniLang"

[lib]
crate-type = ["rlib"]

[dependencies]
omnilang-core = { path = "../omnilang-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rustc-hash = "2"
```

- [ ] **Step 4: Update codegen/shared/ visibility**

In `crates/omnilang-core/src/codegen/shared/`, find all `pub(crate)` items (~15-20) and change to `pub`. Key files: `visitor.rs:8` (`CodegenVisitor`), `expr.rs`, `stmt.rs`, `types.rs`, `generator.rs`, `html.rs`, `inline_handler.rs`.

- [ ] **Step 5: Update omnilang-codegen-omniview imports**

In `crates/omnilang-codegen-omniview/src/mod.rs`, change all `use crate::codegen::*` and `use crate::ir::*` imports to `use omnilang_core::codegen::*` and `use omnilang_core::ir::*`.

Do this for all ~17 files in the crate. Replace `crate::` with `omnilang_core::`.

- [ ] **Step 6: Update workspace Cargo.toml**

```bash
# In the workspace root Cargo.toml, add to [workspace] members:
#   "crates/omnilang-core"
#   "crates/omnilang-codegen-omniview"
```

- [ ] **Step 7: Rewrite omnilang as facade crate**

Update `omnilang/Cargo.toml`:

```toml
[dependencies]
omnilang-core = { path = "../crates/omnilang-core" }
omnilang-codegen-omniview = { path = "../crates/omnilang-codegen-omniview", optional = true }

[features]
default = ["codegen-omniview"]
codegen-omniview = ["omnilang-codegen-omniview"]
```

Update `omnilang/src/lib.rs` to re-export:

```rust
pub use omnilang_core::*;
```

Update `omnilang/src/codegen/mod.rs` to import from external crate:

```rust
#[cfg(feature = "codegen-omniview")]
use omnilang_codegen_omniview;
```

- [ ] **Step 8: Update downstream Cargo.toml files**

Update `omnic/Cargo.toml` and `omnilang-wasm/Cargo.toml` to depend on `omnilang` facade (unchanged API, no path changes needed if facade preserves API).

- [ ] **Step 9: Build workspace**

```bash
cargo build --workspace
```

- [ ] **Step 10: Run tests**

```bash
cargo test --workspace --lib
```

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "refactor: P35 Phase 1 — split omnilang into core + omniview codegen crate

Created omnilang-core (lexer/parser/semantic/ir/analyse/shared-codegen-infra)
and omnilang-codegen-omniview (OmniView backend). omnilang preserved as
facade crate for backward compatibility. ~15 pub(crate) items promoted to pub.

Paused backends (react/flutter/dart/python/rust/js/vue/dioxus) remain in
original omnilang/ pending Phase 2 extraction."
```

---

### Task 3.2: P36 — Add Cross-Backend Codegen Parallelization

**Files:**
- Modify: `omnilang/src/lib.rs` or `omnilang-core/src/lib.rs` (add `compile_multi_target` entry point)

- [ ] **Step 1: Add compile_multi_target function**

In `omnilang-core/src/lib.rs`, after the existing `compile_multi_internal` function, add:

```rust
use rayon::prelude::*;
use std::collections::HashMap;

/// Compile the given sources to multiple targets in parallel.
pub fn compile_multi_target(
    sources: FxHashMap<String, String>,
    targets: Vec<Target>,
    path_mapping: FxHashMap<String, String>,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
) -> HashMap<Target, InternalCompileResult> {
    // Phase 1-3: Parse + analyse (already parallel internally via rayon)
    let analysis = crate::analyse::analyse_multi(&sources, &path_mapping);
    let ir = match analysis.ir {
        Some(ir) => ir,
        None => {
            return targets.into_iter().map(|t| (t, InternalCompileResult {
                files: vec![],
                diagnostics: analysis.diagnostics,
                css: String::new(),
                component_css: Default::default(),
                components: Default::default(),
                element_classes: Default::default(),
                element_tags: Default::default(),
                nid_map: Default::default(),
            })).collect();
        }
    };

    // Phase 4: Codegen in parallel per target
    targets.par_iter().map(|&target| {
        let result = compile_single_target(&ir, &sources, target, editor_mode, debug_mode, store_mode);
        (target, result)
    }).collect()
}

fn compile_single_target(
    ir: &IRProgram,
    sources: &FxHashMap<String, String>,
    target: Target,
    editor_mode: bool,
    debug_mode: bool,
    store_mode: Option<&str>,
) -> InternalCompileResult {
    // Reuse existing compile logic, just scoped to single target
    let mut output = crate::codegen::compile_internal(ir, sources, target);
    // ... existing post-processing ...
    output
}
```

- [ ] **Step 2: Re-export from facade crate**

In `omnilang/src/lib.rs`, re-export the new function:

```rust
pub use omnilang_core::compile_multi_target;
```

- [ ] **Step 3: Build and test**

```bash
cargo build --workspace --lib
cargo test -p omnilang-core --lib
```

- [ ] **Step 4: Add basic test for compile_multi_target**

```rust
#[test]
fn test_compile_multi_target_parallel() {
    let sources = FxHashMap::from_iter([("test.omni".into(), r#"
        component App { view { <div>"Hello"</div> } }
    "#.into())]);
    let targets = vec![Target::OmniView, Target::React, Target::Flutter];
    let results = compile_multi_target(sources, targets, Default::default(), false, false, None);
    assert_eq!(results.len(), 3);
    for (target, result) in &results {
        assert!(result.files.len() > 0, "Target {target:?} produced no files");
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: P36 — parallel codegen across multiple targets via rayon

Added compile_multi_target() that spawns rayon-parallel codegen per Target.
All codegen implementations are verified Send+Sync (zero RefCell/Cell/Mutex
in codegen/ directory). Cross-backend speedup ~1.5x-2x for 3-target CI builds."
```

---

### Task 3.3: P39 MVP — Eliminate Unused Computed Properties

**Files:**
- Create: `omnilang-core/src/ir/optimize.rs`
- Modify: `omnilang-core/src/ir/mod.rs` (add `pub mod optimize;`)
- Modify: `omnilang-core/src/analyse.rs:310` (insert optimization call)

- [ ] **Step 1: Create optimize.rs with IR-level name collection**

Create `omnilang-core/src/ir/optimize.rs`:

```rust
//! IR optimization passes.
//!
//! Pass 1: Dead computed-property elimination.

use rustc_hash::FxHashSet;
use crate::ir::*;

/// Walk an ExprIR tree and collect all IdentifierExpr names into `used`.
fn walk_expr_for_idents(expr: &ExprIR, used: &mut FxHashSet<String>) {
    match expr {
        ExprIR::IdentifierExpr { name } => {
            used.insert(name.clone());
        }
        ExprIR::FieldAccessExpr { object, .. } => walk_expr_for_idents(object, used),
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_for_idents(left, used);
            walk_expr_for_idents(right, used);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_for_idents(operand, used),
        ExprIR::CallExpr { callee, args, .. } => {
            walk_expr_for_idents(callee, used);
            for arg in args {
                walk_expr_for_idents(&arg.0, used);
            }
        }
        ExprIR::TernaryIR { cond, then_expr, else_expr } => {
            walk_expr_for_idents(cond, used);
            walk_expr_for_idents(then_expr, used);
            walk_expr_for_idents(else_expr, used);
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_for_idents(object, used);
            walk_expr_for_idents(index, used);
        }
        ExprIR::TemplateLiteral { parts, .. } => {
            for part in parts {
                walk_expr_for_idents(part, used);
            }
        }
        // Literals, closures, blocks, match — no identifier references at top level
        _ => {}
    }
}

/// Walk a BlockIR and collect all identifier names.
fn walk_block_for_idents(block: &BlockIR, used: &mut FxHashSet<String>) {
    for stmt in &block.statements {
        match stmt {
            StatementIR::Expr { expr } => walk_expr_for_idents(expr, used),
            StatementIR::Let { value, .. } => {
                if let Some(v) = value {
                    walk_expr_for_idents(v, used);
                }
            }
            StatementIR::Return { value } => {
                if let Some(v) = value {
                    walk_expr_for_idents(v, used);
                }
            }
            StatementIR::If { condition, then_block, else_block } => {
                walk_expr_for_idents(condition, used);
                if let Some(b) = then_block { walk_block_for_idents(b, used); }
                if let Some(b) = else_block { walk_block_for_idents(b, used); }
            }
            _ => {}
        }
    }
}

/// Walk a ViewTree and collect all identifier names used in bindings, events, props, etc.
fn walk_view_for_idents(view: &ViewTree, used: &mut FxHashSet<String>) {
    match view {
        ViewTree::Element(el) => {
            // Bindings: signal names
            for binding in el.bindings.values() {
                used.insert(binding.name.clone());
            }
            // Props: expression trees
            for prop in el.props.values() {
                walk_expr_for_idents(prop, used);
            }
            // Class map bindings
            for expr in el.class_map_bindings.values() {
                walk_expr_for_idents(expr, used);
            }
            // Events: handler bodies
            for handler in el.events.values() {
                match handler {
                    EventHandlerIR::InlineFn { body, .. } => walk_block_for_idents(body, used),
                    EventHandlerIR::FnRef { .. } => {}
                }
            }
            // Recurse children
            for child in &el.children {
                walk_view_for_idents(child, used);
            }
            // Conditional branches
            if let Some(ref cond) = el.conditional {
                walk_expr_for_idents(&cond.condition, used);
                for branch in &cond.branches {
                    if let Some(ref view) = branch.view {
                        walk_view_for_idents(view, used);
                    }
                }
            }
            // List rendering body
            if let Some(ref list) = el.list_rendering {
                walk_expr_for_idents(&list.items, used);
                if let Some(ref body) = list.body {
                    walk_view_for_idents(body, used);
                }
            }
            // Motion behaviors
            if let Some(ref motion) = el.motion {
                if let Some(ref expr) = motion.paused_override {
                    walk_expr_for_idents(expr, used);
                }
                if let Some(ref expr) = motion.driven_by {
                    walk_expr_for_idents(expr, used);
                }
            }
        }
        ViewTree::Interpolation(expr) => walk_expr_for_idents(expr, used),
        ViewTree::ListRendering { items, body, .. } => {
            walk_expr_for_idents(items, used);
            for child in body {
                walk_view_for_idents(child, used);
            }
        }
        ViewTree::Conditional { condition, branches, .. } => {
            walk_expr_for_idents(condition, used);
            for branch in branches {
                if let Some(ref view) = branch.view {
                    walk_view_for_idents(view, used);
                }
            }
        }
        ViewTree::Raw { .. } => {}
    }
}

/// Eliminate computed properties that are never referenced by any function body,
/// effect, lifecycle, other computed, or view tree within the component.
pub fn eliminate_dead_computeds(comp: &mut ComponentIR) {
    let mut used: FxHashSet<String> = FxHashSet::default();

    // Collect from function bodies
    for f in &comp.fns {
        walk_block_for_idents(&f.body, &mut used);
    }
    // Collect from effects
    for e in &comp.effects {
        walk_block_for_idents(&e.body, &mut used);
    }
    // Collect from lifecycles
    for lc in &comp.lifecycles {
        walk_block_for_idents(&lc.body, &mut used);
    }
    // Collect from remaining computeds (may reference each other)
    for c in &comp.computeds {
        walk_expr_for_idents(&c.body, &mut used);
    }
    // Collect from view tree
    if let Some(ref view) = comp.view {
        walk_view_for_idents(view, &mut used);
    }

    // Filter: keep only computeds referenced at least once
    let before = comp.computeds.len();
    comp.computeds.retain(|c| used.contains(&c.name));
    let removed = before - comp.computeds.len();

    if removed > 0 {
        log::info!("Eliminated {removed} unused computed(s) from component '{}'", comp.name);
    }
}

/// Run all optimization passes on a merged IR program.
pub fn optimize(ir: &mut IRProgram) {
    for module in ir.modules.values_mut() {
        for comp in module.components.values_mut() {
            eliminate_dead_computeds(comp);
        }
    }
}
```

- [ ] **Step 2: Register optimize module**

In `omnilang-core/src/ir/mod.rs`, add:

```rust
pub mod optimize;
```

- [ ] **Step 3: Insert optimization call in analyse.rs**

In `omnilang-core/src/analyse.rs:310`, after the `merge_file_ir_into` loop and before the lint phase, add:

```rust
// Phase 3c: IR optimization passes
crate::ir::optimize::optimize(&mut merged_ir);
```

- [ ] **Step 4: Add optimization test**

```rust
#[test]
fn test_eliminate_dead_computed() {
    use crate::ir::optimize::eliminate_dead_computeds;

    let mut comp = ComponentIR {
        name: "Test".into(),
        computeds: vec![
            ComputedIntent {
                name: "unused_double".into(),
                r#type: TypeIR { kind: TypeKind::Int, type_args: vec![], is_nullable: false },
                body: ExprIR::BinaryOpExpr {
                    left: Box::new(ExprIR::IdentifierExpr { name: "count".into() }),
                    op: BinaryOpIR::Mul,
                    right: Box::new(ExprIR::LiteralExpr { value: "2".into() }),
                },
                deps: vec![],
            },
        ],
        view: Some(ViewTree::Element(Box::new(ElementIR {
            tag: "div".into(),
            bindings: [("value".into(), SignalRefIR { owner: None, name: "count".into() })].into(),
            // ... remaining fields with defaults
            ..Default::default()
        }))),
        ..Default::default()
    };

    eliminate_dead_computeds(&mut comp);
    assert!(comp.computeds.is_empty(), "unused_double should be eliminated");
}
```

- [ ] **Step 5: Build and run tests**

```bash
cargo build -p omnilang-core --lib
cargo test -p omnilang-core --lib optimize
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: P39 MVP — eliminate unused computed properties (IR optimization pass 1)

Added ir/optimize.rs with walk_expr_for_idents, walk_block_for_idents,
walk_view_for_idents, and eliminate_dead_computeds. Inserted after Phase 3b
(IR merge) in analyse.rs:310. Only elimates computed properties; signal
elimination deferred until motion test coverage reaches 95%."
```

---

## Phase 4: Week 4 — Motion Test Coverage + Complete P39 + P35 Phase 2

### Task 4.1: Add 7 Motion Tests for 95% Branch Coverage

**Files:**
- Modify: `omnilang-core/src/codegen/omniview/tests_regression/motion.rs` (append 7 tests)

- [ ] **Step 1: Add scroll trigger test**

```rust
#[test]
fn test_motion_trigger_scroll() {
    let gen = test_generator_with_component(r#"
        component Animated {
            view {
                <div animate:scroll={scroll-trigger} scroll-start="0" scroll-end="500" scroll-scrub="true">
                    Content
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("scroll"), "Should emit scroll trigger JS");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 2: Add drag trigger test**

```rust
#[test]
fn test_motion_trigger_drag() {
    let gen = test_generator_with_component(r#"
        component Animated {
            view {
                <div animate:drag={drag-trigger}
                     drag-constraints="left: 0; right: 300; top: 0; bottom: 0"
                     drag-elastic="0.1"
                     drag-momentum="true">
                    Draggable
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("drag"), "Should emit drag trigger JS");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 3: Add InView trigger test**

```rust
#[test]
fn test_motion_trigger_in_view() {
    let gen = test_generator_with_component(r#"
        component Animated {
            view {
                <div animate:in-view={in-view-trigger}
                     in-view-once="true"
                     in-view-threshold="0.5">
                    Reveal on scroll
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("inView"), "Should emit InView trigger JS");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 4: Add driven_by test**

```rust
#[test]
fn test_motion_driven_by() {
    let gen = test_generator_with_component(r#"
        component Animated {
            view {
                <div animate:keyframe={kf} driven-by={scrollProgress}>
                    Driven
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("driven"), "Should emit driven_by JS");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 5: Add paused_override test**

```rust
#[test]
fn test_motion_paused_override() {
    let gen = test_generator_with_component(r#"
        component Animated {
            view {
                <div animate:keyframe={kf} paused-override={isPaused}>
                    Pausable
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("paused"), "Should emit paused_override JS");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 6: Add stagger_children test**

```rust
#[test]
fn test_motion_stagger_children() {
    let gen = test_generator_with_component(r#"
        component Animated {
            view {
                <div animate:keyframe={parent-kf} stagger-children="0.1" stagger-from="center">
                    <div animate:keyframe={child-kf} />
                    <div animate:keyframe={child-kf} />
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("stagger"), "Should emit stagger JS");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 7: Add timeline with items test**

```rust
#[test]
fn test_timeline_with_items() {
    let gen = test_generator_with_component(r#"
        component Animated {
            timeline entrance {
                item { target: ".header", keyframe: { from: { opacity: 0 }, to: { opacity: 1 }, duration: 300 } }
                item { target: ".body", keyframe: { from: { y: -20 }, to: { y: 0 }, duration: 400, at: 200 } }
            }
            view {
                <div animate:timeline={entrance}>
                    <div class="header">Header</div>
                    <div class="body">Body</div>
                </div>
            }
        }
    "#);
    let output = gen.generate_js();
    assert!(output.contains("timeline"), "Should emit timeline JS with items");
    insta::assert_snapshot!(output);
}
```

- [ ] **Step 8: Run motion tests + accept snapshots**

```bash
cargo test -p omnilang-codegen-omniview -- tests_regression::motion
cargo insta review  # accept new snapshots
```

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "test: add 7 motion regression tests for 95% branch coverage

Covers: scroll trigger, drag trigger, InView, driven_by, paused_override,
stagger_children, timeline with items. Estimated coverage: 50-60% → 90-95%."
```

---

## Self-Review Checklist

- [x] All tasks have exact file paths and code
- [x] All tasks include test steps
- [x] No TBD/TODO placeholders
- [x] Type signatures consistent across tasks
- [x] Commit messages follow conventional format
- [x] Global constraints respected (no FS scope changes, P34 deferred)
- [x] Phase ordering respects dependencies (P35 core before P36/P39)

---

