# CodeFlowCanvas 遗留清理 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 7 项遗留：① 保存 WASM 门禁失效修复 ② condition 分支 id/索引漂移 ③ 节点位置持久化 ④ condition/return/args 公式框扩展 ⑤ redo 按钮 ⑥ fn 双击空编辑器 ⑦ 延期 minor 清理。

**Architecture:** 三组独立改动——诊断层（diagnostics.ts 信封适配，一处修复三处门禁）；分支一致性（compiler 用真实分支 id + UI id 去重）；持久化链（保存侧实时位置写 project.json + 加载侧读回赋给物化元素）；控件扩展（NodeFormulaField 复用至 condition/return/args）。

**Tech Stack:** React 19、TypeScript、Zustand 5、vitest、@xyflow/react 12、无新依赖。

## Global Constraints

- **397 vitest 基线全绿**（`cd poiesis && npx vitest run`）；`npx tsc --noEmit` 零新增错误（允许基线 diagnostics.ts:151——注意本计划 Task 1 会修改该文件，修复后该基线错误应消失）
- **`@pid` round-trip 契约零改动**：`parseCodeFlowGraph`/`compileCodeFlowGraph` 签名不变；graph-parser 的分支 id 扫描序生成（`br${branchIndex}`）是 round-trip 稳定性既有设计，**不改 parser 的 id 生成**（只改 compiler 发射消费 + UI 生成防碰撞）
- **不改 source-emitter 的 return 契约**：return 表达式存 `el.source`（`onCommitReturn → setElementSource`），与语法节点 props 语义不同，勿混用
- 中文注释风格；提交消息 `fix(canvas): 中文描述` / `feat(canvas): 中文描述`
- 只动本计划任务指定的文件；已完成文件（NodeFormulaField/NodeCandidateMenu/expression-trigger/flow-scope/fn-topology/synthesizeFnEntry）如被 minor 清理触及，只做清理级改动不重构

---

### Task 1: 保存 WASM 门禁修复（MultiCompileEnvelope 信封适配）

**Files:**
- Modify: `poiesis/src/services/monaco/diagnostics.ts:144-163`（validateSources）
- Test: `poiesis/src/services/monaco/__tests__/validate-sources.test.ts`（新建）

**Interfaces:**
- Consumes: `wasmModule.compile_multi_to_omniview(sources, false, false)` 返回 `MultiCompileEnvelope { result: InternalCompileResult, cache_state }`，错误在 `result.result.errors`（omnilang-wasm/src/lib.rs:81-85；omnilang/src/lib.rs:275-290）
- Produces: `validateSources` 返回真实的 `WasmDiagnostic[]`（此前恒 `[]`——design/store/component 三处保存门禁全部静默放行）

- [ ] **Step 1: 写失败测试**

```ts
// validate-sources.test.ts
// 门禁修复：compile_multi_to_omniview 返回 MultiCompileEnvelope（result.result.errors），
// 旧实现读顶层 "errors" 恒空 → 三处保存门禁全部静默放行。
import { describe, it, expect, vi, beforeEach } from "vitest";
import { validateSources } from "../diagnostics";

// mock wasm 模块：返回真实 envelope 形状
const { mockCompile } = vi.hoisted(() => ({ mockCompile: vi.fn() }));
vi.mock("../../wasm/omnilang_wasm", () => ({
  getWasmModule: () => ({ compile_multi_to_omniview: mockCompile }),
}));

const sources = { "App.omni": "component App { }" };

describe("validateSources（MultiCompileEnvelope 适配）", () => {
  beforeEach(() => mockCompile.mockReset());

  it("从 result.result.errors 读取诊断", async () => {
    mockCompile.mockResolvedValue({
      result: {
        success: false,
        files: [],
        errors: [{ file: "App.omni", line: 3, column: 1, message: "Undeclared variable 'x'", code: "E0001" }],
        warnings: [],
        components: [],
        source_map: [],
      },
      cache_state: {},
    });
    const errors = await validateSources(sources);
    expect(errors).toHaveLength(1);
    expect(errors[0]).toMatchObject({ message: "Undeclared variable 'x'" });
  });

  it("无错误 → 空数组", async () => {
    mockCompile.mockResolvedValue({ result: { success: true, files: [], errors: [], warnings: [], components: [], source_map: [] }, cache_state: {} });
    expect(await validateSources(sources)).toEqual([]);
  });

  it("非 envelope 返回值（旧形状兼容）→ 读顶层 errors", async () => {
    mockCompile.mockResolvedValue({ errors: [{ message: "legacy" }] });
    expect(await validateSources(sources)).toHaveLength(1);
  });

  it("WASM 调用失败 → 空数组（门禁放行，记录可诊断）", async () => {
    mockCompile.mockRejectedValue(new Error("wasm down"));
    expect(await validateSources(sources)).toEqual([]);
  });
});
```

