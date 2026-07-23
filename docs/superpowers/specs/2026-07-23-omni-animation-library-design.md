# Omni Animation Library — Design Spec

**Date:** 2026-07-23
**Status:** Design Complete
**Author:** OmniLang Team

---

## 1. 问题陈述

### 1.1 当前架构：两条独立的动画管线

OmniLang 编译器目前有两条完全独立的动画 codegen 路径：

| 能力 | OmniView codegen | React codegen |
|------|-----------------|---------------|
| **运行时交付** | 嵌入 ~1434 行 `motion-runtime.js` IIFE | 内联 `el.animate()` + `cubic-bezier` 近似 |
| 缓动函数 | SpringSolver + 30+ 缓动 | 全部转为 `cubic-bezier()` |
| Stagger | ✅ `_playStaggered()` | ❌ |
| Timeline 编排 | ✅ setTimeout 链 | ❌ 退化为 opacity-only |
| FLIP (列表重排) | ✅ `__omni_flip_before/after` | ❌ |
| Hero (跨组件过渡) | ✅ `__omni_hero_before/after` | ❌ |
| 中断处理 | ✅ compose/cancel/finish/replace | ❌ |
| Spring 物理 | ✅ `SpringSolver` 精确求解 | ❌ `cubic-bezier` 粗略近似 |
| 输出体积 | 46KB (最小化) | ~200B（但是功能极度不完整） |

### 1.2 根本矛盾

两者都运行在浏览器里，用的是同一个 Web Animations API（WAAPI）。本应共用同一份运行时，却成了两套实现。React 端功能严重缺失（spring 变直线、没有 stagger、没有 FLIP），OmniView 端把整个运行时塞进每次编译输出，浪费体积。

**Flutter 不是这个问题**。Flutter 用自己的 `AnimationController` + `Tween` + `Curves` 体系，不需要也不能共用一个 JS 运行时。Flutter codegen 架构不变。

### 1.3 目标

1. 抽取一个独立 npm 包 `@omni/animation`（~350 行 TS），同时服务于 OmniView 和 React 编译输出
2. 删除嵌入的 `motion-runtime.js`（1434 行），改用库引用
3. 修复 React codegen 中 `@hover`/`@press`/`@focus` 硬编码默认动画的 Bug
4. **不改变 OmniLang 语法、Parser、Semantic Analysis、IR**——这是纯 codegen + 运行时层的改动

---

## 2. 核心设计决策

### 2.1 不做跨平台动画运行时

OmniView、React、Vue 都跑在浏览器里，共用 WAAPI。Flutter 是独立平台，用原生 `AnimationController`。所以：

```
.omni 源码
    │
    ▼
编译器 IR (KeyframeIR / EasingIR / PlaybackIR)
    │
    ├─→ Web 目标 (OmniView / React / Vue)
    │       │
    │       └─→ 共用一个 @omni/animation 库
    │           (WAAPI 薄封装 + 纯函数工具集, ~350 行)
    │
    └─→ Flutter 目标
            │
            └─→ codegen 直接输出 AnimationController + Tween + Curves
                不走运行时库，架构不变
```

共享的是 **IR**（编译器中立的数据结构），不是运行时。

### 2.2 库的定位：WAAPI 薄封装 + 纯函数工具

```typescript
// 库的边界清晰：
// ✅ 做：WAAPI 调用、easing 转换、keyframe 格式化、stagger 延时计算、FLIP delta
// ❌ 不做：DOM 事件绑定、IntersectionObserver、scroll listener、pointer events、生命周期管理
```

DOM 事件绑定属于平台胶水层，保留在 codegen 中生成。codegen 生成事件绑定 + 调用库函数执行动画。

### 2.3 API 风格：Motion v2

库暴露简洁的函数式 API，返回原生 `Animation` 对象。调用方可以直接 `.pause()` `.cancel()` `.finished.then()`。

