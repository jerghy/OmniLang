# OmniLang Import Module System Design

## Status

**Approved** | 2026-06-16

## Summary

OmniLang currently has a minimal import system: `import omniview;` as a mode marker and `import { name } from "path"` for cross-file references. The ModuleLoader (`omniview/modules/loader.py`) handles recursive file loading but has no cross-file semantic validation, no explicit visibility control (`export`/`pub`), and no user-defined path aliases. This design fills all those gaps and unifies the config format.

---

## 1. Complete Syntax

### 1.1 Library Import (mode marker)

```omni
import omniview;
```

- The **only** bare-word import form. No other `import X;` is valid.
- Marks the file for the **OmniView (UI) pipeline**. Its absence routes to the OmniLang (pure logic) pipeline.
- The compiler double-checks: if a file has `import omniview;` but no UI declarations, or has UI declarations but no `import omniview;`, it emits an error.

### 1.2 Named Import

```omni
import { add, User } from "./lib/tools.omni";
import { Button } from "@std/ui.omni";
import { Card } from "@components/card.omni";
```

- `from` path with **mandatory `.omni` extension**.
- Multiple names in a single `{ }` block.
- Compile-time error if any imported name is not `pub`-visible in the target file, or the target file does not exist.

### 1.3 Namespace Import

```omni
import "./lib/math.omni" as math;

fn main() {
    math.cos(3.14);
    let u = math.User { name: "x" };
}
```

- All `pub` names in the target file become accessible under the `math.` prefix.
- Target file must exist. Non-`pub` names are not accessible through the namespace.

### 1.4 Type-Only Import

```omni
import type { User, Order } from "./types.omni";
```

- Compile-time only; generates **no runtime import statement** in output.
- Codegen mapping:
  - **JS/TS**: `import type { ... }` (native)
  - **Rust**: `use crate::types::{User, Order};` (no runtime effect)
  - **Dart/Python**: omitted entirely (types are structural / not enforced)

### 1.5 Re-Export

```omni
export { Button } from "./ui/button.omni";
export { Card, Modal } from "./ui/card.omni";
```

- Creates a barrel / facade. Consumers import from this file instead of reaching into internal paths.
- Note: re-export uses `export ... from`, while single-declaration visibility uses `pub` (see §1.6). This keeps the two concepts distinct — `export` re-routes, `pub` opens.

### 1.6 Visibility (`pub`)

```omni
// fn / struct / type: default private, opt in
pub fn add(a: int, b: int) -> int { a + b }
fn helper(x: int) -> int { x * 2 }            // not visible outside this file

pub struct User { name: string, age: int }
pub type ID = int;

// component / store / theme: ALWAYS public (no pub needed)
component App {
    view { <Text value="hello" /> }
}
store AppStore {
    signal count = 0;
}
theme AppTheme { ... }
```

| Declaration | Default | Override |
|-------------|---------|----------|
| `fn` | private | `pub fn` |
| `struct` | private | `pub struct` |
| `type` | private | `pub type` |
| `component` | **always public** | — |
| `store` | **always public** | — |
| `theme` | **always public** | — |

Rationale: `component`/`store`/`theme` are UI declarations that almost always need to be referenced externally. Auto-public avoids boilerplate. `fn`/`struct`/`type` need explicit `pub` so the developer consciously chooses the API surface.

---

## 2. Path System

### 2.1 Path Prefixes

| Prefix | Resolution | Example |
|--------|-----------|---------|
| `./` or `../` | Relative to importing file | `"./utils.omni"`, `"../lib/math.omni"` |
| `@std/` | Standard library (reserved, system-managed) | `"@std/collections/list.omni"` |
| `@<alias>/` | User-defined alias from config | `"@components/button.omni"` |

### 2.2 Extension

`.omni` extension is **mandatory** in import paths. No auto-appending.

### 2.3 Alias Configuration

Defined in `.omnilang.toml` (see §6):

```toml
[aliases]
"@components" = "./src/components"
"@lib" = "./src/lib"
"@models" = "./src/models"
```

---

## 3. Logic/View Boundary

```
┌──────────┐          ┌──────────┐
│  view    │ ──can──→ │  logic   │
│  file    │          │  file    │
└──────────┘          └──────────┘
       ↑                   │
       └─────cannot────────┘
```

| From | To | Allowed |
|------|----|---------|
| view (has `import omniview;`) | view | ✅ |
| view (has `import omniview;`) | logic | ✅ |
| logic (no `import omniview;`) | logic | ✅ |
| logic (no `import omniview;`) | view | ❌ compile error |

Enforcement: the compiler checks each import. If the importing file is a logic file and the target file is a view file (`import omniview;` present), the compiler emits an error.

---

## 4. Cross-File Semantic Validation

### 4.1 Strict Mode

Compile-time **hard error** in all cases:

| Condition | Error |
|-----------|-------|
| Target file does not exist | `error: file not found: './lib.omni'` |
| Imported name not `pub` in target | `error: 'helper' is not pub in './lib.omni'` |
| Imported name does not exist in target | `error: 'foo' not found in './lib.omni'` |
| Importing view from logic | `error: logic file cannot import view file './ui.omni'` |

