# OmniLang 语义级 DSL 语法规范（v1 —— 已 Superseded）

**状态**: Superseded（2026-08-03 起由 v2 设计文档接管）
**日期**: 2026-07-31（v1 基线；2026-08-02 状态转 Approved；2026-08-03 状态转 Superseded）
**依赖**: ADR-052 (Semantic-Level DSL Cross-Target Codegen)
**关联设计文档**: `docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md`
**当前生效设计**: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（唯一权威——状态 Implemented；BNF 与行为事实均以 v2 设计文档 + parser 为准。本 v1 规范为 Superseded 历史基线，事实断言不再单独维护；2026-08-03 修正了本文件中已证伪的断言，见 §3.4/§3.6/§12/§13）

> **⚠️ machine-pinned（v4 注记）**：本文件仍被 verify-vocabulary.py /
> verify-semantic-universe.py 的「文档 ⊆ 代码」单向检查引用——**不得删除**。
> 事实断言已重指向 parser 常量（SEMANTIC_ANIM_PROPS / event_names.rs /
> RefOpIR——见 scripts/verify-vocabulary.py 与 verify-semantic-universe.py）；
> 本文件仅作 BNF 教学参考。

---

## 1. 设计哲学

### 1.1 语义级语言原则

```
.omni 源码 = 平台无关的意图声明
编译器     = 理解意图 → 为目标平台生成该平台开发者会手写的代码
生成代码   ≠ OmniView 模式的翻译
生成代码   = 该平台上最地道的写法
```

### 1.2 语义域分层

| 层 | 内容 | 编译器行为 |
|----|------|-----------|
| **语义原语层** | ref 绑定、动画触发器、design token、slot、store signal、@if/@for、生命周期 | 理解意图 → 按平台映射生成地道代码 |
| **通用计算层** | 表达式、运算、函数、闭包、match、async/await、类型 | 语法糖展开（`?.`、`??=`、block IIFE），其余直接翻译为目标语言 |

> **2026-08-03（Task 4.4）**：平台特有层已删除——`@native` 是死 token（编译器
> 无任何消费方），lexer 令牌已移除；`@native(...)` 现在产生干净解析错误
> （`Unexpected token at top level: Ident("@native")`）。如未来确需 escape
> hatch，1 天内可重新加入（pragmatic-enforcer 决策）。

### 1.3 语法惯例

| 惯例 | 适用场景 | 示例 |
|------|---------|------|
| **声明式绑定** | 高频操作（聚焦、滚动）| `focus={sig}`, `scroll-y={sig}`, `scroll-x={sig}`；值双向用 `bind:value` |
| **语义方法** | 低频一次性操作（选区、测量、指针捕获）| `ref.select()`, `ref.measure()` |
| **属性式触发器** | 元素上的动画/行为声明 | `@enter={motion}`, `@hover={motion}` |
| **关键字声明** | 顶层/组件成员定义 | `signal`, `token`, `@slot`, `@mount` |
| **模板指令** | 控制流 | `@if`, `@for`, `@empty` |

### 1.4 代码生成原则

- 不生成包装函数（`setTheme()`、`__omni_*`）
- 直接用目标平台的惯用 API（React: `useRef`+`useEffect`、Flutter: `FocusNode`+`AnimationController`）
- Feature floor 之外的特性：出 hard diagnostic，不静默降级
- 第三方库一步到位：Zustand v5、Riverpod 3.0
- 非 P0 目标（Python/JS/Dart/Rust/Dioxus/Vue）遇到 design block → error，不静默降级为 warning+零文件

---

## 2. Element Reference System（Ref 系统）

### 2.1 语义定义

Ref 系统提供对元素的**平台无关的语义级访问**。两种形式：

| 形式 | 适用场景 | 语法 |
|------|---------|------|
| **语义绑定** | 高频：聚焦、滚动 | 元素属性，signal 驱动 |
| **双向值绑定** | 表单值双向同步 | `bind:value={sig}`（唯一双向形式） |
| **语义方法** | 低频：选区、测量、指针捕获 | ref 变量上的编译器识别方法 |

> **值绑定收敛（设计 D2）**：`value={sig}` 语义绑定形式已删除——`bind:value` 是**唯一**的双向绑定；`value={expr}`（含 `value="text"`）是**普通单向 prop**，无魔法。写 `value={sig}` 裸 signal 形式 → 编译错误「用 bind:value 双向绑定，或普通 value prop 单向」。

### 2.2 BNF 语法

```
ref_decl ::= 'ref' <ident> ':' <type_annotation> ';'?

// ── 语义绑定（元素属性，需要元素同时有 @ref={name}）──

semantic_binding ::= 'focus' '=' <expr>
                   | 'scroll-x' '=' <expr>
                   | 'scroll-y' '=' <expr>
                   // 'value' '=' <expr> 已删除（D2）→ error 迁移提示

// 这些属性出现在元素上时，编译器理解它们的语义（不只是普通 prop）
// 元素必须已有对应的 @ref 绑定

// ── 双向绑定（无需 @ref）──

value_binding ::= 'bind:value' '=' expr             (* 双向：输入/选择控件 *)
                | 'bind:checked' '=' expr           (* 双向：checkbox 选中态 *)
value_prop    ::= 'value' '=' <expr>               (* 普通单向 prop，无魔法 *)

// ── 语义方法（在 action/expression 中对 ref 变量调用）──

ref_semantic_call ::= <ref_name> '.' ( ref_op_select
                                     | ref_op_set_selection
                                     | ref_op_measure
                                     | ref_op_capture_pointer
                                     | ref_op_release_pointer )

ref_op_select          ::= 'select' '(' ')'
ref_op_set_selection   ::= 'setSelection' '(' expr ',' expr ')'
ref_op_measure         ::= 'measure' '(' ')'     // → { width: Float, height: Float, x: Float, y: Float }
ref_op_capture_pointer ::= 'capturePointer' '(' ')'
ref_op_release_pointer ::= 'releasePointer' '(' ')'
```

### 2.3 完整示例

```omni
component SearchPage {
  ref searchBox: Element
  ref searchInput: Element
  ref scrollPane: Element

  signal isSearchActive: Bool = false
  signal scrollPosition: Float = 0
  signal searchText: String = ""

  // ── 高频：语义绑定（focus/scroll）+ bind:value 双向 ──
  view {
    <div>
      <input
        @ref={searchInput}
        bind:value={searchText}    // 双向值绑定（唯一双向形式）
        focus={isSearchActive}     // 聚焦控制（需 @ref）
        placeholder="搜索..."
      />

      <div @ref={scrollPane} scroll-y={scrollPosition}>
        <span>Results</span>
      </div>
    </div>
  }

  // ── 低频：语义方法 ──
  fn handleScrollToTop() {
    scrollPosition = 0           // signal 驱动 → 编译器映射到 scrollTo
  }

  fn handleSelectAll() {
    searchBox.select()           // 编译器识别为 Select 语义原语
  }

  fn handleMeasure() {
    let rect = searchBox.measure()
    // rect: { width: Float, height: Float, x: Float, y: Float }
  }
}
```

### 2.4 平台映射

| 语义 API | React | Flutter | OmniView |
|----------|-------|---------|----------|
| `focus={sig}` | `useEffect(() => sig ? ref.focus() : ref.blur(), [sig])` | `FocusNode` + listener → `requestFocus()/unfocus()` | `el.focus()/blur()` |
| `scroll-y={sig}` | `useEffect(() => ref.scrollTo({top: sig}), [sig])` | `ScrollController.animateTo(sig)` | `el.scrollTo(0, sig)` |
| `scroll-x={sig}` | `useEffect(() => ref.scrollTo({left: sig}), [sig])` | `ScrollController.animateTo(sig)` (horizontal) | `el.scrollTo(sig, 0)` |
| `bind:value={sig}`（双向） | `value={sig}` + `onChange={e => set(sig, e.target.value)}` | `TextEditingController` + 守卫 listener 回写 `setState` | guard + input listener + 守卫 effect（双向） |
| `value={expr}`（单向 prop） | 普通 `value={expr}` prop | 普通 widget 参数（无回写） | 普通属性 |
| `ref.select()` | `ref.current.select()` | `TextEditingController.selection = TextSelection(...)` | `el.select()` |
| `ref.setSelection(s,e)` | `ref.current.setSelectionRange(s, e)` | `TextEditingController.selection = TextSelection(...)` | `el.setSelectionRange(s, e)` |
| `ref.measure()` | `ref.current.getBoundingClientRect()` | `RenderBox.localToGlobal()` + `size` | `el.getBoundingClientRect()` |
| `ref.capturePointer()` | `ref.current.setPointerCapture(id)` | error「Flutter 不支持指针捕获，请用 @drag 指令代替」（D3 演进注记，ADR-071/073） | `el.setPointerCapture(id)` |
| `ref.releasePointer()` | `ref.current.releasePointerCapture(id)` | error「同上」 | `el.releasePointerCapture(id)` |