> 实现前先读 `diagnostics.ts` 顶部确认 `getWasmModule`/`WasmDiagnostic` 的真实导出名与 `validateSources` 当前签名（sources 参数形状、是否 async）——以实际文件为准调整 mock 路径。

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/monaco/__tests__/validate-sources.test.ts`
Expected: FAIL — 用例 1 期望 1 个错误，实际 `[]`（旧实现读错信封层）

- [ ] **Step 3: 最小实现**

```ts
// diagnostics.ts validateSources 内，Array.isArray 检查后：
if (result && typeof result === "object" && "result" in result) {
  // MultiCompileEnvelope：compile_multi_to_omniview 包了两层（omnilang-wasm lib.rs:81-85）
  const inner = (result as { result?: { errors?: WasmDiagnostic[] } }).result;
  return inner?.errors ?? [];
}
if ("errors" in result) {
  return (result as { errors?: WasmDiagnostic[] }).errors ?? [];
}
return [];
```

- [ ] **Step 4: 运行确认通过 + 门禁行为验证**

Run: `cd poiesis && npx vitest run src/services/monaco/__tests__/validate-sources.test.ts`
Expected: PASS（4 用例）
再跑全量 `npx vitest run`——**注意预期行为变化**：修复后此前静默落盘的非法源（如 `else` 在 `elif` 前）会被保存门禁拦下；若既有测试依赖"非法源保存成功"需更新断言（逐一列出）。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/monaco/diagnostics.ts poiesis/src/services/monaco/__tests__/validate-sources.test.ts
git commit -m "fix(canvas): 保存 WASM 门禁修复——validateSources 读 MultiCompileEnvelope 内层 errors（一处修复 design/store/component 三处门禁）"
```

---

### Task 2: condition 分支 id/索引漂移修复

**Files:**
- Modify: `poiesis/src/services/codegen/graph-compiler.ts:143-193`（emitNode condition 分支）
- Modify: `poiesis/src/components/canvas/code/nodes/ConditionNode.tsx:39-46`（handleAddBranch id 防碰撞）
- Test: `poiesis/src/services/codegen/__tests__/graph-compiler-branch-id.test.ts`（新建）

**Interfaces:**
- Consumes: 分支元素形状 `{ id: string; type: "if"|"elif"|"else"; condition: string }`；wires 的 fromPort = 分支真实 id（连线时 handle id）
- Produces: `compileCodeFlowGraph` 的分支端口 key 用分支真实 id（不再用数组索引 `br${i}`）

- [ ] **Step 1: 写失败测试**（构造 id≠索引的数组，验证编译不丢分支）