### 4.2 Cycle Detection

Tarjan's SCC algorithm detects circular imports. Cycles are a **compile error**.

```
error: circular import detected:
  ./a.omni → ./b.omni → ./c.omni → ./a.omni
```

---

## 5. Compilation Architecture

### 5.1 Overview

```
┌── Python Framework ──────────┐     ┌── Rust Compiler DLL ──────────┐
│                               │     │                                │
│  Read .omnilang.toml          │     │  Parse all sources → AST       │
│  Discover all .omni files     │     │  Resolve imports (multi-file)  │
│  Resolve paths                │     │  Cross-file semantic analysis  │
│  Build {path: source} Map ────┼────→│  Lower to IR                  │
│                               │     │  Codegen (multi-file output)   │
│  Write output files ←─────────┼─────│                                │
│                               │     │                                │
└───────────────────────────────┘     └────────────────────────────────┘
```

### 5.2 Key Constraints

- **Rust compiler DLL is filesystem-free** (must compile to WASM). Python does all file I/O.
- **Rust DLL receives**: `Map<FilePath, SourceText>` — all sources at once.
- **Rust DLL returns**: `Map<FilePath, GeneratedCode>` — one output per input file.
- The compiler is **pure**: same inputs → same outputs. No side effects.

### 5.3 Build Modes

| Command | Behavior |
|---------|----------|
| `omnic build` | Compile all `.omni` files in the project, output one file per source |
| `omnic build main.omni` | Compile `main.omni` and its transitive import tree |

Both modes produce **multi-file output** — never a single bundled file.

### 5.4 Compilation Order

1. Python reads `.omnilang.toml` → resolves aliases
2. Python discovers source files → builds `{path: source}` map
3. Rust DLL parses all files → collects import statements
4. Rust DLL validates all cross-file references (§4.1)
5. Rust DLL detects cycles (§4.2)
6. Rust DLL lower → codegen → returns `{path: code}` map
7. Python writes output files to disk

---

## 6. Configuration: `.omnilang.toml`

### 6.1 Unified Config

`.omnilang.toml` is the single project configuration file. It replaces the dual-config situation (old `.omnilang.toml` + `.omni.json` config fields).

### 6.2 Schema

```toml
[project]
name = "myapp"
version = "0.1.0"
targets = ["js", "python"]       # logic targets
ui_target = "react"              # view target (react | vue | dioxus | flutter | omniview)

[aliases]
"@components" = "./src/components"
"@lib" = "./src/lib"
"@models" = "./src/models"

[source]
dir = "src"

[output]
dir = "build"
```

| Section | Field | Required | Notes |
|---------|-------|----------|-------|
| `[project]` | `name` | yes | Project name |
| `[project]` | `version` | no | Semver |
| `[project]` | `targets` | yes | Array: `js`, `python`, `rust`, `dart` |
| `[project]` | `ui_target` | if view | One of: `react`, `vue`, `dioxus`, `flutter`, `omniview` |
| `[aliases]` | `@<name>` | no | User-defined path aliases |
| `[source]` | `dir` | no | Source directory (default: `src`) |
| `[output]` | `dir` | no | Output directory (default: `build`) |

### 6.3 `.omni.json` Role Change

`.omni.json` is **demoted** from project configuration to **editor runtime snapshot**. It stores:

- Design canvas state (`elements`, `wires`, `positions`)
- Code canvas state (`code_tabs`, `elements`)
- Editor transient state (open tabs, viewport, etc.)

It can be compiled to an Omni project via `omnic compile <file>.omni.json` (reverse codegen → `.omni` sources → normal pipeline).

---

## 7. Codegen Mapping

### 7.1 Visibility (`pub`)

| OmniLang | Rust | JS/TS | Dart | Python |
|----------|------|-------|------|--------|
| `pub fn` | `pub fn` | `export function` | (default public) | (default public) |
| fn (private) | `fn` (no `pub`) | `function` (no `export`) | (default public) | (default public) |
| `pub struct` | `pub struct` | `export class` | (default public) | (default public) |
| `pub type` | `pub type` | `export type` | (default public) | (default public) |

For Dart/Python (no native visibility), codegen ignores `pub` — all top-level declarations are public by language default. This is harmless: private vs public in the omni source is enforced at the OmniLang semantic-analysis layer; by the time code reaches Dart/Python output, the visibility contract has already been validated.

### 7.2 Named Import

| OmniLang | Example output (JS) |
|----------|---------------------|
| `import { add } from "./lib.omni"` | `import { add } from "./lib"` — extension stripped, path adjusted to target language |

Each codegen target strips `.omni` and replaces with the target extension (`.js` → `.js`, `.ts` → `.ts`, `.py` → `.py`, `.rs` → `.rs`, `.dart` → `.dart`).

### 7.3 Namespace Import

