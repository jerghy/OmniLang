# IR 原型验证报告 — Phase 2a Flutter 金丝雀测试

> **基于：** `OmniLang_v0.3_执行计划.md` §1.1 金丝雀测试  
> **目标：** 验证 Intent IR 能否驱动非 Web 框架（Flutter）的代码生成  
> **日期：** 2026-06-05

---

## Case A: ListRenderingIntent → ListView.builder

| 项 | 内容 |
|----|------|
| **输入** | `@for item in items { <ListItem key={item.id}>{item.name}</ListItem> }` |
| **IR** | `ListRenderingIntent { iterable: items, item_binding: item, body: ListItem, key_expr: item.id }` |
| **React** | `{items.map(item => <li key={item.id}>{item.name}</li>)}` |
| **Flutter** | `Column(children: items.map((item) => ListTile(key: ValueKey(item.id))).toList())` |
| **结论** | ✅ **GO** — ListRenderingIntent 的结构足以表达列表渲染意图。Flutter L1 生成 Column+.map()，L2 将优化为 ListView.builder。不需要额外的 `strategy` 字段 |

---

## Case B: BindIntent → TextEditingController

| 项 | 内容 |
|----|------|
| **输入** | `<Input bind:value={email} placeholder="Enter" />` |
| **IR** | `BindIntent { source: SignalRefIR(name="email"), element_type: TEXT_INPUT, direction: BIDIRECTIONAL }` |
| **React** | `value={email} onChange={e => setEmail(e.target.value)}` |
| **Flutter** | `TextField(controller: _emailController)` + StatefulWidget + initState + dispose |
| **结论** | ✅ **GO** — BindIntent + SignalRefIR 提供足够信息生成 TextEditingController 生命周期。L2 将添加 didUpdateWidget 实现外部 signal→controller 同步 |

---

## Case C: Flex → Row/Column

| 项 | 内容 |
|----|------|
| **输入** | `<Flex direction="row" gap="16px" align="center">` |
| **IR** | `ElementIR { tag: "Flex", props: { direction: "row", gap: "16px", align: "center" } }` |
| **React** | `<div style={{display:'flex', flexDirection:'row', gap:'16px', alignItems:'center'}}>` |
| **Flutter** | `Row(spacing: 16, mainAxisAlignment: MainAxisAlignment.center)` |
| **结论** | ✅ **GO** — ElementIR 的 props 字典可驱动 Flutter 的 Row/Column 构造器选择 |

---

## Case D: @if 空分支

| 项 | 内容 |
|----|------|
| **输入** | `@if cond { <Text>Hello</Text> }` (无 @else) |
| **IR** | `ConditionalIntent { branches: [(cond, Text)], fallback: None }` |
| **React** | `{cond ? (<span>Hello</span>) : null}` |
| **Flutter** | `(() { if (cond) { return Text("Hello"); } })()` (L1) / `cond ? Text("Hello") : SizedBox.shrink()` (L2) |
| **结论** | ✅ **GO** — fallback=None 在各框架中可正确映射为 null/SizedBox.shrink()/VNode::empty() |

---

## 综合评估

### IR 设计验证结果：✅ 全部通过

4 个金丝雀 case 均验证了 Intent IR 的结构足以驱动 Web (React/Vue)、Rust (Dioxus)、Dart (Flutter) 四个目标框架的代码生成。

### 发现的 IR 设计问题

| 问题 | 严重度 | 说明 |
|------|--------|------|
| Store access in template | LOW | `store.field` 在各框架的展开方式不同（React: 局部变量, Vue: 属性访问, Dioxus: 函数调用, Flutter: getter）。当前通过 `FieldAccessExpr` 统一表达，由各 codegen 自行处理。IR 层面无需修改 |
| Text content as child vs positional | LOW | Flutter `Text("content")` 是位置参数，Web `<span>content</span>` 是子节点。当前 IR 将文本内容表示为 `children` 中的字符串，各 codegen 自行适配 |
| Inline style vs widget params | MEDIUM | Web 用 CSS，Flutter 用构造器参数。当前 IR 通过 `props` 传递布局属性，由 codegen 映射到 CSS/Wiget params。跨框架样式是根本性限制（见设计文档 §八） |

### IR 调整建议

**当前 IR 版本（0.3）无需调整。** 4 个 case 均通过验证。发现的差异（文本子节点、样式系统）是框架间的根本性差异，不应通过 IR 膨胀来解决——应由各 codegen 的惯用法展开阶段处理。

---

## Flutter L1 Codegen 状态

| 特性 | 状态 | 说明 |
|------|------|------|
| StatefulWidget 骨架 | ✅ | 组件 → StatefulWidget + State 类 |
| signal → 字段 | ✅ | `signal count: i32 = 0` → `int _count = 0` |
| setState 包裹 | ✅ | 修改操作自动包裹 `setState(() => ...)` |
| TextEditingController | ✅ | `bind:value` → TextEditingController + initState/dispose |
| @if 条件渲染 | ✅ | 立即执行函数 / 三元表达式 |
| @for 列表渲染 | ✅ L1 | Column + .map()（附 L2 优化提示） |
| Flex→Row/Column | ✅ | direction 属性驱动构造器选择 |
| Button→ElevatedButton | ✅ | onPress → onPressed |
| Store→ChangeNotifier | ✅ | 简单的 ChangeNotifier 模式 |
| @mount → initState | ✅ | 生命周期映射 |
| ListView.builder (L2) | ⬜ | 当前使用 Column+.map() |

### 判决：IR 设计方向正确，Flutter L1 验证通过。IR 版本 0.3 可进入 Phase 3。
