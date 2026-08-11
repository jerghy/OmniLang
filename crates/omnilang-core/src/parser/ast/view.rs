//! OmniView-specific AST types: ComponentDef, ViewNode, Element, StyleValue, etc.

use serde::Serialize;

use crate::lexer::Span;

use super::motion::{GetterDecl, MotionDef, MotionTrigger, StoreDef};
use super::{Block, Expr, FunctionDef, Param, TypeAnnotation};

// ── OmniView-Specific AST ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ComponentDef {
    pub name: String,
    pub props: Vec<PropDecl>,
    pub signals: Vec<SignalDecl>,
    pub computeds: Vec<ComputedDecl>,
    pub events: Vec<EventDecl>,
    pub refs: Vec<RefDecl>,
    pub store_refs: Vec<String>,
    /// Names of stores defined inline in the component body
    /// (`store Name { ... }`). Inline stores are flattened into the
    /// component (their signals/getters/actions become component-level
    /// declarations) and their names are also present in `store_refs`;
    /// this list records the parse-time distinction so validators can
    /// treat them as locally-defined rather than module-level stores.
    pub inline_stores: Vec<String>,
    /// Full bodies of the inline stores, in declaration order — the same
    /// [`StoreDef`] shape used by module-level stores. The members are ALSO
    /// flattened into the component (see [`Self::inline_stores`]); this list
    /// retains the store-level grouping so lowering can register a
    /// module-level [`crate::ir::StoreIR`] per inline store — top-level and
    /// inline stores share one IR representation (v2 域 5).
    pub inline_store_defs: Vec<StoreDef>,
    pub lifecycles: Vec<LifecycleHook>,
    pub fns: Vec<FunctionDef>,
    pub actions: Vec<ActionDef>,
    pub getters: Vec<GetterDecl>,
    pub motions: Vec<MotionDef>,
    /// Component body slot declarations (`@slot name { fallback }`),
    /// ADR-053 D5 — slots moved from inline view declarations to the body.
    pub slots: Vec<SlotDecl>,
    pub view: Option<ViewBlock>,
    pub styles: Vec<StyleRule>,
    /// Component-level route declarations (`@route "/path"`). At most one per
    /// component is legal (V4 duplicate is a semantic-layer error whose
    /// message names both paths, so the parser keeps every declaration).
    #[serde(default)]
    pub route_decls: Vec<RouteDecl>,
    /// Component-level layout declaration (`@layout "/prefix"`) — exclusive
    /// with route_decls (V3, semantic layer).
    #[serde(default)]
    pub layout: Option<LayoutDecl>,
    /// Component-level query-parameter declarations (`@query name: Type = default`).
    #[serde(default)]
    pub queries: Vec<QueryDecl>,
    /// User-level `@media (condition) { ... }` blocks in the style block
    /// (batch 3, spec 2026-08-09 §2.4) — condition text is passthrough,
    /// parsed with paren pairing and kept verbatim.
    #[serde(default)]
    pub media_blocks: Vec<MediaBlock>,
    pub span: Span,
}

/// Component-level route declaration: `@route "/todos/:id" guard: isAdmin lazy: true redirect: "/login";`
/// (spec 2026-08-09 §2.1/§2.5/§2.8).
#[derive(Debug, Clone, Serialize)]
pub struct RouteDecl {
    /// The declared path — always starts with `/` (V2 parse error otherwise).
    pub path: String,
    /// `:id` path segments extracted from the path, in declaration order
    /// (`"/todos/:id"` → `["id"]`). Injected as String props (C2).
    pub params: Vec<String>,
    /// `guard: fnName` — component fn `() -> Bool` referenced for route
    /// protection (V9 semantic validation).
    pub guard: Option<String>,
    /// `lazy: true|false` — route-level lazy loading (C8).
    pub lazy: Option<bool>,
    /// `redirect: "/path"` — guard-failure redirect target (V11 pairing).
    pub redirect: Option<String>,
    /// Source span of the whole declaration.
    pub span: Span,
}

/// Component-level layout declaration: `@layout "/admin";` (spec 2026-08-09 §2.6).
#[derive(Debug, Clone, Serialize)]
pub struct LayoutDecl {
    /// The declared prefix — always starts with `/` (V2 parse error otherwise).
    pub prefix: String,
    /// Source span of the whole declaration.
    pub span: Span,
}

