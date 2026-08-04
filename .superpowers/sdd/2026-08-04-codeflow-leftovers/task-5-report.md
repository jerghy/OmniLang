# Task 5 Report: args 公式框（NodeArgsRows candidates）

**状态**: ✅ 完成
**Commit**: 见最终提交
**测试**: 417/417（基线 414 + 3 新用例），tsc 零错误

---

## 实现清单（3 文件）

| 文件 | 改动 |
|---|---|
| components/canvas/code/fields/NodeField.tsx | `ArgsProps` 追加可选 `candidates?: FormulaCandidateSource`；行内字段抽 `rowField` 助手——传 candidates 渲染 NodeFormulaField（placeholder/ariaLabel/onCancel/onBlur 透传），缺省退化 NodeTextField（既有行为零漂移） |
| components/canvas/code/nodes/SyntaxNode.tsx | store-call / call 两处 NodeArgsRows 调用追加 `candidates={formulaSource}`（复用既有 useMemo 组装，69-82 行原值直接传入） |
| fields/__tests__/NodeArgsRows.test.tsx | 追加 `describe("NodeArgsRows candidates")` 3 用例（brief 基准全量实现，零微调） |

## 关键实现要点

- **导入方向无循环**：NodeField.tsx 现在运行时导入 NodeFormulaField（组件 + 类型）。已核实 NodeFormulaField 的运行时依赖图（react / expression-trigger / NodeCandidateMenu / NodeField.module.css）不触碰 NodeField.tsx——单向边 NodeField → NodeFormulaField，无循环。
- **向后兼容双通道**：无 candidates 时走 NodeTextField（`data-field-kind="text"`），既有 12 个 NodeArgsRows 用例（全用 `getAllByRole("textbox")` 断言）零改动通过；有 candidates 时每行（含 "+ 添加参数" 的本地 adding 行）均为 NodeFormulaField（`data-field-kind="formula"`、role=combobox、触发式弹层）。
- **行内弹候选链路完整**：行字段 value/onCommit 走既有 `update(i, v)` 管线，候选插入是草稿的一部分，Enter/blur 才 onCommit——args 提交语义（`args` 逗号拼接）不变。
- **SyntaxNode 零新建状态**：`formulaSource` 在 store-call/call 渲染前已 useMemo 组装，仅补一行 prop；无 scope 时 buildFormulaSource 返回 undefined → 退化纯文本（与 assign/let 公式框同行为）。

## 测试（RED → GREEN）

1. RED：2 用例失败（`data-field-kind="formula"` 计数 0 / 无 combobox），13 既有用例通过。
2. GREEN：NodeArgsRows 15/15 + NodeFormulaField 14/14 + NodeField 若干 + SyntaxNode 17/17。
3. 全量 `npx vitest run`：56 文件 417/417 通过（414 基线 + 3 新）。
4. `npx tsc --noEmit`：零错误。

新用例与 brief 基准一致，仅补全 mock 候选源（filter/storSignals 结构对齐 NodeFormulaField.test.tsx 既有模式）；第 3 用例输入值用 `values={[""]}` 起始（React 受控 input 值不变不派发 onChange，与 NodeFormulaField 既有测试模式一致）。

## 回归

- SyntaxNode.test.tsx 17/17（含"新建 store-call 节点可经 + 添加参数"接缝集成——有 scope 时 adding 行已是公式框，blur 提交链路依旧）
- NodeArgsRows / NodeFormulaField / NodeField 全绿；全量 417/417；tsc 零错误

## 疑虑

1. **React Doctor 1 处警告**（NodeField.tsx:164 `key={i}`）：既有代码（本次未改该行）。args 行列表仅 update/remove/append，无重排/过滤操作，且删除行会先 blur 提交草稿；受控 value + 草稿同步 effect 下实际影响可忽略。规范修法需 args 数据模型从 `string[]` 改为带稳定 id 的对象——属组件契约 API 变更，超出本任务范围，未处理（沿用 Task 4 对待既有警告的处理方式）。
2. **React Doctor 中途报 jsx-no-undef**（首次实现只 import type 组件未运行时导入）：已修复为运行时导入 NodeFormulaField，最终扫描仅剩上述既有警告。