```ts
// graph-compiler-branch-id.test.ts
// 分支 id 与数组索引不一致时（历史源/手写编辑产物），编译必须用分支真实 id 作为端口 key，
// 否则 branchSuccessors 查空 → 分支后继整段静默丢失。
import { describe, it, expect } from "vitest";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

const el = (id: string, tag: string, props: Record<string, unknown> = {}): CodeblockElementData =>
  ({ id, nodeType: "codeblock", tag, parentId: null, source: "", props });

describe("compileCodeFlowGraph 分支端口（用真实分支 id）", () => {
  it("id 与索引不一致的数组（[br0, br2, br1]）编译不丢分支", () => {
    const elements = [
      el("fn1", "fn", { name: "f", params: "" }),
      el("c1", "condition", {
        branches: [
          { id: "br0", type: "if", condition: "a > 1" },
          { id: "br2", type: "elif", condition: "a > 0" }, // 注意：id≠索引（br2 在索引 1）
          { id: "br1", type: "else", condition: "" },
        ],
      }),
      el("cb1", "code-block", { description: "then-body" }),
      el("cb2", "code-block", { description: "elif-body" }),
      el("cb3", "code-block", { description: "else-body" }),
    ];
    const wires: WireData[] = [
      { id: "w0", fromId: "fn1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w1", fromId: "c1", fromPort: "br0", toId: "cb1", toPort: "flow-in" },
      { id: "w2", fromId: "c1", fromPort: "br2", toId: "cb2", toPort: "flow-in" },
      { id: "w3", fromId: "c1", fromPort: "br1", toId: "cb3", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("then-body");
    expect(out).toContain("elif-body");
    expect(out).toContain("else-body");
  });

  it("正常索引序数组行为不变（if/elif/else 全发射）", () => {
    const elements = [
      el("fn1", "fn", { name: "f", params: "" }),
      el("c1", "condition", {
        branches: [
          { id: "br0", type: "if", condition: "a > 1" },
          { id: "br1", type: "else", condition: "" },
        ],
      }),
      el("cb1", "code-block", { description: "if-body" }),
      el("cb2", "code-block", { description: "else-body" }),
    ];
    const wires: WireData[] = [
      { id: "w0", fromId: "fn1", fromPort: "flow-out", toId: "c1", toPort: "flow-in" },
      { id: "w1", fromId: "c1", fromPort: "br0", toId: "cb1", toPort: "flow-in" },
      { id: "w2", fromId: "c1", fromPort: "br1", toId: "cb2", toPort: "flow-in" },
    ];
    const out = compileCodeFlowGraph(elements, wires, "fn1");
    expect(out).toContain("if-body");
    expect(out).toContain("else-body");
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-compiler-branch-id.test.ts`
Expected: 用例 1 FAIL — `elif-body` 缺失（compiler 用 `br${i}` 查 `branchSuccessors.get("br1")` 取到 else 的线，elif 后继丢失）

- [ ] **Step 3: 最小实现**

```ts
// graph-compiler.ts emitNode condition 分支（158 行附近）：
// 旧：const brPortId = `br${i}`;  // 索引——注释自称"优先用 br.id"但代码忽略
// 新：
const brPortId = br.id; // 分支真实 id：连线 handle 即用分支 id（ConditionNode 每分支一端口），
                       // 解析/UI 追加均产生唯一 id；用真实 id 消除索引失配丢分支

// ConditionNode.tsx handleAddBranch（41 行附近）：
// 旧：id: `br${branches.length}`  —— 追加序=索引，但若数组曾乱序会重复
// 新：id 基于现有 id 去重的 max+1：
const nextId = branches.reduce((max, b) => Math.max(max, parseInt((b.id ?? "br0").replace(/^br/, ""), 10) || 0), 0) + 1;
// 追加分支用 { id: `br${nextId}`, ... }
```

- [ ] **Step 4: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/services/codegen/__tests__/graph-compiler-branch-id.test.ts`
Expected: PASS（2 用例）
全量 `npx vitest run` 确认无回归（condition round-trip 测试重点）。

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/graph-compiler.ts poiesis/src/components/canvas/code/nodes/ConditionNode.tsx poiesis/src/services/codegen/__tests__/graph-compiler-branch-id.test.ts
git commit -m "fix(canvas): condition 分支端口用真实分支 id（消除 br\${i} 索引失配——id≠索引时分支体静默丢失）+ UI 分支 id 防碰撞"
```

