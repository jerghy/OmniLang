# CodeFlowCanvas 语法节点扩展设计

> 日期：2026-08-04
> 状态：待实现
> 范围：CodeFlowCanvas（节点编辑器）新增 4 种语法节点类型，贯穿全链路

## 一、目标

CodeFlowCanvas 现有 4 种节点（fn / code-block / condition / return）太少，无法覆盖常用界面开发代码。本设计新增 4 种**语句级语法节点**：变量声明（let）、赋值（assign）、store 调用（store-call）、函数调用（call，含 emit）。代码模式素材来自 `test/projects/learn-omniview/src` 的真实使用统计。

## 二、需求澄清（用户确认）

1. **形态**：新增语法节点类型（"一个语法一个节点"），不是整块代码模板，也不是 Monaco snippet
2. **节点类型**（基于 learn-omniview 实际频率统计，用户全选）：
   - 变量声明 let（68×`let` + 21×`let mut`，最高频）
   - 赋值 assign（`count = count + step;`、`isOpen = true;`、`ThemeStore.mode = "dark"`）
   - store 调用（`TodoStore.add(text);` 等 6+ 处）
   - 调用 call（含 emit：`close();`、`emit onSelect(item);`、`console_log(...)`）
3. **编辑交互**：点选节点 → 右侧 FlowInspector 结构化表单（与 fn/return 节点一致）
4. **保存/解析**：严格复用现有 @pid 注释 round-trip 机制（详见第六节）

## 三、节点类型与数据模型

延续 `CodeblockElementData` 模型：`props` 存结构化字段，`source` 存生成代码文本（供节点预览）。

| tag | 节点 | 生成代码 | props 字段 |
|-----|------|---------|-----------|
| `let` | 变量声明 | `let [mut] name[: 类型] = 表达式;` | `{ name: string, mutable: boolean, varType?: string, value?: string }` |
| `assign` | 赋值/改状态 | `target op 表达式;` | `{ target: string, operator: string, value: string }` |
| `store-call` | store 调用 | `StoreName.action(args);` | `{ storeName: string, action: string, args?: string }` |
| `call` | 调用/emit | `fnName(args);` / `emit fnName(args);` | `{ fnName: string, args?: string, emit?: boolean }` |

- **operator 可选值**：`=` `+=` `-=` `*=` `/=` `%=`（与遗留 `code-op.ts` SetVarOp 一致）
- **连线**：与 code-block 相同（1 flow-in + 1 flow-out），控制流语义不变
- **scope**：组件 fn 代码流与 store action 代码流均适用（两者共用 compileCodeFlowGraph / parseCodeFlowGraph）

## 四、菜单与 UI

### 4.1 右键菜单（MENU_SECTIONS）

新增 2 个分类（现有「控制流」保留）：

```
变量与赋值:
  - 变量声明 (let)          shortcut: let
  - 赋值 (assign)           shortcut: =
调用:
  - store 调用 (store-call)
  - 函数调用 (call)          shortcut: fn()
```

菜单项 detail/占位符取自 learn-omniview 统计（见第七节）。

### 4.2 节点卡片

- `TAG_CN` 中文标签：let「变量声明」、assign「赋值」、store-call「store 调用」、call「调用」
- 卡片显示代码预览（`source` 前 50 字符截断，与 code-block 一致）
- 色系区分：let 蓝、assign 青、store-call 橙、call 绿（模块级 CSS，沿用现有 `flow-node` 类）
- 实现：新增 4 个小节点组件，或一个通用 `SyntaxNode.tsx` 按 tag 渲染（实现时二选一，倾向通用组件）

### 4.3 FlowInspector 表单

- **LetInspector**：名称输入、类型下拉（复用 `OMNI_TYPES` 常量）、`mut` 开关、初始表达式输入
- **AssignInspector**：目标输入、运算符下拉（`=` `+=` `-=` `*=` `/=` `%=`）、值表达式输入
- **StoreCallInspector**：store 名下拉（从 `storeReferences[componentName]` 动态取，可手输）、action 输入、参数输入（逗号分隔）
- **CallInspector**：函数名输入、参数输入（逗号分隔）、「emit 事件」开关

## 五、编译（GraphCompiler）

`emitNode` 新增 4 个 case，全部带 `// @pid:xxx` 注释锚点（emitPidComment，格式不变）：

```typescript
case "let":       // → let [mut] name[: type] = value;
case "assign":    // → target op value;
case "store-call":// → storeName.action(args);
case "call":      // → [emit ]fnName(args);
```

成功后继（flow-out）递归逻辑与 code-block 相同。`pipeline.ts` / `source-emitter.ts` 调用方**零改动**。

## 六、解析（GraphParser round-trip）

### 6.1 detectSegmentType 扩展（核心）

段类型联合扩展为 8 种：`"fn" | "code-block" | "condition" | "return" | "let" | "assign" | "store-call" | "call"`。从段首行代码文本分类：