> **方向一致性契约**：`bind:value` 在三目标必须双向且方向一致——React 发射 `value` + `onChange` 写回；Flutter 发射 controller + 守卫 listener 写回（`setState`）；OmniView 发射 input listener + 守卫 effect。禁止任何目标单向化。删除的 `value={sig}` 形式不再产生单向 useEffect / 单向 sync 路径（评审 HIGH-1 分歧消除）。

### 2.5 已删除的 DOM API

以下不能通过 ref 访问——编译器在语义层拒绝（compile error）：

| 删除的 API | 替代方案 |
|-----------|---------|
| `ref.style.*` | `style:{...}` 绑定 signal |
| `ref.innerHTML` | 文本插值 |
| `ref.addEventListener` | `@on*` 事件绑定 |
| `ref.querySelector` | 违反组件封装 |
| `ref.classList` | `class:` 绑定 |
| `ref.parentElement` | 违反组件封装 |

### 2.6 同步注记（Phase 2，2026-08-02）

> Phase 2（域 1 Ref + 域 7 Events）落地后的 parser 事实同步
> （doc/spec/parser 三方一致；与 `2026-08-02-semantic-syntax-v2-design.md`
> 域 1 / 域 7 的映射表一致）。本规范无独立事件域章节（事件绑定 `@on*`
> 仅见 §2.5），事件同步注记就近附于此。

**Ref 操作（域 1）**

- `RefOpIR::SetValue` 已删除（11→10 ops）：`ref.setValue(x)` / `ref.value = x` / `ref = x`
  命令式写 → error「命令式写已删除，用 bind:value 双向绑定」
- `capturePointer(id)` → `capturePointer()` 无参数（语义：锁定后续指针事件到本元素；
  多指针显式锁定 = floor 外诊断）；`releasePointer()` 同
- `@drag={fn}` / `@dragEnd={fn}` 拖拽意图（ADR-071/073）：指针捕获降级为编译器内部
  细节（React setPointerCapture / Flutter GestureDetector pan / OmniView capturePointer
  原语）——demo 源码只写意图，不再显式调用 capturePointer/releasePointer；事件数据
  `{x, y, dx, dy, pointerId, startX, startY}`（dx/dy = 相对按下点累计位移）
- `value={sig}` 语义绑定形式已删除——`bind:value` 是**唯一**双向绑定语法；
  `value={expr}` = 普通单向 prop；隐式 auto-bind 已删除（value 永远是你写什么就是什么）

| 输入 | 诊断 |
|------|------|
| `ref.value = x` / `ref.setValue(x)` / `ref = x` | error：「命令式写已删除，用 bind:value 双向绑定」 |
| `capturePointer(任何参数)` | error：「capturePointer 无参数；多指针锁定不支持」 |
| Flutter `capturePointer()` | error：「Flutter 不支持指针捕获，请用 @drag 指令代替（编译为 GestureDetector onPan 接线）」——D3 演进为目标差异注记（ADR-071/073）：捕获是编译器内部细节，用户面向 @drag 意图 |

**Events（域 7）**

- `map_event` 别名表统一为**单表**（`omnilang/src/codegen/shared/event.rs`）——
  单表共享，跨后端一致；各后端仅叠加各自的 handler 命名约定。历史各后端双写
  （react/utils.rs、omniview/utils.rs）是 `dblclick` 类 bug 源头，已删除
- `KeyEvent.code` → Flutter 稳定映射 `physicalKey.keyId`（layout 无关，DOM-code 类比）；
  `debugName` 永不作为数据发射；类型注记：schema `code: String` vs Flutter keyId int
  （已知运行时失配，per-target 字段类型化为终修）
- 未映射事件字段（如 ClipboardEvent.text / SubmitEvent.values / deltaMode）→
  warning + 降级默认值注释（不静默）

---

## 3. Animation System（动画系统）

### 3.1 语义定义

动画的语义是三方分解：**触发器（何时）+ 关键帧（怎么变）+ 播放控制（变多久/几次）**。所有三个维度都是平台无关的。

### 3.2 BNF 语法

#### Motion 定义

```
motion_def ::= '@motion' <ident> motion_body

motion_body ::= '{' motion_param* keyframe_stops playback_props '}'

motion_param ::= 'over' ':' expr            // duration: 300ms, 1.5s
               | 'easing' ':' easing_expr   // ease-out, cubic(x1:..., y1:..., x2:..., y2:...)

keyframe_stops ::= keyframe_stop (','? keyframe_stop)*

keyframe_stop ::= stop_position '{' stop_property (','? stop_property)* '}' stop_ease?

stop_position ::= 'from'                      // 0%
                | 'to'                        // 100%
                | expr '%'?                   // 数字后可选 %，如 50% 或 50

stop_property ::= semantic_anim_prop ':' expr
                | '--' <ident>                // CSS custom property passthrough（仅 CSS 目标，ADR-053 D3）

semantic_anim_prop ::= 'opacity' | 'scale' | 'scaleX' | 'scaleY'
                     | 'translateX' | 'translateY' | 'translateZ'
                     | 'rotate' | 'rotateX' | 'rotateY' | 'rotateZ'
                     | 'skewX' | 'skewY'
                     | 'backgroundColor' | 'color'
                     | 'width' | 'height'        // diagnostic in Flutter
                     | 'borderRadius' | 'fontSize'   // v2：三目标已验证（语义集，Task 5.2）
// 19 属性白名单（ADR-053 D3，语义集 v1 + v2 新增 borderRadius/fontSize）；白名单外属性在解析期报错

stop_ease ::= 'ease' (':' easing_expr)?       // per-stop easing（关键字为 ease，冒号可选）

easing_expr ::= <ident>                             // kebab-case: ease-in, ease-out, power2-out, back-in-out, ...
              | 'cubic' '(' 'x1' ':' num ',' 'y1' ':' num ',' 'x2' ':' num ',' 'y2' ':' num ')'
              | 'steps' '(' int ',' ('start' | 'end') ')'  // 仅位置参数（无 count: 前缀）；Flutter diagnostic

playback_props ::= (playback_prop)*

playback_prop ::= 'loop' int?                       // 裸 `loop` = 无限循环（无 `infinite` 关键字）
                | 'direction' ('normal' | 'reverse' | 'alternate' | 'alternateReverse')
                | 'fill' ('none' | 'forwards' | 'backwards' | 'both')
                | 'delay' expr                      // 空格形式（无冒号），首次播放延迟 ms
                // 已删除（v2 Task 5.3）：'paused' 不再被解析——解析期硬诊断
                // 「paused 已删除，用 @if 条件渲染替代」（ADR-053；PlaybackIR.paused 已删除）
```

> **parser 对齐说明**（2026-08-01）：以上为 parser 实际接受的形式——
> 命名缓动为 kebab-case（camelCase 旧名在过渡期被规范化，如 `power2Out`
> → `power2-out`）；自定义贝塞尔为 `cubic(x1:, y1:, x2:, y2:)` 命名参数
> 形式；`steps()` 仅位置参数；`delay` 为空格形式；`direction` 含
> `alternateReverse`。

#### 触发器（元素属性）

```
motion_trigger_attr ::= '@enter' '=' '{' motion_expr '}'
                      | '@hover' '=' '{' motion_expr '}'
                      | '@press' '=' '{' motion_expr '}'
                      | '@focus' '=' '{' motion_expr '}'
                      | '@in-view' '=' '{' motion_expr '}'
                      | '@scroll' [scroll_params] '=' '{' motion_expr '}'
                      | '@drag-horizontal' [drag_params] '=' '{' motion_expr '}'
                      | '@drag-vertical' [drag_params] '=' '{' motion_expr '}'

scroll_params ::= 'start:' expr 'end:' expr 'scrub:' bool
drag_params   ::= 'constraints:' '[' expr ',' expr ']' 'elastic:' expr
                  'threshold:' expr

motion_expr ::= <ident>                             // 命名 motion 引用
              | map_literal                         // 内联 map 形式（§3.7）：@enter={{ opacity: 0, scale: 0.8 }}
```

