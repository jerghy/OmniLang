# OmniLang Quickstart

Write once, run anywhere. Compile `.omni` to React, Vue, Python, and more.

## 3 commands to your first app

```bash
# 1. Clone and install
git clone <repo-url> && cd OmniLang
pip install -e .

# 2. Compile the todo app
omnic build omniview/examples/todo-app.omni --target react --output my-todo

# 3. Run it
cd my-todo && npm install && npm run dev
# Open http://localhost:5173 in your browser
```

## What just happened?

You compiled a `.omni` source file into a working React app with state management (Zustand), CSS modules, and full interactivity.

## Try other targets

```bash
# Compile to Vue
omnic build omniview/examples/todo-app.omni --target vue

# Compile pure logic to Python
omnic build omniview/examples/pure-logic.omni --target python

# Compile a dashboard
omnic build omniview/examples/dashboard.omni --target react
```

## Write your own .omni

```omni
import omniview;

component Hello {
    signal name: String = "World";

    view {
        <Container>
            <Heading>Hello, {name}!</Heading>
        </Container>
    }
}
```

Save as `hello.omni`, then:

```bash
omnic build hello.omni --target react --output hello-app
cd hello-app && npm install && npm run dev
```

## Supported targets

| Target | Type | Status |
|--------|------|--------|
| `react` | UI | L2 (production-ready) |
| `vue` | UI | L2 |
| `dioxus` | UI | L2 |
| `flutter` | UI | L1 (basic) |
| `python` | Pure logic | ✅ |
| `javascript` | Pure logic | ✅ |
| `rust` | Pure logic | ✅ |
| `dart` | Pure logic | ✅ |

## More examples

- `omniview/examples/todo-app.omni` — Todo with store, @for, @if, events
- `omniview/examples/dashboard.omni` — KPI cards, multi-store, computed
- `omniview/examples/form-app.omni` — Form with bind:value, validation
- `omniview/examples/style-showcase.omni` — CSS properties demo
- `omniview/examples/pure-logic.omni` — Pure functions, no UI

## Need help?

- Syntax errors? OmniLang provides hints for common mistakes (like using `or` instead of `||`)
- Missing `import omniview;`? The compiler warns you and continues
- See the [ADR](docs/adr/001-omnilang-omniview-unification.md) for architecture details
- See the [Gap Report](docs/REAL_WORLD_GAPS.md) for known limitations