```typescript
import { animate, timeline, buildKeyframes } from "@omni/animation"

const anim = animate(el, keyframes, { duration: 400, easing: "ease-out", fill: "forwards" })
anim.pause()
anim.play()
await anim.finished
```

---

## 3. 库 API 设计

### 3.1 核心函数

```typescript
// ═══ 驱动一次动画 ═══
function animate(
  el: Element,
  keyframes: KeyframeConfig[],
  options: MotionOptions
): Animation
// 返回原生 WAAPI Animation 对象。内部：
//   1. 调用 buildKeyframes() 将 Omni 属性名 → CSS 属性名 + transform/filter 合并
//   2. 调用 resolveEasing() 将 Omni easing → WAAPI easing 字符串
//   3. 调用 el.animate(keyframes, timing)
//   4. 返回 Animation（不包装，透明）

// ═══ 编排多元素动画 ═══
function timeline(steps: TimelineStep[]): Animation[]
// 按 steps[].at 指定的时间偏移并发调用 animate()
// 返回 Animation[]，调用方可统一 pause/cancel

// ═══ 纯函数工具（也支持单独 import）═══
function buildKeyframes(stops: KeyframeStop[]): Keyframe[]
// Omni 属性名 → CSS 属性名:
//   x → translateX(…)px,  rotate → rotate(…)deg,  scale → scale(…)
//   blur → blur(…)px,      opacity → opacity
// transform/filter 自动合并为单个字符串

function resolveEasing(easing: EasingConfig): string
// 输出 WAAPI 兼容字符串:
//   "easeOut"  → "ease-out"
//   { type: "cubicBezier", x1, y1, x2, y2 } → "cubic-bezier(…)"
//   { type: "spring", tension: 300, damping: 20 } → "cubic-bezier(0.25,0.46,0.45,0.94)"
//   { type: "back", intensity: 1.2 } → "cubic-bezier(0.175,1.064,0.320,2.4)"
//   { type: "steps", count: 5, direction: "end" } → "steps(5,end)"

function staggerDelays(
  count: number,
  each: number,
  from: "start" | "center" | "end" | "edges"
): number[]
// 返回延迟数组 [0, 50, 100, …]

function scrollProgress(el: Element, startCfg: string, endCfg: string): number
// 解析 "top 80%" / "center 50%" 等触发配置，返回当前 scroll 进度 0..1

// ═══ FLIP & Hero ═══
function flipBefore(containerSelector: string): void
function flipAfter(containerSelector: string, duration?: number, easing?: string): void

function heroBefore(): void
function heroAfter(): void
```

### 3.2 类型定义

```typescript
type MotionOptions = {
  duration: number
  delay?: number
  easing?: EasingConfig
  fill?: "none" | "forwards" | "backwards" | "both"
  direction?: "normal" | "reverse" | "alternate" | "alternate-reverse"
  iterations?: number | "Infinity"
}

type EasingConfig =
  | string  // "easeOut", "linear", "power2In"...
  | { type: "cubicBezier"; x1: number; y1: number; x2: number; y2: number }
  | { type: "spring"; tension: number; friction: number; velocity?: number }
  | { type: "back"; intensity: number; direction?: string }
  | { type: "elastic"; amplitude: number; period: number; direction?: string }
  | { type: "steps"; count: number; direction: string }

type KeyframeConfig = {
  offset: number    // 0..1
  properties: Record<string, number | string>  // Omni 属性名
  easing?: EasingConfig  // 逐 stop easing（WAAPI 原生支持）
}

type TimelineStep = {
  el: Element
  keyframes: KeyframeConfig[]
  options: MotionOptions
  at: number | string  // 绝对时间(ms) 或 "-=0.1" / "+=0.2"
}
```

### 3.3 边界之外（不做的）

以下功能保留在 codegen 中生成，库不处理：

- **DOM 事件绑定** (`mouseenter`/`mouseleave`/`focus`/`blur`/`pointerdown`/etc.)
- **IntersectionObserver** (`@in-view`)
- **scroll event listener** (`@scroll`)
- **pointer capture 拖拽** (`@drag-horizontal`/`@drag-vertical`)
- **信号监听** (`@driven-by`、`@paused`)
- **@enter 重放** (信号变化时重播入场动画)
- **组件生命周期管理** (动画注册/注销)