> **已实现范围**（2026-08-01，与 parser 对齐）：8 个触发器，kebab-case，
> 值必须是命名 motion 引用。以下**尚未实现**（ADR-053 移除，待重新实现，
> 见 `.architecture/deferrals.md` #19）：
>
> - ~~触发器子属性（旧 `@in-view-once`/`@scroll-start`/`@drag-constraints`
>   等）~~ —— **Task 5.1 已恢复**（deferral #19 关闭）：`@scroll`/`@drag-*`
>   控制维度以冒号形式子参数挂在触发器关键字与 `={motion}` 绑定之间
>   （`@scroll start: 0 end: 1000 scrub: true ={parallaxScroll}`）。未知或
>   畸形子参数是硬错误（不解析忽略）；其余触发器不接受子参数。语义校验
>   （validate_motion）：`start`/`end` 必须是数字且成对、`scrub` 必须是布尔、
>   `constraints` 必须是 `[min, max]` 数字且 min < max、`elastic`/`threshold`
>   必须是非负数字（错误信息为中文硬诊断）。
> - 内联动画体 map 形式已实现（§3.7：`@enter={{ opacity: 0, scale: 0.8 }}`）；
>   `@enter = { over: 200ms; from { ... } to { ... } }` 完整体语法**不解析**
>   ——trigger 值必须是命名 motion 引用或 map 字面量

### 3.3 完整示例

```omni
@motion fadeIn {
  from {
    opacity: 0
    scale: 0.95
  }
  50% {
    opacity: 0.5
  }
  to {
    opacity: 1
    scale: 1
  }
  over: 300ms
  easing: ease-out
}

@motion parallaxScroll {
  from { translateY: -100 }
  to   { translateY: 0 }
  over: 1s
}

@motion scaleUp {
  from { scale: 1 }
  to   { scale: 1.2 }
  over: 200ms
}

@motion swipeCard {
  from { translateX: 0 }
  to   { translateX: 300 }
  over: 300ms
}

component AnimatedCard {
  view {
    <div @enter={fadeIn} @hover={scaleUp} />

    <div @scroll start: 0 end: 1000 scrub: true ={parallaxScroll} />

    <div @drag-horizontal constraints: [-100, 100] elastic: 0.5
             threshold: 0.3 ={swipeCard} />
  }
}
```

### 3.4 触发器语义 → 平台映射

| 触发器 | 语义 | React 实现 | Flutter 实现 |
|--------|------|-----------|-------------|
| `@enter` | 元素首次挂载时正向播放 | `useEffect([], [])` + `element.animate()` | `initState` + `AnimationController.forward()` |
| `@hover` | 指针进入正向，离开反向 | `onMouseEnter/Leave` + `element.animate()` play/reverse | `MouseRegion(onEnter/onExit)` + controller.forward/reverse |
| `@press` | 按下正向，释放反向 | `onPointerDown/Up` + `element.animate()` | `GestureDetector(onTapDown/Up)` + controller |
| `@focus` | 获得焦点时正向播放 | `onFocus` + `element.animate()` | `FocusNode.addListener` + controller |
| `@in-view` | 滚入视口时播放 | `IntersectionObserver` + `element.animate()` | `_OmniInViewWidget` |
| `@scroll` | 滚动位置驱动动画进度 | scroll event + progress scrubbing | `ScrollController` 驱动 |
| `@drag-*` | 拖拽距离驱动动画进度 | pointer events + 距离→进度映射 | `GestureDetector.onPanUpdate` |

**控制维度（Task 5.1，v1 语法恢复）**：

| 子参数 | 语义 | React 实现 | Flutter 实现 |
|--------|------|-----------|-------------|
| `@scroll start:`/`end:` | 数字滚动映射区间 | `const startPos = N; const endPos = M;` → `progress = (scrollY - N) / (M - N)` | `_OmniScrollDrivenWidget(start: N, end: M)`（类型化 `double?` 字段） |
| `@scroll scrub: true` | 平滑系数（布尔 true = 1s） | `scrub > 0` 指数平滑 | `_OmniScrollDrivenWidget(scrub:)` 指数平滑 |
| `@drag-* constraints: [min, max]` | 拖拽范围限制 + 弹性回弹 | `const constraints = [min, max]`（运行时 clamp + `(1 - elastic)` 回弹） | `_OmniDragWidget(constraintsMin: min, constraintsMax: max)` |
| `@drag-* elastic:`/`threshold:` | 弹性系数 / 吸附阈值 | 生成数值常量 | 类型化构造参数 |

> 缺省行为：`@scroll` 无 `start`/`end` 回落视口相对字符串配置（`"top 80%"` →
> `"bottom 20%"`）；`@drag-*` 无 `constraints` 回落关键帧范围推导
> （`maxExtent`），无可用范围时带编译器警告。
>
> ⚠️ 诚实降级（ADR-054）：释放后惯性 decay 在三个目标上均**未实现**——
> 释放行为统一为 threshold 吸附（play/reverse）。`velocity_max` 已于 v4
> 移除（零消费者——拖拽速度跟踪器无消费方，见 deferral #84）。

### 3.5 Keyframe 属性 → 平台映射

| 语义属性 | React (WAAPI) | Flutter |
|---------|--------------|---------|
| `opacity` | `opacity: [from, to]` | `Tween<double>` + `Opacity` widget |
| `scale` | `transform: scale(...)` | `Tween<double>` + `Transform.scale` |
| `translateX/Y` | `transform: translate(...)` | `Tween<double>` + `Transform.translate` |
| `rotate` | `transform: rotate(...deg)` | `Tween<double>` + `Transform.rotate` |
| `backgroundColor` | `backgroundColor: [...]` | `Tween<Color?>` + `Container` |
| `color` | `color: [...]` | `Tween<Color?>` + `TextStyle` |
| `width` / `height` | WAAPI 原生支持 | ⚠️ **warning**: 动画布局属性性能差，推荐用 scale |

每个属性在 Flutter 生成**独立的 `Animatable<T>`**，多 stop keyframe 用 `TweenSequence`。

### 3.6 Diagnostic

| 场景 | 目标 | 级别 | 说明 |
|------|------|------|------|
| keyframe 中 `width`/`height` | Flutter | warning | 动画布局属性触发重 layout |
| `steps()` easing | Flutter | warning | Flutter 无原生 steps easing——回退 `Curves.linear` + 生成警告（2026-08-03 修正：v1 曾误记为 error） |
| `rotateX/Y/Z`, `skewX/Y`, `translateZ` | Flutter | warning | 需 `Transform` widget 组合，3D 变换受限于 Flutter 渲染模型 |

> 2026-08-03 修正：`steps()` 在 Flutter 为 **warning** + `Curves.linear` 回退
> （实现：`omnilang/src/codegen/flutter/motion.rs` `easing_flutter_diagnostic`；
> 与 v2 设计文档域 2 诊断表一致）。

### 3.7 内联动画体（Inline Motion Body）

内联动画体使用 **map 字面量**形式（双花括号），lowering 时提升为匿名命名 motion：

```omni
component InlineMotionDemo {
    view {
        <div @enter={{ opacity: 0, scale: 0.8 }}>Content</div>
    }
}
```

IR lowering（编译期自动进行，源码无需感知）：
- `@enter={{ ... }}` 的 map 字面量 → `KeyframeIR`（命名 `__anon_m_N`）→ 追加到 `ModuleIR.motions`
- trigger 重写为 `motion_ref: "__anon_m_N"` —— 三个 codegen 只处理命名 motion，无重复逻辑

> ⚠️ 非 map 形式（`@enter = { over: 200ms; from { opacity: 0 } to { opacity: 1 } }`）
> **不解析**——trigger 值必须是命名 motion 引用（`@enter={fadeIn}`）或 map 字面量
> （`@enter={{ ... }}`，键为动画属性、值为关键帧值，属性间用逗号分隔）。

---

## 4. Design Tokens（设计系统）

### 4.1 语义定义

Design block 的语义：**定义一组有名字的设计值，可以有主题变体，可以被组件引用**。所有设计值都是平台无关的——编译器知道每个 token 类型的语义，按目标映射。

### 4.2 BNF 语法