| OmniLang | JS | Rust | Dart | Python |
|----------|-----|------|------|--------|
| `import "./lib.omni" as lib` | `import * as lib from "./lib"` | `mod lib;` + `use lib::*;` | `import "./lib.dart" as lib;` | `import lib` |

### 7.4 Re-Export

| OmniLang | JS |
|----------|-----|
| `export { Button } from "./ui.omni"` | `export { Button } from "./ui"` |

---

## 8. Example: Full Project

```
myapp/
├── .omnilang.toml
├── src/
│   ├── main.omni
│   ├── lib/
│   │   ├── math.omni
│   │   └── types.omni
│   └── ui/
│       ├── app.omni
│       ├── button.omni
│       └── index.omni        (barrel)
└── build/
    ├── main.py
    ├── lib/
    │   ├── math.py
    │   └── types.py
    └── ui/
        ├── app.tsx
        ├── button.tsx
        └── index.tsx
```

**`.omnilang.toml`**:
```toml
[project]
name = "myapp"
targets = ["js", "python"]
ui_target = "react"

[aliases]
"@ui" = "./src/ui"
"@lib" = "./src/lib"
```

**`src/lib/math.omni`** (logic):
```omni
pub fn add(a: int, b: int) -> int { a + b }
fn doubler(x: int) -> int { x * 2 }    // private
```

**`src/lib/types.omni`** (logic):
```omni
pub struct User { name: string, age: int }
pub type ID = int;
```

**`src/ui/button.omni`** (view):
```omni
import omniview;

component Button {
    prop label: string;
    prop onclick: fn();
    view { <button @click={onclick}>{label}</button> }
}
```

**`src/ui/index.omni`** (view, barrel):
```omni
import omniview;
export { Button } from "./button.omni";
export { App } from "./app.omni";
```

**`src/main.omni`** (logic):
```omni
import { add } from "./lib/math.omni";
import { User } from "./lib/types.omni";
import "./lib/math.omni" as tools;

fn main() {
    println(add(1, 2));          // 3
    println(tools.add(3, 4));    // 7
    let u = User { name: "x", age: 20 };
}
```

---

## 9. Implementation Phases

### Phase 1: Syntax & Parsing (both Python & Rust)
- Update both grammars (`grammar.lark` ×2, Rust parser)
- Add `pub` token/rule
- Add `export { } from` rule
- Add `import "..." as name` rule
- Add `import type { } from` rule
- Update AST nodes in both Python and Rust

### Phase 2: ModuleLoader V2 (Python)
- Refactor `ModuleLoader` to build `{path: source}` map
- Resolve `@std` and `@alias` paths from `.omnilang.toml`
- Enforce `.omni` extension requirement
- Enforce logic↛view boundary

### Phase 3: Cross-File Semantic Analysis (Rust)
- Extend Rust compiler to receive `Map<FilePath, SourceText>`
- Implement multi-file symbol table
- Validate all imports (name exists, is `pub`)
- Detect circular imports (Tarjan SCC)
- All errors hard-stop compilation

### Phase 4: Codegen
- Emit correct import/export for each target
- `pub` → language-appropriate visibility
- `import type` → target-appropriate type-only import
- `import as` → namespace import
- `export from` → re-export

### Phase 5: CLI & Config
- Unify to `.omnilang.toml`
- Update `omnic build` (project mode) and `omnic build file.omni` (entry mode)
- Add `omnic compile file.omni.json` for editor snapshots
- Remove old config dualism

### Phase 6: Editor Integration
- Update editor schema to reflect `.omni.json` snapshot role
- Import/export via editor GUI
- Cross-file autocomplete foundation

---

## 10. Decision Log

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | `pub` for visibility, `export` for re-export | Distinct concepts; `pub` is shorter (Rust-aligned) |
| 2 | `fn`/`struct`/`type` default private, need `pub` | Conscious API surface; maps cleanly to Rust/JS |
| 3 | `component`/`store`/`theme` always public | Always referenced externally; avoids boilerplate |
| 4 | Logic cannot import view | Maintains pipeline separation; enforces clean architecture |
| 5 | `import omniview;` retained | Explicit mode marker; double-checked by compiler |
| 6 | `.omni` extension mandatory | Unambiguous; no auto-guessing |
| 7 | `@std` system-reserved, `@alias` user-defined | Standard library discoverable; user paths configurable |
| 8 | `.omnilang.toml` unified config | Single source of truth; TOML is human-friendly |
| 9 | `.omni.json` demoted to editor snapshot | Still compilable, but not the primary config |
| 10 | Cycles are compile errors (Tarjan SCC) | Deterministic; avoids unexpected runtime behavior |
| 11 | Strict validation for all imports | Fail fast; prevents silent errors |
| 12 | All output multi-file | No bundling; respects module boundaries |
| 13 | Python framework does FS, Rust DLL is pure | Rust must compile to WASM; clean separation |
| 14 | Rust DLL takes & returns maps | Single call per compilation; no streaming needed |
| 15 | `import type` emits language-appropriate output | Zero-cost in Python/Dart; native support in JS/TS/Rust |