---

## 4. Codegen 对接

### 4.1 OmniView codegen

**改前架构（现状）：**
```
emit_animation_runtime() → 嵌入 motion-runtime.js (46KB minified)
emit_motion_js()         → const __omni_motion_NAME = function(el, overrides) {
                              return new AnimationPlayer(el, {…})
                            }
emit_motion_triggers()   → new AnimationPlayer(el, config) + __omni_animations.set()
emit_timeline_js()       → setTimeout 链 + new AnimationPlayer()
emit_states_js()         → el.animate([from, to], {…})  直接 WAAPI
```

**改后架构：**
```
运行时     → <script src="./omni-animation.js"> 独立文件引用（可浏览器缓存）
           或者 <script> 内联（单文件输出模式），由 codegen 选项控制

motion.js  → const __omni_motion_NAME = function(el, overrides) {
              return Omni.animate(el, buildKeyframes([…]), { duration: …, easing: … })
            }

triggers   → const anim = __omni_motion_NAME(el, { trigger: "enter" })
             // 不再用 new AnimationPlayer，不再手动 __omni_animations.set()
             // 但 registry (nid → Animation) 还是需要的，用于 @driven-by / @paused 索引

timeline   → Omni.timeline(steps)  替换 setTimeout 链

states     → Omni.animate(el, keyframes, options)  替换 el.animate()
```

**全局注册表 `__omni_animations` 保留：**

`__omni_animations` Map (`nid_trigger → Animation`) 是为 `@driven-by`、`@paused`、`@enter` 重放服务的——这些是 codegen 逻辑，不属于库。所以注册表保留，但值从 `AnimationPlayer` 改为原生 `Animation`。

```javascript
// 改后：
const anim = Omni.animate(el, kf, opts)
__omni_animations.set("nid_enter", anim)  // 存原生 Animation，不是 AnimationPlayer
```

### 4.2 React codegen

**改前：**
```tsx
// @enter: _M 常量 + useEffect + el.animate(kf, opt)
const _M = {
  fadeInUp: [[{opacity:0,offset:0},{opacity:1,offset:1}], {duration:400,easing:"ease-out",fill:"forwards"}]
}

// @hover/@press/@focus: 硬编码！永远用默认动画，不走 _M
onMouseEnter={(e) => { e.currentTarget.animate(
  [{ opacity: 0 }, { opacity: 1 }], { duration: 300, easing: 'ease-out' }
)}}
```

**改后：**
```tsx
import { animate } from "@omni/animation"

// _M 映射改为存 keyframes + options（结构更清晰）
const _M = {
  fadeInUp: {
    kf: buildKeyframes([{offset:0,props:{opacity:0,y:20}}, {offset:1,props:{opacity:1,y:0}}]),
    opts: {duration: 400, easing: "ease-out", fill: "forwards"}
  }
}

// @enter 不变，只是改为调 animate()
useEffect(() => {
  const els = document.querySelectorAll('[data-motion-enter]')
  els.forEach(el => {
    const name = el.getAttribute('data-motion-name')
    if (name && _M[name]) animate(el, _M[name].kf, _M[name].opts)
  })
}, []

// @hover/@press/@focus: 修复！走 _M[name] 查找，不再硬编码
onMouseEnter={(e) => {
  const m = _M_trigger["nid_hover"]
  if (m) { const a = animate(e.currentTarget, m.kf, m.opts); _anim.set('nid_hover', a) }
}}
onMouseLeave={(e) => { _anim.get('nid_hover')?.reverse() }}
```

### 4.3 Flutter codegen

**不改。** Flutter 有自己完整的 `AnimatedBuilder` + `Tween` + `Curves` 体系。继续把 IR 编译为 Flutter 原生 Widget。共享点是 IR 数据结构，不是运行时。

