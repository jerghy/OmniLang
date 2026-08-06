# CodeFlow 公式框重新设计：显式斜杠胶囊 + 字面量判定

## Status

Design（brainstorming 定稿，待实现计划）

## Context

### 问题

CodeFlow 节点（assign/let/condition/call）的 value 公式框（`NodeFormulaField`，ADR-059 引入）存在顽固的输入缺陷：**点击公式框后光标跳到最左侧，输入变为"倒着写"**。根因是当前架构（`contentEditable` + `dangerouslySetInnerHTML` + Selection API 手动光标恢复 + memo 隔离重渲染）的固有顽疾——React 19 对 `dangerouslySetInnerHTML` 无条件重设、IME composition 竞态、microtask/rAF 光标恢复竞态，修一轮暴露一轮。

用户裁决：**不继续修 bug，推倒重来重新设计**。核心诉求：

- 一切从简，以输入框为基础
- 保留胶囊（可视化表达式元素），但**胶囊必须显式产生**（斜杠触发），不做"输入即自动分词胶囊化"
- 运算符键入直接弹提示框（无需斜杠），Enter 选中成胶囊
- 胶囊可点击重新选择
- 自由文本 = 字面量：纯数字→数字，非纯数字→字符串（显示无引号，提交自动加引号）

### 历史约束（保持）

- 提交仍是字符串（`onCommit(v: string)`），`.omni` 持久化格式、编译器零改动（ADR-059 契约）
- 候选数据零新数据源（派生自既有 flow-scope 数据层）
- 运算符集合以编译器 `infix.rs` 为权威，UI 不自行定义

## 设计决策

### D1：textarea 编辑层 + 覆盖层胶囊渲染（用户确认）

- **textarea 是唯一编辑层**：光标/IME/选区全部浏览器原生管理——光标跳开头类 bug 从根上失去生存土壤
- **胶囊 = 私有区字符标记段** `<text>`（React Mentions 同款）：覆盖层把标记段渲染成胶囊块（高亮）盖在 textarea 上方；text 不编码类型（提交/点击时用 tokenizer 推断——类型单一来源）
- 提交时标记段输出 `<text>` 原样

### D2：三段内容物模型（用户确认 + 修订）

| 内容物 | 例子 | 编辑行为 | 提交 |
|---|---|---|---|
| **胶囊**（标记段，高亮块） | 变量/信号/属性/函数/`Store.signal`/运算符（`+` `<=` `is`） | 斜杠或运算符键入弹框选中；点击可重选；Backspace 整块删 | 原样 |
| **结构字符**（着色文本，非胶囊） | `( ) [ ] { } , : .` 及 Esc 落点的运算符字符 | 键入即着色，逐字符编辑 | 原样 |
| **自由文本**（普通文本） | `1`、`hello world`、`user-settings` | textarea 原生 | 字面量判定 |

**结构字符非胶囊**（用户裁决）：处于胶囊之外，本质普通文本，仅渲染着色（区别于普通字符串）；无点击重选、无整块删除、逐字符编辑。

**胶囊只属于语法元素**：引用（变量/信号/属性/函数/store 信号）+ 运算符。括号/逗号/点号/冒号/方括号/花括号等标点不是胶囊。

### D3：字面量判定（用户确认）

自由文本段提交时判定（trim 后）：

1. 空（纯空白）→ 无字面量，空白原样保留（`[count] + 1` 间距不变）
2. `/^\d+(\.\d+)?$/` → number 原样（负号归运算符胶囊，`-` + `5` 自然拼成 `-5`）
3. `true` / `false` → bool 原样（编译器 `Literal::Bool`，2026-08-06 核验）
4. `null` → null 原样（token `Null` 存在；白名单无副作用）
5. 其他 → string：显示无引号，提交统一双引号 `"..."`（转义 `\`→`\\`、`"`→`\"`）

### D4：触发与交互（用户确认）

- **边界触发规则**（统一所有弹框）：`/` 和运算符字符（`+ - * % < = ! & | ?`）**仅在边界位置**弹候选框（前字符 = 空/空白/胶囊/结构字符）；词中间直接作为普通字符输入——`user-settings` 的 `-`、`a/b` 的 `/` 不弹框，自然成为字符串内容
- 运算符字符键入 → 弹**运算符候选框**（infix.rs 权威前缀表：`+`→`+`，`<`→`<` `<=`，`=`→`==`，`!`→`!=` `!`，`&`→`&&`，`|`→`||`，`?`→`??` `?.`），Enter/Tab 选中 → 运算符胶囊
- 斜杠 `/` 键入（边界）→ 弹**全量候选框**（变量/信号/属性/计算属性/store 信号/函数/运算符分组）——`/` 是胶囊入口键，与运算符字符不同：运算符字符弹框只有运算符分组，斜杠弹框含全部分组（含运算符，可选中 `/` 除法）
- Esc 关框：**该字符变为着色结构字符（提交原样）**——`/` Esc 后除法语义由编译器解释，`|` Esc 后为闭包边界（用户明确闭包不在公式框范围，仅保证不破坏文本）
- **斜杠逃生口**：斜杠弹框后输入无匹配文本按 Enter → 插入引用胶囊（提交原样）——候选外的任意标识符（闭包参数、未知方法名等）有出路
- 点击胶囊 → 光标定位标记段 + 弹该类型候选框（引用胶囊→引用候选，运算符胶囊→运算符候选）→ 选中替换
- Enter（候选框关）→ 提交；Esc（候选框关）→ 取消恢复 value；blur → 提交（保持 ADR-059 草稿提交语义）
- Backspace：光标在胶囊标记段起点 → 整块删除胶囊；否则逐字符
- IME（中文）：textarea 原生，无 composition 竞态

