# OmniView / OmniLang 前端框架成熟度分析

> 评估日期: 2026-06-20 | 测试: 494 passing | 目标: Web + Flutter 双端

## 总体评价

OmniLang 编译器已经达到了 **v0.7 左右的成熟度**——核心功能完备，但仍缺少若干对生产级前端框架至关重要的特性。以下是按严重程度排序的差距分析。

---

## 🔴 P0: 致命缺失（阻碍生产使用）

### 1. 无路由系统 (Router)
- **现状**: 完全没有客户端路由概念。编译出的 Web 应用是单页面无路由。
- **Web 端影响**: 无法构建多页面应用。无法做 SPA 导航、URL 参数、路由守卫。
- **Flutter 端影响**: 没有 Navigator 2.0 / GoRouter 集成。
- **建议**: 需要 `route` 关键字或 `@route` 指令 + `Router` 组件。

### 2. 无组件内容投影 (Content Projection / Slots)
- **现状**: `@slot` token 存在且被 parser 识别为 `SlotDirective`，IR 也有 `Slot` variant，但只是占位符——没有将父组件的 children 注入到子组件 slot 位置的实际机制。
- **Web 端影响**: 无法做 Layout 组件包裹子内容（如 `<Layout><Sidebar/> <Content/></Layout>`）。
- **Flutter 端影响**: 没有 `child`/`children` 的 Widget 组合模式。
- **建议**: 这是组件组合的基础能力，必须实现 slot projection 机制。

### 3. 无 Provide/Inject（跨层级数据传递）
- **现状**: 数据只能通过 props 逐层传递或全局 Store。没有上下文注入机制。
- **影响**: 深层组件树会出现严重的 prop drilling 问题。
- **建议**: 参照 Vue 的 provide/inject 或 React 的 Context API。

### 4. 响应式系统的关键缺陷
- **a. computed 无自动依赖追踪**: `ComputedDecl` 降低到 IR 时 `deps: vec![]` 硬编码——依赖图是空的。
- **b. 无 signal 等值性检查**: signal 设置相同值时不会跳过更新。
- **c. 无批量更新 (batching)**: 多个 signal 更新会产生多次重渲染。
- **d. 无 `watch` API**: 只有 Effect 生命周期，没有声明式 watcher。
- **Web + Flutter 双端影响**: 这些都是现代响应式框架的标配，缺失会导致性能问题和开发体验差。

---

## 🟠 P1: 严重缺失（大幅限制框架能力）

### 5. CSS/Style 系统严重不足
- **a. 无伪类**: `:hover`, `:focus`, `:active`, `:disabled` 等完全缺失。
- **b. 无伪元素**: `::before`, `::after`。
- **c. 无 Media Queries**: 无法做响应式设计。
- **d. 无 Keyframe 动画**: 无法定义 CSS 动画。
- **e. 无 Scoped CSS**: 类名全局污染，组件间样式冲突。
- **f. Web→Flutter 样式映射缺失**: style block 属性只在 Web 端有 CSS 输出，Flutter 端完全没有从 style block 生成对应的 `BoxDecoration`/`TextStyle` 等。
- **建议**: 至少需要伪类支持 + scoped CSS + 基础 animation。Flutter 端需要一个 style→widget-decoration 的映射层。

### 6. 表单处理不完整
- **a. 无内置表单验证**: 没有 `required`, `minLength`, `pattern` 等验证规则。
- **b. 无 bind:group**: 不能做 radio/checkbox 群组绑定。
- **c. 绑定类型不区分**: 所有绑定都是 `bind:value`，没有 `bind:checked`(checkbox)、`bind:selected`(select) 等类型化绑定。
- **d. 无表单提交封装**: 没有 `<form>` 的 `onSubmit` 包装逻辑。
- **影响**: 表单是最常见的 UI 模式，当前需要大量手动样板代码。

### 7. 组件组合能力弱
- **a. 无动态组件**: 不能 `<component is={expr}>` 动态切换渲染的组件。
- **b. 无 `<template>` / `<fragment>` 显式语法**: 多根元素自动包裹在 fragment 中，但用户无法显式控制。
- **c. 无 `@html` / innerHTML**: 无法渲染富文本 HTML 字符串（CMS 场景必需）。
- **d. 无 `@switch` / `@case` 指令**: 只能用嵌套 @if，代码冗长。

### 8. Flutter 端特殊问题
- **a. HTML→Flutter widget 映射不完整**: 大量 HTML tag 在 Flutter 端没有对应 widget。
- **b. CSS Layout→Flutter layout 无映射**: `display: flex`, `flexDirection` 等 CSS 属性不会变成 Flutter 的 `Row`/`Column`/`Flexible`。
- **c. 事件映射不完整**: Web 的 `onClick`/`onInput` 到 Flutter 的 `onTap`/`onChanged` 的映射不一致。
- **d. 无平台条件编译**: `@native` token 被 lexer 识别但 parser 未使用。无法写 `@native(web) { ... } @native(flutter) { ... }` 的平台特定代码。

---

## 🟡 P2: 中等问题（影响开发体验和代码质量）

### 9. 语言功能缺失
- **a. `class` 关键字**: 已 lex 但 parser 不支持。
- **b. `as` 类型转换**: token 存在但无转换表达式。
- **c. `..` 范围操作符**: 不支持 `0..10`。
- **d. `...` 展开操作符**: parser 返回 `NotYetImplemented`。
- **e. `?.` 可选链**: token 存在但 lowering 未正确处理。
- **f. Template String (反引号)**: AST 有 `TemplateString` 但 parser 不生成它。
- **g. 泛型 (Generics)**: `ComponentIR.generic_params` 硬编码 `vec![]`，组件不能有类型参数。
- **h. `void` / `never` 类型**: 没有底部类型。

