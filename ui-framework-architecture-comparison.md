# UI Framework Architecture Comparison for OmniView

*Generated: 2026-07-26 | Sources: 25+ | Focus: Compile-to-web UI framework architecture*

---

## Executive Summary

This report compares six declarative UI frameworks -- Flutter, Jetpack Compose, SwiftUI, Leptos, Svelte, and SolidJS -- with a focus on architectural patterns relevant to OmniView, a compile-to-web UI framework. Three distinct architectural approaches emerge: (1) **Widget/Element/RenderObject trees** (Flutter), (2) **Virtual DOM with compiler plugins** (Compose), and (3) **Fine-grained reactivity signals without VDOM** (Leptos, Svelte, SolidJS). Key findings: signal-based fine-grained reactivity is the dominant emerging paradigm; compiler-driven optimizations are increasingly critical; and the separation of styling/accessibility from core rendering is a key architectural decoupling.

---

## 1. Individual Framework Architectures

### 1.1 Flutter (Web)

**Core Architecture:** Three-Tree System

Flutter's rendering pipeline uses three parallel trees:

- **Widget Tree (Blueprint):** Immutable, lightweight configuration descriptions. Rebuilt on every `build()`/`setState()`. Describes "what UI should look like."
- **Element Tree (Bridge):** Long-lived objects connecting Widgets to RenderObjects. `BuildContext` IS the Element. Performs diffing via `Widget.canUpdate()` (checks `runtimeType` + key). Holds `State` objects for StatefulWidgets. Uses hash tables for InheritedWidget lookups to avoid O(N^2) parent walks.
- **RenderObject Tree (Renderer):** Handles layout (constraints flow down, sizes flow up), painting, hit-testing, accessibility. Most expensive objects -- reused aggressively.

**Component Model:**
- `StatelessWidget` / `StatefulWidget` -- OOP class hierarchy, not functions.
- `State` object separated from Widget because closures capture `this`. With `build()` on State, you capture the persistent State object, not the ephemeral Widget.
- Parent rebuild does NOT force all descendants to rebuild. `newChild == oldChild` (identity check) short-circuits entire subtrees.
- Key optimization: `const` constructors for identity-based skipping.

**State Management:**
- Built-in: `setState()` on StatefulWidget.
- InheritedWidget: Propagates state down tree. Limitations: no multiple same-type instances, no auto-dispose, cannot combine providers easily.
- Provider (wrapper around InheritedWidget): safer, testable, but inherits fundamental InheritedWidget flaws.
- Riverpod (complete reimplementation): compile-time safety, `autoDispose`, `.family` for parameterized providers, `ref.watch` for composition, Flutter-independent core.

**Styling/Theming:**
- `ThemeData`: Immutable configuration object with ColorScheme, TextTheme, component themes (AppBarTheme, etc.).
- `Theme` widget applies `ThemeData` to descendants. Resolution: direct styles > local Theme > MaterialApp Theme.
- Modern: Material 3 with `ColorScheme.fromSeed()`, `ThemeExtension` for custom design tokens.
- Component themes use `MaterialStateProperty` for state-dependent styling.

**Animation System:**
- `AnimationController`: Drives animation, tied to screen refresh via `Ticker`.
- `Tween<T>`: Stateless mapper from 0.0-1.0 to target range.
- Implicit animations: `AnimatedContainer`, `AnimatedOpacity` -- fire-and-forget, no controller needed.
- Explicit animations: Full control with controllers, `AnimatedBuilder`.
- Physics-based: `SpringSimulation`, `GravitySimulation` for natural motion.
- Hero: Shared element transitions between routes.

**Flutter Web Renderers:**
- **CanvasKit (WASM + Skia):** Compiles Skia to WebAssembly, renders to WebGL canvas. Pixel-perfect with native. ~1.5-2.5MB download. Superior for complex UIs.
- **HTML Renderer:** Maps widgets to DOM elements + CSS. ~1.2MB. Better for text-heavy, SEO-friendly content.
- **skwasm:** Newer WASM renderer, smaller (~1.1MB), multi-threaded via web workers.
- Since 2024: `--wasm` flag produces dual build (dart2wasm+skwasm preferred, dart2js+CanvasKit fallback).

**Elegant vs Awkward:**
- (+) Three-tree separation enables efficient rebuilds; full pixel control via Skia; hot reload preserves state; motion as first-class citizen; every layer is inspectable.
- (-) "Nesting hell" -- UI is 2D but Flutter forces 3D logical tree; Element Tree may be unnecessary indirection; `setState` abuse kills performance; web bundles are large; "uncanny valley" vs native platforms.

---

### 1.2 Jetpack Compose

**Core Architecture:** Composable Functions + Slot Table

