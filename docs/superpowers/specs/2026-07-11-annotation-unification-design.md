# Poiesis 注解格式统一设计

> 2026-07-11 | 头脑风暴产出

## 一、目标

统一 Poiesis 所有板块（Design System、Canvas/View、Code Flow、Store）的 `.omni` 代码生成、JSON 保存和解析数据格式，解决当前多种注解格式并存、协议不一致、手动书写 omni 无法被编辑器解析的问题。

### 三大原则

| 原则 | 含义 |
|------|------|
| **Omni 是主源** | 代码结构、design system 全部在 `.omni`；JSON（project.json）只存编辑器无法从源码推断的元数据 |
| **Git merge 友好** | `.omni` 中注解各占一行（line-based merge）；JSON 中全部用 `Record<string, T>` 而非数组（key-based merge） |
| **手动 omni 可往返** | 人写 omni 时加简单的 `// @<type>` 标注关键节点；编辑器打开自动生成 `// @pid` + JSON 布局；保存后 `// @<type>` 替换为 `// @pid`；再次手写可继续加 `// @<type>` 混在已有 `// @pid` 之间 |

---

## 二、注解分层

### 两层注解

| 层级 | 格式 | 谁写 | 用途 | 存哪里 |
|------|------|------|------|--------|
| **类型标注** | `// @if`、`// @code-block`、`// @return` 等 | **人写**（手动 omni 时） | 帮助 parser 区分 fn/action body 内的节点类型 | `.omni` 源码 |
| **身份标注** | `// @pid:<id>` | **编辑器自动生成** | 往返稳定性，跨保存保持节点身份 | `.omni` 源码 |
| **布局/连线** | JSON `nodes`、`wires`、`codeFlowGraphs` | **编辑器自动生成** | 画布位置、连线路径、节点类型 | `project.json` |

### `.omni` 中只保留一种注解格式

```omni
// @pid:<id>
```

独占一行，放在被注解的声明/语句**上一行**。

所有结构信息（节点类型、连线、位置）移入 `project.json`。

---

## 三、手动书写规范

### 需要标注的构造

**只有 fn/action body 内部的代码语句需要 `// @<type>` 标注。** 所有顶层构造（component、store、design、view、style）和 view 内嵌语法（`@if`、`@for`、`@slot`、`<tag>`）都有唯一关键词，无需额外标注。

### 类型标注全集

| 标注 | 含义 | 对应代码 |
|------|------|----------|
| `// @if` | 条件分支入口 | `if (cond) { }` |
| `// @elif` | else if 分支 | `else if (cond) { }` |
| `// @else` | else 分支 | `else { }` |
| `// @return` | 返回语句 | `return expr;` |
| `// @code-block` | 通用代码块 | 赋值、函数调用、表达式 |
| `// @for` | for 循环 | `for (x in xs) { }` |
| `// @while` | while 循环 | `while (cond) { }` |

### 书写示例

```omni
component Main {
  fn greet(name: String): String {
    // @if
    if (name == "admin") {
      // @return
      return "Admin access";
    }
    // @elif
    else if (name != "") {
      // @return
      return "Hello " + name;
    }
    // @else
    else {
      // @return
      return "Hello World";
    }
  }

  view {
    <Text bind:text={greet("")} />
  }
}
```

### 连线自动推断规则

```
顺序节点:     code-block → code-block → code-block   (flow-out 串联)

if 结构:      if.br0 → true 分支第一个节点
              if.br1 → @elif 的第一个节点（有 elif 时）
              if.br2 → @else 的第一个节点（有 else 时）
              if.flow → if 结束后的下一个节点（无 elif/else 时）

嵌套:         通过 {} 大括号匹配识别层级
```

---

## 四、编辑器往返流程

### 第一轮：人写 omni → Poiesis 打开

```
Parser 处理:
  1. 看到 fn → 创建 fn 入口节点，自动生成 PID
  2. 看到 // @if → 创建 condition 节点，自动生成 PID
  3. 看到 // @return → 创建 return 节点，自动生成 PID
  4. 自动推断连线: fn → if.br0 → return1, if.flow → return2
  5. 自动布局所有节点
```

