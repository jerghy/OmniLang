---
name: omnilang-patterns
description: Coding patterns extracted from OmniLang repository — multi-target compiler, visual editor, and AI-driven automation API
version: 1.0.0
source: local-git-analysis
analyzed_commits: 18
generated: 2026-06-13
---

# OmniLang Repository Patterns

> Auto-generated from `git log -n 200 --name-only` analysis of the OmniLang monorepo.

## Commit Conventions

This project uses **conventional commits** (adopted more recently):

```text
feat:      New features
fix:       Bug fixes
chore:     Maintenance tasks, cleanup
```

**Scopes** (optional, parenthesized):
- `feat(flutter):` — Flutter-specific feature
- `fix(flutter):` — Flutter-specific bug fix

**Early history** contains non-conventional messages ("Update", "Update OmniLang") — these are legacy.

### Commit Message Style

- English descriptions
- Chinese sometimes used in early commits and document filenames (`OmniLang_v0.3_设计方案.md`)
- Use em-dash (—) for separating description from detail: `fix: resolve canvas-empty bug when navigating via 🔗 link button`

## Project Architecture

```
OmniLang/                          # Monorepo root
├── omnilang/                      # Core compiler (Python)
│   ├── parser.py                  # Lark-based .omni parser
│   ├── ast_nodes.py               # AST node definitions
│   ├── semantic/                  # Type checker, resolver, shadow checker
│   ├── codegen/                   # Multi-target codegen
│   │   ├── javascript.py          # → JavaScript
│   │   ├── python.py              # → Python
│   │   ├── rust.py                # → Rust
│   │   └── dart.py                # → Dart
│   ├── analyze/                   # Static analysis rules
│   │   ├── registry.py            # Rule registry
│   │   ├── rules/                 # Categorized by domain
│   │   │   ├── type/              # Type errors (E04xx)
│   │   │   ├── async_rules/       # Async errors (E02xx)
│   │   │   ├── ownership/         # Ownership errors (E03xx)
│   │   │   ├── performance/       # Performance warnings (F01xx)
│   │   │   └── portability/       # Portability warnings (P01xx)
│   │   └── explain/index.toml     # Error explanations
│   ├── lsp/server.py              # Language Server Protocol
│   └── modules/                   # Module system
│
├── omniview/                      # OmniView → OmniLang compiler
│   ├── parser.py                  # Lark-based .omniview parser
│   ├── grammar.lark               # OmniView grammar
│   ├── preprocessor.py            # Preprocessor
│   ├── lowering.py                # IR lowering
│   ├── semantic/                  # Semantic analyzer
│   └── codegen/                   # Multi-target UI codegen
│       ├── base.py                # Base codegen class
│       ├── react.py               # → React (TSX)
│       ├── vue.py                 # → Vue (SFC)
│       ├── flutter.py             # → Flutter (Dart)
│       └── dioxus.py              # → Dioxus (Rust)
│
├── editor/                        # Visual FlowCanvas editor
│   ├── client/                    # React + Vite + Rete.js frontend
│   │   └── src/
│   │       ├── App.tsx            # Root — OmniDriver init, keyboard shortcuts
│   │       ├── main.tsx           # Vite entry point
│   │       ├── canvas/            # FlowCanvas, LayoutView, WireOverlay
│   │       ├── driver/            # OmniDriver — Playwright-style AI API
│   │       ├── commands/          # Command pattern (undo/redo)
│   │       ├── store/             # Zustand stores (editor, undo)
│   │       ├── panels/            # UI panels (PropertyPanel, BlockPalette, etc.)
│   │       ├── editor/            # Monaco code editor integration
│   │       ├── api/               # API client
│   │       ├── a11y/              # Accessibility tools
│   │       └── omniJson/          # Omni JSON format validation
│   └── server/                    # FastAPI + Uvicorn backend
│       ├── main.py                # Server entry point (--dir argument)
│       ├── api/                   # REST endpoints
│       └── omni_json/             # Omni JSON processing + tests
│
├── omnilang-rs/                   # Rust implementation
│   └── crates/
│       ├── omnilang-core/         # Core compiler
│       └── omnilang-cli/          # CLI
│
├── tests/                         # Python integration tests
│   ├── conftest.py                # Shared fixtures
│   ├── test_fixtures.py           # Fixture-based snapshot tests
│   ├── test_e2e.py                # End-to-end tests
│   └── analyze/test_rules.py      # Rule tests
│
├── test/fixtures/                 # Test input files (.omni)
├── docs/                          # Design docs, ADRs, archives
├── build_output/                  # Generated example apps (React, Vue, Flutter, Dart)
└── .claude/skills/                # Claude Code skills
```