```
design_def ::= 'design' <ident> '{' design_body '}'

design_body ::= ( themes_decl | default_theme_directive
                | token_decl | class_def | variant_def )*

// v2 域 3 (design D1, 2026-08-03): 花括号形式取代 v1 的 `@themes a, b;`
// （分号形式解析失败并带迁移提示）；空 `@themes { }` 是解析错误。
themes_decl ::= '@themes' '{' <ident> (',' <ident>)* '}'
default_theme_directive ::= '@default-theme' <ident> ';'?

// ── Token ──

token_decl ::= 'token' <ident> ':' token_type '=' token_value
               (theme_override)* ';'?

token_type ::= 'color' | 'size' | 'radius' | 'fontFamily' | 'fontSize'
             | 'fontWeight' | 'lineHeight' | 'letterSpacing' | 'shadow'
             | 'duration' | 'easing' | 'opacity' | 'zIndex'
// 小写（parser 实际接受形式；旧文档中的大写形式不解析）

token_value ::= <literal>                          // 静态值
              | <const_ref>                        // 常量引用: BRAND_COLOR
              | <expr>                             // 动态: ThemeStore.primaryColor

theme_override ::= <ident> token_value             // dark "#818cf8"

// ── Class ──

class_def ::= 'class' <ident> '{' class_body '}'

class_body ::= ( css_property ':' (css_value | token_ref) ';'?
               | pseudo_state )*

pseudo_state ::= '&:hover' '{' class_body '}'
               | '&:focus' '{' class_body '}'
               | '&:active' '{' class_body '}'
               | '&:disabled' '{' class_body '}'

token_ref ::= 'token' '(' <ident> ')'

// ── Variant ──

variant_def ::= 'variant' <ident> '{' variant_body '}'

variant_body ::= variant_dim* variant_default variant_branch+

variant_dim ::= '@dim' <ident> ':' <ident> ('|' <ident>)+ ';'?

variant_default ::= '@default' '{' css_property ':' css_value (','? css_property ':' css_value)* '}' ';'?

variant_branch ::= <ident> '.' <ident> '{' css_property ':' css_value (','? css_property ':' css_value)* '}' ';'?

// ── 组件中使用 class / variant（元素属性）──

class_attr ::= 'class:' design_class_ref
             | 'class:' '{' class_map_entry (',' class_map_entry)* '}'

design_class_ref ::= <design_name> '.' <class_name>

class_map_entry ::= <ident> ':' expr        // className: booleanExpr

variant_attr ::= 'variant:' design_branch_ref
               | 'variant:' design_call_ref

design_branch_ref ::= <design_name> '.' <variant_name> '.' <branch_name>

design_call_ref ::= <design_name> '.' <variant_name> '(' variant_dim_value (',' variant_dim_value)* ')'

variant_dim_value ::= <ident> '=' <ident>   // dimName=valueName
```

### 4.3 完整示例

```omni
store ThemeStore {
    signal accentColor: String = "#f97316";
}

design AppTheme {
  @themes { light, dark }
  @default-theme light;

  // ── 静态 token（类型为小写关键字；值为引号字符串）──
  token primaryColor: color = "#6366f1" dark "#818cf8";

  token spacingSm: size = "8px";
  token spacingMd: size = "16px";
  token radiusMd: radius = "8px";
  token fontMain: fontFamily = "Inter";
  token fontSizeBase: fontSize = "14px";
  token shadowCard: shadow = "0 2px 8px rgba(0,0,0,0.1)";
  token durationNormal: duration = "300ms";
  token easeOut: easing = "cubic-bezier(0.16, 1, 0.3, 1)";  // design token 值 = CSS easing 字符串（引号字面量）；motion easing 的 cubic() 命名参数形式见 §3.2

  // ── 动态 token（来自运行时 store 信号）──
  token accentColorDynamic: color = ThemeStore.accentColor;

  // ── Class ──
  class Card {
    background-color: token(primaryColor);
    border-radius: token(radiusMd);
    padding: token(spacingMd);
    box-shadow: token(shadowCard);
    &:hover { opacity: 0.9; }
  }

  class Button {
    background-color: token(primaryColor);
    padding: "8px 16px";
    border-radius: token(radiusMd);
    transition: "all " token(durationNormal) token(easeOut);
    &:hover { opacity: 0.85; }
    &:active { scale: 0.97; }
    &:disabled { opacity: 0.5; }
  }

  // ── Variant（@default 只接受 dim=value 对；分支按 dim.value 覆盖）──
  variant ButtonSize {
    @dim color: primary | accent;
    @dim size: sm | md | lg;
    @default { color: primary; size: md; }
    base {
      border: none;
      border-radius: token(radiusMd);
    }
    color.primary { background-color: token(primaryColor); }
    size.sm { padding: token(spacingSm); }
    size.md { padding: token(spacingMd); }
    size.lg { padding: "24px"; }
  }
}
```

### 4.4 使用语法

```omni
component MyPage {
  signal isOpen: bool = false;

  view {
    // ── Class 引用 ──
    <div class: AppTheme.Card>
      <h2>标题</h2>
    </div>

    // ── Class 条件 map ──
    <div class: { overlay: true, visible: isOpen, hidden: !isOpen }>
      弹窗内容
    </div>

    // ── Variant 单维度（调用形式，可省略未指定维度）──
    <button class: AppTheme.Button variant: AppTheme.ButtonSize(size=lg)>
      大按钮
    </button>

    // ── Variant 多维度（调用形式）──
    <button class: AppTheme.Button variant: AppTheme.ButtonSize(color=primary, size=sm)>
      小主色按钮
    </button>

    // ── 直接引用 token（组件内用 CSS 变量；token() 仅限 design 块内）──
    <div style: { color: "var(--primaryColor)" } />
  }
}
```

#### Class 的两种形式

| 形式 | 语法 | 语义 | 示例 |
|------|------|------|------|
| **设计 class 引用** | `class: DesignName.ClassName` | 应用 design block 中定义的 class 样式 | `class: AppTheme.Card` |
| **条件 class map** | `class: { name: bool_expr, ... }` | 根据条件动态开关 CSS class | `class: { active: isActive }` |

#### Variant 的两种形式

| 形式 | 语法 | 语义 | 示例 |
|------|------|------|------|
| **单维度选择** | `variant: DesignName.VariantName.branch` | 选择 variant 的一个分支 | `variant: AppTheme.Size.lg` |
| **多维度指定** | `variant: DesignName.VariantName(dim1=val1, dim2=val2)` | 为每个维度指定值，编译时查表选择对应分支 | `variant: AppTheme.Button(color=primary, size=sm)` |

### 4.5 主题切换（v2，design D1，2026-08-03）

主题系统由两部分组成——**主题集合声明** + **当前主题信号标记**：

```omni
// design 块内声明主题集合（与 token/class 同级）：
design AppTheme {
  @themes { light, dark }
}

// 顶层 store 中用 @theme 标记当前主题信号（仅 String，仅顶层 store）：
store ThemeStore {
  signal mode: String @theme = "light"

  // 切换主题 = 普通 action 写该信号：
  action setMode(m: String) { mode = m }
}
```

- 语义：**`@theme` 标记的信号 = 当前主题模式；切换它 = 切换主题**。编译器因此知道主题状态从哪来，并自动接线三目标（React 订阅该 store → 写 `data-theme` 属性；Flutter 生成 `ThemeModeNotifier` + `MaterialApp(themeMode:)`；OmniView 按 `@theme` 信号识别——不再有 `mode` 魔法名）。接线代码生成由 Task 4.2 实现；`DesignIR { themes, theme_signal: "ThemeStore.mode" }` 契约已在本阶段落地。
- 已删除：`theme = "dark"` 语句（v1 文档虚构语法——编译器从未实现该语句）。遇此写法报错：「主题切换 = 设置 @theme 信号（ThemeStore.setMode("dark")）」。
- 语义验证（本阶段实现）：`@theme` 无 `@themes` 声明 → 错误；`@theme` 非 String 信号（含 `String?`）→ 错误「@theme 信号必须是 String」；`@theme` 多于一个 → 错误；`@themes` 重复主题名 → 错误；空 `@themes { }` 或同一 design 块内重复 `@themes` 声明 → 解析错误；`@theme` 信号的字面量初始值必须是 `@themes` 枚举之一 → 错误「@theme 初始值必须是 @themes 枚举之一：dark, light」（仅静态可知的字面量可验证；运行时赋值不检查——该边界为有意设计）。