- `@Composable` annotation triggers Kotlin Compiler Plugin (KCP). The plugin injects two hidden parameters: `$composer: Composer` (bridge to runtime) and `$changed: Int` (bitmask encoding which params changed).
- The compiler generates: start/end group calls, skip logic (if all params unchanged, `$composer.skipToGroupEnd()`), stability analysis, and restart scope registration.
- `@Composable` changes the function's type -- calling from non-composable context is a compile error.

**Slot Table (instead of VDOM):**
- Two linear arrays: `groups` (IntArray, metadata) and `slots` (Array<Any?>, data values).
- Gap buffer architecture for O(1) insert/delete at current position.
- Groups: RestartGroup (per function), ReplaceableGroup (if/else), MovableGroup (list items via `key()`).

**Recomposition (5 phases):**
1. **State change detection** -- Snapshot system records which `RecomposeScopeImpl` depends on a `mutableStateOf`.
2. **Scheduling** -- `Recomposer` schedules on worker thread.
3. **Diffing** -- `Composer` compares current code against stored slot table. Inserts/deletes/moves recorded as `Pending` objects.
4. **Change list application** -- Batch apply changes: `SlotWriter` updates gap buffer, `Applier` propagates to LayoutNode tree.
5. **Positional memoization** -- Values memoized by position in composition tree, not by key.

**State Management:**
- `mutableStateOf(value)`: Creates `SnapshotMutableStateImpl`. Getter triggers `readObserver` (records state->scope dependency). Setter creates new `StateRecord` and triggers `writeObserver` (invalidates dependent scopes).
- `StateRecord` linked list implements MVCC for snapshot isolation.
- `remember {}` for positional memoization (MUST be inside `@Composable`).
- `derivedStateOf {}`: Only triggers recomposition when RESULT changes, not dependencies.
- `StateFlow`: For ViewModel-level shared state, collected via `collectAsStateWithLifecycle()`.

**Snapshot System:**
- Multi-version concurrency control (MVCC) using `StateRecord` linked list.
- Read-only snapshots, mutable snapshots (with explicit apply), global snapshot.
- `readObserver` / `writeObserver` per snapshot for tracking.
- `ApplyObserver` callbacks fire when snapshot merges to global state.

**Compose Compiler Plugin vs Runtime:**
- Plugin runs at Kotlin IR stage. Extensions: `ComposeIrGenerationExtension` (core transforms), `ComposableCallChecker`, stability analysis.
- Strong skip mode (default K2.0.20+): ALL restartable functions skippable regardless of stability. Unstable params compared by reference. Lambdas capturing unstable vars auto-memoized.
- Communication via standardized `Composer` interface -- enables different runtime backends (Android, Desktop, Web).

**Compose for Web:**
- **Canvas path (future):** Kotlin/Wasm -> Skia/Skiko -> HTML Canvas. Full cross-platform code sharing. Pixel-perfect. Limited accessibility (improved via shadow DOM text elements in 2025).
- **DOM path (stable, deprioritized):** Kotlin/JS -> real DOM elements. Fully accessible. No cross-platform sharing.
- Hybrid direction: Canvas renders pixels + DOM overlay for accessibility/text selection.

**Styling/Theming:**
- `MaterialTheme` provides `ColorScheme`, `Typography`, `Shapes`.
- `CompositionLocal` for implicit data propagation. `compositionLocalOf` (tracks reads, targeted invalidation) vs `staticCompositionLocalOf` (full scope recomposition on change).
- Custom themes: define data class -> create `CompositionLocal` -> write `@Composable` provider -> access via `LocalFoo.current`.

**Elegant vs Awkward:**
- (+) True declarative programming model; compiler-plugin power (compile-time safety, stability contracts); slot table's linear arrays for performance; single language (Kotlin) for UI + logic; structured theming with CompositionLocal.
- (-) "Magic" compiler step is a black box; `remember()` breaks at conditional boundaries surprisingly; verbose state syntax (`var x by remember { mutableStateOf(0) }`); no WYSIWYG designer; Custom design system building is poorly supported; XML still outperforms Compose in scroll-heavy lists.

---

### 1.3 SwiftUI

**Core Architecture:** View Protocol + Value-Type Structs + Result Builders

- Components are **value-type structs** conforming to `View` protocol (mandatory `var body: some View`).
- `some View` uses Swift's opaque return types -- the concrete nested type is preserved (e.g., `TupleView<(Text, Button)>`), enabling the compiler and runtime to know exact tree structure.
- No VDOM. SwiftUI performs **type-based structural diffing**: compares current view tree against previous using concrete types. This enables property-level updates.
- View identity tracked via type + position + explicit `.id()` modifier.