## Key Design Patterns

### 1. Multi-Target Codegen

The core architecture is **parse once, emit many**. The parser produces a language-agnostic AST, then each codegen target walks the AST and emits target-specific code:

```python
# Pattern: BaseCodegen with per-target subclasses
class BaseCodegen:
    def visit_signal(self, node): raise NotImplementedError
    def visit_computed(self, node): raise NotImplementedError

class JavaScriptCodegen(BaseCodegen):
    def visit_signal(self, node): return f"let {node.name} = {node.value};"

class DartCodegen(BaseCodegen):
    def visit_signal(self, node): return f"var {node.name} = {node.value};"
```

**When adding a new codegen target:**
1. Create `omnilang/codegen/<target>.py` or `omniview/codegen/<target>.py`
2. Extend the base class
3. Implement all visitor methods
4. Add tests in `tests/` or `omniview/tests/`

### 2. Command Pattern (Undo/Redo)

All editor mutations go through the Command pattern for undo/redo support:

```typescript
// Pattern: Command interface → execute/undo pairs
interface Command {
  execute(ctx: CommandContext): void;
  undo(ctx: CommandContext): void;
}

// Concrete commands
class AddElement implements Command { ... }
class AddWire implements Command { ... }
class RemoveElement implements Command { ... }
class RemoveWire implements Command { ... }
class SetProp implements Command { ... }
```

**When adding a new editor operation:**
1. Create `editor/client/src/commands/YourCommand.ts`
2. Implement `execute()` and `undo()`
3. Use `useUndoStore.getState().execute(cmd, ctx)` to invoke

### 3. Zustand Store Pattern

State management uses Zustand 5.x with immutable updates:

```typescript
// Pattern: Store with actions that use immer-like set()
const useEditorStore = create<EditorState>((set, get) => ({
  elements: [],
  addElement: (el) => set((s) => ({ elements: [...s.elements, el] })),
  removeElement: (id) => set((s) => ({ elements: s.elements.filter(e => e.id !== id) })),
}));
```

**Key stores:**
- `useEditorStore` — elements, wires, mode, selection
- `useUndoStore` — undoStack, redoStack, execute/undo/redo

### 4. Driver Pattern (OmniDriver)

Wraps store operations behind a Playwright-like API for AI agents:

```typescript
// Pattern: Locator → Handle → Chainable operations
const omni = window.__OMNI__;
const btn = omni.button("Submit");              // Create
btn.setProp("variant", "primary");              // Configure
btn.connectTo(targetBlock, "evt-btn-press-001", "call");  // Wire
omni.switchMode("code");                        // Navigate
await omni.compile();                           // Validate
```

Located in `editor/client/src/driver/`. See `.claude/skills/omni-driver.md` for full API.

### 5. Analyzer Rule Pattern

Static analysis rules follow a numbered error-code convention:

```python
# Pattern: Rule registered in registry with unique code
# - E = Error (blocking)
# - W = Warning
# - F = Performance
# - P = Portability

# Naming: <category><number>_<snake_case_description>
# e.g., e0201_await_non_async.py

# Rules are organized by domain:
# async_rules/  — E0201-E0207
# ownership/    — E0306-E0309
# type/         — E0403
# performance/  — F0101-F0107
# portability/  — P0101-P0105
```

**When adding a new rule:**
1. Create file in appropriate `rules/<category>/` directory
2. Define the rule class with unique error code
3. Register in `registry.py`
4. Add explanation in `explain/index.toml`
5. Add tests in `tests/analyze/test_rules.py`

### 6. Panel Architecture (Editor UI)

Editor panels are modular, mountable components:

```
editor/client/src/panels/
├── ActivityBar.tsx         # Left sidebar navigation
├── BlockPalette.tsx        # Block types (signal, computed, action, etc.)
├── ComponentPalette.tsx    # UI component types (Button, Card, etc.)
├── PropertyPanel.tsx       # Selected element properties
├── FileExplorer.tsx        # File browser
├── ConnectorPanel.tsx      # Event/wire connection UI
├── EventCreationPanel.tsx  # 🔗 button event handler creation
├── ConflictPanel.tsx       # Merge conflict resolution
├── SnippetPanel.tsx        # Code snippets
├── FunctionLibrary.tsx     # Function reference
└── ReadOnlyBanner.tsx      # Read-only mode indicator
```

## File Organization Rules