### 4.6 平台映射

| Token 类型 | React 输出 | Flutter 输出 |
|-----------|-----------|-------------|
| `Color` | CSS Custom Property `--primaryColor` | `static const Color primaryColor = Color(0xFF6366F1)` |
| `Size` / `Radius` | CSS Custom Property | `static const double spacingMd = 16.0` |
| `FontFamily` / `FontSize` / `FontWeight` | CSS Custom Property | 对应 `TextStyle` 字段 |
| `Shadow` | CSS Custom Property | `static const BoxShadow shadowCard = ...` |
| `Duration` | CSS Custom Property | `static const Duration durationNormal = Duration(milliseconds: 300)` |
| `Easing` | CSS Custom Property | `static const Cubic easeOut = Cubic(0.16, 1, 0.3, 1)` |
| `ZIndex` | CSS Custom Property | ⚠️ **warning**: z-index 在 Flutter 中用 widget 顺序/Stack 控制 |

| 构造 | React 输出 | Flutter 输出 |
|------|-----------|-------------|
| Class | 全局 CSS class（`design.css`）| `BoxDecoration` 工厂方法 |
| Variant | CSS 组合 class（`Size-sm`）| `ThemeExtension<T>` |
| 动态 token | `useEffect` + `setProperty` | `Provider` + `ThemeData.copyWith` |
| `@themes` + `@theme` 信号（v2 D1） | 订阅 @theme store → `data-theme` 属性 | `ThemeModeNotifier` + `MaterialApp(themeMode:)` |

### 4.7 非 P0 目标 behavior

- P0 目标（React/Flutter）：必须完整实现
- 非 P0 目标（Python/JS/Dart/Rust/Dioxus/Vue）：**hard error**，不静默降级
- 错误信息说明"该目标不支持 design block，请使用 --target react|flutter|omniview"

---

## 5. Component Composition（Slots）

### 5.1 语义定义

Slot 的语义：**组件声明"这里可以由父组件填充内容"**，可以带默认内容（fallback），可以有名字。
v2（design D4）：slot 纯净为「父传内容」——默认（default）+ 命名（named）+ 回退（fallback）
三形态；**类型约束与 data 参数已删除**（`@slot name: Type` → 编译错误「类型约束与数据参数已删除；
slot = 父传内容」；遗留 `@slot(name)` 指令与 `slot="name"` 消费同样硬诊断）。

### 5.2 BNF 语法

```
// ── Slot 声明（在 component body 中）──

slot_decl ::= '@slot' slot_name slot_default?

slot_name ::= <ident>                             // 命名 slot
            | 'default'                           // 默认 slot

slot_default ::= '{' view_nodes '}'               // fallback 内容

// ── Slot 渲染（在 view 模板中）──

slot_render ::= '{' <ident> '}'                   // 渲染命名 slot: {header}
              | '{' 'children' '}'                // 渲染默认 slot: {children}

// ── Slot 消费（v2 BNF：仅命名 props 投影）──

slot_consume ::= ident '=' '{' view_tree '}'      // name={<.../>}
```

### 5.3 完整示例

```omni
component Modal {
  @slot header { <span>Default Header</span> }
  @slot default
  @slot footer { <span>Default Footer</span> }

  view {
    <div class="modal">
      <div class="modal-header">{header}</div>
      <div class="modal-body">{children}</div>
      <div class="modal-footer">{footer}</div>
    </div>
  }
}

// ── 消费侧（匿名内容 = 默认 slot；命名 slot 用属性投影）──
component Consumer {
  view {
    // 仅填默认 slot：
    <Modal>
      <p>这是内容</p>
    </Modal>

    // 填所有 slot：
    <Modal
      header={<h3>编辑</h3>}
      footer={<button>保存</button>}
    >
      <p>表单</p>
    </Modal>
  }
}
```

### 5.4 平台映射

| 概念 | React | Flutter | OmniView |
|------|-------|---------|----------|
| 默认 slot | `{children}` (props.children) | `Widget? child` | 直接 children |
| 命名 slot | `{header}` (命名 prop) | `Widget? header` | named slot |
| Fallback | `{header ?? <DefaultHeader />}` | `header ?? DefaultHeader()` | fallback |
| 消费侧 children | JSX children 自动透传 | 声明 `child:` / `children:` param | 自动透传 |

> v2（design D4）：删除「类型约束 + data」行——React 不再生成约束型 prop 类型
> （全部 slot prop 为 `ReactNode`），Flutter 不再有 `Widget?` 降级警告路径，
> OmniView 本就无类型消费。

### 5.5 IR 形态（2026-08-01：已实现；2026-08-03：D4 纯净化）

**历史 bug**：`ViewTree::Slot(Option<String>)` 是叶子节点，`default_content`
在 lowering 时丢弃——已修复（ADR-053 D5 实现时完成）。

当前形态（v2 D4：`type_constraint` / `data_param` 已删除）：
```rust
// crates/omnilang-core/src/ir/types.rs
ViewTree::Slot {
    name: Option<String>,              // None = 默认 slot
    fallback: Vec<ViewTree>,            // fallback 内容（原 default_content，lowering 保留）
}
```

消费者命名属性投影（`header={<div/>}`）在 lowering 时转为
`ViewTree::Slot { name: Some("header"), fallback: content }` 插入元素
children，与 view 内 `@slot name { ... }` 同等处理。

---

## 6. Stores & State（状态管理）

### 6.1 语义定义

Store 的语义：**定义一个响应式状态容器**。`signal` = 状态字段，`getter` = 派生状态，`action` = 状态变更。组件内 `signal` 是本地状态。编译器知道每个构造的语义，为每个目标生成该平台的状态管理惯用代码。

### 6.2 BNF 语法

```
// ── Store（顶层定义）──

store_def ::= 'store' <ident> '{' store_member* '}'

store_member ::= signal_decl
               | getter_decl
               | action_decl
               // 域 5 BNF v2: 删除 function_def 分支差异——顶层与内联 store
               // 成员集统一为 signal|getter|action；fn 成员报「用 getter 或 action」

// v2 域 5 (design D1, 2026-08-03): @theme 标记仅允许顶层 store 的 signal
// （组件/内联 store signal 带 @theme → 解析错误）；语义验证见 §4.5。
signal_decl  ::= 'signal' <ident> ':' type_annotation ('@theme')? '=' expr ';'?
getter_decl  ::= 'getter' <ident> ':' type_annotation '=' expr ';'?
action_decl  ::= 'action' <ident> '(' params? ')' block

// ── 组件内状态 ──

component_signal  ::= 'signal' <ident> ':' type_annotation '=' expr ';'?
component_computed ::= 'computed' <ident> ':' type_annotation '=' expr ';'?
                    | 'computed' <ident> '(' ')' ('->' type_annotation)? block
                    // 块形式仅空括号；返回类型可省略

// ── 组件引用 store ──

store_ref ::= '@store' <ident> ';'?

// ── 消费 store ──

store_access ::= <store_name> '.' <signal_name>        // 读 signal
               | <store_name> '.' <action_name> '(' args? ')'  // 调 action
               | <store_name> '.' <getter_name>         // 读 getter
```

### 6.3 完整示例

```omni
struct Todo {
  id: i32,
  text: String,
  status: String,
}

fn nextId() -> i32 {
  return 1;
}

store TodoStore {
  signal items: Todo[] = []
  signal filter: String = "all"

  getter filteredItems: Todo[] = items.filter(|item|
    filter == "all" || item.status == filter
  )

  getter activeCount: Int = items.count(|item| item.status == "active")

  action add(text: String) {
    items = [...items, Todo { id: nextId(), text: text, status: "active" }]
  }

  action toggle(id: Int) {
    items = items.map(|item|
      item.id == id 
        ? { ...item, status: item.status == "active" ? "completed" : "active" } 
        : item
    )
  }

  action clearCompleted() {
    items = items.filter(|item| item.status != "completed")
  }
}

// ── 组件中使用 ──

component TodoPage {
  @store TodoStore
  signal localDraft: String = ""

  fn handleAdd() {
    TodoStore.add(localDraft)
    localDraft = ""
  }

  view {
    <div>
      <input bind:value={localDraft} placeholder="输入待办事项..." />
      <button @onClick={handleAdd}>添加</button>

      <p>剩余 {TodoStore.activeCount} 项</p>

      @for item in TodoStore.filteredItems key={item.id} {
        <div @onClick={() => { TodoStore.toggle(item.id) }}>
          <span>{item.text}</span>
        </div>
      }
    </div>
  }
}
```

