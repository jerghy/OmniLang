# OmniLang

> Write once, run anywhere. Compile `.omni` to React, Vue, Python, and more.

OmniLang is a general-purpose language + code translation engine.  
OmniView is its UI library — `import omniview` activates component/styling capability.  
Language modeled after Dart; OmniView UI framework modeled after Flutter.

## Quickstart

```bash
omnic init my-app
cd my-app && npm install && npm run dev
```

See [QUICKSTART.md](QUICKSTART.md) for more.

## Project Structure

```
omnilang/          # Language core: lexer, parser, semantic, IR, multi-target codegen (Rust)
  src/
    lexer/         # Tokenizer
    parser/        # AST parser
    semantic/      # Semantic analysis
    ir/            # Intent IR
    codegen/       # All targets: omniview, react, vue, dioxus, flutter, python, js, ts, rust, dart
    stdlib/        # Standard library
    runtime/       # Runtime support
    sourcemap/     # Source map generation

omnic/             # CLI tool (Rust)
  src/main.rs

poiesis/           # Visual editor (Tauri + React, Vite)
  src/
    components/    # React components
    services/      # Codegen, compiler services
    store/         # State management

docs/              # Documentation
  omniview/        # OmniView language reference (13 chapters)
  design/          # Design documents
  research/        # Research reports

test/              # Test projects (.omni examples)
  projects/
    learn-omniview/  # .omni examples: todo, dashboard, form, style, etc.
    poiesis-test/

.architecture/     # Architecture Decision Records & reviews
  decisions/adrs/  # 13 ADRs
  reviews/         # Workspace reviews
  metrics/         # CSP, motion, WASM analysis
```

## Targets

| Target | Type | Status |
|--------|------|--------|
| omniview | UI | ✅ L2 |
| react | UI | ✅ L2 |
| vue | UI | ✅ L2 |
| dioxus | UI | ✅ L2 |
| flutter | UI | L1 (basic) |
| python | Logic | ✅ |
| javascript | Logic | ✅ |
| typescript | Logic | ✅ |
| rust | Logic | ✅ |
| dart | Logic | ✅ |

## Examples

| File | Description |
|------|-------------|
| `test/projects/learn-omniview/src/pages/TodoPage.omni` | Todo with store, @for, @if, events |
| `test/projects/learn-omniview/src/pages/Dashboard.omni` | KPI cards, multi-store, computed |
| `test/projects/learn-omniview/src/pages/DataSubmit.omni` | Form with bind:value, validation |
| `test/projects/learn-omniview/src/pages/EasingFunctionsDemo.omni` | CSS/motion properties demo |
| `test/projects/learn-omniview/src/pages/TimelineMotionDemo.omni` | Animation and motion |

## Key Documents

- [QUICKSTART.md](QUICKSTART.md) — 3-command onboarding
- [OmniView Language Reference](docs/omniview/README.md) — 13-chapter tutorial
- [Architecture Decisions](.architecture/decisions/adrs/) — 13 ADRs (compiler pipeline, editor evolution, codegen targets, etc.)
- [Document Index](docs/README.md) — Full doc directory
- [Design System](Design-System/DESIGN.md) — Brand & component design specs

## Known Limitations

Some language features are still under development. See the architecture reviews and ADRs in `.architecture/` for the current status of known gaps and the remediation roadmap. As of the 2026-07-27 architecture review, all CRITICAL items from prior audits have been addressed, with remaining work tracked via ADR-006, ADR-009, and ADR-010.