1. **Many small files** — 200-400 lines typical, 800 max per coding standards
2. **Organize by feature/domain** — not by file type
3. **Barrel exports** — `index.ts` or `__init__.py` re-export public API
4. **Tests co-located or parallel:**
   - Python: `tests/` directory at root, `omniview/tests/`, `editor/server/omni_json/tests/`
   - TypeScript: No test files yet (E2E via Playwright only)
5. **PascalCase for React components, snake_case for Python, camelCase for TS utilities**

## Testing Patterns

### Python (pytest)

```python
# Fixture-based testing in tests/conftest.py
# Snapshot tests: test input → expected output files
# E2E: test_compile_output.py, test_cross_target_equivalence.py
# All tests run via: pytest tests/ -v
```

### TypeScript

- No unit test framework currently configured
- E2E testing via Playwright MCP (`.playwright-mcp/`)
- OmniDriver API tested via `page.evaluate()` in browser

### Rust

- Standard `#[cfg(test)]` module tests (if any)
- `cargo test` in `omnilang-rs/`

## Dev Environment

### Starting the Editor

```bash
# Terminal 1: Backend
python -m editor.server.main --dir test/fixtures/simple --port 8000

# Terminal 2: Frontend
cd editor/client && npm run dev
# → http://localhost:5174 (proxies /api → :8000, /preview → :5175)
```

### Running Tests

```bash
# Python tests (38 server tests, analyzer tests, etc.)
pytest tests/ -v
pytest editor/server/ -v
pytest omniview/tests/ -v

# TypeScript build check
cd editor/client && npm run build   # tsc -b && vite build
```

### Production Build

```bash
cd editor/client && npm run build
# Output: editor/client/dist/
# Requires: tsc -b (type-check) then vite build (bundle)
```

## Frequently Changed Files (High-Churn Areas)

These files change most often — spend extra review attention here:

| File | Changes | Domain |
|------|---------|--------|
| `omnilang/parser.py` | 7 | Core parser |
| `omniview/parser.py` | 5 | OmniView parser |
| `omniview/grammar.lark` | 5 | OmniView grammar |
| `omnilang/ast_nodes.py` | 5 | AST definitions |
| `omniview/preprocessor.py` | 4 | Preprocessor |
| `omniview/codegen/flutter.py` | 4 | Flutter codegen |
| `omniview/cli.py` | 4 | CLI |
| `omnilang/lsp/server.py` | 4 | LSP server |
| `omnilang/grammar/omnilang_v0.2.lark` | 4 | OmniLang grammar |
| `omnilang/analyze/registry.py` | 4 | Rule registry |

## Language-Specific Conventions

### Python (`omnilang/`, `omniview/`, `editor/server/`)
- PEP 8 compliant
- Type hints used (`def parse(source: str) -> Module`)
- `lark` library for parsing (LALR parser)
- Dataclasses for AST nodes (`@dataclass`)
- Frozen dataclasses for immutability (`@dataclass(frozen=True)`)

### TypeScript/React (`editor/client/`)
- React 19 with hooks
- Zustand 5 for state management
- Rete.js v2 for node graph editor
- Monaco Editor for code editing
- Vite 8 for build tooling
- No explicit TypeScript strictness settings beyond defaults

### Rust (`omnilang-rs/`)
- Edition 2021
- Workspace with `omnilang-core` + `omnilang-cli` crates
- Dependencies: serde, clap, thiserror, colored

## Integration Points

| Connection | Mechanism |
|-----------|-----------|
| Frontend ↔ Backend | REST API via Vite proxy (`/api` → `:8000`) |
| Backend ↔ File System | FastAPI file read/write endpoints |
| AI ↔ Editor | `window.__OMNI__` (OmniDriver) + Playwright `page.evaluate()` |
| Editor ↔ Monaco | `@monaco-editor/react` wrapper |
| Canvas ↔ Rete.js | `rete`, `rete-react-plugin`, `rete-area-plugin`, `rete-connection-plugin` |
| Store ↔ React | Zustand 5 `useStore` hooks |

## Key Dependencies

### Python
- **lark>=1.1** — parser generator
- **FastAPI + Uvicorn** — editor backend
- **pytest** — testing

### TypeScript
- **rete@^2.0.6** — node graph editor
- **zustand@^5.0.14** — state management
- **monaco-editor@^0.55.1** — code editor
- **react@^19.2.6** — UI framework
- **vite@^8.0.16** — build tool
- **pako@^2.1.0** — compression

### Rust
- **serde + serde_json** — serialization
- **clap** — CLI argument parsing
- **thiserror** — error handling