### 6.4 平台映射

| 构造 | React (Zustand v5) | Flutter (Riverpod 3.0) | OmniView |
|------|-------------------|------------------------|----------|
| `store` | `create<T>()()(...)` | `Notifier<T>` + `NotifierProvider` | `__omni_createStore` |
| `signal` | state 字段 | Notifier state 字段 + `copyWith` | inline signal |
| `getter` | 独立 selector hook：`useFilteredItems()` | Notifier getter：`get filteredItems => ...` | computed signal |
| `action` | 导出函数：`useAddTodo()` | Notifier 方法：`void add(...)` | action function |
| 读 signal | `useXxx(s => s.field)` | `ref.watch(provider.select(s => s.field))` | signal read |
| 调 action | `useXxx().add(text)` | `ref.read(provider.notifier).add(text)` | action call |
| 组件内 `signal` | `useState(init)` | `_field = init` + `setState` | inline signal |

### 6.5 Store access 规则

编译器区分以下场景，生成不同的代码：

| 表达式 | 语义 | React 生成 | Flutter 生成 |
|--------|------|-----------|-------------|
| `TodoStore.items` | 读 signal（订阅） | `useTodoStore(s => s.items)` | `ref.watch(todoStoreProvider.select(s => s.items))` |
| `TodoStore.items = ...` | 写 signal（action 内） | `set({ items: ... })` | `state = state.copyWith(items: ...)` |
| `TodoStore.add(x)` | 调 action | `const add = useTodoStore(s => s.add); add(x)` | `ref.read(todoStoreProvider.notifier).add(x)` |
| `TodoStore.filteredItems` | 读 getter（派生） | `useFilteredItems()` hook | `ref.watch(todoStoreProvider).filteredItems` |

**关键**：`items = [...]` 在 action 内编译器知道这是 signal 赋值（不是普通变量赋值）→ 生成对应的 setState/copyWith 代码。

### 6.6 保留与废弃

| 特性 | 状态 | 说明 |
|------|------|------|
| `computed`（组件内）| ✅ **保留** | 语义明确的派生值声明，React → `useMemo`，Flutter → getter |
| 组件内 inline store 定义 | ⚠️ **废弃警告** | 建议提取为顶层 store + `@store` 引用，或直接用组件内 `signal` |
| React Hooks 模式 | ❌ **删除** | Zustand v5 唯一方案 |

---

## 7. Control Flow（控制流）

### 7.1 语义定义

控制流的语义：**在视图中根据条件/列表动态决定渲染什么**。`@if` = 条件渲染，`@for` = 列表渲染。

### 7.2 BNF 语法

```
// ── 条件渲染 ──

if_directive ::= '@if' expr transition_mode? '{' view_nodes '}'
                 ( '@else' 'if' expr '{' view_nodes '}' )*
                 ( '@else' '{' view_nodes '}' )?

transition_mode ::= 'mode' ':' (<string_literal> | <ident>)   // "fade" | "wait" | "none"（解析期校验，非法值报错）

// ── 列表渲染 ──

for_directive ::= '@for' <ident> 'in' expr key_expr? scroll_hint? '{' view_nodes '}'
                  ( '@empty' '{' view_nodes '}' )?

key_expr ::= 'key' '=' expr                      // key={item.id}

scroll_hint ::= '[' 'scroll' ']'                  // Flutter: 用 ListView.builder
```

### 7.3 完整示例

```omni
// ── @if / @else ──
@if isLoading mode: "fade" {
  <Spinner />
} @else if error != null {
  <ErrorView message={error} />
} @else {
  <ContentView data={data} />
}

// ── @for ──
@for item in items key={item.id} {
  <TodoItem data={item} />
} @empty {
  <EmptyState message="还没有待办事项" />
}

// ── @for 长列表（Flutter 用 ListView.builder）──
@for item in allItems [scroll] key={item.id} {
  <ListItem data={item} />
}
```

### 7.4 平台映射

| 构造 | React | Flutter | OmniView |
|------|-------|---------|----------|
| `@if cond { ... }` | `{cond && (...)}` | `cond ? ... : SizedBox.shrink()` | JS conditional |
| `@if cond { A } @else { B }` | `{cond ? (A) : (B)}` | `cond ? A() : B()` | JS ternary |
| `@if cond mode:"fade" { ... }` | CSS transition | `AnimatedSwitcher(duration: ...)` | CSS transition |
| `@for item in list { ... }` | `{list.map(item => (...))}` | `Column(children: [...])` | DOM repeat |
| `@for item in list key={expr} { ... }` | `key={expr}` | `ValueKey(expr)` | data-key attr |
| `@for item in list [scroll] { ... }` | —（React 用 CSS overflow） | `ListView.builder(itemCount, itemBuilder)` | — |
| `@empty { ... }` | `{list.length === 0 ? (...) : (map)}` | `list.isEmpty ? Empty() : Column(...)` | JS conditional |

### 7.5 规则

- `@for` 在 Flutter 默认用 `Column`（安全，无限高问题由用户显式 `[scroll]` 控制）
- `@for` 嵌套在可滚动父元素内 → 编译器自动用 `Column`（不嵌套 ListView）
- 无 `key_expr` 且 item 是对象 → compiler warning，建议加 `key=`
- `[scroll]` 是位置固定的 hint，放在 `key={...}` 之后

---

## 8. Lifecycle（生命周期）

### 8.1 语义定义

生命周期的语义：**在组件生命周期的特定时刻执行代码**。

### 8.2 BNF 语法

```
lifecycle_hook ::= mount_hook
                 | unmount_hook
                 | effect_hook

mount_hook   ::= '@mount' '{' block_body '}'
unmount_hook ::= '@unmount' '{' block_body '}'
effect_hook  ::= '@effect' ('(' deps ')')? '{' block_body '}'

deps ::= <ident> (',' <ident>)*
```

### 8.3 完整示例

```omni
component Timer {
  signal elapsed: Float = 0
  signal isRunning: Bool = false

  fn saveToLocal(v: Float) {
    print(v.to_string());
  }

  @mount {
    isRunning = true
  }

  @unmount {
    isRunning = false
  }

  @effect(elapsed) {
    saveToLocal(elapsed)
  }

  view {
    <div>
      <span>已运行 {elapsed}s</span>
    </div>
  }
}
```

### 8.4 平台映射

| 生命周期 | React | Flutter | OmniView |
|---------|-------|---------|----------|
| `@mount { ... }` | `useEffect(() => { ... }, [])` | `initState() { ... }` | `__omni_mount(() => { ... })` |
| `@unmount { ... }` | `useEffect(() => { return () => { ... } }, [])` | `dispose() { ... }` | `__omni_unmount(() => { ... })` |
| `@effect(deps) { ... }` | `useEffect(() => { ... }, [deps])` | 生成 `_runEffect_N()` + `initState` 调用 + `didUpdateWidget` deps 比较重调 | `__omni_effect(() => { ... })` |

### 8.5 规则

- `@mount`/`@unmount` 每个组件最多各一个
- `@effect` 可以有多个，每个独立追踪 deps
- async 代码在 effect 内直接写 `await`，编译器包裹 async IIFE
- `@mount` 和 `@unmount` 可以共享闭包变量（如 timer handle）

---

## 9. Expression & Statement（通用计算层）

### 9.1 设计原则

通用计算层**不是语义原语**——这是普通代码逻辑。编译器不注入平台假设，只做必要的语法糖展开。当前语法基本完整（见 `14-grammar-lang.md`），本节记录需要修复的跨目标缺口。

### 9.2 需要修复的跨目标表达式

#### Optional Chain `?.`

```
React:   obj?.field              （原生 JS）
Flutter: obj?.field              （原生 Dart）

修复: ReactFormatExpr 和 DartFormatExpr override format_optional_chain，
      不使用 shared 的 {obj}.{field} 默认实现
```

#### Block Expression（IIFE）

```
React:   (() => { stmts; return tail; })()
Flutter: (() { stmts; return tail; })()

修复: override format_block_expr，emit IIFE 包裹
```

#### Match Expression

