# OmniLang

> Write once, run anywhere. Compile `.omni` to React, Vue, Python, and more.

OmniLang is a general-purpose language + code translation engine.  
OmniView is its UI library — `import omniview` activates component/styling capability.  
Modeled after Dart + Flutter.

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

poiesis/           # Visual editor (TypeScript/React, Vite)
  src/
    components/    # React components
    services/      # Codegen, compiler services
    store/         # State management

docs/              # Documentation
  adr/             # Architecture Decision Records
  cookbook/        # Cookbook guides
  design/          # Design documents
  tutorial/        # Tutorials

test/              # Test projects (.omni examples)
  projects/
    learn-omniview/  # .omni examples: todo, dashboard, form, style, etc.
    poiesis-test/

omni-test/         # Standalone test files
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
- [ADR-001](docs/adr/001-omnilang-omniview-unification.md) — Architecture decision (OmniLang = Dart, OmniView = Flutter)
- [E2E Validation Report](docs/E2E_VALIDATION_REPORT.md) — Real-world testing results
- [Real World Gaps](docs/REAL_WORLD_GAPS.md) — Known limitations
- [Document Index](docs/README.md) — Full doc directory

## Known Limitations

See [REAL_WORLD_GAPS.md](docs/REAL_WORLD_GAPS.md). All 8 CRITICAL bugs found during validation are fixed. 12 non-critical gaps remain (CSS shorthand, Unicode in text, etc.).