---

## 5. 改动清单

### 5.1 新增文件

| 文件 | 行数 | 说明 |
|------|------|------|
| `packages/omni-animation/package.json` | ~15 | npm 包配置 |
| `packages/omni-animation/tsconfig.json` | ~15 | TS 配置 |
| `packages/omni-animation/src/index.ts` | ~10 | 统一导出 |
| `packages/omni-animation/src/animate.ts` | ~60 | `animate()` + `el.animate()` 封装 |
| `packages/omni-animation/src/timeline.ts` | ~40 | `timeline()` 编排 |
| `packages/omni-animation/src/keyframes.ts` | ~80 | `buildKeyframes()` 属性映射 |
| `packages/omni-animation/src/easing.ts` | ~90 | `resolveEasing()` 缓动转换 |
| `packages/omni-animation/src/stagger.ts` | ~15 | `staggerDelays()` |
| `packages/omni-animation/src/scroll.ts` | ~20 | `scrollProgress()` |
| `packages/omni-animation/src/flip.ts` | ~30 | `flipBefore()` / `flipAfter()` |
| `packages/omni-animation/src/hero.ts` | ~30 | `heroBefore()` / `heroAfter()` |
| `packages/omni-animation/src/types.ts` | ~50 | TypeScript 类型定义 |
| **合计** | **~450** | |

### 5.2 删除文件

| 文件 | 行数 | 原因 |
|------|------|------|
| `omnilang/src/runtime/motion-runtime.js` | 1434 | 功能迁移到 `@omni/animation` |
| `build/omni-motion-runtime.js` | 1 (minified) | 不再需要预构建 |
| **合计** | **~1435** | |

### 5.3 修改文件

#### OmniView codegen（Rust）

| 文件 | 大约改动 | 内容 |
|------|---------|------|
| `omniview/runtime.rs` | 改 `emit_animation_runtime()` | 不再 `include_str!("motion-runtime.js")`，改为输出 `<script src="omni-animation.js">` 或内联库文件 |
| `omniview/motion.rs` | 改 `emit_motion_js()` | `new AnimationPlayer(el, config)` → `Omni.animate(el, kf, opts)` |
| `omniview/motion.rs` | 改 `emit_states_js()` | `el.animate()` → `Omni.animate()` |
| `omniview/motion_config.rs` | 重写 | 配置序列化目标从 "AnimationPlayer config 字符串" 改为 "库 API 配置对象" |
| `omniview/motion_triggers.rs` | 改 40+ 处 | `new AnimationPlayer(el, config)` → `Omni.animate(el, kf, opts)` |
| `omniview/motion_triggers.rs` | 改注册表操作 | `__omni_animations.set()` 的值从 `AnimationPlayer` 改为原生 `Animation`（不影响 set/get/forEach 调用格式） |
| `omniview/timeline.rs` | 重写 `emit_timeline_js()` | setTimeout 链 → `Omni.timeline(steps)` |
| `omniview/html.rs` | 改 3 处 inline animation | 事件 handler 中的 `new AnimationPlayer()` → `Omni.animate()` |
| `omniview/component_html.rs` | 改 1 处 inline animation | 同 html.rs |
| `omniview/editor.rs` | 改 2 处 | `__omni_animations.get()` 的返回值处理（Animation vs AnimationPlayer） |
| `omniview/store.rs` | 改 3 处 | 注册表值类型 + .kill() 改为 .cancel() |
| `omniview/list_render.rs` | 改 2 处 | `__omni_flip_before/after` 改为库导出 |

#### React codegen（Rust）