---

### Task 3: 节点位置持久化（project.json 链）

**Files:**
- Modify: `poiesis/src/core/types/project-json.ts:82-84`（CodeFlowNode 加 position）
- Modify: `poiesis/src/services/codegen/chain-emitter.ts:287-316`（buildCodeFlowGraphs 写 el.position）
- Modify: `poiesis/src/store/slices/serialization.ts`（loadFromProjectJson 保存 positions 到独立字段）
- Modify: `poiesis/src/store/index.ts` 或 types（新状态字段 persistedPositions）
- Modify: `poiesis/src/store/slices/element-crud.ts:173-212`（materializeFnGraphs 消费持久化 position）
- Test: `poiesis/src/store/__tests__/position-persistence.test.ts`（新建）+ chain-emitter 测试

**Interfaces:**
- Consumes: `CodeblockElementData.position`（{x,y}）；保存链 `useSaveProject` → `toProjectJson` → `buildCodeFlowGraphs`；加载链 `loadFromProjectJson`
- Produces:
  ```ts
  // CodeFlowNode（project-json.ts）加字段
  position?: { x: number; y: number };
  // store 新状态（types.ts）
  persistedPositions: Record<string, { x: number; y: number }>;  // pid → position（project.json 读回）
  ```

- [ ] **Step 1: 先探索保存/加载链的两个落点**（实现者开工第一步）

1. `project.json` 文件在磁盘的位置与读取 API（`annotation.saveProject` 写到哪；加载时哪个 API 能读——`api.annotation` 系列或直接 fs read；参考 `App.tsx loadOmniFiles` 的 fs 读取方式）
2. 保存侧实时图构建：`buildCodeFlowGraphs` 的输入 `s.codeFlowGraphs` 是解析侧快照（画布编辑不回流）——需要把保存时刻的 `store.elements/wires` 按 fn 归属构建实时图（**复用 materializeFnGraphs 的归属校验逻辑** element-crud.ts:180-188：fn 元素 id === fnRegistry.pid 才纳入），或最小化：仅把 position 单独写入（独立字段 persistedPositions 直接从 store.elements 收集，不经 codeFlowGraphs）——**后者更小**：保存时 `persistedPositions = elements.filter(codeblock).map(e => [e.id, e.position]).filter(有 position)`，写进 project.json 独立字段；加载时读回赋给物化元素

探索结论写入报告，选择最小方案实现（倾向独立 positions 字段而非改 codeFlowGraphs 形状——避免触碰 v3 图格式）。

- [ ] **Step 2: 写失败测试**（以最小方案为准——独立 positions 字段）

```ts
// position-persistence.test.ts（store 层：保存收集 + 加载赋回）
// —— 具体断言以 Step 1 探索结论为准，核心场景：
// 1. 保存侧：elements 中带 position 的 codeblock 元素 → positions 收集（pid → {x,y}），无 position 的跳过
// 2. 加载侧：materializeFnGraphs 物化时，元素 id 命中 persistedPositions → 用持久化 position（不再行布局兜底）
// 3. 幂等：重复物化 position 不变
// 4. 无 persistedPositions 数据 → 回退既有确定性行布局（Task 8 行为不回归）
```

- [ ] **Step 3: 运行确认失败**

Run: `cd poiesis && npx vitest run src/store/__tests__/position-persistence.test.ts`
Expected: FAIL（新功能未实现）

- [ ] **Step 4: 最小实现 + 端到端验证**

实现保存收集 + project.json 读写 + 加载赋回。验收：真实保存 → reload → 进入 codeflow → **节点布局保持**（E2E 验证，Victauri）。

- [ ] **Step 5: 全量回归 + 提交**

```bash
git add -A
git commit -m "feat(canvas): 节点位置持久化——保存写 positions 进 project.json，加载物化时赋回（reload 布局保持）"
```

---