/// Component-level query-parameter declaration: `@query filter: String = "all";`
/// (spec 2026-08-09 §2.4).
#[derive(Debug, Clone, Serialize)]
pub struct QueryDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub default: Option<Expr>,
    /// Source span of the whole declaration.
    pub span: Span,
}

/// A body-level slot declaration: `@slot name { fallback }`.
/// Mirrors the fields of `ViewNode::SlotDirective` so both forms lower
/// through the same shared logic.
#[derive(Debug, Clone, Serialize)]
pub struct SlotDecl {
    /// Slot name — `None` for the default/unnamed slot (`@slot default`).
    pub name: Option<String>,
    /// Fallback content rendered when the parent provides nothing for this slot.
    pub default_content: Option<Vec<ViewNode>>,
    /// Source span of the whole declaration — used for diagnostics
    /// (e.g. duplicate-slot warnings point at the second declaration).
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct PropDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub init: Expr,
    /// `@theme` marker (v2 域 5, design D1): the signal is THE current theme
    /// mode — `signal mode: String @theme = "light";`. Only allowed on
    /// top-level store signals (enforced at parse time); the program-level
    /// `validate_theme` pass wires it into `DesignIR.theme_signal`.
    #[serde(default)]
    pub is_theme: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComputedDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecycleHook {
    pub kind: LifecycleKind,
    pub body: Block,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum LifecycleKind {
    Mount,
    Unmount,
    Effect,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewBlock {
    pub children: Vec<ViewNode>,
}

/// @load 装载函数引用（spec 2026-08-09 §2.1 `fn_ref := identifier | call_expr`）：
/// 裸函数名（v0 兼容）或带实参的调用表达式（命名实例参数绑定）。
#[derive(Debug, Clone, Serialize)]
pub enum LoaderRef {
    Fn(String),
    Call(CallExpr),
}

/// @load 调用表达式形态——函数名 + 实参列表（AST 层无 CallExpr 类型，
/// Expr::FuncCall 泛化到任意表达式；此处为 @load 特化的窄形态）。
#[derive(Debug, Clone, Serialize)]
pub struct CallExpr {
    pub func: String,
    pub args: Vec<Expr>,
    pub span: Span,
}

/// @load 模式词位（spec §2.2/§2.3/§2.4）：paginated / scroll / interval。
/// 模式词之间的组合约束（V7 分页须命名、V8 scroll 须配 paginated）为
/// 语义层硬错误（Task 4），解析层仅登记位。
#[derive(Debug, Clone, Serialize)]
pub struct LoadMode {
    pub paginated: bool,
    pub scroll: bool,
    /// 轮询间隔毫秒（正整数，非 None 即开启 interval 轮询）。
    pub interval: Option<u64>,
}

/// @load 完整规格：实例名（None = 匿名 v0 形态）+ 装载函数 + 模式词位。
#[derive(Debug, Clone, Serialize)]
pub struct LoadSpec {
    pub name: Option<String>,
    pub loader: LoaderRef,
    pub mode: LoadMode,
}

impl LoaderRef {
    /// 装载函数名——Fn 裸名或 Call 调用名（semantic 使用扫描 / IR 降级提取，
    /// IR LoadIntentIR 形状不变，Task 3 起消费完整 Call 形态）。
    pub fn fn_name(&self) -> &str {
        match self {
            LoaderRef::Fn(name) => name,
            LoaderRef::Call(call) => &call.func,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ViewNode {
    Element(Element),
    IfDirective(IfDirective),
    ForDirective(ForDirective),
    SlotDirective {
        name: Option<String>,
        default_content: Option<Vec<ViewNode>>,
    },
    /// Data three-state intent: `@load={fn}` / `@load name={fn} mode words`
    /// — view-level directive that generates loading/error/data signals
    /// (v0: Array-returning fn only; 批次 4: 命名实例/参数绑定/模式词,
    /// spec 2026-08-09 §2.1-§2.4).
    LoadDirective {
        spec: LoadSpec,
        span: Span,
    },
    Text(String),
    Interpolation(Expr),
}

#[derive(Debug, Clone, Serialize)]
pub struct Element {
    pub tag: String,
    pub props: Vec<ElementProp>,
    pub children: Vec<ViewNode>,
    pub self_closing: bool,
    pub span: Span,
}

// `MotionTrigger` (536 B) dwarfs the other variants; the canonical fix is
// `Box<MotionTrigger>`, but the 8 construction sites live in
// `parser/view/mod.rs` which has uncommitted user changes — boxing is
// deferred until those land (clippy `large_enum_variant`, task 5.8).
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
pub enum ElementProp {
    Static {
        name: String,
        value: Expr,
    },
    Bind {
        name: String,
        signal_path: String,
    },
    /// Semantic ref binding: focus={sig}, scroll-x={sig}, scroll-y={sig}.
    /// These are compiler-recognized — not generic static props. The v1
    /// value={sig} 双向形状 (element WITH @ref) is removed (design D2); it is
    /// parsed as a RefBinding so the semantic pass can hard-error it with the
    /// bind:value migration hint. Without @ref the parser reclassifies
    /// value={sig} as a plain one-way Static prop (v4 5.4 discriminator).
    RefBinding {
        /// The semantic binding name: "focus", "scroll-x", "scroll-y" ("value"
        /// reaches RefBinding only on @ref'd elements — rejected by
        /// validate_semantic_bindings as the removed 双向形状)
        binding: String,
        /// The signal expression driving the binding
        signal: Expr,
    },
    Event {
        name: String,
        handler: EventHandler,
    },
    Class {
        value: ClassValue,
    },
    /// `variant: AppDesign.Button(color=primary, size=md)` in view templates
    Variant {
        design_name: String,
        variant_name: String,
        dimension_values: Vec<(String, String)>,
    },
    /// Named slot projection (ADR-053 D5): `header={<div>Title</div>}` on a
    /// component call. The `<` after `{` discriminates slot content from a
    /// regular `{expr}` prop value.
    SlotContent {
        name: String,
        content: Vec<ViewNode>,
    },
    Style {
        entries: Vec<(String, Expr)>,
    },
    /// `a11y: { label: "...", role: "button", hidden: expr, live: "polite",
    /// alt: expr }` — semantic accessibility block (design doc 域 10,
    /// syntax spec §11). Mirrors the `style: { ... }` block shape: an
    /// element attribute carrying the element's accessible name/role/
    /// visibility/live-region/image-alt intent for the three targets.
    A11y(A11ySpec),
    Ref(String),
    MotionTrigger(MotionTrigger),
    /// Form intent: `@form` bare attribute — activates form validation/error
    /// signals/a11y wiring (spec 2026-08-08 §4.1).
    Form,
    /// Field validation declaration: `validate: { required: true, min: 3 }` —
    /// mirrors the `style: { ... }` block shape.
    Validate {
        rules: Vec<ValidateRule>,
    },
    /// `@drag={onDrag}` `@dragEnd={onDragEnd}` — draggable-intent directive
    /// (ADR-073). Source expresses intent ("element is draggable; call
    /// onDrag while dragging, onDragEnd on release"); each target compiler
    /// wires its own pointer-capture mechanism (React setPointerCapture /
    /// OmniView capturePointer / Flutter GestureDetector pan) —
    /// capturePointer is a compiler-internal detail, never user-facing
    /// (ADR-071 decision 3). The two attributes are order-independent:
    /// `@dragEnd` without `@drag` hard-errors at parse time.
    Drag {
        /// The @drag={fn} move handler (bare function name, required)
        on_drag: String,
        /// The @dragEnd={fn} release handler (bare function name, optional)
        on_drag_end: Option<String>,
    },
    /// Modal dialog intent: `@modal={openSig}` on a `dialog` element (spec
    /// 2026-08-09 §2.1). The value is the user-declared boolean open signal
    /// (a read reference); focus trap / Esc / overlay-click close / a11y
    /// attributes are compiler-built-in, user writes zero of that logic.
    Modal {
        /// The open-signal reference (`{isOpen}` → `VarRef("isOpen")`).
        open_signal: Expr,
    },
    /// Skeleton placeholder intent: bare `@skeleton` or `@skeleton="shape"`
    /// (spec 2026-08-09 §2.3). Only legal inside a `@load` loading branch —
    /// the semantic layer enforces that hard constraint (V9).
    Skeleton {
        /// Shape name (`text`/`line`/`circle`/`card`), None for the default
        /// `text`. Legality of the value is a semantic-layer check (V10),
        /// not a parse-layer one.
        shape: Option<String>,
    },
    /// Render-failure fallback intent: bare `@errorBoundary` (spec
    /// 2026-08-09 §2.4). The fallback UI arrives via a body-level
    /// `@slot fallback`; no value is accepted.
    ErrorBoundary,
}

/// The `a11y: { ... }` block contents (design doc 域 10 BNF):
/// `label`/`role`/`live` are string-valued (BNF `'string'`), `hidden`/`alt`
/// are arbitrary expressions. All entries optional — at least one is
/// required (the parser hard-errors on an empty block).
#[derive(Debug, Clone, Default, Serialize)]
pub struct A11ySpec {
    /// Accessible name (→ aria-label / Semantics(label:) / aria-label).
    pub label: Option<String>,
    /// Semantic role (→ role / Semantics flags subset / role).
    pub role: Option<String>,
    /// Hidden from assistive technology (→ aria-hidden / ExcludeSemantics).
    pub hidden: Option<Expr>,
    /// Live region politeness (→ aria-live / Semantics(liveRegion:)).
    pub live: Option<String>,
    /// Image alternative text (→ alt / Image semanticLabel / alt).
    pub alt: Option<Expr>,
}

/// One rule inside a `validate: { ... }` block: `required: true` →
/// `ValidateRule { name: "required", value: Expr::Bool(true) }`. Rule names
/// are identifiers; values are arbitrary expressions (spec 2026-08-08 §4.1).
#[derive(Debug, Clone, Serialize)]
pub struct ValidateRule {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub enum EventHandler {
    InlineFn { params: Vec<Param>, body: Block },
    FnRef(String),
    StoreAction { store: String, action: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum ClassValue {
    StyleRef(String),
    StringLiteral(String),
    ClassMap(Vec<(String, Expr)>),
    /// `class: AppDesign.Card` — reference to a design block class
    DesignClass {
        design_name: String,
        class_name: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct IfDirective {
    pub condition: Expr,
    pub body: Vec<ViewNode>,
    pub else_ifs: Vec<(Expr, Vec<ViewNode>)>,
    pub else_body: Option<Vec<ViewNode>>,
    pub transition_mode: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForDirective {
    pub item_name: String,
    pub iterable: Expr,
    pub body: Vec<ViewNode>,
    pub empty_body: Option<Vec<ViewNode>>,
    pub key_expr: Option<Expr>,
    /// `@for item in list [scroll]` — hint for Flutter ListView.builder codegen.
    pub scroll_hint: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct StyleRule {
    pub class_name: String,
    pub properties: Vec<(String, StyleValue)>,
    pub is_tag_selector: bool,
    /// When the selector contains parts the parser cannot represent (sibling
    /// `+`/`~`, child `>`, attribute `[attr]`, parameterized pseudo-classes,
    /// pseudo-elements, selector lists), the rule is silently truncated at the
    /// first unsupported token — this records what was skipped so the semantic
    /// layer can emit the W-301 warning (honest degradation, ADR-054) instead
    /// of silently dropping the selector part. `None` = fully supported.
    #[serde(default)]
    pub unsupported_selector: Option<UnsupportedSelector>,
}

/// Record of a selector part skipped by the parser (batch 3, spec 2026-08-09 §2.3).
/// The semantic-layer `validate_style_selectors` pass turns this into the
/// W-301 warning with the full selector text, the skip point, and the category.
#[derive(Debug, Clone, Serialize)]
pub struct UnsupportedSelector {
    /// Full selector text (class name + skipped part), e.g. `input:checked +span`.
    pub text: String,
    /// Span of the first skipped token (rule anchor for the warning).
    pub span: Span,
    /// Category word: 兄弟选择器 / 子选择器 / 属性选择器 / 选择器列表 / 带参伪类.
    pub category: String,
}

/// User-level `@media (condition) { rules }` block in a style block
/// (batch 3, spec 2026-08-09 §2.4). The condition text is passthrough —
/// the compiler does not parse its inner structure.
#[derive(Debug, Clone, Serialize)]
pub struct MediaBlock {
    /// Raw condition text including the outer parens: `(min-width: 768px)`.
    pub condition: String,
    /// Rules inside the block (same syntax as top-level style rules).
    pub rules: Vec<StyleRule>,
}

#[derive(Debug, Clone, Serialize)]
pub enum StyleValue {
    Dimension(f64, String),
    HexColor(String),
    String(String),
    Number(f64),
    Ident(String),
    List(Vec<StyleValuePart>),
}

#[derive(Debug, Clone, Serialize)]
pub enum StyleValuePart {
    Dimension(f64, String),
    Ident(String),
    Number(f64),
    HexColor(String),
}