**判定顺序**（detectSegmentType 内自上而下，命中即返回）：

```
1. fn … / if … / else … / return …  → 现状不变（fn / condition / return）
2. let [mut] …                      → let
3. emit …                           → call（emit=true）
4. PascalCase 前缀 + . + ( 调用      → store-call
5. 标识符( … ) 调用                  → call
6. 含赋值符（= += -= *= /= %=）      → assign
7. 其余（循环、多行语句等）          → code-block（现状不变）
```

**歧义规则**（round-trip 无损）：
- signal 赋值 `count = count + 1` 与变量赋值 `x = y` 统一为 assign（编译回同一段代码，语义不变）
- `ThemeStore.mode = "dark"`（store 信号**写**）不匹配规则 4（无 `(` 调用），落入规则 6 → assign。store-call 仅限**调用**（`Xxx.xxx(...)` 带括号）
- `Foo.bar()`（PascalCase 前缀）→ store-call；`items.push(x)`（小写前缀）→ call（数组方法/本地调用，编译回原文）

### 6.2 元素构建扩展

构建循环（graph-parser.ts 464-514 行区域）新增 4 个分支，从 `seg.code` 提取 props：

- **let**：`/let (mut )?(\w+)(: (\w+))? ?= ?(.*)/` → `{ name, mutable, varType, value }`
- **assign**：`/(\w+(?:\.\w+)?) (\+=|-=|\*=|/=|%=|=) (.*)/` → `{ target, operator, value }`
- **store-call**：`/([A-Z]\w+)\.(\w+)\((.*)\)/` → `{ storeName, action, args }`
- **call**：`/emit (\w+)\((.*)\)/` 或 `/(\w+)\((.*)\)/` → `{ fnName, args, emit }`

`source` 字段同时保存生成的代码文本（供节点预览）。`parser.ts` 调用方**零改动**。

### 6.3 不动的东西

- `// @pid:xxx` 注释格式不变（round-trip 锚点）
- `TYPE_REGEX` / `VALID_TYPES` 不扩展（手写 `// @let` 类型注释不加——detectSegmentType 从代码文本分类已足够，YAGNI）
- `TYPE_TO_SEGMENT` 不变（只服务 @type 注释）
- 分支上下文（assignBranchContext）、连线重建（wire）、brace depth 分析全部不动（语法节点是单行语句，不引入 brace）

## 七、learn-omniview 模式整合

表单占位符与菜单 detail 取自 `test/projects/learn-omniview/src` 真实统计：

| 节点 | 占位符/示例（来源） |
|------|---------------------|
| let | name: `filtered`、value: `todos.filter(...)`；`count` / `0`；`total` / `0` |
| assign | target: `count`、value: `count + step`（Counter.omni）；`isOpen = !isOpen`（Modal.omni） |
| store-call | `TodoStore.add(text)`（TodoList.omni）；`NotificationStore.clear()`；`ThemeStore.toggle()` |
| call | `close()`（Modal.omni）；`emit onSelect(item)`（EmitEventDemo）；`console_log(...)`（StdLib） |

## 八、测试

1. **graph-compiler 单测**：4 种节点 emit 正确（顶层链 + 嵌套在 condition 分支内 + fn 入口链 + store action 上下文）
2. **graph-parser 单测**：5 种语句 round-trip（let/assign/store-call/call/emit）→ 节点类型 + 字段完整保留；歧义语句（signal 赋值）归入 assign
3. **端到端 round-trip**：parse → 图 → compile → 源码 → 再 parse，节点类型与代码双向稳定（对齐现有 roundtrip-annotation.test.ts 模式）
4. **UI**：FlowInspector 表单渲染/更新测试（沿用 `__tests__` 组件测试模式）

## 九、涉及文件

| 文件 | 改动 |
|------|------|
| `components/canvas/code/flowConstants.ts` | CODE_TAGS、TAG_CN、MENU_SECTIONS 扩展 |
| `components/canvas/code/FlowCanvas.tsx` | nodeTypes 注册、handleAddNode 默认 props |
| `components/canvas/code/nodes/` | 新增 SyntaxNode.tsx（或 4 个小组件）+ CSS |
| `components/canvas/code/FlowInspector.tsx` | 新增 4 个表单组件 |
| `services/codegen/graph-compiler.ts` | emitNode 新增 4 case |
| `services/codegen/graph-parser.ts` | detectSegmentType + 元素构建分支 |
| `services/codegen/__tests__/` | 新增测试 |

## 十、不做（YAGNI）

- ❌ for/while 循环节点（2026-07-10 spec 明确后续迭代）
- ❌ signal/computed 声明节点（左侧面板已有）
- ❌ 遗留 operations/SlotValue 体系（CodeCard v3 废弃机制，不触碰）
- ❌ 拖拽式表达式编辑器（表单文本输入足够）
- ❌ store action 自动补全（下拉取 storeReferences 即可）
- ❌ `// @let` 手写类型注释（detectSegmentType 已覆盖）