### Task 4: condition/return 公式框扩展

**Files:**
- Modify: `poiesis/src/components/canvas/code/nodes/ConditionNode.tsx`（条件字段 NodeTextField → NodeFormulaField；Props 补 scope/letVars 类型）
- Modify: `poiesis/src/components/canvas/code/nodes/ReturnNode.tsx`（同上，提交仍走 onCommitReturn → setElementSource）
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx`（追加用例）

**Interfaces:**
- Consumes: `NodeFormulaField`/`FormulaCandidateSource`（fields/）；`buildExpressionCandidates`（flow-scope）；FlowCanvas 已注入 data.scope/letVars（FlowCanvas.tsx:319-325 的 spread 已覆盖 condition/return——只是节点未消费）
- Produces: ConditionNode/ReturnNode 消费 scope/letVars，条件/表达式字段用公式框（无 scope 时退化纯文本）

- [ ] **Step 1: 写失败测试**（追加到既有 ConditionReturnNode.test.tsx）

```tsx
// 追加 describe（沿用既有 render/mock 方式；scope fixture 参考 SyntaxNode.test.tsx）
// 前置：先读 ConditionNode.tsx/ReturnNode.tsx 确认 data props 真实字段名（分支行渲染结构），
// 断言选择器以实际组件为准微调；以下为基准形态。
import { ConditionNode } from "../ConditionNode";
import { ReturnNode } from "../ReturnNode";

const scope = {
  signals: [{ name: "count", type: "i32" }],
  props: [], computeds: [], events: [],
  fns: [], stores: [], fnParams: [{ name: "step", type: "i32" }],
  letVars: [{ name: "total" }],
};

