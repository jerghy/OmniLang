# Golden Test Fixtures Manifest

This manifest lists representative `.omni` source files covering the full OmniLang language surface.
These files serve as the reference corpus for dual-parser consistency testing.

Files are grouped by primary feature. Many files demonstrate multiple features.

> **Path resolution:** manifest paths are repo-root-relative and resolve to the LIVE
> `test/projects/learn-omniview/src/` tree — test runners do NOT read the mirror below.
> The `test/` subtree next to this manifest is a byte-identical documentary snapshot;
> resync with `diff -rq test/projects/learn-omniview/src tests/fixtures/golden/test/projects/learn-omniview`.

---

## Views

- `test/projects/learn-omniview/src/pages/Overview.omni` — Capability matrix page with CapabilityCard grid + theme stats
- `test/projects/learn-omniview/src/components/SlotDefaultDemo.omni` — `@slot` with default fallback content (3 slot forms: named, named+default, anonymous+default)
- `test/projects/learn-omniview/src/components/Modal.omni` — `@slot` projection (header/body/footer), `@onKeyDown` (Escape), mouse hover tracking
- `test/projects/learn-omniview/src/components/ThemeSwitcher.omni` — Multi-theme selector with 4 theme cards and token palette preview

## Components

- `test/projects/learn-omniview/src/components/StatsPanel.omni` — `prop` declaration, if-expression in text interpolation, cross-file import
- `test/projects/learn-omniview/src/components/AddTodo.omni` — `signal`, `computed`, `bind:value`, `@mount`, `@effect`, custom `event`
- `test/projects/learn-omniview/src/components/DataTable.omni` — Custom HTML tags (Table/List/Button), self-closing elements, Map literals, closures `|x| expr`, `struct` definition
- `test/projects/learn-omniview/src/components/CodeFlowMidChain.omni` — 中链 condition 代码流（前驱→condition→三分支→后置段 flow-out 续接，@pid round-trip 回归）

## Stores

- `test/projects/learn-omniview/src/stores/TodoStore.omni` — `store` with `signal`, `getter`, `action` (add/remove/clear/setFilter)
- `test/projects/learn-omniview/src/stores/NotificationStore.omni` — `action` with return value, `getter` derived count, string operations

## Signals

- `test/projects/learn-omniview/src/components/Counter.omni` — `signal` declaration, `computed` derived values (doubled, label), inline `fn` callbacks
- `test/projects/learn-omniview/src/components/InlineStoreDemo.omni` — Inline `store` definition inside component, getter, action

## Control Flow

- `test/projects/learn-omniview/src/components/ForDirectiveKeyDemo.omni` — `@for item in iterable key=expr` syntax
- `test/projects/learn-omniview/src/components/TodoList.omni` — `@for` list rendering + `@empty` empty state + component composition
- `test/projects/learn-omniview/src/lib/validation.omni` — Pure-logic `for..in` loop, early `return`, tagged union definition

## Motion / Animation

- `test/projects/learn-omniview/src/pages/MotionDemo.omni` — `@motion` definitions (fadeInUp, scalePress, listItem, pulse) + `@enter`/`@hover`/`@press` triggers + `@if` with `mode: "fade"`
- `test/projects/learn-omniview/src/pages/MotionTriggersDemo.omni` — Motion trigger demo: `@in-view`, `@scroll` (start/end/scrub), `@focus`, `@drag-horizontal` / `@drag-vertical` (constraints/elastic/threshold)
- `test/projects/learn-omniview/src/pages/KeyframeAnimationDemo.omni` — Full animation surface: 8 triggers, 4 playback props (loop/direction/fill/delay), 4 easing variants, multi-stop keyframes
- `test/projects/learn-omniview/src/pages/EasingFunctionsDemo.omni` — 23 easing functions: power group, back/bounce/elastic (named-only), `cubic(x1,y1,x2,y2)`, `steps(count, direction)`
- `test/projects/learn-omniview/src/pages/AnimPropertiesDemo.omni` — 17 animatable properties: scale/rotate/skew transforms, translateX/Y/Z, opacity/backgroundColor/color/width/height

## Events

- `test/projects/learn-omniview/src/components/EmitEventDemo.omni` — `event` declaration (with return type), `event?` optional event, parent `@onEvent` binding
- `test/projects/learn-omniview/src/components/EmitKeywordDemo.omni` — `emit` keyword syntax vs function-call event triggering
- `test/projects/learn-omniview/src/components/EventShorthand.omni` — `{fnName(args)}` auto-wrapping, `{store.action}` reference, `@eventName` shorthand

## Accessibility

- `test/projects/learn-omniview/src/components/A11yDemo.omni` — `a11y: { label/role/hidden/live/alt }` semantic accessibility block (5 primitives, three-target mapping)
- `test/projects/learn-omniview/src/components/FormPage.omni` — a11y blocks on form controls, `data-testid`, boolean/hyphenated attributes

## Data / Async

- `test/projects/learn-omniview/src/pages/DataFetching.omni` — `http_get` + `await` + `json_parse` + `@if` loading/success/error states
- `test/projects/learn-omniview/src/lib/api.omni` — Tagged union `ApiResult = Ok | Err | Loading`, `match` exhaustive pattern, `async fn`