| 文件 | 大约改动 | 内容 |
|------|---------|------|
| `react/motion/mod.rs` | 重写 `emit_motion_effects_tsx()` | `_M` 映射格式从 `[kf_str, opts_str]` 元组改为 `{kf, opts}` 对象，调用 `animate()` 替代 `el.animate()` |
| `react/motion/keyframes.rs` | 重写 `format_motion_to_waapi()` | 不再手工拼 JS 字符串 `[{opacity:0,offset:0}…]`，输出结构化 `KeyframeConfig[]` |
| `react/motion/transforms.rs` | 改 3 处 | `el.animate()` → `animate()`（@in-view, @scroll, @drag 的 useEffect） |
| `react/view.rs` | 改 6 处 | **修复 Bug**：`@hover`/`@press`/`@focus`/`@unhover` 的硬编码默认动画改为走 `_M` 查找表 |
| `react/component.rs` | 改 1 处 | 生成 `import { animate } from "@omni/animation"` 语句 |

### 5.4 不变文件

| 分类 | 文件 | 原因 |
|------|------|------|
| **Parser** | `parser/top_level/motion.rs`, `timeline.rs`, `design.rs`, `parser/expr/motion.rs`, `parser/ast/motion.rs`, `parser/view/directives.rs`, `element.rs`, `style.rs` | OmniLang 语法不变 |
| **Lexer** | `lexer/token.rs`, `keywords.rs` | Token 不变 |
| **Semantic** | `semantic/motion.rs`, `validate_motion.rs`, `validate_motion_triggers.rs`, `validate_types.rs`, `validate_store_access.rs` | 验证逻辑不变 |
| **IR** | `ir/types_motion.rs`, `ir/lower/motion.rs`, `ir/lower/view.rs`, `ir/lower/component.rs`, `ir/logic.rs`, `ir/types.rs` | IR 不变——这是核心共享层 |
| **Flutter** | `flutter/motion.rs`, `flutter/motion_widgets.rs`, `flutter/view.rs`, `flutter/mod.rs` | Flutter 架构不动 |
| **其他 codegen** | `dioxus/**`, `vue/**`, `javascript/**`, `rust/**`, `dart/**` | 不动 |

---

## 6. 迁移策略

### 6.1 分步实施

1. **Phase 1**：新建 `packages/omni-animation/` npm 包，实现全部核心函数
2. **Phase 2**：修改 OmniView codegen，改 `emit_motion_js()` + `emit_motion_triggers_for_element()`，删 `motion-runtime.js`。通过现有 OmniView 测试套件验证
3. **Phase 3**：修改 OmniView codegen 剩余文件（timeline, list_render, store, html, editor）
4. **Phase 4**：修改 React codegen，引入 `animate()` 替代 `el.animate()`，修复 `@hover`/`@press`/`@focus` 硬编码 Bug。通过 React Playwright 测试套件验证
5. **Phase 5**：清理 `motion-runtime.js` 和 `build/omni-motion-runtime.js`

### 6.2 回归测试

- OmniView 编译：所有 15 个 `*Demo.omni` 文件编译后功能不变
- React 编译：现有的 18 个 Playwright 测试 + 动画相关测试
- `omnic` CLI 编译流程不变

### 6.3 回退策略

在整个迁移过程中，`motion-runtime.js` 可以选择先保留。如果某个 codegen 改动引入问题，可以快速回退到 `new AnimationPlayer()` 路径。两个运行时可以短暂共存。

---

## 7. 规模估算

| 指标 | 值 |
|------|-----|
| 新增 TypeScript | ~450 行 |
| 删除 JavaScript (motion-runtime.js) | ~1435 行 |
| 修改 Rust (codegen) | ~300 行（分布在 ~15 个文件） |
| **净代码变化** | **~ -685 行** |
| OmniView 编译输出体积减少 | ~46KB（改用独立文件引用）/ 与原来相当（内联模式） |
| React 编译输出功能增加 | spring/stagger/timeline/FLIP/hero 从无到有 |

---

## 8. 设计参考

- 当前 motion-runtime.js: `omnilang/src/runtime/motion-runtime.js` (1434 行)
- 当前 motion 设计文档: `docs/superpowers/specs/2026-06-20-omniview-motion-animation-design.md`
- Motion v2 参考: https://motion.dev
- WAAPI 规范: https://drafts.csswg.org/web-animations/