```
React:   (() => { switch(x) { case A: return a; default: return b; } })()
Flutter: (() { switch(x) { case A: return a; default: return b; } })()

修复: DartFormatExpr 需完整实现 match（当前 shared 实现被砍掉）
```

#### Spread, Compound Assignment, Await

```
React:   [...list], count += 1, await fn()
Flutter: [...list], count += 1, await fn()

三个目标均可直接翻译，语法糖展开即可
```

### 9.3 恢复的表达式支持

| 表达式 | 说明 |
|--------|------|
| `items.filter(\|x\| cond)` | 语言方法 → JS `filter((x) => cond)` / Dart `where((x) => cond).toList()` |
| `items.map(\|x\| expr)` | 语言方法 → JS `map((x) => expr)` / Dart `map((x) => expr).toList()` |
| `items.count(\|x\| cond)` | 展开为 `items.filter(\|x\| cond).length`（JS）；Dart `where((x) => cond).length` |
| `Array<T>` 类型 | `T[]` 语法糖 → `Array<T>` |
| 模板字符串 | `` `Hello ${name}` `` 保持不变 |

数组方法语法：`array_method ::= expr '.' ('filter' | 'map' | 'count') '(' closure ')'`。
谓词必须是管道闭包 `|x| ...`（函数引用也可）；`count` 无谓词报错「count 需要一个谓词闭包」（E_ARRAY_METHOD）。

---

## 10. 附录：完整语义属性映射表

### 10.1 Ref 语义绑定 → 平台输出

| .omni 语义属性 | React 输出 | Flutter 输出 | OmniView 输出 |
|---------------|-----------|-------------|--------------|
| `focus={sig}` | `useEffect(() => sig ? ref.focus() : ref.blur(), [sig])` | `FocusNode.requestFocus/unfocus` | `el.focus/blur` |
| `scroll-x={sig}` | `ref.scrollTo({left: sig})` | `ScrollController.animateTo(sig)` (horizontal) | `el.scrollTo(sig, 0)` |
| `scroll-y={sig}` | `ref.scrollTo({top: sig})` | `ScrollController.animateTo(sig)` | `el.scrollTo(0, sig)` |
| `bind:value={sig}`（双向，唯一形式） | `value={sig}` + `onChange` setter 写回 | `TextEditingController` + 守卫 listener 写回（`setState`） | guard + input listener + 守卫 effect（双向） |
| `value={expr}`（单向 prop） | 普通 `value={expr}` prop | 普通 widget 参数（无回写） | 普通属性 |
| ~~`value={sig}`（已删除，D2）~~ | — 编译错误「用 bind:value 双向绑定，或普通 value prop 单向」— | | |

### 10.2 Ref 语义方法 → 平台输出

| .omni 语义方法 | React | Flutter | OmniView |
|---------------|-------|---------|----------|
| `ref.select()` | `ref.current.select()` | `controller.selection = TextSelection(...)` | `el.select()` |
| `ref.setSelection(s,e)` | `ref.current.setSelectionRange(s,e)` | `controller.selection = TextSelection(...)` | `el.setSelectionRange(s,e)` |
| `ref.measure()` | `ref.current.getBoundingClientRect()` | `RenderBox.localToGlobal()` + `size` | `el.getBoundingClientRect()` |
| `ref.capturePointer()` | `ref.current.setPointerCapture(id)` | error「Flutter 不支持指针捕获，请用 @drag 指令代替」（D3 演进注记，ADR-071/073） | `el.setPointerCapture(id)` |
| `ref.releasePointer()` | `ref.current.releasePointerCapture(id)` | error「同上」 | `el.releasePointerCapture(id)` |
| `@drag={fn}` / `@dragEnd={fn}`（意图，ADR-073） | 合成 onPointerDown/Move/Up（setPointerCapture + 起点快照 → DragIntent `{x,y,dx,dy,pointerId,startX,startY}`） | GestureDetector onPanStart/Update/End（DragIntent 同形） | capturePointer 原语 + pointermove 直写（DragIntent 同形） |

### 10.3 Diagnostic 完整表

| 特性 | 目标 | 级别 | 说明 |
|------|------|------|------|
| `ref.value = x` / `ref.setValue(x)` / `ref = x` | 全部 | error | 命令式写已删除，用 bind:value 双向绑定 |
| `capturePointer(任何参数)` | 全部 | error | capturePointer 无参数；多指针锁定不支持 |
| `capturePointer` / `releasePointer` | Flutter | error | Flutter 不支持指针捕获，请用 @drag 指令代替（编译为 GestureDetector onPan 接线）——D3 演进为目标差异注记（ADR-071/073）：捕获是编译器内部细节，用户面向 @drag 意图 |
| `steps()` easing | Flutter | warning | Flutter 无原生 steps easing——回退 `Curves.linear` + 生成警告（2026-08-03 修正：v1 曾误记为 error） |
| ZIndex token | Flutter | warning | z-index 在 Flutter 中用 widget 顺序/Stack 控制 |
| CSS `display: grid` | Flutter | error | Flutter 不支持 CSS grid |
| CSS `float` | Flutter | error | Flutter widget 布局无 float 概念 |
| CSS `cursor` | Flutter | warning | cursor 在 Flutter 中无效果 |
| keyframe `width`/`height` | Flutter | warning | 动画布局属性触发重 layout，推荐用 scale |
| @for 无 key_expr（对象 item）| React | warning | 建议加 key= 避免重复 key |
| 非 P0 目标 + design block | 全部非 P0 | error | 该目标不支持 design block |

> **实现状态（2026-08-03，Task 4.4）**：ZIndex token、CSS `display: grid`、
> CSS `float`、CSS `cursor` 四行已在 Flutter design codegen 实现——ZIndex
> 不再生成无意义的 `static const int`（token 跳过 + warning，消息
> 「Flutter z 序由 widget 顺序决定」）；`display: grid` / `float` 走 codegen
> error 通道（`ECodegen`，锚定 design 块 span）；`cursor` 走 warnings 通道
> （「cursor 在 Flutter 中无效果」）。类（含伪状态）、变体 base 与分支统一
> 扫描。其余行见各自任务。

---

## 11. Accessibility Domain（a11y 域）

### 11.1 语义定义

「可访问性」让编译器知道：**这个控件的可访问名称/角色/可见性/实时区域**——
辅助技术如何理解这个界面（设计文档 域 10；评审 MEDIUM-2：以前只能写
aria-*（DOM 词汇），Flutter 目标静默丢失）。

语义原语集（v1 范围）：`label`（可访问名称）/ `role`（角色）/ `hidden`
（辅助技术隐藏）/ `live`（实时区域）/ `alt`（图像替代）。`required` 保持
表单语义属性（不进本域）。

### 11.2 BNF 语法

```
a11y_block    ::= 'a11y:' '{' a11y_entry+ '}'     (* 元素属性，与 style: {...} 同形态 *)
a11y_entry    ::= 'label' ':' string               (* 至少一项；空块 = 编译错误「a11y 块不能为空」*)
                | 'role' ':' string
                | 'hidden' ':' expr                (* 动态隐藏 *)
                | 'live' ':' '"polite"' | '"assertive"'
                | 'alt' ':' expr                   (* 图像替代文本 *)
```

语法规则：

- **至少一项**：空 `a11y: { }` 块是硬解析错误「a11y 块不能为空」——
  空块不携带任何意图（设计文档 域 10 验证清单）。
- **`label`/`role`/`live` 是字符串字面量**（BNF `'string'`）：`label: {expr}`
  形式（动态可访问名称）暂不支持，解析错误。
- **`live` 值域**：`"polite"`/`"assertive"`（语义 pass 验证，其余值 →
  error「a11y live 仅支持 "polite"/"assertive"」）。
- **`role` 子集**：Flutter 可映射角色
  `button`/`textbox`/`link`/`checkbox`/`radio`/`img` → Semantics flags
  （`textbox`→`textField: true`、`img`→`image: true` 内部映射；遗留旗标名
  `textField`/`image` → warning + 建议规范名 textbox/img）；
  其余角色 → **warning**（语义 pass）+ Flutter 生成代码注释（诚实降级）；
  非 Flutter 目标原样透传（诊断表记录子集）。
- **`hidden`/`alt` 是任意表达式**（动态）：`hidden: !visible`、
  `alt: iconName` 均合法。
- **未知键** → 解析错误「未知 a11y 键：'…'（支持 label/role/hidden/live/alt）」。

### 11.3 完整示例

