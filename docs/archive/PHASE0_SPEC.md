# OmniView Phase 0 Specification

> **版本：** 1.0（冻结 — 不可变任务书）  
> **基于：** `OmniLang_v0.3_设计方案.md` v3.0-final  
> **IR 版本：** 0.3  
> **目标：** React + Zustand + CSS Modules L2 地道代码生成  
> **日期：** 2026-06-05

---

## 一、设计哲学

1. **语义优先**：描述 UI 意图而非实现。`<Button onPress={fn}>` 而非 `<button onClick={fn}>`
2. **受限即自由**：不提供任意 JS 表达式。在受限空间内做精确跨框架映射
3. **渐进式可逃逸**：90% OmniView 抽象 + 10% `@native(target)` 框架原生代码

### 安全表达式子集

| 位置 | 允许 | 禁止 |
|------|------|------|
| 属性值 | 三元、比较、逻辑、属性访问、字面量 | 函数调用、副作用 |
| 文本插值 | 属性访问、三元、比较 | 函数调用（除 `.len()`） |
| `class_map()` | 布尔表达式键值对 | 方法调用 |
| `style={{}}` | 三元、属性访问 | 复杂计算 |

---

## 二、组件模型

```
component ComponentName {
    prop name: Type = default;          // Props
    signal name: Type = initial;        // 响应式状态
    computed name: Type = expression;   // 计算属性
    event name: PayloadType;            // 必须事件
    event name: PayloadType?;           // 可选事件
    @store StoreName;                   // Store 连接
    
    @effect(signal) { ... }            // 副作用
    @mount { ... }                      // 挂载后
    @unmount { ... }                    // 卸载前
    
    fn name(params) -> ReturnType { ... }  // 方法
    view { ... }                            // 视图
    style { .class { property: value; } }  // 样式
}
```

---

## 三、20 个核心元素

| 类别 | 元素 |
|------|------|
| 布局 | Container, Flex, Grid, Stack, Spacer |
| 内容 | Text, Heading, Image, Link, Divider |
| 交互 | Button, Input, TextArea, Checkbox, Select |
| 列表 | List, ListItem |
| 反馈 | Spinner, Progress, Alert |
| 复合 | Card, Form |

### 部分映射

| OmniView | React |
|----------|-------|
| `<Container>` | `<div>` |
| `<Flex direction="row">` | `<div style={{display:'flex', flexDirection:'row'}}>` |
| `<Text>` | `<span>` |
| `<Button onPress={fn}>` | `<button onClick={fn}>` |
| `<Input bind:value={v}>` | `<input value={v} onChange={e => setV(e.target.value)}>` |
| `<Checkbox bind:checked={c}>` | `<input type="checkbox" checked={c} onChange={...}>` |
| `<List>` | `<ul>` |

---

## 四、@指令系统

### @if / @else if / @else

```
@if state == "loading" { <Spinner /> }
@else if state == "error" { <Alert /> }
@else { <Content /> }
```

React: `{cond ? (...) : other ? (...) : (...)}`

### @for / @empty

```
@for item in items {
    <ListItem key={item.id}>{item.name}</ListItem>
}
@empty { <Text>No items</Text> }
```

React: `{items.map(item => <li key={item.id}>...)}`

`key` 属性强制要求（P0201 规则）。

### bind:prop

```
<Input bind:value={email} />
```

约束（P0214）：只能绑定 component signal 或 Store signal，不能绑定 prop。

### @native / @fallback

```
@native(react) { <Suspense>...</Suspense> }
@fallback { <div>...</div> }
```

@native 块不经过 OmniView 解析，直接透传到目标输出。P0 无校验。
编译到非指定目标 → ERROR。

---

## 五、响应式状态

### signal

| 框架 | 编译 | 读取 | 写入 |
|------|------|------|------|
| React | `useState(0)` | `count` | `setCount(5)` |

### computed

```
computed doubled: i32 = count * 2;
```

React: `const doubled = useMemo(() => count * 2, [count])`

### 生命周期

| OmniView | React |
|----------|-------|
| `@mount` | `useEffect(fn, [])` |
| `@unmount` | cleanup fn in useEffect |
| `@effect(x)` | `useEffect(fn, [x])` |

---

## 六、Store（Zustand）