**ViewBuilder and Result Builders:**
- `@ViewBuilder` is a Swift `@resultBuilder` that transforms closure bodies into view trees.
- Methods: `buildBlock` (combines 1-10 views into `TupleView`), `buildOptional` (if without else), `buildEither` (if/else -> `_ConditionalContent`), `buildArray` (for-in loops), `buildExpression` (type conversion).
- Swift 5.7's `buildPartialBlock` replaces 10 overloads with accumulation pattern.
- Key: `return` inside a builder bypasses the builder entirely -- causes compilation errors if types differ.

**Modifier Chain:**
- Modifiers (`.font()`, `.padding()`) return **new modified views** (not mutation).
- Internally: each modifier creates a wrapper View that applies the modification.
- `ViewModifier` protocol enables custom modifiers.
- Order matters -- modifiers are applied in sequence, outer to inner.

**State Management:**
- `@State`: Property wrapper for local value-type state. Stored outside the view struct, survives re-creation.
- `@Binding`: Read/write reference to `@State` owned elsewhere.
- `@StateObject`: Owns a reference-type ObservableObject (reference type), created once.
- `@ObservedObject`: Read-only reference to ObservableObject owned elsewhere.
- `@EnvironmentObject`: Implicit dependency injection via environment.
- `@Environment`: Reads system-level values (colorScheme, locale, etc.).

**Styling System:**
- `@Environment` for propagating styling values down tree
- `ViewModifier` protocol for reusable styling
- `ShapeStyle` protocol for fills, strokes
- `@ScaledMetric` for dynamic type adaptation
- No centralized theme object -- styling flows through environment + modifiers

**Layout System (Two-Phase):**
1. Parent proposes size (`ProposedViewSize`) to child.
2. Child responds with its chosen size. Parent places it.
- Special proposals: `.zero` (minimum size), `.infinity` (maximum), `.unspecified` (ideal).
- `Layout` protocol (iOS 16+): Custom layout containers with `sizeThatFits(proposal:subviews:cache:)` + `placeSubviews(in:bounds:proposal:subviews:cache:)`.
- Cache: optional bidirectional cache for expensive calculations.
- `ViewThatFits`: Auto-selects first child that fits available space.

**Identity, Diffing, and AnyView Cost:**
- Strongly typed view hierarchy enables structural diffing.
- `AnyView` breaks this: type-erased wrapper forces full subtree teardown/rebuild on every update.
- Worst case: `AnyView` inside `ForEach` -- measured 10-17% FPS degradation. With frequent updates, drops below 50 FPS vs 58-59 FPS without.
- Alternatives: `@ViewBuilder`, generics, `Group` -- all preserve type identity.

**Elegant vs Awkward:**
- (+) Minimal syntax; result builders feel natural; strong type system prevents many errors; property wrappers for state are ergonomic; tight Apple ecosystem integration; Layout protocol enables clean custom layouts.
- (-) Compiler error messages are notoriously bad for complex views; AnyView performance trap; too many state property wrappers to learn; Environment makes implicit dependencies hard to reason about; modifier chaining can feel like callback hell; SwiftUI 1.0-3.0 had severe stability issues.

---

### 1.4 Leptos (Rust WASM)

**Core Architecture:** Signal-Based Fine-Grained Reactivity (SolidJS-inspired)

- No virtual DOM. Signals update specific DOM nodes directly via the reactive graph.
- Components are setup functions that run **once** to create real DOM nodes and wire up the reactivity graph. Never re-execute.
- `view!` macro provides JSX-like syntax, compiles static HTML to template cloning, wraps dynamic parts in reactive bindings.