### D5：member 段规则（唯一保留的上下文语义规则）

`.`（结构字符）后的自由文本段 → **原样提交（member 语义，不做字面量判定）**：

- `mirrorInput.value`：`mirrorInput`(引用胶囊) + `.`(结构字符) + `value`(member 段) → `mirrorInput.value`
- `sizeBox.measure().height`、`items.get(items.len() - 1)`：链式拼出
- round-trip 反向结构化时 `Store.signal`（member token）拆成 `Store`(引用胶囊) + `.`(结构字符) + `signal`(member 段)，提交拼接回 `Store.signal`——双向一致

**出范围（用户裁决，编译器门禁兜底）**：结构体字面量 key（`Point { x: 0 }` 的 `x` 按字面量判定处理）、`await` 关键字、闭包（`|s| s + 5` 的 `|` 按结构字符原样、参数走斜杠逃生口，但不保证语义）。

### D6：Round-trip 反向结构化（.omni → 标记文本）

tokenize（复用/微调现有 tokenizer）逐 token：

| token | 重开形态 |
|---|---|
| identifier / member | 引用胶囊（**未知标识符也成引用胶囊**，提交原样——`count + 1` 重开不退化 `"count"`） |
| operator | 运算符胶囊 |
| number | 数字自由文本 |
| string | 剥引号 + 反转义（`\"`→`"`、`\\`→`\`）自由文本 |
| 结构字符（括号/逗号/点/冒号/方括号/花括号） | 着色结构字符 |
| 空白 | 原样保留 |

### D7：提交拼接

逐段顺序拼接：胶囊段 → 文本原样；结构字符段 → 原样；自由文本段 → 字面量判定结果（空白/数字/bool/null/字符串）。用户输入的空格原样保留。

## 架构与范围

**重写**：
- `poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx`——textarea + 覆盖层 + 交互逻辑
- `poiesis/src/components/canvas/code/fields/expression-trigger.ts`——纯函数：tokenize（含结构字符分类）/ 字面量判定 / 提交拼接 / 反向结构化 / 边界判定

**复用**：
- `NodeCandidateMenu.tsx`（listbox 交互成熟：↑↓/Tab/Enter/mousedown 防夺焦）
- `buildExpressionCandidates` / `buildFormulaSource`（flow-scope，零新数据源）
- `OPERATOR_PREFIX_CANDIDATES`（infix.rs 权威表）

**不动**：SyntaxNode/ConditionNode/ReturnNode 接口、NodeSelect（assign target 下拉）、`onCommit(v: string)` 契约、`.omni` 格式、编译器。

## 测试

- 纯函数单测（vitest）：字面量判定表（数字/bool/null/字符串/转义）、member 段、边界触发、提交拼接、round-trip 双向（`count + 1`、`Store.signal`、`"hello"`、`user-settings` 等语料）
- 组件测试：斜杠弹框/运算符弹框/逃生口/点击胶囊重选/Backspace 整块删/Enter 提交/Esc 取消
- tsc 仅允许基线
- 真机 Victauri E2E：真实输入流（含中文 IME）、保存→重开 round-trip、CodeFlowMidChain 语料

## 已知取舍

- 复制粘贴会带出不可见标记字符（React Mentions 同款，接受）
- 候选无匹配时无"无结果"提示（沿用 ADR-059 决策）
- 结构体 key / await / 闭包不在公式框范围（编译器保存门禁兜底）
- 字符串内嵌引号：编辑面 `say "hi"` → 提交 `"say \"hi\""` → 重开还原 `say "hi"`（双向一致）

## 备选方案（brainstorming 记录）

- **方案 A：textarea + 覆盖层胶囊（采纳）**——光标原生管理，业界成熟（React Mentions/Notion）
- **方案 B：编辑/预览分离**——编辑态胶囊显示为标记文本（`@count`），失焦渲染胶囊预览；最简但编辑时看不到胶囊块，被用户否决
- **方案 C：contenteditable 混排**——维持 contentEditable 仅插入斜杠胶囊 span；光标仍需手动管理，bug 类风险重现，被用户否决
- **自动分词保留**（现架构）：输入即胶囊化——光标顽疾根源，推倒

## 参考

- 现实现：`poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx`、`expression-trigger.ts`
- 上游设计：ADR-059（触发式自动补全）、ADR-057（语法节点 + `@pid` round-trip）
- 真实语料：`test/projects/learn-omniview/src/components/CodeFlowMidChain.omni` 等
- 编译器事实：`crates/omnilang-core/src/lexer/token.rs`（True/False/Null）、`parser/expr/prefix.rs`（`Literal::Bool`）