```
store TodoStore {
    signal todos: Array<Todo> = [];
    signal filter: String = "all";
    getter filtered: Array<Todo> = ...;
    action add(text: String) { todos = [...todos, Todo{...}]; }
}
```

### 编译目标

```typescript
interface TodoStoreState {
  todos: Todo[];
  filter: string;
  filtered: () => Todo[];
  add: (text: string) => void;
}

export const useTodoStore = create<TodoStoreState>()((set, get) => ({
  todos: [],
  filter: "all",
  filtered: () => { const state = get(); return ...; },
  add: (text) => set(state => ({ todos: [...state.todos, ...] })),
}));
```

### 组件中使用

```
component TodoApp {
    @store TodoStore;
    view { <Text>{store.filtered}</Text> }
    fn handle() { store.add("text"); }
}
```

React:
```typescript
const filtered = useTodoStore(s => s.filtered());
function handle() { useTodoStore.getState().add("text"); }
```

---

## 七、语义样式 → CSS Modules

```
style {
    .card {
        layout: column;
        gap: 16px;
        padding: 24px;
        border_radius: 8px;
        shadow: md;
    }
}
```

编译为 `Component.module.css` + `import styles from "./Component.module.css"`

50+ 语义属性映射表（`codegen/css_mapping.py`）。

---

## 八、编译管道

```
.omni → Parser → AST → Semantic Analysis → Intent IR → Codegen → React TSX + CSS
```

### Intent IR（8 个核心类型 + 6 个基础类型）

| 意图类型 | 说明 |
|---------|------|
| StateIntent | 响应式状态声明 |
| MutationIntent | 状态修改操作（8 种 MutationOp） |
| ComputedIntent | 派生值计算 |
| EffectIntent | 副作用 |
| LifecycleIntent | 挂载/卸载 |
| ListRenderingIntent | 列表渲染 |
| ConditionalIntent | 条件渲染 |
| BindIntent | 双向绑定 |
| EventIntent | 事件绑定 |
| SlotIntent | 插槽投影 |
| StyleIntent | 样式意图 |
| LayoutIntent | 布局意图 |

---

## 九、转译约束

| 规则 | 说明 |
|------|------|
| P0201 | @for 必须提供 key |
| P0214 | bind: 只能绑定 signal，不能绑定 prop |
| P0203 | computed 依赖精确收集 |

---

## 十、Phase 0 精确范围

### 包含
- React 18+ Function Components + Hooks
- Zustand 4.x 状态管理
- CSS Modules 样式隔离
- 20 个核心元素 L2 地道生成
- @if/@for/@slot/@native/@fallback 指令
- signal/computed/@effect/@mount/@unmount
- bind:value / bind:checked
- 行号注释 `/* .omni:N */`
- test-id → data-testid

### 不包含
- Vue/Dioxus/Flutter 目标（Phase 1+）
- `@error` 错误边界（Phase 2）
- `<Suspense>` / `@load`（Phase 3）
- 动画系统（Phase 3）
- 表单验证（Phase 3）
- Portal / Teleport（Phase 3）
- SSR / RSC（永久不在范围）

---

## 十一、测试策略

| 测试类型 | 最低数量 | 状态 |
|---------|---------|------|
| 解析测试 | 20+ | ✅ |
| 语义分析测试 | 8+ | ✅ |
| Codegen 快照测试 | 50+ | ✅ |
| 诊断测试 | 3/规则 | ✅ |
| 行为测试 (Playwright) | 5 | 待实现 |

---

## 十二、CLI

| 命令 | 功能 |
|------|------|
| `omnic init [name]` | 脚手架 React 项目 |
| `omnic build <file> [--target]` | 编译 .omni |
| `omnic check <file>` | 静态分析 |
| `omnic watch <file>` | 文件监听自动编译 |
| `omnic fmt <file>` | 格式化 (Phase 1) |

---

## 十三、已知限制

1. snake_case 命名保留 omni 源码风格（Phase 0 不转换）
2. `@for` 内闭包捕获循环变量需手动处理
3. 复杂 CSS（动画/伪元素/滤镜）需通过字符串 class 或 @native
4. `async action` 关键字解析不完全支持

---

> **本文档为 Phase 0 不可变任务书。** 任何调整通过设计偏差审查会议讨论，记录在 `DECISIONS.md`。