**Signal System:**
- `create_signal(cx, value)` -> `(ReadSignal<T>, WriteSignal<T>)` -- enforces read/write separation.
- Arena-allocated: `ReadSignal<T>` is a slotmap index (Copy + 'static), not an owned value. No `.clone()` needed for closures -- solves Rust UI's biggest ergonomic pain point.
- `create_memo()`: Cached derived value, recomputes only on dependency changes.
- `create_effect()`: Side effects with auto dependency tracking. Dependencies re-detected on each run (supports dynamic conditional dependencies).
- `create_resource()` / `create_action()`: Async operations integrated into reactive system.

**Component Model:**
- `#[component]` attribute on plain Rust functions returning `impl IntoView`.
- PascalCase names, auto-converted from snake_case.
- Props via attributes: `#[prop(into)]` (auto .into()), `#[prop(optional)]` (Option<T>), `#[prop(default = ...)]`.
- Children: `Children` (FnOnce), `ChildrenFn`, `ChildrenFragment`.
- Zero `unsafe` code in Leptos core.

**SSR + Hydration:**
- Three modes via feature flags (mutually exclusive per build target): `csr`, `ssr`, `hydrate`.
- True isomorphic code: same Rust code runs on server and client.
- Server functions: declare server-side logic (DB queries, auth) in client code, call like local functions. No separate REST API.
- HTML streaming: supports ordered/unordered `<Suspense/>` streaming.
- `leptos_store` for automatic state serialization: server serializes state via serde into HTML, client deserializes during hydration.

**Build Pipeline:**
- `cargo-leptos`: Parallel binary build (native for server, `wasm32-unknown-unknown` for client). Steps: Rust -> WASM -> `wasm-bindgen` -> `wasm-opt`.
- Code splitting v0.5+: `#[lazy]` for per-function lazy loading, `#[lazy_route]` for route-level code splitting.
- WASM size optimization: `opt-level = 'z'`, avoid `regex`, use `miniserde`.

**Rust Type System Integration:**
- Arena allocation gives signals `Copy + 'static` -- eliminates lifetime complexity.
- Read/write separation enforces observable discipline.
- Data lifecycle bound to UI tree scope -- auto-cleanup on component unmount.

**Elegant vs Awkward:**
- (+) No VDOM -- surgical DOM updates; arena allocation eliminates `.clone()` boilerplate; true full-stack with server functions; streaming SSR; zero unsafe code; performance near vanilla JS.
- (-) 2+ min cold compilation; debugging via println only; WASM can be 1.6MB+ vs React <300KB; immature component library ecosystem; store API design friction between `Field` and `Signal`; upgrade difficulty between versions.

---

### 1.5 Svelte

**Core Architecture:** Compiler-First + Reactive Declarations

- Svelte is fundamentally a **compiler**, not a runtime. It reads `.svelte` files at build time and generates vanilla JavaScript with direct DOM update instructions. The framework "disappears" before the browser loads the page.
- No virtual DOM. State changes go directly to affected DOM nodes.
- The compiler generates `$$.update()` -- a single function packaging all reactive statements, ordered topologically.

**Reactive Declarations (Legacy `$:` Syntax, Svelte 3/4):**
- `$: sum = a + b;` -- compiler statically analyzes dependencies.
- Every assignment augmented with `$$invalidate()` call.
- Variables assigned bit flags for dirty checking via bitmask.
- Topological ordering ensures dependencies execute first.
- **Critical limitation**: Dependencies determined at compile time via static analysis. Hidden dependencies inside function calls are invisible.
- **Ordering caveat**: Since dirty flags checked serially, statement order matters for indirect side effects.

**Svelte 5 Runes (Explicit Reactivity):**
- `$state`, `$derived`, `$effect`, `$props` -- function-like syntax but are compiler instructions.
- Key change: dependency detection moves from compile-time static analysis to **runtime** (when expressions evaluate).
- Works in `.svelte.js` and `.svelte.ts` files, not just `.svelte` components.
- Underlying mechanism: Signals (implementation detail, not exposed to developers).
- Eager consistency: `$derived` values always current immediately vs `$:` microtask batching.

**Component Model:**
- Single-file components: `<script>`, `<style>`, HTML template.
- Props: `let { name, version } = $props()` (Svelte 5) or `export let name;` (legacy).
- Module-level code: `<script context="module">` runs once per component.
- Top-level variables are reactive by default (trigger on reassignment).

**Scoped CSS (Built-in):**
- Compiler hashes component content -> `svelte-xyz123` class.
- CSS selectors rewritten to include this class.
- Each scoped selector gains 0-1-0 specificity bump. `:where()` wrapper prevents stacking.
- Keyframes auto-renamed for component scope.
- `:global()` escape hatch.

**Compiler Philosophy (Rich Harris, 10 Principles):**
1. The web matters; 2. Optimize for vibes; 3. Don't optimize for adoption; 4. HTML, the mother language; 5. Embrace progress; 6. Numbers lie; 7. Magical, not magic; 8. Dream big; 9. No one cares; 10. Design by consensus.
- Core thesis: "What if the framework was actually just a compiler?" -- the framework writes the runtime code for you.
- React components are typically ~40% larger than equivalent Svelte.

**Elegant vs Awkward:**
- (+) Smallest bundles (~50KB); zero runtime overhead; scoped CSS built-in; minimal syntax; truly "disappearing framework."
- (-) Legacy `$:` hidden dependencies create subtle bugs; microtask-based inconsistency (derived values stale until next tick); "magic" can confuse at scale; Svelte 5 migration is disruptive.

---

### 1.6 SolidJS

**Core Architecture:** Signal-Based Fine-Grained Reactivity (No VDOM)

- JSX compiled at build time into optimized imperative DOM operations.
- Static HTML extracted into cloned template nodes (module-level const).
- Dynamic expressions compiled into reactive bindings targeting specific DOM nodes.
- When a signal changes, only the bound text node / attribute / class updates. No VDOM diff.
- Benchmarks within ~7% of vanilla JS (23.0ms vs 22.2ms for 1000 rows).

**Signal System:**
- `createSignal(initial)` -> `[getter, setter]`. Getter is a function (not value) -- tracking happens by calling it in reactive context.
- Global `currentSubscriber` variable tracks active computation. Getter registers subscriber. Setter notifies all subscribers.
- Equality check: if new value === old value, no notification.
- `createMemo`: Cached derived value, only recomputes on dependency changes.
- `createEffect`: Side effects, auto-tracking, runs synchronously after DOM updates.
- `createStore`: Deeply reactive proxy state for nested objects. Granular property-level updates.

**Component Model: Components Run Once**
- Components execute exactly once as **setup functions**.
- Create real DOM nodes, initialize reactive graph, return DOM elements.
- **Never re-execute** when state changes.
- Implications: Component body is NOT reactive -- read signal in body but outside JSX = no dependency tracking.
- Control flow must use `<Show>`, `<Switch>/<Match>`, `<For>`, `<Index>` instead of native JS.
- No early returns in component setup.
- Props destructuring breaks reactivity -- use `splitProps` or direct access.

**Consistency Model: Eager Consistency**
- State updates are synchronous -- derived values and DOM effects update immediately.
- Batched only when explicitly wrapped in `batch()` or transition.
- Matches developer intuition: `setCount(5); console.log(count());` works.

**Elegant vs Awkward:**
- (+) Eager consistency; no VDOM overhead; surgical DOM updates; plain JS with no magic syntax; ~7KB core runtime; components-run-once eliminates infinite re-render class of bugs.
- (-) No built-in scoped CSS; props destructuring trap; must use control-flow components instead of native JS; smaller ecosystem; signal getter syntax (`count()`) vs variable access is foreign to React developers.

---

## 2. Cross-Cutting Architectural Comparison

### 2.1 Component Model

| Framework | Component Definition | Execution Model | Lifecycle |
|---|---|---|---|
| **Flutter** | Class hierarchy extending Widget | `build()` called on state change | initState/didChangeDependencies/build/didUpdateWidget/dispose |
| **Compose** | `@Composable` function | Function potentially re-executes on recomposition | Positional: LaunchedEffect, DisposableEffect, SideEffect |
| **SwiftUI** | Struct conforming to `View` | `body` re-evaluated on state change | `.onAppear`/`.onDisappear` modifiers |
| **Leptos** | `#[component]` fn returning `impl IntoView` | Runs **once** (setup only) | On cleanup: `on_cleanup()` |
| **Svelte** | `.svelte` single-file component | Re-runs template sections on state change | `onMount`/`onDestroy`/`$effect` cleanup |
| **SolidJS** | JSX function returning DOM elements | Runs **once** (setup only) | `onCleanup`/`onMount` |

**Key Insights:**
- **Setup-only execution** (Leptos, SolidJS) eliminates entire class of re-render bugs and performance issues but requires dedicated control-flow components.
- **OOP Widgets** (Flutter) are more verbose but provide clearer lifecycle contracts than functional equivalents.
- **Compiler-managed** (Svelte) removes developer responsibility for fine-grained reactivity but creates "magic" that confuses.

### 2.2 State Management

| Framework | Local State | Derived State | Shared State | External Observation |
|---|---|---|---|---|
| **Flutter** | `StatefulWidget.setState()` | Computed in `build()` | InheritedWidget / Provider / Riverpod | `ValueNotifier`, `ChangeNotifier` |
| **Compose** | `remember { mutableStateOf() }` | `derivedStateOf {}` | StateFlow + collectAsState() | `snapshotFlow {}` |
| **SwiftUI** | `@State`, `@StateObject` | Computed `var` | `@EnvironmentObject`, `@ObservedObject` | `onReceive` |
| **Leptos** | `create_signal()` | `create_memo()` / derived signal | `create_rw_signal()` + provide_context | `create_effect()` |
| **Svelte** | `let x = $state()` (Svelte 5) | `$derived()` | Svelte stores / context | `$effect()` |
| **SolidJS** | `createSignal()` | `createMemo()` | `createStore()` / context | `createEffect()` |

**Key Insights:**
- **Signals** (SolidJS, Leptos, Svelte 5) are converging as the universal primitive for reactive state.
- **MVCC-style snapshots** (Compose) enable concurrent modification tracking at the cost of complexity.
- **Property wrappers** (SwiftUI `@State`) are ergonomic but obscure the actual mechanism.
- Consistency models differ: SolidJS and Leptos are eagerly consistent; Svelte uses microtask batching; SwiftUI and Flutter batch per frame.

### 2.3 Styling / Theming

| Framework | Mechanism | Scope | Theme Architecture |
|---|---|---|---|
| **Flutter** | Widget constructor params + Theme widget | Explicit widget params | `ThemeData` with `copyWith()`, `ThemeExtension` for custom |
| **Compose** | `Modifier` chain + `MaterialTheme` | Modifier args + CompositionLocal | `MaterialTheme` with ColorScheme/Typography/Shapes + custom CL |
| **SwiftUI** | Chained modifiers + Environment | Modifier chain + @Environment | Environment-based + ViewModifier protocol |
| **Leptos** | `style:`, `class:`, `prop:` attrs + CSS | Direct CSS + class toggles | None built-in (use CSS/custom) |
| **Svelte** | Scoped `<style>` tag (built-in) | Auto-scoped by compiler | None built-in (CSS custom properties) |
| **SolidJS** | Standard CSS / CSS Modules / CSS-in-JS | No built-in scoping | None built-in (ecosystem) |

**Key Insights:**
- Compiler-based frameworks (Svelte) can offer zero-cost scoped CSS via compile-time hashing.
- Platform-style frameworks (Flutter, Compose, SwiftUI) use **theme objects + implicit propagation** which enables consistent theming but requires more ceremony.
- Signal-based frameworks (SolidJS, Leptos) tend to leave styling to the ecosystem -- lower built-in complexity but more choices for the developer.

### 2.4 Animation Systems

| Framework | Implicit Animations | Explicit Animations | Physics |
|---|---|---|---|
| **Flutter** | `AnimatedContainer`, `AnimatedOpacity` | `AnimationController` + `Tween` | `SpringSimulation` |
| **Compose** | `animate*AsState` | `Animatable`, `AnimationSpec` | `spring()`, `tween()` in AnimationSpec |
| **SwiftUI** | `.animation()` modifier | `.withAnimation {}` | `.spring()` animation |
| **Svelte** | `transition:`, `animate:` directives | JS `requestAnimationFrame` | JS-based (no built-in) |
| **SolidJS** | CSS transitions/animations | JS-based | No built-in animation system |

**Key Insights:**
- Flutter has the most mature, deeply integrated animation system (implicit/explicit/physics/Hero/staggered).
- Compose and SwiftUI provide comparable two-tier implicit/explicit systems.
- Compiler/signal-based frameworks (Svelte, SolidJS, Leptos) generally leave animation to CSS or JS libraries -- a gap for OmniView to fill.

### 2.5 Routing

| Framework | Approach |
|---|---|
| **Flutter** | Navigator, Router (declarative), go_router, auto_route |
| **Compose** | `NavHost`, `composable()` routes |
| **SwiftUI** | `NavigationStack`, `NavigationLink` (value-based) |
| **Leptos** | Nested router: `<Routes>` + `<Route>` + `<Outlet>`, SSR-aware, progressive enhancement |
| **Svelte** | SvelteKit file-based routing (not Svelte itself) |
| **SolidJS** | Solid Router / SolidStart file-based routing |

---

## 3. Compilation Pipeline Analysis

### 3.1 How Each Framework Relates to Its Compiler

| Framework | Compilation Stage | What Compiler Does | Runtime After Compilation |
|---|---|---|---|
| **Flutter** | Dart -> AOT native or JS/WASM | Dart compiler, not Flutter-specific | Framework runtime always present |
| **Compose** | Kotlin -> JVM/JS/ Native via KCP plugin | @Composable IR transforms, stability analysis | Compose runtime + Composer always present |
| **SwiftUI** | Swift -> native via LLVM | Swift compiler, not SwiftUI-specific | SwiftUI runtime always present |
| **Leptos** | Rust -> WASM via standard Rust compiler | `view!` macro expansion, no special compiler | Small signal runtime (~7KB core) |
| **Svelte** | .svelte -> JS via Svelte compiler | **Heavy**: rewrites entire component, removes framework | Minimal runtime (~50KB total) |
| **SolidJS** | JSX -> JS via Babel/SWC plugin | Compiles JSX to DOM operations, wraps expressions | Small signal runtime (~7KB core) |

**Key Insight**: Svelte is the only framework where the compiler does the **majority of the work** -- the framework "disappears." Flutter, Compose, and SwiftUI all apply compiler transformations but keep a substantial runtime. Leptos and SolidJS use signals with tiny runtimes but far less compile-time transformation than Svelte.

### 3.2 What Compilers Do Well (and Poorly)

**Well:**
- Static analysis for dependency graphs (Svelte `$:`, Solid/Svelte 5 compile-time JSX optimization)
- Dead code elimination (Svelte strips unused components)
- Scoped CSS generation (Svelte compile-time hashing)
- Code generation from domain-specific syntax (Svelte `.svelte` files)

**Poorly:**
- Tracking dependencies through function boundaries (Svelte legacy `$:` limitation)
- Handling dynamic or conditionally-accessed dependencies
- Providing good error messages for complex view hierarchies (SwiftUI)
- Optimizing patterns that subvert the compiler's model (interior mutability, escape hatches)

---

## 4. Framework Design Strategies: What Works

### 4.1 Successful Patterns

**1. Fine-grained reactivity over component re-execution**
- SolidJS and Leptos prove that running components once and updating specific DOM nodes via signals is superior to re-executing entire component trees. Eliminates re-render bugs and unnecessary computation.
- Svelte 5's adoption of signals validates this direction.

**2. Compiler-time extraction of static content**
- Svelte and SolidJS both extract static HTML into template clones at compile time, avoiding DOM creation overhead.
- Result: only dynamic expressions need runtime tracking.

**3. Separation of runtime from rendering backend**
- Compose's `Composer` interface enables multiple backends (Android, Desktop, Web Canvas).
- Flutter's Skia engine allows consistent rendering across platforms.
- Leptos's feature-gated CSR/SSR/Hydrate modes separate rendering from logic.

**4. Arena allocation for shared mutable state in Rust**
- Leptos's approach of using slotmap indices instead of owned values is a breakthrough for Rust UI ergonomics.

**5. Implicit vs explicit trade-off managed by compiler evolution**
- Svelte's journey from implicit `$:` to explicit runes shows how to evolve from "magical" to understandable without losing the compiler advantage.

### 4.2 Common Pitfalls

**1. Type erasure destroying optimization**
- SwiftUI's `AnyView` forces full subtree teardown. Compose's `Any` in unstable types prevents skip optimization. Avoid anything that hides concrete types from the diffing system.

**2. "Magic" that confuses developers**
- Svelte legacy `$:` hidden dependencies. Compose `remember` breaking at conditional boundaries. SwiftUI's opaque error messages. Compiler magic needs escape hatches and clear mental models.

**3. Verbose boilerplate for simple things**
- Flutter's class hierarchy for what could be a function. Compose's `remember { mutableStateOf() }` wrapping. Leptos's create-signal ceremony. These add friction without proportional benefit.

**4. Runtime/compiler model mismatch**
- React Compiler breaks with interior mutability. Svelte legacy breaks with function-call-wrapped dependencies. The compiler must match the runtime model precisely, or patterns that subvert it silently break.

**5. Design systems baked into core**
- Flutter's multi-year "Decoupling Design" refactor to separate Material/Cupertino from core SDK. Lesson: keep core design-agnostic; design systems are plugins.

**6. Large WASM binaries**
- Rust WASM frameworks still struggle with binary size (1.6MB+ with serde/regex). Svelte/SolidJS produce significantly smaller output for the web.

### 4.3 Balancing Expressiveness vs Complexity

| Level of "Magic" | Examples | Pros | Cons |
|---|---|---|---|
| **Minimal** (SolidJS, Leptos) | Explicit signals, control-flow components | Predictable, debuggable | More ceremony, learning curve |
| **Moderate** (Svelte 5 runes) | Compiler-based but explicit runes | Good DX without confusion | Migration cost, some weird edges |
| **Heavy** (Svelte legacy `$:`) | Implicit reactivity via labels | Extremely concise | Hidden dependencies, hard to debug |
| **Platform** (Flutter, Compose, SwiftUI) | Full frameworks with theme engines | Complete solution for target platform | "Will not work that way on other platforms" |

**Key takeaway for OmniView**: The most successful frameworks meet the developer where they are -- explicit enough to debug, concise enough to be productive. The Sweet spot is probably **Svelte 5 runes + SolidJS signal explicitness**: compiler-augmented but not compiler-dependent, with clear reactive boundaries.

---

## 5. Implications for OmniView

### 5.1 Architecture Recommendations

Based on this analysis, an ideal compile-to-web UI framework should:

**Adopt signals as the reactivity primitive.** Signals (SolidJS, Leptos, Svelte 5) are the convergent architecture for reactive UI. They combine fine-grained tracking with a simple mental model (getter/setter + auto-tracking).

**Use a compiler for optimization, not replacement.** Extract static content at compile time (template clones), generate scoped CSS, and optimize signal wiring. But don't make the framework compiler-only -- maintain a clear runtime that is independently understandable.

**Separate rendering backend from logic.** Like Compose's `Composer` interface, define a rendering abstraction that allows multiple backends (Canvas, DOM, native). This enables OmniView to target different platforms.

**Design-agnostic core with pluggable theming.** Follow Flutter's "Decoupling Design" lesson: the core framework should not embed Material or any design system. Theme systems should be layers on top of core primitives, using composition (Modifier/CompositionLocal pattern).

**Provide built-in scoped styling.** Svelte's compile-time CSS scoping is zero-cost and solves a major developer pain point. This should be built into OmniView's compiler.

**Implement an integrated animation system.** Flutter's implicit/explicit/physics three-tier system is the gold standard. OmniView should provide comparable capabilities since ecosystem libraries often lag for new frameworks.

**Support SSR + hydration for the web.** Leptos's isomorphic architecture (same code on server and client) and multiple Hydration modes (ordered/unordered streaming) represent the state of the art for web frameworks.

**Keep WASM binary sizes manageable.** The Rust WASM frameworks show that excessive binary size is a major adoption barrier. Compile to lean output, with aggressive tree-shaking.

### 5.2 Anti-Patterns to Avoid

- Don't embed any design system into the core framework.
- Don't make implicit magic the *only* way to do things (provide explicit alternatives).
- Don't erase type information that the diffing system needs.
- Don't force re-execution of component functions on every state change.
- Don't use a bridge architecture (like React Native) for cross-platform -- compile directly.
- Don't ship a large runtime -- leverage the compiler to minimize what's delivered to the browser.

---

## Sources

1. [Flutter Architectural Overview](https://docs.flutter.dev/resources/architectural-overview)
2. [Flutter Web Renderers](https://docs.flutter.dev/platform-integration/web/renderers)
3. [Flutter Widget-Element-RenderObject Explained](https://github.com/nimafarzin-pr/flutter-ui-rendering)
4. [Flutter Three-Tree Rendering](https://cloud.tencent.com.cn/developer/article/1771780)
5. [Riverpod vs Provider Architecture](https://docs-v2.riverpod.dev)
6. [Jetpack Compose Compiler Plugin Deep Dive](https://github.com/skydoves/compose-mechanisms)
7. [How Compose Works (Google Source)](https://android.googleid.googlesource.com/platform/frameworks/support/+/5019da32ee84e736e83b3a736043e2c79a42bb93/compose/runtime/design/how-compose-works.md)
8. [Jetpack Compose Slot Table and Recomposition](https://blog.stackademic.com/jetpack-compose-internals-the-slot-table-snapshot-system-recomposition-mechanics)
9. [Compose Snapshot System](https://cloud.tencent.com.cn/developer/article/1940556)
10. [SwiftUI ViewBuilder and Result Builders](https://developer.apple.com/documentation/SwiftUI/ViewBuilder)
11. [SwiftUI buildPartialBlock](https://www.hackingwithswift.com/swift/5.7/buildpartialblock)
12. [SwiftUI AnyView Performance Benchmarks](https://forums.swift.org/t/swiftui-and-anyview-performance-benchmarks/65717)
13. [SwiftUI Layout Protocol](https://developer.apple.com/documentation/SwiftUI/Layout)
14. [Leptos Documentation -- Dynamic Attributes](https://leptos-rs.github.io/leptos/view/02_dynamic_attributes.html)
15. [Leptos GitHub -- Discussions on SSR/Hydration](https://github.com/leptos-rs/leptos/discussions/3959)
16. [Leptos vs Yew vs Dioxus Comparison](https://github.com/superfield-ai/wham/issues/59)
17. [Rust WASM Frameworks 2026 Guide](https://cloud.tencent.com.cn/developer/article/2684145)
18. [Svelte Documentation -- Reactive Assignments](https://svelte.dev/docs/svelte/legacy-reactive-assignments)
19. [Vercel -- What is Svelte? Compiler-First Framework](https://vercel.com/i/what-is-svelte)
20. [Vercel -- What's new in Svelte 5](https://vercel.com/blog/whats-new-in-svelte-5)
21. [SolidJS Documentation -- Fine-Grained Reactivity](https://docs.solidjs.com/advanced-concepts/fine-grained-reactivity)
22. [SolidJS -- Component Model](https://deepwiki.com/solidjs/solid/2.2-component-model)
23. [SolidJS Architecture: Performance by Default](https://feature-sliced.design/kr/blog/solidjs-fine-grained-reactivity)
24. [Building Vel: Compile-to-Native UI Language](https://dev.to/chan27/building-vel-a-token-efficient-compile-to-native-ui-language-275f)
25. [Svelte Architecture with Feature-Sliced Design](https://feature-sliced.design/blog/simple-svelte-architecture)
26. [Compiled CSS-in-JS Limitations](https://compiledcssinjs.com/docs/limitations)
27. [Rust UI is Hard (Warp)](https://www.warp.dev/blog/why-is-building-a-ui-in-rust-so-hard)