### 10. 性能优化缺失
- **a. 无 lazy loading / code splitting**: 所有组件打包在一起。
- **b. 无 `memo` / `pure` 组件**: 无法标记纯组件跳过无变化重渲染。
- **c. `@if` 分支无 key 支持**: DOM 状态无法跨分支保持（`@for` 有 key 但 `@if` 没有）。

### 11. 开发者体验 (DX)
- **a. 无 HMR (Hot Module Replacement)**: 只能 livereload。
- **b. TypeScript 代码生成基础**: ts_mode 只做字段类型标注，不做接口导出/类型推导。
- **c. 无 DevTools**: 没有浏览器扩展或调试面板来检查 signal 状态。
- **d. 错误信息质量**: parse error 在复杂文件中经常报告 line 1，定位困难。

### 12. 生命周期不完整
- **a. 无 `beforeUpdate` / `afterUpdate`**: 只有 mount/unmount/effect。
- **b. Effect 的依赖数组手动指定但 IR 为空的 bug**: `lc.deps.clone()` 可以正确传递，但 computed 的 deps 是 `vec![]` 硬编码。

---

## 🟢 P3: 次要改进（锦上添花）

### 13. 小语法糖
- **a. 无 `?.` 安全调用链完整支持**
- **b. 无 `&&` / `||` 短路值的 view 内快速条件**: `{isReady && <Component/>}` 模式不支持
- **c. 无 `#each` / `{#if}` 块级简写**: 当前 `@if` 已经不错但必须用 `{ }` 括起来

### 14. 标准库增强
- **a. 无 Set/Map 专用方法**: 目前只有 Array 方法
- **b. 无 Debounce/Throttle 内置**: 需要手工实现
- **c. 无 WebSocket 客户端**: `std/http` 只有 HTTP 请求
- **d. 无国际化 (i18n) 支持**

### 15. 测试
- **a. 494 个测试对于编译器来说偏少**: 理想应该在 1000+ 以覆盖所有边界情况
- **b. 无 snapshot 测试**: 没有编译输出快照对比
- **c. 无 Fuzz 测试**

---

## 📊 与主流框架的功能对比

| 特性 | OmniView | Vue 3 | Svelte 5 | React 19 | Flutter | 
|------|----------|-------|----------|----------|---------|
| 响应式信号 | ✅ | ✅ | ✅ (runes) | ❌ (useState) | ✅ (ValueNotifier) |
| Computed 自动追踪 | ❌ | ✅ | ✅ | ❌ | ❌ |
| 条件渲染 | ✅ @if | ✅ v-if | ✅ {#if} | ✅ ternary | ✅ if/else |
| 列表渲染 | ✅ @for | ✅ v-for | ✅ {#each} | ✅ map() | ✅ ListView |
| 路由 | ❌ | ✅ | ❌ (社区) | ❌ (社区) | ✅ Navigator |
| 插槽/投影 | ❌ | ✅ | ✅ | ✅ children | ✅ child |
| 表单验证 | ❌ | ❌ (社区) | ❌ (社区) | ❌ (社区) | ✅ Form |
| Scoped CSS | ❌ | ✅ | ✅ | ❌ (CSS Module) | N/A |
| 动画/过渡 | ❌ | ✅ | ✅ | ❌ (社区) | ✅ |
| Provide/Inject | ❌ | ✅ | ✅ | ✅ Context | ✅ InheritedWidget |
| 泛型组件 | ❌ | ✅ | ✅ | ✅ (TS) | ✅ |
| HMR | ❌ | ✅ | ✅ | ✅ | ✅ |
| 跨平台(Web+Mobile) | ✅ | ❌ | ❌ | ❌ (RN) | ✅ |
| 条件编译 | ❌ | ❌ | ❌ | ❌ | ✅ (dart:io) |

---

## 🎯 建议优先级路线图

### Phase 1 (P0 — 立即需要，~3-4 周)
1. **Computed 自动依赖追踪**: 在 semantic 阶段 walk 表达式收集 deps
2. **Slot 内容投影**: 实现父→子的 children 传递
3. **Provide/Inject**: 实现简单的作用域上下文传递
4. **Signal 等值检查和批量更新**: 基础性能

### Phase 2 (P1 — 框架可用，~4-6 周)
5. **伪类支持** (`:hover`, `:focus`, `:active`)
6. **Scoped CSS**: 自动添加 component-id data 属性
7. **路由**: 基础的路由定义 + router-link + URL 参数
8. **表单系统**: 验证规则 + bind 类型增强
9. **Flutter→CSS 基本样式映射**: 最常用的 20 个属性

### Phase 3 (P2 — 框架好用，~4-6 周)
10. **`@switch` / `@html` / dynamic component**
11. **泛型支持**
12. **`...` 展开操作符**
13. **Lazy loading / code splitting**
14. **DevTools 基础版**
15. **动画基础支持** (`@keyframes` 或 JS 动画 API)

### Phase 4 (P3 — 完善，持续)
16. `class` 关键字完整实现
17. `as` 类型转换
18. 模板字符串
19. i18n 支持
20. WebSocket stdlib