## CSS

- `test/projects/learn-omniview/src/components/CssVarsDemo.omni` — 4 CSS variable patterns: style-block `--*`, `var()` reference, inline static, inline dynamic (signal-driven setProperty)
- `test/projects/learn-omniview/src/components/CssUnitsDemo.omni` — 30 CSS units: vh, vw, fr, pt, cm, deg, cqw, dvh, rlh, etc.
- `test/projects/learn-omniview/src/components/ThemeCssVarsDemo.omni` — Theme-driven CSS variables cascading, `setProperty()` runtime updates with `ThemeStore`
- `test/projects/learn-omniview/src/components/StylePseudoDemo.omni` — Pseudo-classes in style blocks: `:hover`, `:focus`, `:active`
- `test/projects/learn-omniview/src/components/AdvancedPseudoDemo.omni` — Compound selectors: `.class1.class2`, `.class:pseudo`, complex selector chains

## Types

- `test/projects/learn-omniview/src/components/TypeSystemDemo.omni` — All integer/float types (i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, isize, usize)
- `test/projects/learn-omniview/src/lib/types.omni` — `pub struct`, `nominal struct`, tagged union, `type` alias
- `test/projects/learn-omniview/src/components/OptionalChainDemo.omni` — Optional chaining `?.` for nullable types (`T?`)
- `test/projects/learn-omniview/src/components/NullishDemo.omni` — Nullish coalescing `??` operator

## Expressions

- `test/projects/learn-omniview/src/components/TernaryExprDemo.omni` — Ternary `?:` and `if-else` expressions in computed and template interpolation
- `test/projects/learn-omniview/src/components/StringLiteralsDemo.omni` — Backtick strings, single-quote strings, template `${expr}` interpolation
- `test/projects/learn-omniview/src/components/BlockExprDemo.omni` — Block expressions `{ stmts; last_expr }` returning values in computed
- `test/projects/learn-omniview/src/components/ComputedFnFormDemo.omni` — Function-form `computed` with `=>` block body

## Advanced (Pure Logic Libraries)

- `test/projects/learn-omniview/src/lib/array_ext.omni` — `std/array` methods: `array_map`, `array_find`, `array_contains`, `array_remove`
- `test/projects/learn-omniview/src/lib/string_ext.omni` — `std/string` methods: `string_split`, `string_join`, `string_replace`, `string_to_upper`, `string_ends_with`
- `test/projects/learn-omniview/src/lib/datetime_demo.omni` — `std/datetime`: `datetime_now`, `datetime_format`, date-part extraction
- `test/projects/learn-omniview/src/lib/regex_demo.omni` — `std/regex`: `regex_match`, `regex_search`, `regex_replace`
- `test/projects/learn-omniview/src/lib/math_ext.omni` — `std/math`: min, max, pow, trig (sin/cos/tan), clamp, random_int, log, pi, e
- `test/projects/learn-omniview/src/lib/error_handling.omni` — `throw` statement, `try`/`catch` error handling
- `test/projects/learn-omniview/src/lib/encode_demo.omni` — `std/encode`: base64/hex/url encode+decode roundtrips
- `test/projects/learn-omniview/src/lib/async_demo.omni` — `std/async`: `async_sleep`/`async_spawn`/`async_all`/`async_race`/`async_timeout` with `async fn` + `await`
- `test/projects/learn-omniview/src/lib/transform.omni` — `for..in` with `break`/`continue` semantics, return-as-break pattern
- `test/projects/learn-omniview/src/lib/stats.omni` — `let`, `let mut`, `while` loop, comparison operators (`&&`, `||`, `!=`, `<`, `>`)
- `test/projects/learn-omniview/src/lib.omni` — Minimal pure-logic utility function (zero-dependency)

## Design System

- `test/projects/learn-omniview/src/design.omni` — `design` block: design tokens (Color, Size, Radius, Shadow, FontFamily, etc.), CSS classes, component variants, `@themes light, dark, sepia, highContrast`
- `test/projects/learn-omniview/src/stores/ThemeStore.omni` — Multi-theme store with 4-theme cycling toggle/setMode
- `test/projects/learn-omniview/src/components/DesignTokenPanel.omni` — Live design token visualizer with `var(--name)` CSS variables
- `test/projects/learn-omniview/src/components/ThemeSourceDemo.omni` — `@themes` syntax showcase with generated CSS output
- `test/projects/learn-omniview/src/components/VariantShowcase.omni` — Button component variant cross-theme demo
- `test/projects/learn-omniview/src/main.omni` — Application root: imports, 9-tab routing, component-to-page mapping

## Minimal

- `test/projects/learn-omniview/src/simple.omni` — Minimal component: signal + fn + view + style (baseline test case)

---

**(total: 55 files)**

## Usage

These files are the reference corpus for dual-parser consistency testing.
Test runners should:
1. Parse each file with both parsers (Rust and TypeScript)
2. Compare ASTs for structural equivalence
3. Compile each file with `omnic build --target omniview` and verify output