### 保存：编辑器写回

```
.omni 变化:
  // @if       →  // @pid:xxx    （类型标注消失）
  // @return   →  // @pid:yyy    （类型标注消失）
  // @elif     →  // @pid:zzz
  // @else     →  // @pid:www

project.json 新增:
  nodes[pid].nodeType = "if" | "return" | "code-block" | ...
  nodes[pid].blockName = "greet"
  codeFlowGraphs[fnPid] = { nodes: {...}, wires: {...} }
```

### 第二轮：混合编辑（手写 + 已有 PID）

已有项目的基础上，用户手动加新代码：

```omni
fn greet(name: String): String {
  // @if                          ← 手写新节点
  if (name == "admin") {
    // @return                     ← 手写新节点
    return "Admin access";
  }
  // @pid:a2                      ← 旧 PID 保留
  if (name != "") {
    // @pid:a3
    return "Hello " + name;
  }
  // @pid:a4
  return "Hello World";
}
```

```
Parser: // @if/@return → 自动生成 PID + 连线（新节点）
        // @pid → 从 JSON 恢复（旧节点位置不变）
        自动重新连线: fn → new-if → (br0: new-return, br1: old-pid-a2) → ...
```

### Parser 决策表

| 看到 | JSON 有数据 | 行为 |
|------|------------|------|
| `// @pid:xxx` | ✓ | 恢复已有节点，位置/连线从 JSON 读取 |
| `// @pid:xxx` | ✗ | 创建节点，自动推断类型和连线（异常兼容） |
| `// @<type>` | — | 新建节点，自动生成 PID + 自动布局 + 自动连线 |
| 都无 | — | **报错** |

---

## 五、project.json v3 Schema

### 改动

```typescript
interface ProjectJson {
  version: 3;                              // v2 → v3

  // ── 不变 ──
  architecture: ArchitectureMeta;
  canvasStates: Record<string, CanvasViewport>;
  panels: PanelLayout;

  // ── 扩展 ──
  nodes: Record<string, NodeMeta>;         // +nodeType +blockName
  wires: Record<string, WireMeta>;

  // ── 新增 ──
  codeFlowGraphs: Record<string, CodeFlowGraph>;  // 按 fn/action PID
}

interface NodeMeta {
  pos: { x: number; y: number };
  folded?: boolean;
  nodeType: string;        // "if" | "elif" | "else" | "return" | "code-block" | "for" | "while" | "fn" | "action"
  blockName: string;       // 所属 fn/action 名
}

interface CodeFlowGraph {
  pid: string;                                 // 所属 fn/action 的 PID
  nodes: Record<string, CodeFlowNode>;         // 按节点 PID
  wires: Record<string, CodeFlowWire>;         // 按连线 ID
}

interface CodeFlowNode {
  nodeType: string;    // "if" | "elif" | "else" | "return" | "code-block" | "for" | "while"
}

interface CodeFlowWire {
  from: string;        // 源节点 PID
  fromPort: string;    // "flow-out" | "br0" | "br1" | "br2"
  to: string;          // 目标节点 PID
  toPort: string;      // "flow-in"
}
```

### 不存 project.json 的数据（由 .omni 主源）

- `componentDefs` — 从 .omni 源解析（运行时缓存）
- `elements` / `wires` / `rootOrder` — 实时画布数据，由 .omni 源提供
- `storeDefs` / `inlineStoreDefs` — 从 .omni 解析
- `pidIndex` — 从 .omni 的 `// @pid` 重建
- `undoStacks` — 会话内存

### v2 → v3 迁移

打开 v2 项目时自动迁移：
1. `nodes[pid]` 无 `nodeType` → 从旧 chain 注解 + 代码语法推断
2. 无 `codeFlowGraphs` → 从 `// @chain/@node/@wire` 迁移到 JSON，源码中去掉 chain 注解
3. design block 内 `// @token/@class/@variant id=xxx` → `// @pid:xxx`
4. 保存时升级为 v3

---

## 六、实现架构

