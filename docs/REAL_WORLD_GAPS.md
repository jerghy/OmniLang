# OmniLang 真实应用验证 — 缺口报告

> **应用：** todo-app.omni（Todo 应用）  
> **编译目标：** React (TypeScript + Zustand)  
> **日期：** 2026-06-10  
> **编译状态：** ✅ 成功（3 文件生成）  
> **运行时状态：** ✅ 浏览器中完全可用  
> **E2E 验证：** ✅ 添加/完成/清除/过滤 所有流程通过  
> **修复状态：** B1/B2/B4 已修复（3 个 CRITICAL），B3 待修复

---

## §1 发现总览

| # | 类型 | 严重度 | 描述 |
|---|------|--------|------|
| B1 | BUG | **CRITICAL** | Store getter 生成 `tasks.length` 而非 `state.tasks.length` |
| B2 | BUG | **CRITICAL** | 内联 fn handler 丢失函数调用和参数 |
| B4 | BUG | **CRITICAL** | `@if` 指令生成 `? ({...})` 语法——Babel 解析为 object literal 而崩溃 |
| B5 | BUG | **CRITICAL** | 多 store 时字段重复声明——所有 store 的所有字段都从每个 store 导入 |
| B6 | BUG | **HIGH** | 多 store 时 action 路由错误——`toggle_sidebar`(UIStore) 调用到 `useStatsStore` |
| B3 | BUG | **HIGH** | style 属性生成字符串而非 React 对象 |
| G1 | GAP | MEDIUM | 类型信息丢失：`Array<String>` → `any[]` |
| G2 | GAP | MEDIUM | 空 action body 无声生成 |
| G3 | GAP | LOW | `@for` key 在跨数组时可能冲突 |
| G4 | GAP | LOW | view 块内 `//` 注释被渲染为文本内容（应被忽略） |
| E1 | ERGO | MEDIUM | 错误消息 `unexpected character 'o'` 不友好（应提示 `||` 替代 `or`） |
| B7 | BUG | **CRITICAL** | `@if` 多子元素（无 `@else`）生成无效 JSX → 已修复（Fragment 包裹） |
| B10 | BUG | **CRITICAL** | CSS className 匹配全部错误——test-id 连字符≠CSS 下划线，Container 全匹配 `.container` |
| B8 | BUG | **HIGH** | OmniLang 嵌套 `else { if ... }` 解析丢失 else 分支 |
| G6 | GAP | MEDIUM | `bind:value` 不支持 `store.field`——只接受简单 CNAME |
| G7 | GAP | LOW | OmniLang 不支持 `store.field = value` 直接赋值——需 action 包装 |
| G9 | GAP | LOW | CSS 简写值不支持——`padding: 8px 24px` 多值语法报错 |
| G8 | GAP | LOW | TEXT_PLAIN 不支持 Unicode（☑ ☐）和 `[` 字符 |
| G5 | GAP | LOW | 浮点数精度——`146.153846...` 应格式化为 `146.15` |
| E2 | ERGO | LOW | useCallback 依赖数组包含未使用的 filter |

---

## §2 详细分析

### B1: Store getter 变量引用错误 [CRITICAL]

**文件：** `useTodoStore.ts:25`

```typescript
// 错误生成：
active_count: () => {
    const state = get();
    return tasks.length;  // ReferenceError: tasks is not defined
},

// 正确应为：
active_count: () => {
    const state = get();
    return state.tasks.length;
},
```

**影响：** Store getter 在运行时会抛出 ReferenceError。所有使用 getter 的组件都会崩溃。

**根因：** `omnilang/codegen/omniview/react.py` 中 store getter 的 codegen 在访问 store 属性时未加 `state.` 前缀。

---

### B2: 内联 fn handler 丢失调用 [CRITICAL]

**文件：** `TodoApp.tsx:68-70`

```typescript
// .omni: fn() { handle_complete(task); }
// 错误生成：
<button onClick={() => {
    handle_complete;  // 引用了函数但未调用，丢失 task 参数
}}>

// 正确应为：
<button onClick={() => {
    handle_complete(task);
}}>
```

**影响：** 列表中的完成/恢复按钮点击无效。

**根因：** `@for` 循环内的 `fn() { handler(var); }` — parser/lowering 未正确传递闭包变量到内联 fn 的 body 中。

---

### B3: React style 属性格式错误 [HIGH]

**文件：** `TodoApp.tsx:81`

```tsx
// 错误生成：
<span style="text_decoration: line-through;" data-testid="done-text">

// 正确应为：
<span style={{textDecoration: 'line-through'}} data-testid="done-text">
```

