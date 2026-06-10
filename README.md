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
omnilang/          # Language core: grammar, parser, AST, pure-logic codegen
  grammar/         # omnilang.lark — base grammar (Earley)
  parser.py        # AST transformer
  codegen/         # python, javascript, rust, dart targets
  lexer.py         # Unified parser entry (create_parser / create_omniview_parser)

omniview/          # UI SDK: components, stores, directives, styling
  grammar.lark     # UI grammar (extends base)
  parser.py        # UI transformer + OmniLang delegation
  codegen/         # react, vue, dioxus, flutter targets
  cli.py           # omnic CLI (build, init, check, watch, fmt)

docs/              # Documentation (see docs/README.md for index)
  adr/             # Architecture Decision Records
  archive/         # Superseded v0.1-v0.3 documents

tests/             # 331 tests (unit + E2E)
omniview/examples/ # .omni source examples
build_output/      # Compiled output from examples
```

## Targets

| Target | Type | Status |
|--------|------|--------|
| react | UI | ✅ L2 |
| vue | UI | ✅ L2 |
| dioxus | UI | ✅ L2 |
| flutter | UI | L1 (basic) |
| python | Logic | ✅ |
| javascript | Logic | ✅ |
| rust | Logic | ✅ |
| dart | Logic | ✅ |

## Examples

| File | Description |
|------|-------------|
| `omniview/examples/todo-app.omni` | Todo with store, @for, @if, events |
| `omniview/examples/dashboard.omni` | KPI cards, multi-store, computed |
| `omniview/examples/form-app.omni` | Form with bind:value, validation |
| `omniview/examples/style-showcase.omni` | CSS properties demo |
| `omniview/examples/pure-logic.omni` | Pure functions (no UI) |

## Key Documents

- [QUICKSTART.md](QUICKSTART.md) — 3-command onboarding
- [ADR-001](docs/adr/001-omnilang-omniview-unification.md) — Architecture decision (OmniLang = Dart, OmniView = Flutter)
- [E2E Validation Report](docs/E2E_VALIDATION_REPORT.md) — Real-world testing results
- [Real World Gaps](docs/REAL_WORLD_GAPS.md) — Known limitations
- [Document Index](docs/README.md) — Full doc directory

## Known Limitations

See [REAL_WORLD_GAPS.md](docs/REAL_WORLD_GAPS.md). All 8 CRITICAL bugs found during validation are fixed. 12 non-critical gaps remain (CSS shorthand, Unicode in text, etc.).