### 分层

```
Layer 3: Parser/Emitter 适配层
  parser.ts, parser-view.ts, parser-helpers.ts, parser-finalize.ts
  graph-parser.ts（新增 // @<type> 支持 + 连线自动推断）
  source-emitter.ts（统一 // @pid 输出，消除行尾 pid）
  design-emitter.ts（// @token/@class/@variant → // @pid）
  chain-emitter.ts（// @chain/@node/@wire → JSON，源码移除）

Layer 2: 注解基础设施（新增 annotation-registry.ts）
  parsePidAnnotation(line)     → { pid } | null
  parseTypeAnnotation(line)    → { nodeType } | null
  parseAnyAnnotation(line)     → { pid?, nodeType? } | null
  emitPidAnnotation(pid)       → "// @pid:xxx"
  ALL_PID_REGEX                → /^\/\/\s*@pid:(\S+)/
  ALL_TYPE_REGEX               → /^\/\/\s*@(if|elif|else|return|code-block|for|while)\s*$/

Layer 1: 持久化层
  project-json.ts  → v3 schema serialize/deserialize
  save-manager.ts  → 写入 codeFlowGraphs + nodeType
  load-manager.ts  → 读取 codeFlowGraphs + nodeType

Store 适配
  serialization.ts → toProjectJson/loadFromProjectJson 适配 v3
```

### 文件改动清单

| # | 文件 | 改动 |
|---|------|------|
| **新增** |
| 1 | `services/codegen/annotation-registry.ts` | 统一注解解析/发射 |
| **类型** |
| 2 | `core/types/project-json.ts` | v3 schema：`NodeMeta` + `nodeType`/`blockName`，新增 `CodeFlowGraph` |
| **解析器** |
| 3 | `services/codegen/parser.ts` | 全部注解改用 annotation-registry；design block `@token/@class/@variant` → `@pid` |
| 4 | `services/codegen/parser-view.ts` | view 元素用 annotation-registry |
| 5 | `services/codegen/parser-helpers.ts` | 适配新注解格式 |
| 6 | `services/codegen/parser-finalize.ts` | 适配 |
| 7 | `services/codegen/graph-parser.ts` | 新增 `// @<type>` 支持；连线自动推断 |
| **发射器** |
| 8 | `services/codegen/source-emitter.ts` | `emitPidAnnotation()`；消除行尾 pid |
| 9 | `services/codegen/design-emitter.ts` | `@token/@class/@variant` → `@pid` |
| 10 | `services/codegen/chain-emitter.ts` | chain 注解 → JSON，源码去除 |
| **管道** |
| 11 | `services/codegen/pipeline.ts` | 消除 `storeDefToStoreDefinition` 转换；适配新注解 |
| **持久化** |
| 12 | `services/persistence/project-json.ts` | v3 schema |
| 13 | `services/persistence/save-manager.ts` | codeFlowGraphs 保存 |
| 14 | `services/persistence/load-manager.ts` | codeFlowGraphs 加载 |
| **Store** |
| 15 | `store/slices/serialization.ts` | `toProjectJson`/`loadFromProjectJson` → v3 |

---

## 七、自审检查

### 一致性
- 所有板块统一使用 `// @pid:xxx` 注解格式 ✓
- JSON 全部 `Record<string, T>` 结构，git merge 友好 ✓
- `.omni` 为主源，JSON 只存布局/连线/节点类型 ✓

### 往返完整性
- 手动 omni → parser（`// @<type>` 识别） ✓
- 第一次保存 → `// @pid` 替换 + JSON 写入 ✓
- 重新打开 → `// @pid` + JSON 恢复 ✓
- 增量编辑 → 混合 `// @pid` + `// @<type>` 共存解析 ✓

### 边界情况
- v2 项目迁移 → 自动推断 + 升级 ✓
- JSON 数据丢失 → 从 `@pid` + 代码语法重新推断 ✓
- 无类型标注的代码 → 报错 ✓

### 范围
- 只涉及注解格式统一和持久化协议，不改变画布渲染、WASM 编译等 ✓