describe("ConditionNode 公式框", () => {
  const data = {
    branches: [{ id: "br0", type: "if", condition: "a > 1" }],
    scope,
    letVars: [{ name: "total" }],
    onEditCondition: vi.fn(),
  };

  it("条件字段渲染公式框（data-field-kind=formula）", () => {
    const { container } = render(<ConditionNode data={data} />);
    expect(container.querySelector('[data-field-kind="formula"]')).toBeTruthy();
  });

  it("条件输入字母弹候选（scope 信号）", () => {
    const { container } = render(<ConditionNode data={data} />);
    const input = container.querySelector('[data-field-kind="formula"]') as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    expect(screen.getByRole("listbox")).toBeTruthy();
    expect(screen.getByText("count")).toBeTruthy();
  });

  it("提交走 onEditCondition（条件文本更新到 branches）", () => {
    const onEditCondition = vi.fn();
    const { container } = render(<ConditionNode data={{ ...data, onEditCondition }} />);
    const input = container.querySelector('[data-field-kind="formula"]') as HTMLInputElement;
    fireEvent.change(input, { target: { value: "count > 0", selectionStart: 9 } });
    fireEvent.blur(input);
    expect(onEditCondition).toHaveBeenCalledWith("br0", "count > 0");
  });

  it("无 scope → 退化纯文本不弹层", () => {
    const { container } = render(<ConditionNode data={{ ...data, scope: undefined }} />);
    const input = container.querySelector('[data-field-kind="formula"]') as HTMLInputElement;
    fireEvent.change(input, { target: { value: "cou", selectionStart: 3 } });
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});

describe("ReturnNode 公式框", () => {
  it("表达式字段渲染公式框且提交走 onCommitReturn", () => {
    const onCommitReturn = vi.fn();
    const { container } = render(
      <ReturnNode data={{ returnExpr: "", scope, letVars: [], onCommitReturn }} />,
    );
    const input = container.querySelector('[data-field-kind="formula"]') as HTMLInputElement;
    fireEvent.change(input, { target: { value: "count + 1", selectionStart: 9 } });
    fireEvent.blur(input);
    expect(onCommitReturn).toHaveBeenCalledWith("count + 1");
  });
});
```

> 注意：else 分支行条件不可编辑（ConditionNode 既有行为）——公式框只替换 if/elif 行的编辑控件；断言时确保选中 if 行。

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx`
Expected: FAIL（仍是 NodeTextField）

- [ ] **Step 3: 最小实现**

```tsx
// ConditionNode.tsx：
// - Props 接口补：scope?: FlowScope; letVars?: Array<{ name: string }>;
// - formulaSource = useMemo(buildFormulaSource(scope, letVars))（复用 SyntaxNode.tsx:69-82 的组装模式——如已存在共享函数则直接引用；否则在 flow-scope.ts 导出 buildFormulaSource 供三处共用，SyntaxNode 同步改用）
// - 条件编辑 NodeTextField → NodeFormulaField（提交仍走 onEditCondition(branchId, v)）

// ReturnNode.tsx：同模式，提交走 onCommitReturn(v)
```

> **共享抽提**：SyntaxNode.tsx:69-82 的 formulaSource 组装（buildExpressionCandidates + filter/storeSignals）若三处复用，抽到 flow-scope.ts 导出 `buildFormulaSource(scope, letVars)`——SyntaxNode/ConditionNode/ReturnNode 三处调用（DRY，Task 9 的 minor 顺带解决）。

- [ ] **Step 4: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx src/components/canvas/code/nodes/__tests__/SyntaxNode.test.tsx`
Expected: 新用例 PASS + 既有不回归

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/services/codegen/flow-scope.ts poiesis/src/components/canvas/code/nodes/ConditionNode.tsx poiesis/src/components/canvas/code/nodes/ReturnNode.tsx poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx poiesis/src/components/canvas/code/nodes/__tests__/ConditionReturnNode.test.tsx
git commit -m "feat(canvas): condition/return 公式框——条件与表达式字段触发式补全（buildFormulaSource 三处共用）"
```

---

### Task 5: args 公式框（NodeArgsRows candidates）

**Files:**
- Modify: `poiesis/src/components/canvas/code/fields/NodeField.tsx:120-186`（NodeArgsRows 加可选 candidates）
- Modify: `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx:184-188,212-216`（两处调用传 formulaSource）
- Test: `poiesis/src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx`（追加用例）

**Interfaces:**
- Consumes: `FormulaCandidateSource`（NodeFormulaField.tsx:9-14）
- Produces: `NodeArgsRows` 新增可选 prop `candidates?: FormulaCandidateSource`——内部每行 NodeTextField → NodeFormulaField（无 candidates 退化纯文本，向后兼容）

- [ ] **Step 1: 写失败测试**

```tsx
// NodeArgsRows.test.tsx 追加
describe("NodeArgsRows candidates", () => {
  it("传 candidates 时每行渲染公式框", () => {
    // render(<NodeArgsRows values={["a"]} onChange={vi.fn()} candidates={cands} />)
    // expect(container.querySelectorAll('[data-field-kind="formula"]')).toHaveLength(1)
  });
  it("不传 candidates 时仍渲染普通文本字段（既有行为）", () => {
    // render(<NodeArgsRows values={["a"]} onChange={vi.fn()} />)
    // expect(container.querySelector('[data-field-kind="text"]')).toBeTruthy()
  });
  it("行内输入弹候选", () => {
    // change "cou" → 候选弹层出现
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx`
Expected: FAIL（无 candidates prop）

- [ ] **Step 3: 最小实现**

```tsx
// NodeField.tsx NodeArgsRows：
// - Props 追加 candidates?: FormulaCandidateSource（import type from NodeFormulaField——注意 NodeField 与 NodeFormulaField 同目录互引：NodeFormulaField 依赖 NodeField.module.css 但不依赖 NodeField.tsx 组件，无循环；若循环则把 FormulaCandidateSource 类型移到独立文件）
// - 行内 NodeTextField → NodeFormulaField（placeholder/ariaLabel 透传）
```

- [ ] **Step 4: 运行确认通过 + 回归**

Run: `cd poiesis && npx vitest run src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx src/components/canvas/code/fields/__tests__/NodeFormulaField.test.tsx src/components/canvas/code/fields/__tests__/NodeField.test.tsx`
Expected: 全绿

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/canvas/code/fields/NodeField.tsx poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx poiesis/src/components/canvas/code/fields/__tests__/NodeArgsRows.test.tsx
git commit -m "feat(canvas): args 参数行公式框——NodeArgsRows 可选 candidates（store-call/call 参数行触发式补全）"
```

---

### Task 6: redo 按钮 + fn 双击空编辑器

**Files:**
- Modify: `poiesis/src/components/design/DesignToolbar.tsx:32-46`（加 redo 按钮）
- Modify: `poiesis/src/pages/DesignPage.tsx:146,245`（取 s.redo 传入）
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx:307`（fn 不绑双击）或 FnNode.tsx:21
- Test: `poiesis/src/components/design/__tests__/DesignToolbar.test.tsx`（追加，如存在）

**Interfaces:**
- Consumes: `store.redo`（undo-slice.ts:89-111 已存在）
- Produces: DesignToolbar `onRedo` prop

- [ ] **Step 1: 写失败测试**

```tsx
// DesignToolbar.test.tsx 追加（如该文件不存在则新建，mock onUndo/onRedo）
describe("DesignToolbar redo", () => {
  it("渲染 redo 按钮且点击触发 onRedo", () => {
    const onRedo = vi.fn();
    render(<DesignToolbar onUndo={vi.fn()} onRedo={onRedo} />);
    fireEvent.click(screen.getByLabelText(/redo|重做/i));
    expect(onRedo).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: 运行确认失败** → **Step 3: 最小实现**

```tsx
// DesignToolbar.tsx：Props 加 onRedo?: () => void；undo 按钮旁加 redo 按钮（同风格）
// DesignPage.tsx：const redo = usePoiesisStore((s) => s.redo); → onRedo={redo}
// FnNode/FlowCanvas：fn 节点不绑 onDoubleClick（移除 FnNode.tsx:21 的绑定或 FlowCanvas data 中 fn 不传 onDoubleClick——fn 双击改行为：聚焦选中即可，参数编辑走 FlowInspector 表单）
```

- [ ] **Step 4: 运行确认通过 + 回归**（undo/redo 交互测试 + 全量）

- [ ] **Step 5: 提交**

```bash
git add poiesis/src/components/design/DesignToolbar.tsx poiesis/src/pages/DesignPage.tsx poiesis/src/components/canvas/code/nodes/FnNode.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/design/__tests__/DesignToolbar.test.tsx
git commit -m "feat(canvas): redo 按钮（DesignToolbar）+ fn 双击不再打开空 Monaco 编辑器"
```

---

### Task 7: 延期 minor 清理

**Files:**
- Modify: `poiesis/src/components/canvas/code/fields/expression-trigger.ts`（显式化 + 运算符串多字符回扫 + 约定注释 + 转义引号）
- Modify: `poiesis/src/components/canvas/code/fields/NodeFormulaField.tsx`（openMenu 死参数）
- Modify: `poiesis/src/services/codegen/fn-topology.ts`（fn 边界截断 + id→element Map）
- Modify: `poiesis/src/services/codegen/graph-parser.ts` 测试（view {expr} 裸文本专测，锁定 651bae7a）
- Modify: `poiesis/src/services/codegen/parser.ts`（pendingPid 跨成员泄漏注释说明）
- Modify: `poiesis/src/store/slices/element-crud.ts`（空合并短路）
- Test: 各文件对应测试追加

**Interfaces:** 无新接口——纯清理，行为不变（除明确标注的加固）。

- [ ] **Step 1: 逐项清理（每项：改代码 + 补/改测试 + 单独 commit）**

1. `expression-trigger.ts`：
   - `before[cursor - 1]!` → `before[cursor - 1] ?? ""`（显式化，Task1 minor）
   - at-cursor 分支补注释说明约定（"光标压在运算符字符上"）
   - 运算符串回扫改**多字符**（`OPERATOR_CHAR` 连续收集——现状单字符粒度，光标在运算符串中间时 token 截断）：`while (i > 0 && OPERATOR_CHAR.test(before[i-1])) i--` 已收集连续串？——核实现状：`getTriggerContext` 的运算符分支若只回扫一个字符则改连续回扫；测试补"光标在 `<=` 中间"用例
   - `countUnbalancedParens` 补转义引号跳过（`\` 后字符不结束引号）
2. `NodeFormulaField.tsx`：openMenu 的未用参数清理
3. `fn-topology.ts`：BFS 命中 fn 节点截断（不跨 fn）+ `elements.find` → 预先建 `id → element` Map（O(N) 预处理）
4. `parser.ts`：pendingPid 跨成员泄漏处补注释（说明"紧邻 fn 的 @pid 注释为有效前提"）
5. `element-crud.ts`：materializeFnGraphs 空合并短路（newElements/newWires 均空时返回原引用）
6. `graph-parser` 测试：view 正文裸文本 `{expr}` 用例（锁定 651bae7a 修复——`view { Hello {name} }` 解析后 designElements 文本保持）

- [ ] **Step 2: 每项运行对应测试 + 全量回归**

Run: `cd poiesis && npx vitest run`（397 + 新增全绿）

- [ ] **Step 3: 提交（按项拆分或合并为 1-2 个清理 commit）**

```bash
git commit -m "refactor(canvas): minor 清理——表达式触发显式化/多字符回扫/转义引号、fn 拓扑边界截断 + Map 索引、空合并短路、view {expr} 专测"
```

---

### Task 8: 全量回归 + E2E + 文档

**Files:** 无代码改动（除回归修复）

- [ ] **Step 1: 全量测试 + tsc**

Run: `cd poiesis && npx vitest run && npx tsc --noEmit`
Expected: 全绿（397 + 各任务新增）；tsc 零新增错误（**Task 1 修复后 diagnostics.ts:151 基线错误应消失**——确认）

- [ ] **Step 2: Victauri E2E（重点交互）**

1. **门禁**：构造非法源（如 condition 空条件 `if  {`）→ 保存 → 断言被拦（错误提示出现）——此前静默放行
2. **分支漂移**：画布 condition 节点 → +elif/+else → 各分支连线节点 → 保存 → reload → 分支体保持
3. **位置持久化**：拖拽节点布局 → 保存 → reload → 进入 codeflow → 布局保持（Task 3 验收）
4. **公式框扩展**：condition 条件输入字母弹候选；return 表达式弹候选；store-call/call 参数行弹候选
5. **redo**：编辑节点 → undo → redo → 恢复
6. **fn 双击**：双击 fn 不再打开空白 Monaco
7. 回归抽查：公式框核心（value 字段）、F1 reload 恢复、round-trip

- [ ] **Step 3: 文档更新**

- ADR-060（或扩展 ADR-059）：位置持久化决策（独立 positions 字段 vs codeFlowGraphs 形状）、分支真实 id 决策、门禁信封适配
- memory：会话记录 + poiesis-features 更新

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "docs: ADR-060 遗留清理决策（位置持久化/分支真实 id/门禁信封适配）+ 全量回归通过"
```

---

## 任务依赖图

```
Task 1 (门禁) ─────────────┐
Task 2 (分支 id) ──────────┤（同批验证：门禁修好后非法分支序会被拦）
Task 3 (位置持久化) ────────┤
Task 4 (condition/return) ─┼─→ Task 8 (回归 + E2E + 文档)
Task 5 (args 公式框) ──────┤
Task 6 (redo + fn 双击) ───┤
Task 7 (minor 清理) ───────┘
```

Task 1-7 相互独立，可并行推进（按顺序执行亦可）；Task 8 收尾统一验证交互（门禁 × 分支序 × 布局 × 公式框）。