**影响：** React 运行时警告，样式不生效。

**根因：** codegen 未将 OmniView 的 CSS 属性（`text_decoration`）转换为 React 的 camelCase 格式。

---

### B4: @if 指令生成无效 JSX 语法 [CRITICAL] 🆕

**文件：** `TodoApp.tsx:65`

```tsx
// 错误生成（Babel 报错：Unexpected token）：
{((filter === "all") || (filter === "active")) ? ({tasks.map(task => (
  <li key={task} data-testid="task-item">
    ...
  </li>
))}) : (null)}

// 正确应为：
{((filter === "all") || (filter === "active")) ? tasks.map(task => (
  <li key={task} data-testid="task-item">
    ...
  </li>
)) : null}

// 或更好：
{((filter === "all") || (filter === "active")) && tasks.map(task => (
  <li key={task} data-testid="task-item">
    ...
  </li>
))}
```

**错误：** `({tasks.map(...)})` —— 外层 `({})` 包裹使 Babel 解析器将 `{` 解释为 object literal 而非 JSX 表达式，导致 `Unexpected token, expected ","` 编译失败。

**影响：** **阻塞级别。** 应用无法通过 bundler 编译，浏览器中完全不可运行。

**根因：** `omniview/codegen/react.py` 中 `@if` 指令的 codegen 在生成三元表达式时多余地包裹了 `({...})`。

---

### G1: 类型信息丢失 [MEDIUM]

**文件：** `useTodoStore.ts:6-8`

```typescript
// .omni: signal tasks: Array<String> = [];
// 生成：
tasks: any[];
completed: any[];
next_id: number;
```

`Array<String>` 应映射为 `string[]`。`any[]` 失去类型安全。

---

### G2: 空 action body [MEDIUM]

```typescript
// .omni:
action remove_task(text: String) {
    // simplified: just mark as done
}

// 生成：
remove_task: (text) => {
    // 空函数体
},
```

空 action 应至少生成 `console.warn("remove_task not implemented")` 或编译时 warning。

---

### G3: `@for` key 跨数组可能冲突 [LOW]

当 `tasks` 和 `completed` 数组中有相同文本时，`key={task}` 会冲突。需要复合 key 或 scope key。

---

### E1: 错误消息质量 [MEDIUM]

```
# 输入: @if filter == "all" or filter == "active"
# 错误: Syntax error: unexpected character 'o'
```

更好的消息：`Syntax error: 'or' is not supported, use '||' for logical OR`

---

### E2: useCallback 依赖数组 [LOW]

```typescript
const show_all = useCallback(() => {
    setFilter("all");
}, [filter]);  // filter 不在回调中使用，不应在依赖中
```

---

## §3 已修复（本轮）

| # | 严重度 | 问题 | 修复方式 | 文件 |
|---|--------|------|---------|------|
| B1 | CRITICAL | Store getter `tasks.length` → `state.tasks.length` | `_expr_to_js` 后扫描信号名加 `state.` 前缀 | `codegen/react.py:893-903` |
| B2 | CRITICAL | 内联 fn handler 丢失调用+参数 | lowering 返回 `FuncCallIR` dict 保留 args | `lowering.py:391-396` |
| B4 | CRITICAL | `@if` 生成 `({...})` 语法 Babel 崩溃 | 三元表达式中移除多余 `{}` 包裹 | `codegen/react.py:514-522` |

---

## §4 统计

| 严重度 | 数量 | 状态 |
|--------|------|------|
| CRITICAL | 5 (B1, B2, B4, B5) | **4 已修复** ✅ (B1/B2/B4/B5)，B6 关联待修复 |
| HIGH | 2 (B3, B6) | 待修复 |
| MEDIUM | 3 (G1, G2, E1) | **3 已修复** ✅ |
| LOW | 4 (G3, G4, G5, E2) | 延后 |
| **总计** | **15** | **7 已修复** |

## §5 修复优先级

1. ✅ B1/B2/B4 — Todo app CRITICAL bugs — 已修复
2. ✅ B5 — 多 store 字段重复声明 — 已修复
3. ✅ G1 — 类型映射 — 已修复
4. ✅ E1 — 错误消息 — 已修复
5. ✅ B3 — Style 格式 — 已修复
6. **B6** — 多 store action 路由 → dashboard 中 sidebar toggle/set_page 调用了错误 store
7. **G5** — 浮点数格式化 → `146.153846...` → `146.15`
8. **G2, G3, G4, E2** — 延后