```
<button a11y: { label: "Close", role: "button", hidden: !visible }>×</button>
<input a11y: { label: "Search", role: "searchbox" } />
<div a11y: { live: "polite" }>{status}</div>
<img src="icon.png" a11y: { alt: "Settings icon" } />
```

### 11.4 平台映射

| 语义 | React | Flutter | OmniView |
|------|-------|---------|----------|
| `label` | `aria-label="..."`（静态字符串） | `Semantics(label: '...', child: ...)` | `aria-label="..."` 透传 |
| `role` | `role="..."` | 语义角色子集 → `Semantics(button: true)` 等 flags；不可映射 → warning + 生成注释 | `role="..."` 透传 |
| `hidden` | `aria-hidden={expr}`（动态） | `ExcludeSemantics(excluding: expr, child: ...)` | `aria-hidden` 透传（动态 → `__omni_effect` setAttribute） |
| `live` | `aria-live="polite"` | `Semantics(liveRegion: true)` | `aria-live="polite"` 透传 |
| `alt` | `alt="..."` / `alt={expr}` | img → `Image.network(semanticLabel:)`；其余 → `Semantics(label:)`（与 `label` 同时出现时 `label` 优先 + warning，见 §11.6） | `alt` 透传（动态同 hidden） |

**Flutter role 子集**：`button`/`textbox`/`link`/`checkbox`/`radio`/`img`
→ 对应 `Semantics` flag（`textbox`/`img` 内部映射回 `textField: true`/`image: true`）；
子集外角色 → warnings 通道
（`[flutter] a11y role 'X' is not in the Flutter Semantics subset …`）+
生成代码内 `/* a11y: role 'X' has no Flutter Semantics flag … */` 注释。

**组件标签上的 a11y 块**：React 以 aria-* prop 透传给子组件；Flutter 以
Semantics/ExcludeSemantics 包裹子组件 widget；OmniView 内联组件无包裹
机制 → warning「a11y 意图被忽略」。

### 11.5 完整示例产物

```dart
// Flutter（三目标语义等价 — 语义树断言）
ExcludeSemantics(
  excluding: !visible,
  child: Semantics(label: 'Close', button: true,
    child: ElevatedButton(...)),
)
```

```tsx
// React
<button aria-label="Close" role="button" aria-hidden={!visible}>×</button>
```

### 11.6 Diagnostic

| 特性 | 目标 | 级别 | 说明 |
|------|------|------|------|
| 空 `a11y: { }` 块 | 全部 | error | 「a11y 块不能为空」（解析期） |
| `label`/`role`/`live` 非字符串字面量 | 全部 | error | BNF `'string'` 值域 |
| `live` 值非 polite/assertive | 全部 | error | 「a11y live 仅支持 "polite"/"assertive"」（语义 pass） |
| `role` 不在 Flutter 子集 | 全部 | warning | Flutter 降级 + 生成注释；其余目标透传 |
| 未知 a11y 键 | 全部 | error | 「未知 a11y 键：'…'」 |
| a11y 块用于组件标签 | OmniView | warning | 内联组件无 aria-* 包裹机制 |
| `role` 不在 Flutter 子集 | Flutter | warning | T2.4 warnings 通道 + `/* a11y: … */` 生成注释 |
| `label` + `alt` 同时出现（非 img 元素） | Flutter | warning | 「alt 仅对 img 元素生效」— `label` 优先（即可访问名称），`alt` 忽略（两者都会映射为 `label:`，重复命名参数 = 非法 Dart） |

> **实现状态（2026-08-03，Task 4.5）**：parser（`ElementProp::A11y`/
> `A11ySpec`）、IR（`ElementIR.a11y: Option<A11yIR>`）、语义
> （`semantic/validate_a11y.rs`：role 子集 + live 值域）、三后端
> （React aria-*；Flutter `Semantics`/`ExcludeSemantics`/`semanticLabel`/
> `liveRegion`；OmniView aria-* 透传）已实现；fixture `a11y.omni` 三目标
> 生成形状断言 + Flutter 语义树节点断言（`Semantics(label:)`/
> `ExcludeSemantics` 存在性）；demo FormPage 8 处 `aria-label` 迁移到
> a11y 块并在三目标验证（OmniView 属性不变、React 不再产生占位符重复
> aria-label、Flutter 新出 Semantics 节点）。

---

## 12. 权威关系（2026-08-03 更新）

本 v1 规范已 **Superseded**——BNF 与行为事实的唯一权威是
`docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`
（v2 设计文档，状态 Implemented：逐域 BNF + 平台映射 + 诊断表 + 验证清单）。

v1 初版本节曾以对比表声明「设计文档引用本规范的 BNF 作为语法权威」——该关系
已不成立（v2 起设计文档自带逐域 BNF，不再引用本规范），本节改写为权威关系声明。

---

## 13. 实现影响

本语法规范要求以下编译器变更：

| 变更 | 层 | 说明 |
|------|-----|------|
| `semantic/validate_ref_ops.rs` | 新增 | ✅ 已实现：ref 上的合法语义方法（focus/blur/scrollTo/scrollBy/select/setSelection/measure/capturePointer/releasePointer）→ RefOpIR；非法操作（getBoundingClientRect、offsetWidth 等）→ compile error + 替代建议 |
| `ir/types.rs` — `RefOpIR` 枚举 | 新增 | ✅ 已实现（10 ops）：Focus, Blur, ScrollTo, ScrollBy, Measure, Select, SetSelection, Value, CapturePointer, ReleasePointer |
| `ir/types.rs` — `ViewTree::Slot` | 修改 | ✅ 已实现：`Slot { name, fallback }`（v2 D4 删除 type_constraint/data_param，见 §5.5） |
| `ir/lower/view.rs` — slot lowering | 修复 | ✅ 已实现：`default_content` 保留并填入 `fallback`；消费者命名属性投影同样转 `ViewTree::Slot` |
| `ir/lower/lower_inline_motions.rs` | 新增 | ✅ 已实现：map 形式内联动画体 lowering → 匿名命名 motion（§3.7；2026-08-03 修正：v1 曾误标未实现） |
| `ir/types_motion.rs` | 修改 | 删除 `PlaybackIR.paused`、`MotionBehavior.paused_override`；新增 `TriggerKind::Focus` |
| `codegen/mod.rs` | 修改 | P0 目标移除 design block gate；非 P0 目标 warning → error |
| `codegen/react/` — 全部模块 | 重写 | Zustand v5、React 19 ref prop、slot children、design CSS、动画 → 平台原生 ref+WAAPI |
| `codegen/flutter/` — 全部模块 | 重写 | Riverpod 3.0、Ref handle、TweenSequence、design tokens、slot |
| `codegen/shared/expr.rs` | 修改 | OptionalChain/BlockExpr/Match override |
| `parser/ast/view.rs` + `parser/view/element.rs` | 修改 | ✅ 已实现（Task 4.5）：`ElementProp::A11y(A11ySpec)` + `a11y: { ... }` 块解析（§11.2） |
| `ir/types.rs` — `A11yIR` + `ElementIR.a11y` | 新增 | ✅ 已实现（Task 4.5）：`A11yIR { label, role, hidden, live, alt }` |
| `semantic/validate_a11y.rs` | 新增 | ✅ 已实现（Task 4.5）：role 子集 warning + live 值域 error |
| `codegen/react/view.rs` | 修改 | ✅ 已实现（Task 4.5）：a11y → aria-label/role/aria-hidden/aria-live/alt |
| `codegen/flutter/view.rs` | 修改 | ✅ 已实现（Task 4.5）：Semantics(label:)/flags/liveRegion + ExcludeSemantics + img semanticLabel；不可映射 role → warning + 生成注释 |
| `codegen/omniview/view/element.rs` | 修改 | ✅ 已实现（Task 4.5）：aria-* 静态 + 动态透传；组件标签 → warning |

---

## 14. References

- [ADR-052 — Semantic-Level DSL Cross-Target Codegen](.architecture/decisions/adrs/ADR-052-semantic-level-dsl-cross-target-codegen.md)
- [Semantic-Level DSL Design Document](docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md)
- [Cross-Target Feature Gap Review](.architecture/reviews/cross-target-feature-gap-review-2026-07-31.md)
- [OmniLang Grammar — Core Language](docs/omniview/14-grammar-lang.md)
- [OmniLang Grammar — OmniView UI](docs/omniview/15-grammar-view.md)
