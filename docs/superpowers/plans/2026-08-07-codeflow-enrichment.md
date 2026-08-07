# CodeFlow Canvas 丰富升级实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 CodeFlow Canvas 能生成 `test/projects/learn-omniview/src` 中除 view 内容与样式外的全部语法：生命周期（@mount/@effect/@unmount）、try/catch/finally、throw/await、组件 action、lib 文件 pub/async fn + struct/type/import 全链路（解析 → 画布 → 保存 → WASM 门禁 round-trip）。

**Architecture:** 三宿主（组件 tab / store tab / code tab）复用同一 FlowCanvas；ROOT 层声明统一收进 `fileDefs: Record<filePath, LibFileDef>` 模型；新语法全部复用现有承载机制（SyntaxNode 轻量 case / FlowInspector 表单 / ConditionNode 分支行模式 / StoreMembersPanel 面板模式），不造新 UI 组件。解析-发射 round-trip 以 `// @pid:xxx` 注释为稳定性基础。

**Tech Stack:** TypeScript / React 19 / Zustand v5 / Vitest / CodeFlow Canvas（ReactFlow-based，poiesis 前端）+ OmniLang 编译器（WASM 门禁）。

## Global Constraints

- **设计原则（用户拍板）**：节点编辑器承载逻辑架构（语句/块/分支/调用链），细节交给右侧 FlowInspector 属性面板与公式框；复用现有承载机制不造新 UI 组件；round-trip 以 `// @pid:xxx` 注释为稳定性基础。
- **生命周期节点**：单例（每组件每种 hookType 仅一个）；deps 仅 effect 发射括号形式 `@effect(deps)`。
- **try 分支语义**（BNF `try_catch_stmt`）：try 必需；catch ≤ 1；finally ≤ 1；至少 catch 或 finally 之一。UI 层 guard（创建/按钮禁用），不阻塞保存——WASM 门禁兜底。
- **emitLibFile 顺序**：imports → structs → types → fns；**不发射 `import omniview;`**（纯逻辑文件不写）。
- **组件/store 文件保存**：ROOT 层 structs/types 前置发射在 `component X {` / `store X {` 之前（FeatureCard/TodoStore 头部 struct 保存后不丢）；imports 不前置（pipeline import-collector 已处理）。
- **范围外**（spec §8）：struct/type 仅 ROOT 层（不做成员内声明）；tagged union 不做结构化表单（类型表达式文本承载）；组件内 import 编辑不做；class 不做（Design Canvas 域）。
- **测试命令**：`cd poiesis && npm test -- <测试文件>`（package.json `"test": "vitest run"`）；类型检查 `cd poiesis && npx tsc --noEmit`。每任务以 tsc + 新增测试 + 既有测试套件全绿为完成门槛。
- **commit 格式**：`<type>: <description>`（git-workflow 全局规则；无 Co-Authored-By 归属）。

## File Structure Map

**新建：**
- `poiesis/src/store/slices/lib-defs.ts` — libDefs slice（setLibDef + materializeLibGraphs）
- `poiesis/src/services/codegen/emit-lib-file.ts` — emitLibFile / emitRootDeclarations
- `poiesis/src/components/canvas/code/nodes/LifecycleNode.tsx` — 生命周期块节点
- `poiesis/src/components/canvas/code/nodes/TryNode.tsx` — try 块节点（分支行模式）
- `poiesis/src/components/code/LibMembersPanel.tsx` — code tab 常驻左面板
- 测试文件（各任务内列出）

**修改（按任务）：**
- T1: `services/codegen/types.ts`、`omni-parser.ts:16`、`parser.ts:249-318`（+ 新方法）
- T2: `store/types.ts:26`、`store/index.ts`、`App.tsx:94-96`、`App.tsx` 批量应用、`pages/DesignPage.tsx`（物化 effect）
- T3: `parser.ts:387-436`（action case）、`graph-compiler.ts:91-115`、`source-emitter.ts:317-339`、`pipeline.ts:276-327`
- T4: `services/codegen/graph-parser.ts`、`services/codegen/syntax-nodes.ts`、`graph-compiler.ts`、`nodes/SyntaxNode.tsx`、`components/canvas/code/flowConstants.ts`、`FlowCanvas.tsx:43-55`
- T5: `components/canvas/code/FlowInspector.tsx:159-161,193-273`
- T6: `parser.ts:345-347,438-453`（AT case 重构）、`parser-types.ts`、`graph-parser.ts`、`graph-compiler.ts`、`source-emitter.ts:339-341`、`pipeline.ts:87-94,276-327`、`component-builder.ts`、`types.ts`、`core/types/project-json.ts:100-108`、`store/types.ts`、`store/index.ts`、`store/slices/element-crud.ts:176-221`、`App.tsx`
- T7: `nodes/LifecycleNode.tsx`（新）、`FlowCanvas.tsx:43-55`、`FlowInspector.tsx`、`flowConstants.ts`
- T8: `graph-parser.ts`、`graph-compiler.ts`、`nodes/TryNode.tsx`（新）、`FlowCanvas.tsx:43-55`、`flowConstants.ts`、`types.ts`
- T9: `emit-lib-file.ts`（新）、`hooks/useSaveProject.ts:139-185,216-259`
- T10: `components/code/LibMembersPanel.tsx`（新）、`pages/DesignPage.tsx:224-235,258,302`

**任务顺序与依赖**：T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 → T9 → T10。
（T9 依赖 T4/T8 的编译器 case——error_handling round-trip 需要 throw/await/try 发射；T6 依赖 T3 的 synthesizeFnEntry entryTag 扩展。）

---

## 关键锚点速查（实现者必读）

所有行号以本计划撰写时的文件为准，改动前先按文件名+描述定位。

| 锚点 | 位置 | 用途 |
|------|------|------|
| parseRootIdentifier（if-chain，每个 case 自行 consume 关键字） | parser.ts:249-318，317 行 unknown-skip | T1 加 case |
| parseComponentBody 的 AT case | parser.ts:345-347（`case "AT": this.parseAtStore(); break;`） | T6 重构 |
| parseComponentKeyword（switch on kw text） | parser.ts:387-436；store case 413-430 是签名收集参考 | T3 加 action case |
| parseAtStore 首行 `this.stream.consume(); // @` | parser.ts:438-439 | T6 重构后不消费 @ |
| parseFn（签名/body 收集 + synthesizeFnEntry + fnRegistry） | parser.ts:557-619 | T1/T3/T6 镜像模板 |
| synthesizeFnEntry(fnName, rawParams, returnType, fnPid, elements, wires) | parser.ts:628-656 | T3 扩展 entryTag/propsExtras |
| parseComponentComment pendingPid 消费方文档注释 | parser.ts:376-379 | T3/T6 更新注释 |
| TokenStream API | token-stream.ts：peek(offset)/consume()/match(kind)/expect(kind,msg)/isEof()/skipNewlines() | 全部解析任务 |
| 已实证 TokenKind：IDENTIFIER/BRACE_OPEN/BRACE_CLOSE/COLON/EQUALS/SEMICOLON/COMMA/NEWLINE/AT/EOF | parser.ts 各 expect/switch | 解析任务；`(`/`)`/`,` 用 text 比较（kind 名未实证） |
| graph-compiler emitNode：resolvePid 59-61 / emitPidComment 64-66 / fn case 91-115 / code-block 117-141 / condition 143+ | graph-compiler.ts | T3/T6/T8 改 case；emitPidComment(pid) 返回 `// @pid:<pid>\n` |
| source-emitter fn loop（332 pid 注释 / 333 header `fn ${name}(${params}) -> ${returnType} {` / 338 blank / 341 `// View`） | source-emitter.ts:317-341 | T3 改 333 双格式；T6 插 339-341 之间 |
| pipeline fn 注入循环（277 filter / 284 compile / 291-295 slice / 298-303 existing 回填 / 317-323 新 push `{name, params, returnType: props||"void", body, pid}`） | pipeline.ts:276-327 | T3 改 283 tag filter + 298-303 + 317-323；T6 在 327 后加 lifecycle 循环 |
| defaultLoweredCode | pipeline.ts:87-94 | T6 加 lifecycles: [] |
| component-builder 两处 `fns: phaseB.lowered.functions` | component-builder.ts ~150 与 ~162 | T6 旁加 lifecycles |
| FnDef 76-84 / ValidComponentPart 86-98 / PartialComponentPart 100-113 / LoweredCode 271-276 / ParseResult 286-338（codeFlowGraphs 310 / fnRegistry 313） | services/codegen/types.ts | T1/T3/T6/T8 类型扩展 |
| ComponentSourceDef（fns: Array<FnDef> 107） | core/types/project-json.ts:100-108 | T6 加 lifecycles |
| PoiesisState | store/types.ts:26（theme 段后为 designPage 等；codeFlowGraphs/persistedPositions 在 state 中） | T2/T6 加字段 |
| create<PoiesisState> 初始值（codeFlowGraphs: {} 于 54、persistedPositions: {} 于 55）+ slice 工厂 spread | store/index.ts:1-70 | T2/T6 加初始值与注册 |
| createElementCrudSlice(set: SetFn, get: GetFn) 工厂形态 + SetFn/GetFn 类型 | store/slices/element-crud.ts:1-35（materializeFnGraphs 176-221） | T2 slice 镜像 |
| App.tsx loadFiles（94 parseOmniSource / 96 过滤） | App.tsx:85-100 | T1/T2 |
| App.tsx 批量应用（stateUpdate 262-269 / mergedGraphs 278 / main 分支 292-308 / else 分支 309-324 / setState 340-351；materializeFns 调用 307/319） | App.tsx:255-359 | T2/T6 |
| DesignPage 组件物化 effect（167-176）/ grid（224-235）/ 中心画布 gridColumn（~302）/ 裸 `<CodeFlowCanvas />`（349） | pages/DesignPage.tsx | T2/T7/T10 |
| useSaveProject：store 分支 139-183（updateStoreAction 154-160 / emitStoreDefinition 163 / collectProjectSources+validateSources 170-177 / writeFile 179 / markClean 180-181）/ 组件分支 216-259（generateOmniFiles 216-227 / `if (genResult.value)` 229 / 写盘循环 247-249 / setComponentDef 269-297） | hooks/useSaveProject.ts | T9 |
| FlowCanvas nodeTypes（43-55：fn/code-block/condition/return/let/assign/store-call/call/while/for-in） | components/canvas/code/FlowCanvas.tsx | T4/T7/T8 |
| FlowInspector 分派（159 fn / 163 return / SYNTAX_TAGS 121 / 171 loop）+ FnInspector 193-273 | components/canvas/code/FlowInspector.tsx | T5/T6 |
| flowConstants：TAG_CN 5-16 / CODE_TAGS 18-29 / MENU_SECTIONS 42-68（控制流 45-53 / 变量与赋值 55-60 / 调用 62-67） | components/canvas/code/flowConstants.ts | T4/T7/T8 |
| parseOmniSource 导出 | omni-parser.ts:16（`parseOmniSource(source: string): ParseResult`，thin facade：tokenize → TokenStream → Parser.parse → pidMap） | T1 加 filePath 参数 |

**既有测试模板**（新测试镜像这些文件的形态）：
- 解析/发射：`services/codegen/__tests__/graph-parser-syntax.test.ts`、`graph-compiler-syntax.test.ts`、`syntax-nodes.test.ts`、`syntax-roundtrip.test.ts`、`fn-synthesis.test.ts`、`fn-node-pipeline.test.ts`、`fn-registry.test.ts`、`graph-compiler-branch-id.test.ts`
- store：`store/__tests__/materialize-fn-graphs.test.ts`、`position-persistence.test.ts`
- 组件：`components/canvas/code/__tests__/FlowInspector.test.tsx`、`nodes/__tests__/SyntaxNode.test.tsx`、`nodes/__tests__/LoopNode.test.tsx`
- 保存（mock 门禁）：`services/codegen/__tests__/save-inline-store.test.ts`

---

### Task 1: ROOT 层声明解析（fileDefs 模型 + parseRootIdentifier 扩展，修复 G2/G3）

**Files:**
- Modify: `poiesis/src/services/codegen/types.ts`（新增 LibFileDef/StructDef/TypeDef；FnDef 加 pub/isAsync/declKind；ParseResult 加 fileDefs）
- Modify: `poiesis/src/services/codegen/omni-parser.ts:16`（parseOmniSource 加可选 filePath）
- Modify: `poiesis/src/services/codegen/parser.ts`（构造函数加 filePath；parseRootIdentifier 249-318 加 pub/async/struct/type/fn case；新方法 parseRootFn/parseRootStruct/parseRootType/currentLibDef/ensureBodyFallback；parseFn 557-619 末尾加 ensureBodyFallback 调用）
- Test: `poiesis/src/services/codegen/__tests__/parser-lib-decls.test.ts`（新建）

**Interfaces:**
- Produces: `LibFileDef = { imports: string[]; structs: StructDef[]; types: TypeDef[]; fns: FnDef[] }`（types.ts）
- Produces: `StructDef = { pid: string; name: string; pub: boolean; nominal: boolean; fields: Array<{ name: string; type: string }> }`
- Produces: `TypeDef = { pid: string; name: string; pub: boolean; value: string }`
- Produces: `FnDef` 扩展 `pub?: boolean; isAsync?: boolean; declKind?: "fn" | "action"`
- Produces: `ParseResult.fileDefs?: Record<string, LibFileDef>`
- Produces: `parseOmniSource(source: string, filePath?: string): ParseResult`（filePath 缺省 → fileDefs 不产出，现有调用输出不变）
- Consumes: `synthesizeFnEntry`（T3 才加 entryTag 参数——T1 用现有 4 参形态）+ `parsedFnBodies`（buildResult 已从它组装 codeFlowGraphs，parseRootFn 复用）

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/services/codegen/__tests__/parser-lib-decls.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parseOmniSource ROOT 层声明（fileDefs）", () => {
  const src = `import { sleep } from "std/async";

pub struct Point {
  x: i32,
  y: i32,
}
struct nominal UserId { value: i32, }
pub type Percentage = f64;
type ValidationResult = Valid { message: String } | Invalid { errors: Array<String> } | Pending

pub async fn fetch_user(id: u64) -> User {
  return fetch_user(id);
}
fn parse_config(file_path) {
  return 0;
}
`;

  it("struct/type/pub async fn 进入 fileDefs（按路径键）", () => {
    const r = parseOmniSource(src, "src/lib/types.omni");
    const def = r.fileDefs?.["src/lib/types.omni"];
    expect(def).toBeDefined();
    expect(def!.structs.map((s) => s.name)).toEqual(["Point", "UserId"]);
    expect(def!.structs[0]).toMatchObject({ pub: true, nominal: false,
      fields: [{ name: "x", type: "i32" }, { name: "y", type: "i32" }] });
    expect(def!.structs[1]).toMatchObject({ pub: false, nominal: true,
      fields: [{ name: "value", type: "i32" }] });
    expect(def!.types.map((t) => t.name)).toEqual(["Percentage", "ValidationResult"]);
    expect(def!.types[0]).toMatchObject({ pub: true, value: "f64" });
    // union 值：token 无空格拼接（OmniLang 空白不敏感；括号拼接保真）
    expect(def!.types[1].value).toBe("Valid{message:String}|Invalid{errors:Array<String>}|Pending");
    expect(def!.imports).toEqual([`import{sleep}from"std/async";`]);
    expect(def!.fns.map((f) => f.name)).toEqual(["fetch_user", "parse_config"]);
    expect(def!.fns[0]).toMatchObject({ pub: true, isAsync: true, returnType: "User", declKind: "fn" });
    // 无 -> 无类型参数：returnType "" / type ""（error_handling round-trip 关键）
    expect(def!.fns[1]).toMatchObject({ pub: false, isAsync: false, returnType: "" });
    expect(def!.fns[1].params).toEqual([{ name: "file_path", type: "" }]);
    expect(r.codeFlowGraphs?.["fetch_user"]).toBeDefined();
    expect(r.codeFlowGraphs?.["parse_config"]).toBeDefined();
  });

  it("无 filePath 调用（现有调用形态）输出不变——fileDefs 不产出", () => {
    const r = parseOmniSource(src);
    expect(r.fileDefs).toBeUndefined();
    expect(r.codeFlowGraphs?.["fetch_user"]).toBeDefined();
  });

  it("未注解 body 兜底为 code-block（手写源码保真，保存不丢正文）", () => {
    const r = parseOmniSource(`fn parse_config(file_path) {
  let data = fs_read(file_path);
  return data;
}
`, "src/lib/error_handling.omni");
    const g = r.codeFlowGraphs?.["parse_config"];
    expect(g).toBeDefined();
    // 入口 fn 元素 + 兜底 code-block（body 文本）
    expect(g!.elements.some((e) => e.tag === "fn")).toBe(true);
    expect(g!.elements.some((e) => e.tag === "code-block" && (e.source ?? "").includes("fs_read"))).toBe(true);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/parser-lib-decls.test.ts`
Expected: FAIL（fileDefs undefined / fns 空）

- [ ] **Step 3: types.ts 加类型**

`services/codegen/types.ts` 的 FnDef（76-84）内加：
```ts
  /** lib 文件 ROOT 层 fn 修饰符（组件 fn 默认 false/undefined，行为不变） */
  pub?: boolean;
  isAsync?: boolean;
  /** "fn" 普通函数 / "action" 组件动作（action 无返回类型，async 后缀式） */
  declKind?: "fn" | "action";
```
ParseResult（286-338）末尾加：
```ts
  /** ROOT 层顶层声明（lib/组件头部/store 头部），按文件路径键。filePath 缺省时（现有
   *  调用形态）不产出——回归关键属性。 */
  fileDefs?: Record<string, LibFileDef>;
```
文件尾部加：
```ts
export interface StructDef {
  pid: string;
  name: string;
  pub: boolean;
  nominal: boolean;
  fields: Array<{ name: string; type: string }>;
}
export interface TypeDef {
  pid: string;
  name: string;
  pub: boolean;
  /** 类型表达式文本（alias 与 tagged union 共用；token 无空格拼接） */
  value: string;
}
export interface LibFileDef {
  imports: string[];
  structs: StructDef[];
  types: TypeDef[];
  fns: FnDef[];
}
```

- [ ] **Step 4: omni-parser.ts 传 filePath**

`omni-parser.ts:16`：
```ts
export function parseOmniSource(source: string, filePath?: string): ParseResult {
  const tokens = tokenize(source);
  const stream = new TokenStream(tokens);
  const parser = new Parser(stream, filePath);
  const result = parser.parse();
  ...
```
（`Parser` 已从 parser.ts re-export，构造签名同步改——见 Step 5。）

- [ ] **Step 5: parser.ts 实现**

(a) Parser 字段与构造函数（56-59 附近）：
```ts
  private libDef: LibFileDef | null = null;
  private filePath?: string;

  constructor(stream: TokenStream, filePath?: string) {
    this.stream = stream;
    this.filePath = filePath;
    this.ctx = this.createContext();
  }
```
顶部 import 补 `import type { LibFileDef, StructDef, TypeDef } from "./types";`

(b) parseRootIdentifier（249-318）在 unknown-skip（316 行）**之前**插入：
```ts
    if (kw === "pub") {
      // pub 是前缀关键字：peek 后一个 token 分派（pub 本身不是声明类型）
      this.stream.consume(); // "pub"
      const nxt = this.stream.peek().text;
      if (nxt === "async") { this.stream.consume(); this.parseRootFn(true, true); return; }
      if (nxt === "struct") { this.parseRootStruct(true); return; }
      if (nxt === "type") { this.parseRootType(true); return; }
      if (nxt === "fn") { this.parseRootFn(true, false); return; }
      return; // 防御：pub 前缀后跟未知 token（已消费）
    }
    if (kw === "async") {
      this.stream.consume(); // "async"
      this.parseRootFn(false, true);
      return;
    }
    if (kw === "struct") { this.parseRootStruct(false); return; }
    if (kw === "type") { this.parseRootType(false); return; }
    if (kw === "fn") { this.parseRootFn(false, false); return; }
```

(c) 新方法（parseRootIdentifier 之后插入）。**注意 `(`/`)`/`,` 一律用 text 比较**（kind 名未实证）：
```ts
  /** ROOT 层顶层 fn（lib 文件，修复 G2）。镜像 parseFn 的签名/body 收集，但：
   *  returnType 默认 ""（无 -> 不发射）、参数无类型默认 ""（不补 String，
   *  error_handling.omni `fn parse_config(file_path)` round-trip 关键）。 */
  private parseRootFn(pub: boolean, isAsync: boolean): void {
    const pid = this.ctx.pendingPid || `fn_${uid()}`;
    this.ctx.pendingPid = null;
    this.stream.consume(); // "fn"

    // 签名收集（与 parseFn 相同：token + 空格拼接，直到 {）
    let signature = "";
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") { this.stream.consume(); break; }
      signature += tok.text + " ";
      this.stream.consume();
    }
    const sigMatch = signature.match(/(\w+)\s*\(([^)]*)\)/);
    const name = sigMatch?.[1] || "unnamed";

    // body 收集（花括号计数，保留 // @pid 注释——与 parseFn 相同）
    let body = "";
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") { depth++; body += "{\n"; }
      else if (tok.kind === "BRACE_CLOSE") { depth--; if (depth > 0) body += "}\n"; }
      else { body += tok.text + " "; }
      this.stream.consume();
    }

    const { elements, wires } = parseCodeFlowGraph(body);
    const rawParams = (sigMatch?.[2] || "").trim();
    const params = rawParams
      ? rawParams.split(",").map((p) => {
          const [pName, pType] = p.trim().split(":").map((s) => s.trim());
          return { name: pName || "", type: pType || "" }; // 无类型不补 String
        })
      : [];
    const returnType = signature.match(/->\s*(\w+)/)?.[1] || ""; // 无 -> 不发射
    this.synthesizeFnEntry(name, rawParams, returnType, pid, elements, wires);
    this.ensureBodyFallback(pid, body, elements, wires);
    this.parsedFnBodies.set(name, { elements, wires });
    this.currentLibDef().fns.push({ pid, name, params, returnType, pub, isAsync, declKind: "fn" });
  }

  /** body 兜底（parseFn 与 parseRootFn 共用）：图空且 body 非空 → 整块作为 code-block
   *  （手写源码保真——否则未注解 body 保存时正文丢失）。空 body 不兜底。 */
  private ensureBodyFallback(pid: string, body: string, elements: CodeblockElementData[], wires: WireData[]): void {
    if (body.trim() === "" || elements.length > 1) return;
    const cbId = `cb_${pid}_${uid()}`;
    elements.push({
      id: cbId,
      nodeType: "codeblock",
      tag: "code-block",
      parentId: null,
      props: {},
      source: body.trim(),
    });
    wires.push({ id: `w_${pid}_0`, fromId: pid, fromPort: "flow-out", toId: cbId, toPort: "flow-in" });
  }

  /** ROOT 层 struct（修复 G3）：`[pub] struct [nominal] Name { field: Type, ... }`。
   *  字段类型 token 无空格拼接（镜像 import 收集模式，OmniLang 空白不敏感）。 */
  private parseRootStruct(pub: boolean): void {
    const pid = this.ctx.pendingPid || `struct_${uid()}`;
    this.ctx.pendingPid = null;
    this.stream.consume(); // "struct"
    let nominal = false;
    if (this.stream.peek().text === "nominal") {
      nominal = true;
      this.stream.consume(); // "nominal"
    }
    const nameTok = this.stream.expect("IDENTIFIER");
    if (nameTok.kind === "EOF") {
      this.pushError("error", "P012", "Invalid struct declaration: missing name", nameTok);
      return;
    }
    const fields: Array<{ name: string; type: string }> = [];
    this.stream.expect("BRACE_OPEN");
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_CLOSE") { this.stream.consume(); break; }
      if (tok.text === ",") { this.stream.consume(); continue; } // 尾逗号容忍
      if (tok.kind === "NEWLINE") { this.stream.consume(); continue; }
      const fname = tok.text;
      this.stream.consume();
      let ftype = "";
      if (this.stream.peek().kind === "COLON") {
        this.stream.consume(); // ":"
        let t = "";
        while (!this.stream.isEof() && this.stream.peek().text !== "," && this.stream.peek().kind !== "BRACE_CLOSE") {
          if (this.stream.peek().kind !== "NEWLINE") t += this.stream.consume().text;
          else this.stream.consume();
        }
        ftype = t;
      }
      fields.push({ name: fname, type: ftype });
    }
    this.currentLibDef().structs.push({ pid, name: nameTok.text, pub, nominal, fields });
  }

  /** ROOT 层 type（修复 G3）：`[pub] type Name = <表达式文本>;`（alias 与 tagged union 共用）。
   *  收集到 SEMICOLON 或行尾（types.omni 的 union 无分号）。 */
  private parseRootType(pub: boolean): void {
    const pid = this.ctx.pendingPid || `type_${uid()}`;
    this.ctx.pendingPid = null;
    this.stream.consume(); // "type"
    const nameTok = this.stream.expect("IDENTIFIER");
    if (nameTok.kind === "EOF") {
      this.pushError("error", "P013", "Invalid type declaration: missing name", nameTok);
      return;
    }
    this.stream.expect("EQUALS");
    let value = "";
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "SEMICOLON" || tok.kind === "NEWLINE" || tok.kind === "EOF") break;
      value += tok.text; // token 无空格拼接（union 含 {} 与 |）
      this.stream.consume();
    }
    if (this.stream.peek().kind === "SEMICOLON") this.stream.consume();
    this.currentLibDef().types.push({ pid, name: nameTok.text, pub, value });
  }

  /** 当前文件的 ROOT 声明容器（filePath 为空时一次性容器不落结果——回归关键属性）。 */
  private currentLibDef(): LibFileDef {
    if (!this.libDef) this.libDef = { imports: [], structs: [], types: [], fns: [] };
    return this.libDef;
  }
```

(d) `parse()` 方法内组装 ParseResult 的代码块末尾（该处已组装 codeFlowGraphs/fnRegistry——在现有组装语句之后、返回 result 之前）追加：
```ts
    if (
      this.filePath &&
      this.libDef &&
      (this.libDef.structs.length > 0 || this.libDef.types.length > 0 ||
       this.libDef.fns.length > 0 || this.ctx.imports.length > 0)
    ) {
      result.fileDefs = { [this.filePath]: { ...this.libDef, imports: this.ctx.imports } };
    }
```
（imports 复用 ctx.imports——import case 已收集，不双写。）

(e) parseFn（557-619）在 `this.parsedFnBodies.set(fnName, ...)` 之后加一行：
```ts
    this.ensureBodyFallback(fnPid, fnBody, elements, wires);
```
（组件 fn 未注解 body 同样兜底——当前保存会丢正文，属真实缺陷修复。若既有测试断言未注解 body 不含 code-block，按新行为更新断言。）

- [ ] **Step 6: 运行测试确认通过**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/parser-lib-decls.test.ts`
Expected: PASS

- [ ] **Step 7: 回归全量 + tsc**

Run: `cd poiesis && npm test` 与 `cd poiesis && npx tsc --noEmit`
Expected: 既有套件全绿（重点是 fn-synthesis / fn-topology / graph-parser-syntax / roundtrip / code-op 系列）；tsc 无新错误。

- [ ] **Step 8: Commit**

```bash
git add poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/omni-parser.ts poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/__tests__/parser-lib-decls.test.ts
git commit -m "feat(codeflow): ROOT 层 struct/type/pub async fn 解析（fileDefs 模型 + parseRootIdentifier 扩展 + body 兜底）"
```

---

### Task 2: code tab 物化链路（libDefs slice + 加载接线 + materializeLibGraphs，修复 G4）

**Files:**
- Create: `poiesis/src/store/slices/lib-defs.ts`
- Modify: `poiesis/src/store/types.ts:26`（PoiesisState 加 libDefs/setLibDef/materializeLibGraphs）
- Modify: `poiesis/src/store/index.ts`（libDefs 初始值 + createLibDefsSlice 注册）
- Modify: `poiesis/src/App.tsx`（94 传 filePath / 96 过滤 / 批量应用 libDefs + else 分支条件）
- Modify: `poiesis/src/pages/DesignPage.tsx`（code tab 物化 effect，对标 167-176）
- Test: `poiesis/src/store/__tests__/lib-defs.test.ts`（新建，镜像 materialize-fn-graphs.test.ts）

**Interfaces:**
- Consumes: `LibFileDef` / `ParseResult.fileDefs`（T1）、`codeFlowGraphs`（现有 store 状态）
- Produces: `PoiesisState.libDefs: Record<string, LibFileDef>`（PoiesisState 定义在 store/types.ts:26）
- Produces: `setLibDef(filePath: string, def: LibFileDef): void`
- Produces: `materializeLibGraphs(filePath: string): void`——幂等合并 lib fn 图到全局 elements/wires
- Produces: `createLibDefsSlice(set, get)`——SetFn/GetFn 类型从 element-crud.ts 复制（`(partial: Partial<PoiesisState> | ((state) => Partial<PoiesisState>)) => void` 与 `() => PoiesisState`）
- Consumes: `parseOmniSource(src, filePath)`（T1）、`p.parseResult.fileDefs`、`p.parseResult.designElements`

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/store/__tests__/lib-defs.test.ts
import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { CodeblockElementData } from "../../core/types/element";

const codeblock = (id: string, tag: string, extra: Record<string, unknown> = {}): CodeblockElementData => ({
  id, nodeType: "codeblock", tag, parentId: null, props: {}, source: "", ...extra,
});

describe("libDefs slice", () => {
  beforeEach(() => {
    // 重置 store（镜像 materialize-fn-graphs.test.ts 的既有 reset 形态）
    usePoiesisStore.setState({ libDefs: {}, codeFlowGraphs: {}, elements: [], wires: [] });
  });

  it("setLibDef 按路径写入且不覆盖其他路径", () => {
    const s = usePoiesisStore.getState();
    s.setLibDef("src/lib/a.omni", { imports: [], structs: [], types: [], fns: [] });
    s.setLibDef("src/lib/b.omni", { imports: [], structs: [], types: [], fns: [] });
    const state = usePoiesisStore.getState();
    expect(Object.keys(state.libDefs)).toEqual(["src/lib/a.omni", "src/lib/b.omni"]);
  });

  it("materializeLibGraphs 幂等合并 fn 图（pid 归属校验 + 缺 position 行布局）", () => {
    const s = usePoiesisStore.getState();
    const pidA = "fn_fetch_el_aaa", pidB = "fn_parse_el_bbb";
    s.setLibDef("src/lib/a.omni", {
      imports: [],
      structs: [],
      types: [],
      fns: [{ pid: pidA, name: "fetch", params: [], returnType: "", body: "", pub: false, isAsync: true, declKind: "fn" }],
    });
    usePoiesisStore.setState({
      codeFlowGraphs: {
        fetch: {
          elements: [codeblock(pidA, "fn", { name: "fetch" }), codeblock("st_1", "let", { value: "x = 1" })],
          wires: [{ id: "w0", fromId: pidA, fromPort: "flow-out", toId: "st_1", toPort: "flow-in" }],
        },
      },
    });
    s.materializeLibGraphs("src/lib/a.omni");
    let state = usePoiesisStore.getState();
    expect(state.elements.map((e) => e.id)).toEqual([pidA, "st_1"]);
    expect(state.wires).toHaveLength(1);
    // 幂等：二次调用不重复
    usePoiesisStore.getState().materializeLibGraphs("src/lib/a.omni");
    state = usePoiesisStore.getState();
    expect(state.elements).toHaveLength(2);
    // 缺失 position 的元素获得确定性行布局
    expect((state.elements[0] as { position?: unknown }).position).toBeDefined();
  });

  it("pid 归属校验：fn 入口元素 id ≠ f.pid 的图不物化（防跨文件同名污染）", () => {
    const s = usePoiesisStore.getState();
    s.setLibDef("src/lib/a.omni", {
      imports: [], structs: [], types: [],
      fns: [{ pid: "fn_other_el_zzz", name: "fetch", params: [], returnType: "", body: "", pub: false, isAsync: false, declKind: "fn" }],
    });
    usePoiesisStore.setState({
      codeFlowGraphs: {
        fetch: { elements: [codeblock("fn_fetch_el_aaa", "fn")], wires: [] },
      },
    });
    s.materializeLibGraphs("src/lib/a.omni");
    expect(usePoiesisStore.getState().elements).toHaveLength(0);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/store/__tests__/lib-defs.test.ts`
Expected: FAIL（libDefs 不存在）

- [ ] **Step 3: store/types.ts 加状态字段**

`PoiesisState`（store/types.ts:26 接口内，codeFlowGraphs 字段附近）加：
```ts
  // ── Code tab（lib 文件）──
  libDefs: Record<string, import("../services/codegen/types").LibFileDef>;
  setLibDef: (filePath: string, def: import("../services/codegen/types").LibFileDef) => void;
  materializeLibGraphs: (filePath: string) => void;
```

- [ ] **Step 4: 写 slice**

```ts
// poiesis/src/store/slices/lib-defs.ts
// Code tab（lib 文件）声明集合 + fn 图物化（对标 element-crud 的 materializeFnGraphs）。

import type { LibFileDef } from "../../services/codegen/types";
import type { CodeblockElementData } from "../../core/types/element";
import type { ElementData, WireData } from "../../core/types";
import type { PoiesisState } from "../types";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createLibDefsSlice(set: SetFn, get: GetFn) {
  return {
    setLibDef: (filePath: string, def: LibFileDef) => {
      set((s) => ({ libDefs: { ...s.libDefs, [filePath]: def } }));
    },

    /** 物化 lib 文件的 fn 图到当前画布（对标 materializeFnGraphs，element-crud.ts:176）。
     *  幂等：pid 去重、已存在不覆盖；缺 position 时确定性行布局。
     *  空合并短路：无新元素不重建数组引用。 */
    materializeLibGraphs: (filePath: string) => {
      const s = get();
      const def = s.libDefs[filePath];
      if (!def) return;
      const newElements: ElementData[] = [...s.elements];
      const newWires: WireData[] = [...s.wires];
      const existIds = new Set(newElements.map((e) => e.id));
      let layoutIdx = 0;
      for (const f of def.fns) {
        const graph = s.codeFlowGraphs[f.name];
        if (!graph) continue;
        // pid 归属校验：fn 入口元素须以 f.pid 为 id——跨文件同名 fn 不混入
        const fnEl = graph.elements.find((e) => e.id === f.pid);
        if (!fnEl) continue;
        for (const e of graph.elements) {
          if (existIds.has(e.id)) continue;
          newElements.push({
            ...e,
            position: e.position ?? s.persistedPositions[e.id] ?? { x: 0, y: layoutIdx++ * 60 },
          });
          existIds.add(e.id);
        }
        for (const w of graph.wires) {
          if (!newWires.some((nw) => nw.id === w.id)) newWires.push(w);
        }
      }
      if (newElements.length === s.elements.length && newWires.length === s.wires.length) return;
      set({ elements: newElements, wires: newWires });
    },
  };
}
```
（`position`/`persistedPositions` 的字段名与物化语义以 element-crud.ts:176-221 的 materializeFnGraphs 为准——若既有实现形态与本代码有出入，以既有为准微调。）

- [ ] **Step 5: store/index.ts 注册**

`store/index.ts` 的 create 调用（54-55 codeFlowGraphs/persistedPositions 附近）加 `libDefs: {}` 初始值；slice 工厂 spread 处加：
```ts
  ...createLibDefsSlice(set, get),
```
顶部 import `createLibDefsSlice` from `./slices/lib-defs`。

- [ ] **Step 6: App.tsx 加载接线**

(a) 94 行 `parseOmniSource(res.content)` → `parseOmniSource(res.content, res.filePath)`。
(b) 96 行过滤加 fileDefs 放行（lib 文件此前被直接 return null）：
```ts
      if (!parseResult.designElements && !parseResult.storeDef && !parseResult.fileDefs) return null;
```
(c) 批量应用（255-359）：
- stateUpdate（262-269）加 `libDefs: Record<string, LibFileDef>` 字段（类型注解处）。
- `const mergedGraphs = ...`（278）之后加：
```ts
      const mergedFileDefs = { ...p.mergedState.libDefs, ...p.parseResult.fileDefs };
```
（形态与 mergedGraphs 同——若 mergedGraphs 是合并 p.mergedState 与 parseResult 的写法，逐行镜像。）
- 循环内（main 分支与 else 分支**之前**、每文件无条件执行）给 stateUpdate 赋值：
```ts
      stateUpdate.libDefs = mergedFileDefs;
```
- **else 分支（309-324）条件改为 `} else if (p.parseResult.designElements) {`**：
  修正说明——不能用「`!p.parseResult.fileDefs`」（预研时的初版判断）：FeatureCard.omni **既有 designElements 又有 fileDefs**（头部 struct），`!fileDefs` 会把它误杀；正确条件是「有 design 内容才建 ComponentDef」。副作用：纯 store 文件（designElements 为 null）不再建空壳 ComponentDef——空壳本来就是死数据（elements 恒 []，无消费者），安全。
- setState（340-351）的赋值清单加 `libDefs: stateUpdate.libDefs`。
- 顶部 import `type { LibFileDef }` from codegen/types。

- [ ] **Step 7: DesignPage code tab 物化 effect**

`pages/DesignPage.tsx` 在组件物化 effect（167-176）后加：
```ts
  // ── 物化：code tab 进入时合并当前 lib 文件的 fn 图（幂等，对标组件 F1 物化）──
  const materializeLibGraphs = usePoiesisStore((s) => s.materializeLibGraphs);
  useEffect(() => {
    if (activeTabType === "code" && activeTab?.filePath) {
      materializeLibGraphs(activeTab.filePath);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 物化幂等，依赖仅触发时机
  }, [activeTabType, activeTab?.filePath]);
```

- [ ] **Step 8: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/store/__tests__/lib-defs.test.ts` → PASS
Run: `cd poiesis && npm test`（重点既有：materialize-fn-graphs / fn-registry / fn-node-pipeline——App.tsx 接线不得破坏组件物化）
Run: `cd poiesis && npx tsc --noEmit`

- [ ] **Step 9: Commit**

```bash
git add poiesis/src/store/slices/lib-defs.ts poiesis/src/store/types.ts poiesis/src/store/index.ts poiesis/src/App.tsx poiesis/src/pages/DesignPage.tsx poiesis/src/store/__tests__/lib-defs.test.ts
git commit -m "feat(codeflow): code tab 物化链路（libDefs slice + 加载接线 + materializeLibGraphs）"
```

---

### Task 3: 组件 action 解析 + fn/action 双格式发射（修复 G1）

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`（parseComponentKeyword 387-436 加 action case；新方法 parseComponentAction；synthesizeFnEntry 628-656 加 entryTag/propsExtras；parseComponentComment 376-379 文档注释更新）
- Modify: `poiesis/src/services/codegen/graph-compiler.ts:91-115`（fn case 双格式）
- Modify: `poiesis/src/services/codegen/source-emitter.ts:317-339`（fn loop 双格式）
- Modify: `poiesis/src/services/codegen/pipeline.ts:276-327`（tag filter + 回填字段）
- Test: `poiesis/src/services/codegen/__tests__/action-syntax.test.ts`（新建，镜像 fn-node-pipeline.test.ts）

**Interfaces:**
- Consumes: `FnDef.pub/isAsync/declKind`（T1）
- Produces: `synthesizeFnEntry(fnName, rawParams, returnType, fnPid, elements, wires, entryTag?: "fn" | "action" | "lifecycle", propsExtras?: Record<string, unknown>)`——T6 复用它
- Produces: 解析 `action <name>(<params>) [async] { body }` → fnRegistry 条目（declKind "action"）+ parsedFnBodies + synthesizeFnEntry(entryTag "action", propsExtras {declKind, isAsync})
- 发射格式（graph-compiler / source-emitter 一致）：
  - `fn:      ${pub ? "pub " : ""}${isAsync ? "async " : ""}fn ${name}(${params})${ret} {`
  - `action:  action ${name}(${params})${isAsync ? " async" : ""} {`

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/services/codegen/__tests__/action-syntax.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";

// 形态来自 learn-omniview pages/DataFetching.omni（2 个 async action，spec §11 G1 回归文件）
const fixture = `component DataFetching {
  signal data: String = "";
  action loadData() async {
    data = "loading";
    return;
  }
  view {
    <div id="root"></div>
  }
}
`;

describe("组件 action（G1 修复）", () => {
  it("解析：action 进入 fnRegistry（declKind action）与 codeFlowGraphs", () => {
    const r = parseOmniSource(fixture);
    const reg = r.fnRegistry?.["DataFetching"];
    expect(reg).toBeDefined();
    expect(reg!.map((f) => f.name)).toEqual(["loadData"]);
    expect(reg![0]).toMatchObject({ returnType: "", declKind: "action", isAsync: true });
    expect(r.codeFlowGraphs?.["loadData"]).toBeDefined();
  });

  it("编译-再解析闭环：action 双格式发射且 fixpoint", () => {
    const r = parseOmniSource(fixture);
    const graph = r.codeFlowGraphs!["loadData"];
    const compiled = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled).toContain("action loadData() async {");
    expect(compiled).not.toContain("->");
    // 再解析 → 再编译 → 不变（pid 注释稳定）
    const r2 = parseOmniSource(compiled, "src/components/DataFetching.omni");
    const g2 = r2.codeFlowGraphs!["loadData"];
    const compiled2 = compileCodeFlowGraph(g2.elements, g2.wires, g2.elements[0].id)!;
    expect(compiled2).toBe(compiled);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/action-syntax.test.ts`
Expected: FAIL（fnRegistry 无 loadData——action 被 skip）

- [ ] **Step 3: parser.ts —— synthesizeFnEntry 扩展**

`parseFn`（557-619）调用处加两参：`this.synthesizeFnEntry(fnName, rawParams, returnType, fnPid, elements, wires, "fn", {});`
`parseRootFn`（T1 写的）同样加 `"fn", {}`。
`synthesizeFnEntry`（628-656）签名与实现改为：
```ts
  private synthesizeFnEntry(
    fnName: string,
    rawParams: string,
    returnType: string,
    fnPid: string,
    elements: CodeblockElementData[],
    wires: WireData[],
    entryTag: "fn" | "action" | "lifecycle" = "fn",
    propsExtras: Record<string, unknown> = {},
  ): void {
    if (elements.some((e) => e.tag === entryTag)) return;
    elements.unshift({
      id: fnPid,
      nodeType: "codeblock",
      tag: entryTag,
      parentId: null,
      props: { name: fnName, params: rawParams, returnType, ...propsExtras },
      source: "",
    });
    if (elements.length > 1) {
      wires.push({
        id: `w_${fnPid}_0`,
        fromId: fnPid,
        fromPort: "flow-out",
        toId: elements[1].id,
        toPort: "flow-in",
      });
    }
  }
```
`parseComponentComment`（376-379）文档注释更新：`fn/store` 两消费方 → `fn/store/action/lifecycle` 四消费方。

- [ ] **Step 4: parser.ts —— action case 与 parseComponentAction**

`parseComponentKeyword`（390-435）switch 加：
```ts
      case "action":
        this.parseComponentAction();
        break;
```
新方法（parseFn 之后插入）：
```ts
  /** 组件 action（修复 G1）：`action <name>(<params>) [async] { body }`（DataFetching.omni 实证）。
   *  镜像 parseFn 的签名/body 收集；async 是签名尾关键字（`() async`）；returnType 恒 ""。 */
  private parseComponentAction(): void {
    this.stream.consume(); // "action"

    let signature = "";
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") { this.stream.consume(); break; }
      signature += tok.text + " ";
      this.stream.consume();
    }
    const sigMatch = signature.match(/(\w+)\s*\(([^)]*)\)/);
    const name = sigMatch?.[1] || "unnamed";
    const rawParams = (sigMatch?.[2] || "").trim();
    const isAsync = /async/.test(signature);

    let body = "";
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") { depth++; body += "{\n"; }
      else if (tok.kind === "BRACE_CLOSE") { depth--; if (depth > 0) body += "}\n"; }
      else { body += tok.text + " "; }
      this.stream.consume();
    }

    const { elements, wires } = parseCodeFlowGraph(body);
    const pid = this.ctx.pendingPid || `fn_${name}_${uid()}`;
    this.ctx.pendingPid = null;
    const params = rawParams
      ? rawParams.split(",").map((p) => {
          const [pName, pType] = p.trim().split(":").map((s) => s.trim());
          return { name: pName || "", type: pType || "String" };
        })
      : [];
    this.synthesizeFnEntry(name, rawParams, "", pid, elements, wires, "action", { declKind: "action", isAsync });
    this.ensureBodyFallback(pid, body, elements, wires);
    this.parsedFnBodies.set(name, { elements, wires });
    const comp = this.ctx.currentComponent;
    if (comp) {
      const list = this.fnRegistry.get(comp) ?? [];
      list.push({ pid, name, params, returnType: "", declKind: "action", isAsync });
      this.fnRegistry.set(comp, list);
    }
  }
```
（fnRegistry 条目类型在 `fnRegistry` 字段声明处同步扩展 declKind/isAsync——见 parser.ts:54 的 Map 泛型，加 `declKind?: "fn" | "action"; isAsync?: boolean;` 到元素类型。）

- [ ] **Step 5: graph-compiler.ts fn case 双格式**

`graph-compiler.ts` fn case（91-115）——`const ret = ...`（95）与 header（97）之间加：
```ts
      const declKind = (node.element.props?.declKind as "fn" | "action") || "fn";
      const pub = (node.element.props?.pub as boolean) || false;
      const isAsync = (node.element.props?.isAsync as boolean) || false;
```
97 行改为：
```ts
      if (declKind === "action") {
        out += `action ${name}(${params})${isAsync ? " async" : ""} {\n`;
      } else {
        out += `${pub ? "pub " : ""}${isAsync ? "async " : ""}fn ${name}(${params})${ret} {\n`;
      }
```
（fn 分支保持 `${ret}` 条件发射——无 -> 不发射，T1 的 returnType "" 依赖此。）

- [ ] **Step 6: source-emitter.ts fn loop 双格式**

`source-emitter.ts` fn loop（317-339）333 行改为：
```ts
      if (fn.declKind === "action") {
        w.emit(`action ${fn.name}(${params})${fn.isAsync ? " async" : ""} {`);
      } else {
        w.emit(`${fn.pub ? "pub " : ""}${fn.isAsync ? "async " : ""}fn ${fn.name}(${params}) -> ${fn.returnType} {`);
      }
```
（fn 分支保持 ` -> ${returnType}` **恒发射**——现有输出格式不动、round-trip 稳定；action 分支无返回类型。）

- [ ] **Step 7: pipeline.ts 接线**

(a) 283 行 `if (fnEl.tag !== "fn") continue;` → `if (fnEl.tag !== "fn" && fnEl.tag !== "action") continue;`
（否则 action 节点在保存时被 fn 注入循环跳过——动作体丢失。）
(b) existingFn 回填（298-303）追加：
```ts
        existingFn.declKind = (fnEl.props?.declKind as "fn" | "action") || "fn";
        existingFn.isAsync = (fnEl.props?.isAsync as boolean) || false;
        existingFn.pub = (fnEl.props?.pub as boolean) || false;
```
(c) 新 push（317-323）对象内追加同三字段：
```ts
          declKind: (fnEl.props?.declKind as "fn" | "action") || "fn",
          isAsync: (fnEl.props?.isAsync as boolean) || false,
          pub: (fnEl.props?.pub as boolean) || false,
```

- [ ] **Step 8: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/action-syntax.test.ts` → PASS
Run: `cd poiesis && npm test`（重点：fn-node-pipeline / fn-synthesis / pipeline / source-emitter-attrs / syntax-roundtrip）
Run: `cd poiesis && npx tsc --noEmit`

- [ ] **Step 9: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/graph-compiler.ts poiesis/src/services/codegen/source-emitter.ts poiesis/src/services/codegen/pipeline.ts poiesis/src/services/codegen/__tests__/action-syntax.test.ts
git commit -m "feat(codeflow): 组件 action 解析 + fn/action 双格式发射（G1 修复）"
```

---

### Task 4: throw / await 语法节点

**Files:**
- Modify: `poiesis/src/services/codegen/graph-parser.ts`（detectSegmentType 加 `/^throw\s+/`、`/^await\s+/`，置于 call 正则之前；parseSyntaxProps 加两 case）
- Modify: `poiesis/src/services/codegen/syntax-nodes.ts`（defaultSyntaxProps + formatSyntaxNode 加两 case）
- Modify: `poiesis/src/services/codegen/graph-compiler.ts`（case "throw" / "await" 走 formatSyntaxNode，与 let/assign/store-call/call 同构）
- Modify: `poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx`（加两渲染 case：label + NodeFormulaField）
- Modify: `poiesis/src/components/canvas/code/flowConstants.ts`（TAG_CN / CODE_TAGS / MENU_SECTIONS 调用段）
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx:43-55`（nodeTypes 加 throw/await → SyntaxNode）
- Test: `poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts`（追加）与 `syntax-nodes.test.ts`（追加）

**Interfaces:**
- Produces: 节点 tag `"throw"` / `"await"`，props `{ value: string }`
- 发射：`throw ${value};` / `await ${value};`
- 值提取：关键字后到行尾的 token 无空格拼接**但保留单词间单空格**（`new Error("boom")` 不得变成 `newError("boom")`）——复用现有语法 case 的 compactSpacedMembers 助手（以 graph-parser.ts 现有 let/assign/store-call/call 值提取为同款实现）

- [ ] **Step 1: 写失败测试**（追加到既有文件）

```ts
// graph-parser-syntax.test.ts 追加：
it("throw/await 段解析为语法节点", () => {
  const { elements, wires } = parseCodeFlowGraph(`
throw new Error("boom");
await async_sleep(20);
`);
  const tags = elements.map((e) => e.tag);
  expect(tags).toEqual(["throw", "await"]);
  // 值保留单词间空格（compactSpacedMembers 语义）
  expect((elements[0] as CodeblockElementData).props?.value).toBe(`new Error("boom")`);
  expect((elements[1] as CodeblockElementData).props?.value).toBe("async_sleep(20)");
  expect(wires).toHaveLength(1); // throw → await
});

// syntax-nodes.test.ts 追加：
it("formatSyntaxNode 发射 throw/await", () => {
  expect(formatSyntaxNode({ tag: "throw", props: { value: `new Error("boom")` } } as never))
    .toBe(`throw new Error("boom");`);
  expect(formatSyntaxNode({ tag: "await", props: { value: "async_sleep(20)" } } as never))
    .toBe("await async_sleep(20);");
});

// 编译-再解析 fixpoint（graph-compiler-syntax.test.ts 追加）：
it("throw/await 编译-再解析闭环", () => {
  const graph = parseCodeFlowGraph(`throw new Error("x");\nawait cleanup();\n`);
  const compiled = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
  expect(compiled).toContain(`throw new Error("x");`);
  expect(compiled).toContain("await cleanup();");
  const graph2 = parseCodeFlowGraph(compiled);
  expect(graph2.elements.map((e) => e.tag)).toEqual(["throw", "await"]);
});
```
（若既有测试文件的 import 形态与上述不同，按文件内既有 import 调整。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/graph-parser-syntax.test.ts src/services/codegen/__tests__/syntax-nodes.test.ts src/services/codegen/__tests__/graph-compiler-syntax.test.ts`
Expected: FAIL（throw/await 落入 call 或 fallback）

- [ ] **Step 3: graph-parser.ts**

(a) `detectSegmentType` 在 **call 正则判断之前**加：
```ts
    if (/^throw\s+/.test(first)) return "throw";
    if (/^await\s+/.test(first)) return "await";
```
（`await async_sleep(20)` 首 token 是 await——必须置于 call 之前。）
(b) `parseSyntaxProps`（现有 let/assign/store-call/call 分支所在方法）加两分支，**值与现有语法 case 同款提取**（值 = 关键字之后 token 序列、单词间保留单空格——`new Error("boom")` 不得变成 `newError("boom")`）：
```ts
    if (/^throw\s+/.test(source)) {
      return { ...base, tag: "throw", props: { value: extractAfterKeyword(source, "throw") } };
    }
    if (/^await\s+/.test(source)) {
      return { ...base, tag: "await", props: { value: extractAfterKeyword(source, "await") } };
    }
```
（`base` 为现有语法 case 的公共 props 基座、`extractAfterKeyword` 为关键字后文本提取（含单词间空格整理）——两助手均复用现有语法 case 的既有实现与命名，若文件名不同以文件内既有语法 case 结构为准，行为由 Step 1 测试钉死。）

- [ ] **Step 4: syntax-nodes.ts**

`defaultSyntaxProps` 加（镜像 return 的 props 形态——`{ value: string }` 字段）：
```ts
    throw: { label: "throw", value: "" },
    await: { label: "await", value: "" },
```
`formatSyntaxNode` 加：
```ts
    case "throw":
      return `throw ${node.props?.value ?? ""};`;
    case "await":
      return `await ${node.props?.value ?? ""};`;
```

- [ ] **Step 5: graph-compiler.ts**

switch 加两 case（与现有语法 case 同构——pid 注释 + formatSyntaxNode 行 + 后继链，循环结构镜像 fn case 的 successors 循环）：
```ts
    case "throw":
    case "await": {
      // 与 let/assign 语法 case 同构：pid 注释 + 单行 + 后继链
      let out = emitPidComment(pid);
      out += formatSyntaxNode(node.element);
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent + 1, visited);
      }
      return out;
    }
```
（行尾换行的处理以既有语法 case 为准——若既有 case 用 `out += formatSyntaxNode(...) + "\n"` 形态则保持同款；Step 1 的编译断言钉死输出内容。）

- [ ] **Step 6: UI 注册**

(a) `nodes/SyntaxNode.tsx`：内部渲染 switch 加两 case（一行 label + NodeFormulaField，镜像现有语法 case 的渲染结构；label 用 `throw` / `await`，公式框绑 props.value）。
(b) `flowConstants.ts`：CODE_TAGS 加 `"throw"`、`"await"`；TAG_CN 加 `throw: "throw"`、`await: "await"`；MENU_SECTIONS 的「调用」段（62-67）条目加两项（tag 与 menu key 与既有条目同构）。
(c) `FlowCanvas.tsx` nodeTypes（43-55）加：
```ts
  "throw": SyntaxNode,
  "await": SyntaxNode,
```

- [ ] **Step 7: 测试通过 + 回归 + tsc**

Run: Step 1 三个测试文件 → PASS
Run: `cd poiesis && npm test`（重点：syntax-roundtrip / code-op 系列 / flow-drag-create）
Run: `cd poiesis && npx tsc --noEmit`

- [ ] **Step 8: Commit**

```bash
git add poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/syntax-nodes.ts poiesis/src/services/codegen/graph-compiler.ts poiesis/src/components/canvas/code/nodes/SyntaxNode.tsx poiesis/src/components/canvas/code/flowConstants.ts poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/services/codegen/__tests__/graph-parser-syntax.test.ts poiesis/src/services/codegen/__tests__/syntax-nodes.test.ts poiesis/src/services/codegen/__tests__/graph-compiler-syntax.test.ts
git commit -m "feat(codeflow): throw/await 语法节点（解析-发射-画布注册）"
```

---

### Task 5: FnInspector pub/async/declKind 表单

**Files:**
- Modify: `poiesis/src/components/canvas/code/FlowInspector.tsx`（分派 159 行加 action；FnInspector 193-273 加三字段表单）
- Test: `poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx`（追加）

**Interfaces:**
- Consumes: 节点 props `{ pub?: boolean; isAsync?: boolean; declKind?: "fn" | "action" }`（T3 发射已支持）
- Produces: 面板编辑直接写 props（setProp(id, "pub"/"isAsync"/"declKind", value) → store.elements 更新 → 保存经 T3/T9 发射）
- pub 开关**仅 code tab 显示**（`activeTabType === "code"` 判断——从 designPage.openTabs 找 activeTabId）

- [ ] **Step 1: 写失败测试**（追加到 FlowInspector.test.tsx）

```ts
// 镜像该文件既有渲染 helper（render FlowInspector + selectedNodeIds）
it("FnInspector 显示 declKind/pub/async 并回写 props", async () => {
  const el = codeblockEl("fn_x", { name: "load", params: "", returnType: "" });
  usePoiesisStore.setState({ elements: [el], designPage: { ...state.designPage, activeTabId: "t1" } });
  // openTabs 含 code tab（tabType: "code", filePath: "src/lib/x.omni"）
  render(<FlowInspector selectedNodeIds={["fn_x"]} />);
  expect(screen.getByLabelText("Function name")).toBeInTheDocument();
  expect(screen.getByText("Declaration kind")).toBeInTheDocument();
  // code tab → pub 可见
  expect(screen.getByText("pub")).toBeInTheDocument();
  fireEvent.click(screen.getByText("pub").closest("label")!.querySelector("input")!);
  expect(usePoiesisStore.getState().elements.find((e) => e.id === "fn_x")!.props?.pub).toBe(true);
});
```
（若既有测试文件无 codeblockEl 等 helper，按该文件既有构造方式写。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/components/canvas/code/__tests__/FlowInspector.test.tsx`
Expected: FAIL（无 Declaration kind 字段）

- [ ] **Step 3: 分派加 action**

`FlowInspector.tsx:159`：
```ts
  if (el.tag === "fn" || el.tag === "action") {
    return <FnInspector el={el} setProp={setProp} />;
  }
```

- [ ] **Step 4: FnInspector 表单**

`FnInspector`（193-273）内、现有 hooks 之后（Rules of Hooks 合规——所有 usePoiesisStore 调用必须在 return 之前无条件执行）加：
```ts
  const pub = (el.props?.pub as boolean) || false;
  const isAsync = (el.props?.isAsync as boolean) || false;
  const declKind = (el.props?.declKind as "fn" | "action") || "fn";
  const activeTabType = usePoiesisStore((s) => {
    const activeId = s.designPage.activeTabId;
    if (!activeId) return "component" as const;
    return s.designPage.openTabs.find((t) => t.id === activeId)?.tabType ?? "component";
  });
```
Parameters 字段之后追加三块表单（样式复用 S.field / S.label / S.input / S.select / S.btn）：
```tsx
      {activeTabType === "code" && (
        <div style={S.field}>
          <div style={S.label}>Visibility</div>
          <label style={{ display: "flex", gap: 6, alignItems: "center", fontSize: 13 }}>
            <input type="checkbox" checked={pub}
              onChange={(e) => setProp(el.id, "pub", e.target.checked)} />
            <span>pub</span>
          </label>
        </div>
      )}

      <div style={S.field}>
        <div style={S.label}>Declaration kind</div>
        <select style={S.select} value={declKind}
          onChange={(e) => setProp(el.id, "declKind", e.target.value)}>
          <option value="fn">fn</option>
          <option value="action">action</option>
        </select>
      </div>

      <div style={S.field}>
        <div style={S.label}>Async</div>
        <label style={{ display: "flex", gap: 6, alignItems: "center", fontSize: 13 }}>
          <input type="checkbox" checked={isAsync}
            onChange={(e) => setProp(el.id, "isAsync", e.target.checked)} />
          <span>async</span>
        </label>
      </div>
```

- [ ] **Step 5: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/components/canvas/code/__tests__/FlowInspector.test.tsx` → PASS
Run: `cd poiesis && npm test`、`cd poiesis && npx tsc --noEmit`

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/components/canvas/code/FlowInspector.tsx poiesis/src/components/canvas/code/__tests__/FlowInspector.test.tsx
git commit -m "feat(codeflow): FnInspector pub/async/declKind 表单（code tab 显示 pub）"
```

---

### Task 6: 生命周期数据链（@mount/@effect/@unmount 解析-发射-物化，修复 G6）

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`（parseComponentBody AT case 345-347 重构；parseAtStore 438 去 @ 消费；新 parseLifecycleMember；Parser 字段 lifecycleRegistry/parsedLifecycleBodies；parseComponentComment 注释更新）
- Modify: `poiesis/src/services/codegen/parser-types.ts`（ParseContext 无需改——registry 用 Parser 私有字段）
- Modify: `poiesis/src/services/codegen/types.ts`（LifecycleDef；Valid/PartialComponentPart 加 lifecycles；LoweredCode 加 lifecycles；ParseResult 加 lifecycleRegistry/lifecycleGraphs）
- Modify: `poiesis/src/services/codegen/graph-parser.ts`（detectSegmentType "lifecycle" case + parseLifecycleSegment）
- Modify: `poiesis/src/services/codegen/graph-compiler.ts`（case "lifecycle"）
- Modify: `poiesis/src/services/codegen/source-emitter.ts`（fn loop 339 后、view 341 前插入生命周期发射）
- Modify: `poiesis/src/services/codegen/pipeline.ts`（defaultLoweredCode + lifecycle 注入循环 327 后）
- Modify: `poiesis/src/services/codegen/component-builder.ts`（~150/162 `fns: phaseB.lowered.functions` 旁加 lifecycles）
- Modify: `poiesis/src/core/types/project-json.ts:100-108`（ComponentSourceDef 加 lifecycles）
- Modify: `poiesis/src/store/types.ts` + `poiesis/src/store/index.ts`（lifecycleGraphs 状态）
- Modify: `poiesis/src/store/slices/element-crud.ts:176-221`（materializeFnGraphs 加生命周期合并循环）
- Modify: `poiesis/src/App.tsx`（main/else 分支加 lifecycles + materializeLifecycles helper）
- Modify: `poiesis/src/hooks/useSaveProject.ts:266-297`（componentDefs 回写加 lifecycles 字段——必填字段编译强制）
- Test: `poiesis/src/services/codegen/__tests__/parser-lifecycle.test.ts`（新建）+ `graph-parser-syntax.test.ts`（追加）+ `fn-node-pipeline.test.ts`（追加形态）+ `store/__tests__/materialize-fn-graphs.test.ts`（追加）

**Interfaces:**
- Produces: `LifecycleDef = { pid: string; hookType: "mount" | "effect" | "unmount"; deps?: string; body: string }`（types.ts）
- Produces: `ParseResult.lifecycleRegistry?: Record<componentName, Array<{ pid; hookType; deps? }>>` + `ParseResult.lifecycleGraphs?: Record<pid, { elements; wires }>`
- Produces: `ComponentSourceDef.lifecycles: Array<LifecycleDef>`（project-json.ts，持久化）
- Produces: `LoweredCode.lifecycles?: LifecycleDef[]` + Valid/PartialComponentPart `lifecycles?: LifecycleDef[]`
- Produces: `PoiesisState.lifecycleGraphs: Record<string, { elements; wires }>`（store）
- 发射格式：`// @pid:<pid>\n@effect(searchText, setFilter) {\n  <body>\n}`；deps 仅 effect 括号形式
- 单例约束：解析不强制（WASM 门禁兜底），UI 层守卫在 T7

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/services/codegen/__tests__/parser-lifecycle.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";

const fixture = `component Main {
  signal searchText: String = "";
  signal filter: String = "";
  @effect(searchText, setFilter) {
    filterTasks(searchText);
  }
  @mount {
    initRows();
  }
  fn handleClick() {
    return;
  }
  view {
    <div id="root"></div>
  }
}
`;

describe("生命周期（@mount/@effect/@unmount）", () => {
  it("解析：lifecycleRegistry 按组件名登记 + 图按 pid 键", () => {
    const r = parseOmniSource(fixture);
    expect(r.lifecycleRegistry?.["Main"]).toBeDefined();
    expect(r.lifecycleRegistry!["Main"].map((l) => l.hookType)).toEqual(["effect", "mount"]);
    expect(r.lifecycleRegistry!["Main"][0].deps).toBe("searchText,setFilter"); // token 无空格拼接
    expect(r.lifecycleRegistry!["Main"][1].deps).toBeUndefined();
    const pid = r.lifecycleRegistry!["Main"][0].pid;
    expect(r.lifecycleGraphs?.[pid]).toBeDefined();
    // fnRegistry 不含生命周期（生命周期不是 fn）
    expect(r.fnRegistry?.["Main"]?.map((f) => f.name)).toEqual(["handleClick"]);
  });

  it("编译-再解析闭环：deps 括号形式 + fixpoint", () => {
    const r = parseOmniSource(fixture);
    const pid = r.lifecycleRegistry!["Main"][0].pid;
    const graph = r.lifecycleGraphs![pid];
    const compiled = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled).toContain("@effect(searchText,setFilter) {");
    expect(compiled).toContain("filterTasks(searchText)");
    const compiled2 = compileCodeFlowGraph(graph.elements, graph.wires, graph.elements[0].id)!;
    expect(compiled2).toBe(compiled);
  });

  it("graph-parser 防御路径：@effect 段直接解析（单测，段级）", () => {
    const graph = parseCodeFlowGraph(`@effect(searchText, setFilter) {
  filterTasks(searchText);
}
`);
    expect(graph.elements[0]).toMatchObject({ tag: "lifecycle" });
    expect(graph.elements[0].props).toMatchObject({ hookType: "effect", deps: "searchText, setFilter" });
    expect(graph.wires).toHaveLength(1);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/parser-lifecycle.test.ts`
Expected: FAIL（@effect 被 @ 指令路径 skip）

- [ ] **Step 3: parser.ts —— AT case 重构 + parseLifecycleMember**

(a) parseComponentBody 的 AT case（345-347）改为：
```ts
        case "AT": {
          this.stream.consume(); // "@"
          const hook = this.stream.peek().text;
          if (hook === "mount" || hook === "effect" || hook === "unmount") {
            this.parseLifecycleMember(hook as "mount" | "effect" | "unmount");
            break;
          }
          // 非生命周期 @ 指令（如 @store）——parseAtStore 重构后不再消费 @
          this.parseAtStore();
          break;
        }
```
(b) parseAtStore（438-453）**删除首行** `this.stream.consume(); // @`（@ 已由 AT case 消费；唯一调用点 346 已确认）。
(c) parseComponentComment（376-379）文档注释更新为四消费方：`fn/store/action/lifecycle`。
(d) Parser 字段（54 行附近）加：
```ts
  private lifecycleRegistry = new Map<string, Array<{ pid: string; hookType: "mount" | "effect" | "unmount"; deps?: string }>>();
  private parsedLifecycleBodies = new Map<string, { elements: CodeblockElementData[]; wires: WireData[] }>();
```
(e) 新方法（parseComponentAction 之后插入）：
```ts
  /** 生命周期成员（修复 G6）：`@mount {` / `@effect(deps) {` / `@unmount {`。
   *  body-only 镜像 parseFn——全块文本进 parseCodeFlowGraph 会把未注解块 header
   *  塞进 code-block source，故先提取 header 再收集 body。deps token 无空格拼接
   *  （"searchText,setFilter"）——与 graph-parser 侧原始文本解析各自路径稳定。 */
  private parseLifecycleMember(hook: "mount" | "effect" | "unmount"): void {
    const pid = this.ctx.pendingPid || `lifecycle_${hook}_${uid()}`;
    this.ctx.pendingPid = null;
    this.stream.consume(); // "mount" | "effect" | "unmount"
    let deps: string | undefined;
    if (this.stream.peek().text === "(") {
      this.stream.consume(); // "("
      let d = "";
      while (!this.stream.isEof() && this.stream.peek().text !== ")") {
        d += this.stream.peek().text;
        this.stream.consume();
      }
      if (this.stream.peek().text === ")") this.stream.consume();
      deps = d;
    }
    // body 收集（花括号计数）
    let body = "";
    let depth = 0;
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") {
        depth++;
        if (depth > 1) body += "{\n";
        this.stream.consume();
        continue;
      }
      if (tok.kind === "BRACE_CLOSE") {
        depth--;
        this.stream.consume();
        if (depth === 0) break;
        body += "}\n";
        continue;
      }
      body += tok.text + " ";
      this.stream.consume();
    }
    const { elements, wires } = parseCodeFlowGraph(body);
    this.synthesizeFnEntry(hook, "", "", pid, elements, wires, "lifecycle", { hookType: hook, deps });
    this.ensureBodyFallback(pid, body, elements, wires);
    this.parsedLifecycleBodies.set(pid, { elements, wires });
    const comp = this.ctx.currentComponent;
    if (comp) {
      const list = this.lifecycleRegistry.get(comp) ?? [];
      list.push({ pid, hookType: hook, deps });
      this.lifecycleRegistry.set(comp, list);
    }
  }
```
(f) `parse()` 方法内组装 ParseResult 的代码块（T1 的 fileDefs 装配同一处）追加：
```ts
    if (this.lifecycleRegistry.size > 0) {
      result.lifecycleRegistry = Object.fromEntries(this.lifecycleRegistry);
    }
    if (this.parsedLifecycleBodies.size > 0) {
      result.lifecycleGraphs = Object.fromEntries(this.parsedLifecycleBodies);
    }
```

- [ ] **Step 4: types.ts / project-json.ts**

types.ts：
```ts
export interface LifecycleDef {
  pid: string;
  hookType: "mount" | "effect" | "unmount";
  deps?: string;
  body: string;
}
```
ValidComponentPart（86-98）与 PartialComponentPart（100-113）各加 `lifecycles?: LifecycleDef[];`；LoweredCode（271-276）加 `lifecycles?: LifecycleDef[];`；ParseResult 加：
```ts
  /** 组件 → 生命周期成员（componentDefs.lifecycles 物化来源；键为声明组件名，见 componentName 注释） */
  lifecycleRegistry?: Record<string, Array<{ pid: string; hookType: "mount" | "effect" | "unmount"; deps?: string }>>;
  /** 生命周期图（pid 键）——materializeFnGraphs 合并来源 */
  lifecycleGraphs?: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>;
```
project-json.ts ComponentSourceDef（100-108）加：
```ts
  lifecycles: Array<{ pid: string; hookType: "mount" | "effect" | "unmount"; deps?: string }>;
```
（必填——所有构造点由编译器强制补齐，防漏。迁移：旧 project.json 缺字段时运行时为 undefined，App.tsx/useSaveProject 的 `?? []` 兜底。）

- [ ] **Step 5: graph-parser.ts / graph-compiler.ts**

graph-parser.ts：
(a) `detectSegmentType` 在 fn/condition 判断之后、call 之前加（`@` 开头不匹配现有正则，显式前置防回归）：
```ts
    if (/^@(mount|effect|unmount)\b/.test(first)) return "lifecycle";
```
(b) 新 `parseLifecycleSegment(code)`（防御路径，行为由 Step 1 第三测试钉死）：首行 `@effect(...)` 提取 hookType/deps（**原始文本**，保留 `"searchText, setFilter"` 的空格）；段内 `// @pid:` 注释取入口 id（无则 `lifecycle_${hookType}_${uid()}`）；产出入口元素 `{ id, nodeType: "codeblock", tag: "lifecycle", parentId: null, props: { hookType, deps, name: hookType, params: "", returnType: "" }, source: "" }` + 首语句 wire `w_<pid>_0`（空 body 不 wire）。

graph-compiler.ts 加 case（镜像 fn case 91-115 的循环结构）：
```ts
    case "lifecycle": {
      const hookType = (node.element.props?.hookType as "mount" | "effect" | "unmount") ?? "mount";
      const deps = (node.element.props?.deps as string) || "";
      const depsPart = hookType === "effect" && deps ? `(${deps})` : "";
      let out = emitPidComment(pid);
      out += `@${hookType}${depsPart} {\n`;
      // 后继（body 链）——与 fn case 100-111 相同双循环
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) out += emitNode(succ, adj, indent + 1, visited);
      }
      for (const [, branchIds] of node.branchSuccessors) {
        for (const succId of branchIds) {
          const succ = adj.get(succId);
          if (succ) out += emitNode(succ, adj, indent + 1, visited);
        }
      }
      out += `}`;
      return out;
    }
```

- [ ] **Step 6: source-emitter.ts / pipeline.ts / component-builder.ts**

source-emitter.ts：fn loop 结束后（338 blank 之后）、`// View` 注释（341）之前插入：
```ts
      // ── Lifecycle 成员（@mount/@effect/@unmount）──
      for (const lc of component.lifecycles ?? []) {
        if (lc.pid) w.emit(`// @pid:${lc.pid}`);
        const depsPart = lc.hookType === "effect" && lc.deps ? `(${lc.deps})` : "";
        w.emit(`@${lc.hookType}${depsPart} {`);
        w.indent();
        for (const line of lc.body.split("\n")) w.emit(line);
        w.dedent();
        w.emit(`}`);
        w.blank();
      }
```
（`component.lifecycles ?? []`——Valid/Partial 均为可选，旧路径无生命周期时零输出。）

pipeline.ts：
(a) defaultLoweredCode（87-94）加 `lifecycles: [],`。
(b) fn 注入循环（327 行 `}` 之后）加 lifecycle 注入循环：
```ts
  // ── Lifecycle 注入：lifecycle 图编译进 lowered.lifecycles（source-emitter 发射）──
  const lifecycleElements = codeFlowElements.filter((e) => e.tag === "lifecycle");
  if (lifecycleElements.length > 0) {
    lowered.lifecycles ??= []; // lowered 已在 280 行定义（fn 注入循环同作用域）
    for (const lcEl of lifecycleElements) {
      const compiled = compileCodeFlowGraph(codeFlowElements, options.designWires, lcEl.id);
      if (!compiled) continue;
      const openBrace = compiled.indexOf("{");
      const closeBrace = compiled.lastIndexOf("}");
      const innerBody = openBrace !== -1 && closeBrace > openBrace
        ? compiled.slice(openBrace + 1, closeBrace).trim()
        : compiled;
      const existing = lowered.lifecycles.find((l) => l.pid === lcEl.id);
      if (existing) {
        existing.body = innerBody;
      } else {
        lowered.lifecycles.push({
          pid: lcEl.id,
          hookType: (lcEl.props?.hookType as "mount" | "effect" | "unmount") ?? "mount",
          deps: (lcEl.props?.deps as string) || undefined,
          body: innerBody,
        });
      }
    }
  }
```
（注意 `codeFlowElements` 在 276-278 已定义——lifecycle 元素必须已进入 designElements，依赖 T7 的物化/创建接线；此处是保存侧的编译兜底。）

component-builder.ts：两处 `fns: phaseB.lowered.functions`（约 150 与 162）各在相邻处加：
```ts
      lifecycles: phaseB.lowered.lifecycles ?? [],
```

- [ ] **Step 7: store + App.tsx 物化接线**

store/types.ts（PoiesisState）加：
```ts
  lifecycleGraphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>;
```
store/index.ts 加 `lifecycleGraphs: {}` 初始值。

element-crud.ts materializeFnGraphs（176-221）在 fn 循环后追加生命周期合并循环（同款 pid 去重/布局/persistedPositions）：
```ts
      // 生命周期图合并（组件级成员，与 fn 图平级）
      const lifecycles = def.lifecycles ?? [];
      for (const lc of lifecycles) {
        const graph = s.lifecycleGraphs?.[lc.pid];
        if (!graph) continue;
        const lcEl = graph.elements.find((e) => e.id === lc.pid);
        if (!lcEl) continue; // pid 归属校验
        for (const e of graph.elements) {
          if (existIds.has(e.id)) continue;
          newElements.push({ ...e, position: e.position ?? s.persistedPositions[e.id] ?? { x: 0, y: layoutIdx++ * 60 } });
          existIds.add(e.id);
        }
        for (const w of graph.wires) {
          if (!newWires.some((nw) => nw.id === w.id)) newWires.push(w);
        }
      }
```
（`def`/`layoutIdx`/`existIds` 为 materializeFnGraphs 既有局部变量——循环插在既有循环之后、set() 之前。）

App.tsx：
(a) 主分支（292-308）与 else 分支（309-324，T2 已改条件）的 ComponentDef 构造各加：
```ts
          lifecycles: existing?.lifecycles ?? materializeLifecycles(p),
```
(b) 新 helper（materializeFns 定义处附近）：
```ts
  // 生命周期元数据（组件维度）——声明名与文件名键可能不一致（main vs Main），
  // 与 materializeFns 同款处理（见 ParseResult.componentName 注释）。
  const materializeLifecycles = (p: ParsedFile) => {
    const reg = p.parseResult.lifecycleRegistry;
    if (!reg) return [];
    const compName = p.parseResult.componentName ?? p.componentName;
    return reg[compName] ?? [];
  };
```
(c) **useSaveProject.ts 回写点**（266-297 setComponentDef/componentDefs 回写处，`fns: currentDef.fns || []` 旁）加：
```ts
          lifecycles: currentDef.lifecycles || [],
```
（ComponentSourceDef.lifecycles 是 T6 新增的必填字段——所有构造点必须补齐，否则 tsc 编译错误。App.tsx 两分支已在 (a) 补齐；useSaveProject 回写是最后一个构造点。）

- [ ] **Step 8: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/parser-lifecycle.test.ts` → PASS
Run: `cd poiesis && npm test`（重点：fn-node-pipeline / fn-registry / materialize-fn-graphs / pipeline / roundtrip / save-inline-store）
Run: `cd poiesis && npx tsc --noEmit`

- [ ] **Step 9: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/graph-compiler.ts poiesis/src/services/codegen/source-emitter.ts poiesis/src/services/codegen/pipeline.ts poiesis/src/services/codegen/component-builder.ts poiesis/src/core/types/project-json.ts poiesis/src/store/types.ts poiesis/src/store/index.ts poiesis/src/store/slices/element-crud.ts poiesis/src/App.tsx poiesis/src/hooks/useSaveProject.ts poiesis/src/services/codegen/__tests__/parser-lifecycle.test.ts
git commit -m "feat(codeflow): 生命周期数据链（@mount/@effect/@unmount 解析-发射-物化）"
```

---

### Task 7: 生命周期节点 UI（LifecycleNode + 单例守卫）

**Files:**
- Create: `poiesis/src/components/canvas/code/nodes/LifecycleNode.tsx`
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx:43-55`（nodeTypes 加 lifecycle）
- Modify: `poiesis/src/components/canvas/code/FlowInspector.tsx`（分派加 lifecycle 分支 + LifecycleForm）
- Modify: `poiesis/src/components/canvas/code/flowConstants.ts`（TAG_CN/CODE_TAGS/MENU_SECTIONS 控制流段）
- Test: `poiesis/src/components/canvas/code/nodes/__tests__/LifecycleNode.test.tsx`（新建）+ `FlowInspector.test.tsx`（追加）

**Interfaces:**
- Consumes: `LifecycleDef`（T6）、`props { hookType, deps }`、`materializeFnGraphs`（T6 已合并生命周期图进 componentDefs 画布）
- Produces: 节点摘要渲染 `@effect(searchText, setFilter)` / `@mount` / `@unmount`；flow-out 底部 handle → body 链（对标 FnNode）
- 单例守卫：创建时若画布已有同 hookType 的 lifecycle 节点则禁用（检查 `store.elements` 中 `tag === "lifecycle" && props.hookType === hookType`——覆盖已保存与画布新建两种来源）

- [ ] **Step 1: 写失败测试**

```tsx
// poiesis/src/components/canvas/code/nodes/__tests__/LifecycleNode.test.tsx
// LifecycleNode（mount/effect/unmount）形态渲染 + 端口 handle 断言（镜像 LoopNode.test.tsx）。
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { LifecycleNode } from "../LifecycleNode";

vi.mock("@xyflow/react", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@xyflow/react")>();
  return {
    ...actual,
    Handle: (props: Record<string, unknown>) => <div {...props} />,
  };
});

const baseData = {
  tag: "lifecycle",
  props: { hookType: "effect", deps: "searchText, setFilter" },
  onEditProp: vi.fn(),
  scope: undefined,
  letVars: [],
  onDoubleClick: undefined,
};

describe("LifecycleNode", () => {
  it("effect 形态渲染摘要（含 deps）与 flow-in/flow-out 端口", () => {
    const { container } = render(<LifecycleNode id="lc-1" data={baseData as never} />);
    expect(container.querySelector('[data-testid="lifecycle-summary"]')?.textContent).toBe("@effect(searchText, setFilter)");
    expect(document.querySelector('[data-handle-id="flow-in"]')).toBeTruthy();
    expect(document.querySelector('[data-handle-id="flow-out"]')).toBeTruthy();
  });
  it("mount 形态渲染摘要（无 deps 括号）", () => {
    render(<LifecycleNode id="lc-2" data={{ ...baseData, props: { hookType: "mount" } } as never} />);
    expect(screen.getByText("@mount")).toBeTruthy();
  });
});
```
（data 形状与 LoopNode.test.tsx 的 baseData 同构；`@effect(searchText, setFilter)` 摘要带空格——发射端 deps 原样保留（解析提取括号内文本）。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/components/canvas/code/nodes/__tests__/LifecycleNode.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: LifecycleNode.tsx**

```tsx
// poiesis/src/components/canvas/code/nodes/LifecycleNode.tsx
// 生命周期块节点：摘要渲染 @mount / @effect(deps) / @unmount，flow-out → body 链。
// 对标 LoopNode 块形态（CSS module + HandlesBoundary + data-handle-id 端口类）。

import React, { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import type { FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./LifecycleNode.module.css";
import "./node-base.css";

interface LifecycleNodeData {
  tag: "lifecycle";
  props: { hookType: "mount" | "effect" | "unmount"; deps?: string };
  source?: string;
  scope?: FlowScope;
  letVars?: Array<{ name: string }>;
  onEditProp?: (id: string, prop: string, value: unknown) => void;
}

interface Props {
  id?: string;
  data: LifecycleNodeData;
}

// Handle 依赖 ReactFlowProvider（连线渲染）；单元测试直接渲染节点时静默跳过，避免抛错。
class HandlesBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

export const LifecycleNode = memo(function LifecycleNode({ id, data }: Props) {
  const hookType = data.props?.hookType ?? "mount";
  const deps = data.props?.deps ?? "";
  const depsPart = hookType === "effect" && deps ? `(${deps})` : "";
  return (
    <div className={`${styles.node} codeflow-node nodrag`}>
      <HandlesBoundary>
        <Handle type="target" position={Position.Top} id="flow-in" className={`${styles.handle} codeflow-handle`} data-handle-id="flow-in" />
      </HandlesBoundary>
      <div className={`${styles.header} codeflow-header`} data-testid="lifecycle-summary">{`@${hookType}${depsPart}`}</div>
      <HandlesBoundary>
        <Handle type="source" position={Position.Bottom} id="flow-out" className={`${styles.handle} codeflow-handle`} data-handle-id="flow-out" />
      </HandlesBoundary>
    </div>
  );
});
```

同目录新文件 `LifecycleNode.module.css`（镜像 LoopNode.module.css 的 node/header/handle 三件套）：
```css
.node {
  min-width: 180px;
  border-radius: 8px;
  border: 1px solid var(--ol-color-primary, #14b8a6);
  background: var(--ol-surface, #fff);
  font-size: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}
.header { padding: 6px 10px; background: rgba(20, 184, 166, 0.12); font-weight: 600; text-align: center; font-family: ui-monospace, monospace; border-radius: 8px 8px 0 0; }
.handle { width: 10px; height: 10px; background: #14b8a6; }
```
（数据契约与 LoopNodeData 同构：FlowCanvas nodes useMemo 注入的 `id/scope/letVars/onEditProp` 字段全部透传；`data.props` 读 hookType/deps。）

- [ ] **Step 4: FlowCanvas 注册 + NodeSearchMenu 透传 item + 单例守卫**

nodeTypes（43-55）加 `"lifecycle": LifecycleNode,`（import 同步加）。

**NodeSearchMenu 透传 item**：现有 `onSelect: (tag: string) => void` 只传 tag——lifecycle 三个菜单项 tag 都是 `"lifecycle"`，单例守卫无法区分 hookType。改 `MenuItem` 加可选 `props` 字段、`onSelect` 改传完整 item：
```ts
// flowConstants.ts
export interface MenuItem {
  label: string;
  tag: string;
  shortcut?: string;
  props?: Record<string, unknown>; // 创建时初始 props（lifecycle 菜单项携带 hookType）
}
```
```ts
// NodeSearchMenu.tsx
interface Props {
  x: number;
  y: number;
  onSelect: (item: MenuItem) => void; // 改：传完整 item（含 props）
  onClose: () => void;
}
// select(item) / onKeyDown Enter / 按钮 onClick 三处调用点同步改 `select(item)`（原传 flat[activeIdx].tag）
```
（menuKey 唯一性注意：三项 tag 相同，`key={item.tag}` 会 React key 冲突——按钮 key 改 `item.label`（或 `${item.tag}-${item.label}`），filterMenuItems 的匹配逻辑不变。）

**FlowCanvas handleMenuSelect 改造 + 单例守卫**（现签名 `(tag: string)` → `(item: MenuItem)`，内部 `const tag = item.tag`；既有 tag 分支不动，只加 lifecycle 分支与创建 props）：
```ts
  const handleMenuSelect = useCallback(
    (item: MenuItem) => {
      if (!menuState) return;
      const tag = item.tag;
      const id = uid(tag);
      const jitter = Math.random() * 40 - 20;
      let props: Record<string, unknown> = {};
      if (tag === "fn") { props = { name: "newFn", params: "" }; }
      if (tag === "condition") { props = { branches: [{ id: "br0", type: "if", condition: "" }] }; }
      if (["let", "assign", "store-call", "call", "while", "for-in"].includes(tag)) { props = { ...defaultSyntaxProps(tag) }; }
      if (tag === "lifecycle") {
        // 单例约束：每组件每种 hookType 仅一个（learn-omniview 实证）——检查 store.elements
        // 中已存在节点（覆盖已保存与画布新建两种来源）
        const hookType = (item.props?.hookType as "mount" | "effect" | "unmount") ?? "mount";
        const hasSame = usePoiesisStore.getState().elements.some(
          (e) => e.nodeType === "codeblock" && e.tag === "lifecycle" && (e.props?.hookType as string) === hookType,
        );
        if (hasSame) return; // 守卫命中：菜单关闭由 NodeSearchMenu select() 统一处理
        props = { hookType, deps: "" };
      }
      addElement({ id, nodeType: "codeblock", tag, props, parentId: null, position: { x: menuState.flowX + jitter, y: menuState.flowY + jitter }, source: "" });
      // ... 后续 pendingDragRef wire 逻辑不变
    },
    [menuState, addElement, addWire],
  );
```
（`usePoiesisStore.getState()` 直接读 store——守卫在 setState 回调内，不能用 hook 订阅；`deps: ""` 保证 effect 创建后 FlowInspector 的 deps 输入框有受控值。菜单关闭逻辑在 NodeSearchMenu.select 内已统一，守卫 return 后 onClose 仍执行。）

- [ ] **Step 5: flowConstants.ts + FlowInspector**

flowConstants.ts：
- CODE_TAGS 加 `"lifecycle"`；
- TAG_CN 加 `lifecycle: "lifecycle"`；
- MENU_SECTIONS「控制流」段（42-53）加**三项**（mount/effect/unmount 各一项——单例约束要求每种 hookType 独立创建路径；tag 恒为 `"lifecycle"`、label/shortcut 与既有条目同构、props 按上表——Step 4 已把 `props` 加进 `MenuItem` 类型并在创建 handler 读 `item.props`）：
  - `{ label: "@mount", tag: "lifecycle", props: { hookType: "mount" } }`
  - `{ label: "@effect", tag: "lifecycle", props: { hookType: "effect", deps: "" } }`
  - `{ label: "@unmount", tag: "lifecycle", props: { hookType: "unmount" } }`
（节点创建后的初始 props 在 FlowCanvas handleMenuSelect 的 lifecycle 分支读取 `item.props`（Step 4 已给出完整代码）——菜单条目与创建 handler 两处同源；`deps: ""` 保证 effect 创建后 FlowInspector 的 deps 输入框有受控值。）

FlowInspector.tsx：分派（159 附近）加：
```ts
  if (el.tag === "lifecycle") {
    return <LifecycleForm el={el} setProp={setProp} />;
  }
```
新组件 LifecycleForm（镜像 FnInspector 表单结构）：
```tsx
function LifecycleForm({ el, setProp }: FnInspectorProps) {
  const hookType = (el.props?.hookType as "mount" | "effect" | "unmount") ?? "mount";
  const deps = (el.props?.deps as string) || "";
  return (
    <div style={S.panel}>
      <div style={S.header}>⚡ Lifecycle</div>
      <div style={S.field}>
        <div style={S.label}>Hook type</div>
        <select style={S.select} value={hookType}
          onChange={(e) => setProp(el.id, "hookType", e.target.value)}>
          <option value="mount">mount</option>
          <option value="effect">effect</option>
          <option value="unmount">unmount</option>
        </select>
      </div>
      {hookType === "effect" && (
        <div style={S.field}>
          <div style={S.label}>Deps</div>
          <input style={S.input} value={deps} aria-label="Deps"
            placeholder="a,b" onChange={(e) => setProp(el.id, "deps", e.target.value)} />
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 6: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/components/canvas/code/nodes/__tests__/LifecycleNode.test.tsx` → PASS
Run: `cd poiesis && npm test`、`cd poiesis && npx tsc --noEmit`

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/components/canvas/code/nodes/LifecycleNode.tsx poiesis/src/components/canvas/code/nodes/__tests__/LifecycleNode.test.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/FlowInspector.tsx poiesis/src/components/canvas/code/flowConstants.ts
git commit -m "feat(codeflow): 生命周期节点 UI（LifecycleNode + 单例守卫 + LifecycleForm）"
```

---

### Task 8: try 块节点（分支行 + catch/finally 解析发射）

**Files:**
- Modify: `poiesis/src/services/codegen/graph-parser.ts`（detectSegmentType `/^try\s*\{/`；isElifElseCode 改名 isBranchContinuation 并扩展 catch/finally 正则；新 parseTryBranches/collectTryBranches；元素循环 try case；flow-out 续接循环加 try；parseSyntaxProps stmtEnd 扩展）
- Modify: `poiesis/src/services/codegen/graph-compiler.ts`（case "try"，精确镜像 condition case 143-194 行）
- Create: `poiesis/src/components/canvas/code/nodes/TryNode.tsx`（完整组件，本任务 Step 5 含全部代码）
- Modify: `poiesis/src/components/canvas/code/FlowCanvas.tsx`（nodeTypes 加 try；handleMenuSelect 加 try 默认 props；data 装配 branches cast 放宽 + onEditCatchVar）
- Modify: `poiesis/src/components/canvas/code/flowConstants.ts`（CODE_TAGS/TAG_CN/MENU_SECTIONS 三处）
- Test: `poiesis/src/services/codegen/__tests__/try-branch.test.ts`（新建）+ `poiesis/src/components/canvas/code/nodes/__tests__/TryNode.test.tsx`（新建）

**Interfaces:**
- graph-parser 内部新接口（镜像 BranchDef 51-55 行的本地定义，**不加 types.ts**——ConditionNode/parseConditionBranches 均本地定义，保持同构）：
  `TryBranchDef = { id: string; type: "try" | "catch" | "finally"; catchVar?: string }`
- 节点 props：`{ branches: Array<{ id, type: "try"|"catch"|"finally", catchVar? }> }`；分支 source handle key = `br.id`（镜像 ConditionNode）
- **分支 id 用 `br${idx}` 索引 id（与 ConditionNode 分支行同构）**——对 spec §4「真实 uid id」的**有意偏离**：分支 id 不落文本，uid 每次解析随机化会导致 wire 断连；索引 id 跨解析稳定（镜像 ConditionNode 的 br${max+1} 经验）。
- **发射格式 = 新行风格**（镜像 condition 发射器 171-182 行：每分支 `${pad}${keyword}${catchVarStr} {\n` + 分支体 + `${pad}}\n`）→ 产出 `}\ncatch err {`（`}` 是上一分支收尾，catch 行无前导 `}`）。真实文件 error_handling.omni 虽是同行风格 `} catch err {`，但现有 condition 发射器对 else 同样产出新行风格（真实文件 ThemeStore/DynamicTokenStore 两种风格混用），round-trip 标准是**重解析图稳定性**（midchain 测试的 out2 === out1 断言）而非字节相等——try 与 condition 一致，不引入第二种发射风格。
- 语义约束（UI 层）：try 必需（节点创建即含 try 分支）；catch ≤ 1；finally ≤ 1；至少 catch 或 finally 之一（`+ catch`/`+ finally` 按钮按 hasCatch/hasFinally guard 禁用）。
- 解析侧段约定（镜像中链 condition）：**延续行（`} catch err {` / `} finally {`）不带 `// @pid` 注释**——挂在前一段段尾，由 2d pendingTransition + parseSyntaxProps stmtEnd 截断处理；带 pid 的延续行会生成 code-block 元素被发射进分支体（错误）。

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/services/codegen/__tests__/try-branch.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { WireData } from "../../../core/types/wire";

/** try 中链源：镜像 graph-parser-midchain.test.ts 的段约定——
 *  fn wrapper 首行（无 pid，被 splitByPid 丢弃）+ 每 statement 一个 `// @pid:xxx` 注释
 *  + 延续行（`} catch err {` / `} finally {`）裸挂在上一 statement 段尾（无 pid）：
 *  结构行不生成元素，由 2d pendingTransition + parseSyntaxProps stmtEnd 截断处理。 */
const TRY_SRC = `fn f() {
// @pid:try-1
try {
// @pid:let-2
let raw = fs_read(file_path);
} catch err {
// @pid:call-4
log_error(err);
} finally {
// @pid:call-6
cleanup();
}
}
`;

function wireList(wires: WireData[]): string[] {
  return wires
    .map((w) => `${w.fromId}(${w.fromPort})->${w.toId}(${w.toPort})`)
    .sort();
}

describe("graph-parser try 块", () => {
  it("解析：try/catch/finally 三分支 + catchVar 提取 + 分支 wire", () => {
    const graph = parseCodeFlowGraph(TRY_SRC);
    const tryEl = graph.elements.find((e) => e.tag === "try");
    expect(tryEl).toBeDefined();
    const branches = tryEl!.props?.branches as Array<{ id: string; type: string; catchVar?: string }>;
    expect(branches.map((b) => b.type)).toEqual(["try", "catch", "finally"]);
    expect(branches.map((b) => b.id)).toEqual(["br0", "br1", "br2"]);
    expect(branches[1].catchVar).toBe("err");
    // 分支体归位：br0→let-2、br1→call-4、br2→call-6
    expect(wireList(graph.wires)).toContain("try-1(br0)->let-2(flow-in)");
    expect(wireList(graph.wires)).toContain("try-1(br1)->call-4(flow-in)");
    expect(wireList(graph.wires)).toContain("try-1(br2)->call-6(flow-in)");
  });

  it("编译 → 再解析 → 再编译幂等（新行风格发射 + fixpoint）", () => {
    const g1 = parseCodeFlowGraph(TRY_SRC);
    const out1 = compileCodeFlowGraph(g1.elements, g1.wires, "try-1")!;
    // 发射镜像 condition：每分支 `${keyword}${catchVarStr} {\n`——catch 行无前导 `}`
    expect(out1).toContain("try {");
    expect(out1).toContain("catch err {");
    expect(out1).toContain("finally {");
    expect(out1).not.toContain("} catch err {");
    expect(out1).toContain("log_error(err);");
    expect(out1).toContain("cleanup();");

    const g2 = parseCodeFlowGraph(out1);
    const out2 = compileCodeFlowGraph(g2.elements, g2.wires, "try-1")!;
    expect(out2).toBe(out1);
    expect(wireList(g2.wires)).toEqual(wireList(g1.wires));
  });

  it("catch 无变量形式（catch {）", () => {
    const graph = parseCodeFlowGraph(`fn f() {
// @pid:t-1
try {
// @pid:a-2
do_something();
} catch {
// @pid:b-3
recover();
}
}
`);
    const branches = graph.elements.find((e) => e.tag === "try")!.props?.branches as Array<{ type: string; catchVar?: string }>;
    expect(branches[1]).toMatchObject({ type: "catch", catchVar: undefined });
    expect(wireList(graph.wires)).toContain("t-1(br1)->b-3(flow-in)");
  });
});
```

TryNode 组件测试（完整文件，镜像 LoopNode.test.tsx 形态——vi.mock @xyflow/react 把 Handle 替换为 div + toBeTruthy/disabled 断言，**不依赖 jest-dom**）：

```tsx
// poiesis/src/components/canvas/code/nodes/__tests__/TryNode.test.tsx
import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/react";

vi.mock("@xyflow/react", () => ({
  Handle: () => <div data-testid="mock-handle" />,
  Position: { Top: "top", Right: "right", Bottom: "bottom" },
}));

import { TryNode } from "../TryNode";

const baseData = {
  label: "try 块",
  branches: [
    { id: "br0", type: "try" as const },
    { id: "br1", type: "catch" as const, catchVar: "err" },
    { id: "br2", type: "finally" as const },
  ],
};

describe("TryNode", () => {
  it("渲染三分支行 + catchVar 展示", () => {
    const { container } = render(<TryNode data={baseData} />);
    const rows = container.querySelectorAll('[data-testid="try-branch-row"]');
    expect(rows.length).toBe(3);
    expect(container.textContent).toContain("尝试");
    expect(container.textContent).toContain("捕获");
    expect(container.textContent).toContain("收尾");
    expect(container.textContent).toContain("err");
  });

  it("hasCatch/hasFinally 时 + 按钮禁用", () => {
    const { getByTestId } = render(<TryNode data={baseData} />);
    const addCatch = getByTestId("add-catch") as HTMLButtonElement;
    const addFinally = getByTestId("add-finally") as HTMLButtonElement;
    expect(addCatch.disabled).toBe(true);
    expect(addFinally.disabled).toBe(true);
  });

  it("仅 try 分支时按钮可用，点击追加新分支（br${max+1}）", () => {
    const onUpdateBranches = vi.fn();
    const { getByTestId } = render(
      <TryNode data={{ label: "try 块", branches: [{ id: "br0", type: "try" }], onUpdateBranches }} />,
    );
    fireEvent.click(getByTestId("add-catch"));
    expect(onUpdateBranches).toHaveBeenCalledWith([
      { id: "br0", type: "try" },
      { id: "br1", type: "catch" },
    ]);
    fireEvent.click(getByTestId("add-finally"));
    expect(onUpdateBranches).toHaveBeenCalledWith([
      { id: "br0", type: "try" },
      { id: "br1", type: "finally" },
    ]);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/try-branch.test.ts`
Expected: FAIL（try 落 fallback code-block）

- [ ] **Step 3: graph-parser.ts**（全部修改点按序执行）

(a) `SegmentType` union（31 行与 159 行两处）：加 `"try"`。
(b) `detectSegmentType`（162-193 行）：在 call 正则**之前**加：
```ts
  if (/^try\s*\{/.test(trimmed)) return "try";
```
（try 首 token 不匹配现有正则，显式前置防回归。）
(c) `isElifElseCode`（328-332 行）改名 `isBranchContinuation` 并扩展为 catch/finally 四正则全覆盖：
```ts
    // Detect if this segment is a branch continuation (elif/else/catch/finally).
    // Handles both `} else {` / `} catch err {` (legacy @pid style) and bare
    // `else {` / `catch err {` (new @type style where the closing `}` is in
    // the preceding segment).
    const isBranchContinuation =
      /^\}\s*else\s*\{/.test(trimmed) ||
      /^\}\s*else\s+if\b/.test(trimmed) ||
      /^\}\s*catch\b/.test(trimmed) ||
      /^\}\s*finally\b/.test(trimmed) ||
      /^else\s*\{/.test(trimmed) ||
      /^else\s+if\b/.test(trimmed) ||
      /^catch\b/.test(trimmed) ||
      /^finally\b/.test(trimmed);
```
同步更新全部引用：pop guard（339-354 行 `isElifElseCode && startDepth === top.baseDepth` → `isBranchContinuation && ...`）、2d guard（386-391 行 `!isElifElseCode` → `!isBranchContinuation`）。
(d) 2a 分支延续（365-369 行）：
```ts
      // 2a. @pid-style: segment starts with `} else {` / `} else if` / `} catch ...` / `} finally`
      if (
        /^\}\s*else\s*\{/.test(trimmed) ||
        /^\}\s*else\s+if\b/.test(trimmed) ||
        /^\}\s*catch\b/.test(trimmed) ||
        /^\}\s*finally\b/.test(trimmed)
      ) {
```
(e) 2b（371-375 行）：
```ts
      // 2b. @type-style continuation (no leading `}`)
      else if (
        /^else\s*\{/.test(trimmed) ||
        /^else\s+if\b/.test(trimmed) ||
        /^catch\b/.test(trimmed) ||
        /^finally\b/.test(trimmed)
      ) {
```
（发射器输出重解析时 catch 行无前导 `}` → 走 2b；真实文件同行 `} catch err {` 段首 → 走 2a。）
(f) 2d（386-391 行）：
```ts
      if (
        !isBranchContinuation &&
        (/\}\s*else\s*\{/.test(seg.code) ||
          /\}\s*else\s+if\b/.test(seg.code) ||
          /\}\s*catch\b/.test(seg.code) ||
          /\}\s*finally\b/.test(seg.code))
      ) {
        cond.pendingTransition = true;
      }
```
（挂载 `} catch err {` / `} finally {` 段尾 → pendingTransition → 下一段 2c 消费——与 `} else {` 同构。）
(g) push 规则（408-424 行）：
```ts
    if (
      seg.type === "while" ||
      seg.type === "for-in" ||
      seg.type === "try" ||
      (seg.type === "condition" && !isBranchContinuation)
    ) {
      const branches =
        seg.type === "condition"
          ? parseConditionBranches(seg.code)
          : seg.type === "try"
            ? parseTryBranches(seg.code)
            : [{ id: "br0", type: "if" as const, condition: "" }];
```
（try 段 code 恒为 `try {` → parseTryBranches 得 `[br0 try]`；完整三分支列表由 (j) 的 collectTryBranches 汇总给元素。）
(h) 新 `parseTryBranches`（放在 parseConditionBranches 之后；**完整行扫描**——逐行匹配三个 keyword 正则，出现新 keyword 时保存上一分支，末尾 push 最后一个。**不要**只用首行判断——segment 文本可能是 join 过的多行文本）：
```ts
/**
 * Parse a try segment's raw code (plus continuation segments via
 * collectTryBranches) to extract branch definitions.
 * Mirrors parseConditionBranches: full-line scan, saving each branch when
 * the next keyword appears, pushing the final one at the end.
 */
function parseTryBranches(code: string): TryBranchDef[] {
  const branches: TryBranchDef[] = [];
  const lines = code.split("\n");

  let currentType: "try" | "catch" | "finally" = "try";
  let currentVar: string | undefined;
  let branchIdx = 0;

  const tryRe = /^try\s*\{?\s*$/;
  const catchRe = /^(?:\}\s*)?catch(?:\s+(\w+))?\s*\{?\s*$/;
  const finallyRe = /^(?:\}\s*)?finally\s*\{?\s*$/;

  for (const line of lines) {
    const trimmed = line.trim();
    const match = trimmed.match(catchRe);

    if (tryRe.test(trimmed)) {
      currentType = "try";
      currentVar = undefined;
    } else if (match) {
      // Save previous branch, then switch to catch
      branches.push({ id: `br${branchIdx++}`, type: currentType, catchVar: currentVar });
      currentType = "catch";
      currentVar = match[1] || undefined;
    } else if (finallyRe.test(trimmed)) {
      // Save previous branch, then switch to finally
      branches.push({ id: `br${branchIdx++}`, type: currentType, catchVar: currentVar });
      currentType = "finally";
      currentVar = undefined;
    }
  }
  // Save last branch
  branches.push({ id: `br${branchIdx}`, type: currentType, catchVar: currentVar });

  return branches;
}
```
（`catchRe` 同时匹配同行 `} catch err {`（`(?:\}\s*)?` 消费前导 `}`）与新行 `catch err {`——两条路径都覆盖；`\w+` 捕获 catchVar。）
(i) 新 `collectTryBranches`（放在 collectConditionBranches 之后；循环终止条件同款——只收集 branchCtx.conditionPid 仍指向 try pid 的后续段）：
```ts
/**
 * Combine the try segment's own code with code from all subsequent
 * segments that fall within the try block's brace scope. This gives
 * `parseTryBranches` the full picture including catch/finally keywords
 * that live in separate segments.
 */
function collectTryBranches(
  segments: ParsedSegment[],
  tryIdx: number,
  branchCtx: Map<number, BranchContext>,
): TryBranchDef[] {
  const codeParts: string[] = [segments[tryIdx].code];
  const tryPid = segments[tryIdx].pid;

  for (let j = tryIdx + 1; j < segments.length; j++) {
    const ctx = branchCtx.get(j);
    if (ctx && ctx.conditionPid === tryPid) {
      codeParts.push(segments[j].code);
    } else {
      break;
    }
  }

  return parseTryBranches(codeParts.join("\n"));
}
```
(j) 元素循环（512-516 行的 condition case 之后加）：
```ts
    } else if (seg.type === "try") {
      // Use combined code from all segments inside the try block so that
      // catch/finally keywords in subsequent segments are picked up.
      const branches = collectTryBranches(segments, i, branchCtx);
      props = { branches };
    }
```
(k) flow-out 续接循环（628-652 行）：
```ts
    if (segType !== "condition" && segType !== "while" && segType !== "for-in" && segType !== "try") continue;
```
（try 块后置语句经 try 的 flow-out → successors 续接，与 condition 同构。）
(l) `parseSyntaxProps` 的 stmtEnd（750-753 行）：
```ts
  const stmtEnd = lines.findIndex((l) => {
    const t = l.trim();
    return (
      t === "}" ||
      /^\}\s*else/.test(t) ||
      /^else\s*(if\b|\{)/.test(t) ||
      /^\}\s*catch\b/.test(t) ||
      /^\}\s*finally\b/.test(t) ||
      /^catch\b/.test(t) ||
      /^finally\b/.test(t)
    );
  });
```
（catch/finally 结构与 else 同构：段尾延续行不落入 stmtCode；`\b` 防误伤 `catch_*`/`finally_*` 标识符。）
(m) 新本地接口（放 BranchDef 51-55 行旁，**本地定义**镜像 BranchDef——不加 types.ts）：
```ts
interface TryBranchDef {
  id: string;
  type: "try" | "catch" | "finally";
  catchVar?: string;
}
```

- [ ] **Step 4: graph-compiler.ts case "try"**（精确镜像 condition case 143-194 行——逐分支发射，分支体只走本分支的 `branchSuccessors.get(br.id)`，`@pid` 仅首行，末尾 flow-out 后继）

```ts
    case "try": {
      const branches = (node.element.props?.branches as Array<{
        id: string;
        type: "try" | "catch" | "finally";
        catchVar?: string;
      }>) || [];

      if (branches.length === 0) return "";

      let out = "";
      for (let i = 0; i < branches.length; i++) {
        const br = branches[i];
        const brPortId = br.id;

        const keyword = br.type === "try" ? "try" : br.type === "catch" ? "catch" : "finally";
        const catchVarStr = br.type === "catch" && br.catchVar ? ` ${br.catchVar}` : "";

        // Emit @pid annotation only on the first branch line (try)
        if (i === 0) {
          out += emitPidComment(pid);
        }
        out += `${pad}${keyword}${catchVarStr} {\n`;

        // Emit only the successors wired to this specific branch port
        const branchSuccIds = node.branchSuccessors.get(brPortId) || [];
        for (const succId of branchSuccIds) {
          const succ = adj.get(succId);
          if (succ) {
            out += emitNode(succ, adj, indent + 1, visited);
          }
        }

        out += `${pad}}\n`;
      }

      // Emit flow-out successors as post-try code
      for (const succId of node.successors) {
        const succ = adj.get(succId);
        if (succ) {
          out += emitNode(succ, adj, indent, visited);
        }
      }

      return out;
    }
```
（switch 位置：condition case 之后。`branchSuccessors`/`successors`/`pad`/`indent`/`visited`/`adj` 均为 condition case 已用的同级作用域变量。发射 → `}\ncatch err {` 新行风格；重解析时 catch 行走 Step 3(e) 2b，同行 `} catch err {` 走 Step 3(d) 2a。）
**注意：不要**用 `for (const [, branchIds] of node.branchSuccessors)` 全量循环——那会把 catch/finally 链也发射进 try 块。每分支只发射 `branchSuccessors.get(br.id)`。

- [ ] **Step 5: TryNode.tsx**（完整组件——分支行/公式框/按钮/样式以 `nodes/ConditionNode.tsx` 为模板逐行对齐，换 danger 色系；props 更新走 data.onUpdateBranches / data.onEditCatchVar，由 FlowCanvas 装配）

```tsx
// poiesis/src/components/canvas/code/nodes/TryNode.tsx
// try 块节点：对标 ConditionNode 分支行模式——每分支一行（行首 label + catchVar 行内
// 公式框 + source handle id=br.id），底部 flow-out source handle 接块后继。
// 语义约束：try 必需；catch ≤ 1；finally ≤ 1；至少 catch/finally 其一
// （`+ catch`/`+ finally` 按钮按 hasCatch/hasFinally 禁用）。

import { Handle, Position } from "@xyflow/react";
import React, { useMemo, useState } from "react";
import { NodeFormulaField } from "../fields/NodeFormulaField";
import { TokenPreview } from "../fields/TokenPreview";
import { buildFormulaSource, type FlowScope } from "../../../../services/codegen/flow-scope";
import "./node-base.css";

interface TryBranchData {
  id: string;
  type: "try" | "catch" | "finally";
  catchVar?: string;
}

interface Props {
  data: {
    label: string;
    branches?: TryBranchData[];
    scope?: FlowScope;
    letVars?: Array<{ name: string }>;
    onUpdateBranches?: (branches: TryBranchData[]) => void;
    onEditCatchVar?: (branchId: string, catchVar: string) => void;
  };
}

const TYPE_CN: Record<string, string> = { try: "尝试", catch: "捕获", finally: "收尾" };

// Handle 依赖 ReactFlowProvider（连线渲染）；单元测试直接渲染节点时静默跳过，避免抛错。
class HandlesBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

export function TryNode({ data }: Props) {
  const { scope, letVars } = data;
  const branches: TryBranchData[] = data.branches || [{ id: "br0", type: "try" }];
  const [editingBranch, setEditingBranch] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const formulaSource = useMemo(() => buildFormulaSource(scope, letVars ?? []), [scope, letVars]);

  const handleAddBranch = (type: "catch" | "finally") => {
    const nextId =
      branches.reduce(
        (max, b) => Math.max(max, parseInt((b.id ?? "br0").replace(/^br/, ""), 10) || 0),
        0,
      ) + 1;
    data.onUpdateBranches?.([...branches, { id: `br${nextId}`, type }]);
  };

  const handleEditStart = (br: TryBranchData) => {
    setEditingBranch(br.id);
    setEditValue(br.catchVar ?? "");
  };

  const hasCatch = branches.some((b) => b.type === "catch");
  const hasFinally = branches.some((b) => b.type === "finally");

  return (
    <div
      className="flow-node codeflow-node"
      data-node-type="try"
      style={{
        background: "var(--ol-color-danger-subtle)",
        border: "2px solid var(--ol-color-danger)",
        borderRadius: 8,
        padding: "6px 0",
        minWidth: 200,
        fontSize: 12,
      }}
    >
      <HandlesBoundary>
        <Handle
          className="codeflow-handle"
          type="target"
          position={Position.Top}
          id="flow-in"
          style={{ background: "var(--ol-color-danger)" }}
        />
      </HandlesBoundary>

      <div style={{ padding: "0 10px", fontWeight: 600, marginBottom: 4, color: "var(--ol-color-danger-active)" }}>
        ⚠ {data.label}
      </div>

      {branches.map((br, i) => (
        <div
          key={br.id}
          data-testid="try-branch-row"
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "3px 10px",
            borderTop: i > 0 ? "1px solid var(--ol-color-danger-muted)" : "none",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 4, flex: 1, minWidth: 0 }}>
            <span
              style={{
                fontSize: 12,
                fontWeight: 600,
                minWidth: 36,
                color:
                  br.type === "try"
                    ? "var(--ol-color-success-active)"
                    : br.type === "catch"
                      ? "var(--ol-color-danger)"
                      : "var(--ol-color-warning)",
              }}
            >
              {TYPE_CN[br.type]}
            </span>
            {br.type === "catch" &&
              (editingBranch === br.id ? (
                <NodeFormulaField
                  value={editValue}
                  candidates={formulaSource}
                  onCommit={(v) => {
                    data.onEditCatchVar?.(br.id, v);
                    setEditingBranch(null);
                  }}
                  onCancel={() => setEditingBranch(null)}
                  placeholder="catch 变量"
                  ariaLabel="编辑 catch 变量"
                />
              ) : (
                <button
                  type="button"
                  tabIndex={0}
                  onClick={() => handleEditStart(br)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      handleEditStart(br);
                    }
                  }}
                  style={{
                    fontFamily: "monospace",
                    fontSize: 12,
                    color: "var(--ol-gray-800)",
                    cursor: "pointer",
                    flex: 1,
                    background: "none",
                    border: "none",
                    textAlign: "left",
                    padding: 0,
                    minWidth: 0,
                  }}
                >
                  {br.catchVar ? (
                    <TokenPreview value={br.catchVar} />
                  ) : (
                    <span style={{ color: "var(--ol-text-muted)" }}>...</span>
                  )}
                </button>
              ))}
          </div>

          <HandlesBoundary>
            <Handle
              className="codeflow-handle"
              type="source"
              position={Position.Right}
              id={br.id}
              style={{ background: "var(--ol-color-danger)", position: "relative", right: -6, transform: "none" }}
            />
          </HandlesBoundary>
        </div>
      ))}

      <div style={{ display: "flex", gap: 4, padding: "4px 10px 0", borderTop: "1px solid var(--ol-color-danger-muted)" }}>
        <button
          type="button"
          data-testid="add-catch"
          disabled={hasCatch}
          onClick={() => handleAddBranch("catch")}
          style={{
            fontSize: 12,
            padding: "2px 6px",
            border: "1px solid var(--ol-color-danger)",
            borderRadius: 3,
            background: "var(--ol-surface)",
            color: "var(--ol-color-danger)",
            cursor: hasCatch ? "not-allowed" : "pointer",
            opacity: hasCatch ? 0.5 : 1,
          }}
        >
          + catch
        </button>
        <button
          type="button"
          data-testid="add-finally"
          disabled={hasFinally}
          onClick={() => handleAddBranch("finally")}
          style={{
            fontSize: 12,
            padding: "2px 6px",
            border: "1px solid var(--ol-color-danger)",
            borderRadius: 3,
            background: "var(--ol-surface)",
            color: "var(--ol-color-danger)",
            cursor: hasFinally ? "not-allowed" : "pointer",
            opacity: hasFinally ? 0.5 : 1,
          }}
        >
          + finally
        </button>
      </div>

      <HandlesBoundary>
        <Handle
          className="codeflow-handle"
          type="source"
          position={Position.Bottom}
          id="flow-out"
          style={{ background: "var(--ol-text-muted)" }}
        />
      </HandlesBoundary>
    </div>
  );
}
```
（危险色系 CSS 变量已实证存在于 tokens.css:51-52 `--ol-color-danger-subtle`/`-muted`（light+dark 双主题）；`data-node-type="try"` 对齐既有节点惯例；不需要 selected/memo——ConditionNode 也是普通函数组件。）

- [ ] **Step 6: 注册**

(a) `flowConstants.ts` 三处：
- CODE_TAGS 加 `"try"`
- TAG_CN 加 `try: "try 块"`
- MENU_SECTIONS「控制流」段加一项：`{ label: "try 块 (try)", tag: "try", shortcut: "try" }`（对象形态，对齐 T7 的 lifecycle 项）

(b) `FlowCanvas.tsx` nodeTypes 加 `"try": TryNode,` + 顶部 import `TryNode`（对齐既有节点 import 行）。
(c) `handleMenuSelect`——T7 已把签名改成 `(item: MenuItem)`（若 T7 未合入则按当前 `(tag: string)` 形态等价处理）：在 condition 默认 props 分支后加：
```ts
      if (item.tag === "try") {
        props = { branches: [{ id: "br0", type: "try" }] };
      }
```
（创建即含 try 分支——语义约束「try 必需」；catch/finally 由节点内 `+ catch`/`+ finally` 按钮追加。）
(d) data 装配（316-360 行）：
- 331 行 branches cast 放宽（condition 三字段 → 四字段可选）：
```ts
branches: el.props?.branches as Array<{id:string;type:string;condition?:string;catchVar?:string}> | undefined,
```
- 342-349 行 onEditCondition 之后加平行的 `onEditCatchVar`：
```ts
onEditCatchVar: (branchId: string, catchVar: string) => {
  const currentBranches = [...(el.props?.branches as Array<{id:string;type:string;catchVar?:string}> || [])];
  const idx = currentBranches.findIndex((b) => b.id === branchId);
  if (idx >= 0) {
    currentBranches[idx] = { ...currentBranches[idx], catchVar };
    setProp(el.id, "branches", currentBranches);
  }
},
```
（onUpdateBranches 339-341 行泛型已兼容 try 分支——直接复用。）
(e) `nodeTypes` 注册后，FlowCanvas 的 onDoubleClick 注入（338 行：fn/while/for-in 排除）**不需要改**——try 不在排除列表，自动获得双击进 Monaco 行为（与 condition 一致，编辑态由行内公式框承载）。

- [ ] **Step 7: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/try-branch.test.ts src/components/canvas/code/nodes/__tests__/TryNode.test.tsx` → PASS
Run: `cd poiesis && npm test`（重点：graph-compiler-branch-id / graph-parser-midchain / condition 系列——分支机制改动不得破坏 condition）
Run: `cd poiesis && npx tsc --noEmit`

- [ ] **Step 8: Commit**

```bash
git add poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/graph-compiler.ts poiesis/src/components/canvas/code/nodes/TryNode.tsx poiesis/src/components/canvas/code/FlowCanvas.tsx poiesis/src/components/canvas/code/flowConstants.ts poiesis/src/services/codegen/__tests__/try-branch.test.ts poiesis/src/components/canvas/code/nodes/__tests__/TryNode.test.tsx
git commit -m "feat(codeflow): try 块节点（分支行 + catch/finally 解析发射）"
```

---

### Task 9: lib 文件保存全链路（emitLibFile + code tab 分支 + round-trip，修复 G5）

**Files:**
- Create: `poiesis/src/services/codegen/emit-lib-file.ts`
- Modify: `poiesis/src/hooks/useSaveProject.ts`（store 分支 183 与组件分支之间插 code 分支；组件分支写盘前 ROOT 前置；store 分支 writeFile 前 ROOT 前置）
- Test: `poiesis/src/services/codegen/__tests__/lib-file-roundtrip.test.ts`（新建，真实文件三件套 + fixpoint）

**Interfaces:**
- Consumes: `LibFileDef`（T1）、`compileCodeFlowGraph(elements, wires, entryId)`（graph-compiler）、`CodeWriter`（source-emitter 导出）、store 分支的 `collectProjectSources` / `validateSources` / `formatDiagnostics` / `api.project.writeFile` / `markClean`（全部已在 useSaveProject 作用域）
- Produces: `emitLibFile(def: LibFileDef, graphs: Record<string, { elements; wires }>): string`——imports → structs → types → fns；**不发射 `import omniview;`**；fn 逐个 compileCodeFlowGraph（pub/async/action 双格式经 T3 的 fn case）
- Produces: `emitRootDeclarations(def: LibFileDef): string`——仅 structs + types（组件/store 文件头部前置用；imports 不前置）
- code tab 保存分支：画布 → libDef 写回（fn 增删对齐）→ emitLibFile → WASM 验证 → 写盘 + saveProject + markClean

- [ ] **Step 1: 写失败测试**（真实 learn-omniview 文件三件套 + fixpoint）

```ts
// poiesis/src/services/codegen/__tests__/lib-file-roundtrip.test.ts
import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { parseOmniSource } from "../omni-parser";
import { emitLibFile, emitRootDeclarations } from "../emit-lib-file";

// __tests__ 上溯 5 级到仓库根（poiesis/src/services/codegen/__tests__ → OmniLang 根）
const FIXTURE = (rel: string) =>
  new URL(`../../../../../test/projects/learn-omniview/src/${rel}`, import.meta.url);

describe("emitLibFile round-trip（真实 learn-omniview 文件）", () => {
  const fixtures = ["lib/types.omni", "lib/async_demo.omni", "lib/error_handling.omni"];

  for (const rel of fixtures) {
    it(`fixpoint: ${rel}`, () => {
      const src = readFileSync(FIXTURE(rel), "utf-8");
      const r = parseOmniSource(src, rel);
      const def = r.fileDefs![rel];
      expect(def).toBeDefined();
      const out1 = emitLibFile(def, r.codeFlowGraphs ?? {});
      // 纯逻辑文件不发射 import omniview;
      expect(out1).not.toContain("import omniview");
      // 再解析 → 再发射 → 文本不变（pid 注释稳定）
      const r2 = parseOmniSource(out1, rel);
      const out2 = emitLibFile(r2.fileDefs![rel], r2.codeFlowGraphs ?? {});
      expect(out2).toBe(out1);
    });
  }

  it("ROOT 段 round-trip：组件/store 文件头部 struct 保存不丢（FeatureCard/TodoStore 实证）", () => {
    for (const rel of ["components/FeatureCard.omni", "stores/TodoStore.omni"]) {
      const src = readFileSync(FIXTURE(rel), "utf-8");
      const r = parseOmniSource(src, rel);
      const def = r.fileDefs![rel];
      expect(def).toBeDefined();
      if (def.structs.length > 0 || def.types.length > 0) {
        const out1 = emitRootDeclarations(def);
        const r2 = parseOmniSource(out1, rel);
        const out2 = emitRootDeclarations(r2.fileDefs![rel]);
        expect(out2).toBe(out1); // ROOT 段文本稳定
      }
    }
  });

  it("emitRootDeclarations 只含 structs/types（组件/store 头部前置用）", () => {
    const r = parseOmniSource(
      `pub struct Point { x: i32, }\ncomponent A { view { <div id="a"></div> } }\n`,
      "src/components/A.omni",
    );
    const prefix = emitRootDeclarations(r.fileDefs!["src/components/A.omni"]);
    expect(prefix).toContain("pub struct Point {");
    expect(prefix).toContain("x: i32,");
    expect(prefix).not.toContain("import");
  });

  it("画布新建 fn（无磁盘对应）也被发射（writeback 语义）", () => {
    const src = readFileSync(FIXTURE("lib/types.omni"), "utf-8");
    const r = parseOmniSource(src, "src/lib/types.omni");
    const def = r.fileDefs!["src/lib/types.omni"];
    // 手工补一个未在磁盘的 fn 图（模拟画布新建 fn 节点 → writeback 追加 def.fns）
    const extraPid = "fn_extra_el_abc";
    const out = emitLibFile({ ...def, fns: [...def.fns, { pid: extraPid, name: "extra", params: [], returnType: "", body: "", pub: false, isAsync: false, declKind: "fn" }] }, {
      ...(r.codeFlowGraphs ?? {}),
      extra: { elements: [{ id: extraPid, nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "extra", params: "", returnType: "" }, source: "" }], wires: [] },
    });
    expect(out).toContain("fn extra() {");
  });
});
```
（若 async_demo.omni / error_handling.omni 的断言需要按文件实际内容微调——如 union 行——以文件为准；fixpoint 断言不变。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/lib-file-roundtrip.test.ts`
Expected: FAIL（emit-lib-file 模块不存在）

- [ ] **Step 3: emit-lib-file.ts**

```ts
// poiesis/src/services/codegen/emit-lib-file.ts
// lib 文件发射器：imports → structs → types → fns（spec §6.4）。
// 纯逻辑文件不发射 `import omniview;`（learn-omniview 实证）。

import { CodeWriter } from "./source-emitter";
import { compileCodeFlowGraph } from "./graph-compiler";
import type { LibFileDef } from "./types";
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types";

/** 发射整个 lib 文件（code tab 保存 / round-trip 共用）。
 *  graphs 按 fn 名键（parse 侧 codeFlowGraphs / 保存侧画布 writeback 均为此形态）。 */
export function emitLibFile(
  def: LibFileDef,
  graphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }>,
): string {
  const w = new CodeWriter();
  for (const imp of def.imports) {
    // 纯逻辑文件不发射 import omniview;（存储形态是 token 无空格拼接 "importomniview;"）
    if (imp.replace(/\s+/g, "") === "importomniview;") continue;
    w.emit(imp);
  }
  if (def.imports.length > 0) w.blank();

  const root = emitRootDeclarations(def);
  w.emit(root.trimEnd());

  const blocks: string[] = [];
  for (const f of def.fns) {
    const graph = graphs[f.name];
    if (!graph) continue;
    const compiled = compileCodeFlowGraph(graph.elements, graph.wires, f.pid);
    if (compiled) blocks.push(compiled);
  }
  const text = w.toString();
  const rootText = root.trim();
  // 组装：imports + structs/types + fns（fn 间空行分隔）
  const parts = [text.trimEnd(), rootText, blocks.join("\n\n")].filter((p) => p.length > 0);
  return parts.join("\n\n") + "\n";
}

/** ROOT 声明发射（lib 文件的 structs/types 段；组件/store 文件头部前置共用）。
 *  不发射 imports——组件/store 的 imports 由 pipeline import-collector 处理。 */
export function emitRootDeclarations(def: LibFileDef): string {
  const w = new CodeWriter();
  for (const s of def.structs) {
    w.emit(`${s.pub ? "pub " : ""}struct${s.nominal ? " nominal" : ""} ${s.name} {`);
    w.indent();
    for (const f of s.fields) {
      if (f.type) w.emit(`${f.name}: ${f.type},`);
    }
    w.dedent();
    w.emit(`}`);
    w.blank();
  }
  for (const t of def.types) {
    w.emit(`${t.pub ? "pub " : ""}type ${t.name} = ${t.value};`);
    w.blank();
  }
  return w.toString();
}
```
（发射顺序验证：`emitRootDeclarations` 在 imports 之后发射——`w.emit(root.trimEnd())` 追加到已含 imports 的 writer。若顺序与测试预期不符，调整组装逻辑——**imports → structs → types → fns 为硬性顺序**。）

- [ ] **Step 4: useSaveProject.ts —— code tab 分支**

`useSaveProject.ts` 在 store 分支闭合（183 行 `}`）之后、组件分支之前插入（若组件分支是 `} else {`，改为 `} else if (activeTab?.tabType === "code") { ... } else {`）：
```ts
      } else if (activeTab?.tabType === "code" && activeTab.filePath) {
        // ── Code tab：lib 文件保存（对标 store 分支结构）──
        const libPath = activeTab.filePath;
        const libDef = store.libDefs[libPath];
        if (!libDef) {
          setSaveError("libDef 未加载，无法保存");
          setSaving(false);
          return;
        }
        // 1. 画布 → libDef 写回：fn 增删对齐（画布新建 fn 追加、已删 fn 剔除）
        const workingFns = store.elements.filter(
          (e): e is CodeblockElementData => e.nodeType === "codeblock" && (e.tag === "fn" || e.tag === "action"),
        );
        const workingPids = new Set(workingFns.map((e) => e.id));
        const addedFns = workingFns
          .filter((e) => !libDef.fns.some((f) => f.pid === e.id))
          .map((e) => ({
            pid: e.id,
            name: (e.props?.name as string) || "unnamed",
            params: ((e.props?.params as string) || "").split(",").flatMap((p) => {
              const [pName, pType] = p.trim().split(":").map((s) => s.trim());
              return pName ? [{ name: pName, type: pType || "" }] : [];
            }),
            returnType: (e.props?.returnType as string) || "",
            pub: (e.props?.pub as boolean) || false,
            isAsync: (e.props?.isAsync as boolean) || false,
            declKind: (e.props?.declKind as "fn" | "action") || "fn",
            body: "",
          }));
        const updatedDef = { ...libDef, fns: [...libDef.fns.filter((f) => workingPids.has(f.pid)), ...addedFns] };
        usePoiesisStore.setState({ libDefs: { ...store.libDefs, [libPath]: updatedDef } });

        // 2. 画布 writeback 图（按 fn 入口 pid 提取子图）→ emitLibFile
        const graphs: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> = {};
        for (const f of updatedDef.fns) {
          graphs[f.name] = extractFnSubgraph(f.pid, store.elements, store.wires);
        }
        const source = emitLibFile(updatedDef, graphs);

        // 3. WASM 验证（门禁为最终裁决，与 store/组件分支同款）
        const extraSources: Array<{ path: string; content: string }> = [];
        if (designSource && designFilePath) {
          extraSources.push({ path: designFilePath, content: designSource });
        }
        const allSources = await collectProjectSources({ [libPath]: source }, extraSources);
        const analyseErrors = await validateSources(allSources);
        const criticalErrors = analyseErrors.filter((e) => e.severity === "error");
        if (criticalErrors.length > 0) {
          setSaveError(formatDiagnostics(criticalErrors));
          log.error("WASM analyse errors:", criticalErrors);
          setSaving(false);
          return;
        }

        // 4. 写盘 + saveProject + markClean
        await api.project.writeFile(libPath, source);
        usePoiesisStore.setState({ activeFileContent: source });
        const result = await api.annotation.saveProject();
        if (result.ok) markClean();
      }
```
（`designSource` / `designFilePath` / `log` / `setSaveError` / `setSaving` / `collectProjectSources` / `validateSources` / `formatDiagnostics` / `api` / `markClean` 均已在 useSaveProject 既有作用域——形态与 store 分支 170-180 一致。）

**helper**（useSaveProject.ts 文件级，或就近放 code 分支外）：
```ts
/** 从全局画布提取 fn 子图（入口 pid + wire 闭包可达的全部节点）。 */
function extractFnSubgraph(
  entryPid: string,
  allElements: ElementData[],
  allWires: WireData[],
): { elements: CodeblockElementData[]; wires: WireData[] } {
  const ids = new Set<string>([entryPid]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const w of allWires) {
      if (ids.has(w.fromId) && !ids.has(w.toId)) { ids.add(w.toId); grew = true; }
      if (ids.has(w.toId) && !ids.has(w.fromId)) { ids.add(w.fromId); grew = true; }
    }
  }
  return {
    elements: allElements.filter((e) => ids.has(e.id)) as CodeblockElementData[],
    wires: allWires.filter((w) => ids.has(w.fromId) && ids.has(w.toId)),
  };
}
```
（`ElementData` 等类型 import 按 useSaveProject.ts 既有 import 调整；`CodeblockElementData` 从 core/types/element 导入。）

- [ ] **Step 5: ROOT 层前置（组件/store 分支）**

(a) store 分支：writeFile（179）**之前**、emitStoreDefinition 产出 `source` 之后：
```ts
        // ROOT 层前置：store 文件头部 structs/types 不丢（TodoStore.omni 实证）
        const storeRoot = store.libDefs[activeTab?.filePath ?? ""];
        if (storeRoot && (storeRoot.structs.length > 0 || storeRoot.types.length > 0)) {
          source = emitRootDeclarations(storeRoot) + source;
        }
```
(b) 组件分支：写盘循环（247-249）**之前**：
```ts
        // ROOT 层前置：组件文件头部 structs/types 不丢（FeatureCard.omni 实证）
        const compRoot = store.libDefs[activeTab?.filePath ?? ""];
        if (compRoot && (compRoot.structs.length > 0 || compRoot.types.length > 0)) {
          const tPath = activeTab?.filePath ?? "src/main.omni";
          if (genResult.value[tPath]) {
            genResult.value[tPath] = emitRootDeclarations(compRoot) + genResult.value[tPath];
          }
        }
```
顶部 import `emitLibFile, emitRootDeclarations` from "../services/codegen/emit-lib-file"。

- [ ] **Step 6: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/services/codegen/__tests__/lib-file-roundtrip.test.ts` → PASS
Run: `cd poiesis && npm test`（重点：save-inline-store / inline-store-roundtrip / roundtrip——useSaveProject 改动不得破坏 store/组件保存）
Run: `cd poiesis && npx tsc --noEmit`

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/services/codegen/emit-lib-file.ts poiesis/src/hooks/useSaveProject.ts poiesis/src/services/codegen/__tests__/lib-file-roundtrip.test.ts
git commit -m "feat(codeflow): lib 文件保存全链路（emitLibFile + code tab 分支 + ROOT 前置 + round-trip）"
```

---

### Task 10: LibMembersPanel（imports/structs/types 常驻左面板）

**Files:**
- Create: `poiesis/src/components/code/LibMembersPanel.tsx`
- Modify: `poiesis/src/pages/DesignPage.tsx`（grid 224-235 加 code 分支；左面板 258 后加 code tab 分支；中心画布 gridColumn ~302 加 code 分支）
- Test: `poiesis/src/components/code/__tests__/LibMembersPanel.test.tsx`（新建，镜像 StoreMembersPanel 测试形态——若其无测试则以 ComponentDefPanel.test.tsx 为模型）

**Interfaces:**
- Consumes: `PoiesisState.libDefs` / `setLibDef`（T2）、`activeTab.filePath`（designPage.openTabs）、`LibFileDef/StructDef/TypeDef`（T1）
- Produces: 三段面板——Imports（自由文本行编辑 + 增删）/ Structs（pub/nominal 开关 + name + fields 行列表增删）/ Types（pub 开关 + name + 类型表达式文本）；全部经 setLibDef 不可变写回
- 发射格式（与 emitLibFile/emitRootDeclarations 一致——面板只改数据不发射，保存走 T9）
- 位置：左侧固定 280px 常驻列（进入 code tab 自动显示，无按钮开关——用户已确认）

- [ ] **Step 1: 写失败测试**

```ts
// poiesis/src/components/code/__tests__/LibMembersPanel.test.tsx
import { describe, it, expect } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { LibMembersPanel } from "../LibMembersPanel";
import { usePoiesisStore } from "../../store";

const stubDef = {
  imports: [`import{sleep}from"std/async";`],
  structs: [{ pid: "s1", name: "Point", pub: true, nominal: false, fields: [{ name: "x", type: "i32" }] }],
  types: [{ pid: "t1", name: "Score", pub: false, value: "i32" }],
  fns: [],
};

describe("LibMembersPanel", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      libDefs: { "src/lib/a.omni": stubDef },
      designPage: { ...usePoiesisStore.getState().designPage, activeTabId: "code-a", openTabs: [{ id: "code-a", tabType: "code", filePath: "src/lib/a.omni", componentName: "a", label: "a" } as never] },
    });
  });

  it("三段渲染：Imports/Structs/Types", () => {
    render(<LibMembersPanel />);
    expect(screen.getByText("Imports")).toBeInTheDocument();
    expect(screen.getByText("Structs")).toBeInTheDocument();
    expect(screen.getByText("Types")).toBeInTheDocument();
    expect(screen.getByDisplayValue(`import{sleep}from"std/async";`)).toBeInTheDocument();
    expect(screen.getByDisplayValue("Point")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Score")).toBeInTheDocument();
  });

  it("添加 import 写回 libDefs", () => {
    render(<LibMembersPanel />);
    fireEvent.click(screen.getByText("+ Import"));
    const input = screen.getByPlaceholderText("import ...");
    fireEvent.change(input, { target: { value: "import{other}from\"std/x\";" } });
    expect(usePoiesisStore.getState().libDefs["src/lib/a.omni"].imports).toContain(`import{other}from"std/x";`);
  });

  it("struct pub 开关切换写回", () => {
    render(<LibMembersPanel />);
    fireEvent.click(screen.getAllByRole("checkbox")[0]);
    expect(usePoiesisStore.getState().libDefs["src/lib/a.omni"].structs[0].pub).toBe(false);
  });
});
```
（openTabs 条目形状以 designPage.openTabs 既有类型为准；若 Tab 类型字段名不同，按类型调整。）

- [ ] **Step 2: 运行测试确认失败**

Run: `cd poiesis && npm test -- src/components/code/__tests__/LibMembersPanel.test.tsx`
Expected: FAIL（模块不存在）

- [ ] **Step 3: LibMembersPanel.tsx**

以 `components/store/StoreMembersPanel.tsx` 为模板（S 样式对象 / OMNI_TYPES 下拉 / 面板结构逐行对齐）：
```tsx
// poiesis/src/components/code/LibMembersPanel.tsx
// Code tab 文件级成员面板（常驻左侧 280px，无按钮开关——用户已确认）。
// 三段：Imports（自由文本）/ Structs（pub/nominal + fields 行列表）/ Types（pub + 类型表达式文本）。
// 只改数据（setLibDef 不可变写回），发射由保存侧 emitLibFile 完成。

import { usePoiesisStore } from "../../store";
import type { LibFileDef, StructDef, TypeDef } from "../../services/codegen/types";

// S 样式对象照抄 StoreMembersPanel（panel/sectionHeader/addBtn/itemRow/itemHeader/input/select/removeBtn/typeSep/valueRow）
const S = { ... };

export function LibMembersPanel() {
  const libDefs = usePoiesisStore((s) => s.libDefs);
  const setLibDef = usePoiesisStore((s) => s.setLibDef);
  const activeTab = usePoiesisStore((s) => {
    const id = s.designPage.activeTabId;
    if (!id) return null;
    return s.designPage.openTabs.find((t) => t.id === id) ?? null;
  });
  const filePath = activeTab?.filePath ?? "";
  const def: LibFileDef = libDefs[filePath] ?? { imports: [], structs: [], types: [], fns: [] };

  const update = (fn: (d: LibFileDef) => LibFileDef) => {
    if (!filePath) return;
    setLibDef(filePath, fn(def));
  };

  const addImport = () => update((d) => ({ ...d, imports: [...d.imports, ""] }));
  const setImport = (i: number, v: string) =>
    update((d) => ({ ...d, imports: d.imports.map((x, idx) => (idx === i ? v : x)) }));
  const removeImport = (i: number) =>
    update((d) => ({ ...d, imports: d.imports.filter((_, idx) => idx !== i) }));

  const addStruct = () =>
    update((d) => ({ ...d, structs: [...d.structs, { pid: `struct_${crypto.randomUUID()}`, name: "", pub: false, nominal: false, fields: [] }] }));
  const setStruct = (i: number, patch: Partial<StructDef>) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => (idx === i ? { ...s, ...patch } : s)) }));
  const removeStruct = (i: number) =>
    update((d) => ({ ...d, structs: d.structs.filter((_, idx) => idx !== i) }));
  const setStructField = (si: number, fi: number, patch: { name?: string; type?: string }) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => idx === si
      ? { ...s, fields: s.fields.map((f, fIdx) => (fIdx === fi ? { ...f, ...patch } : f)) } : s) }));
  const addStructField = (si: number) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => idx === si
      ? { ...s, fields: [...s.fields, { name: "", type: "" }] } : s) }));
  const removeStructField = (si: number, fi: number) =>
    update((d) => ({ ...d, structs: d.structs.map((s, idx) => idx === si
      ? { ...s, fields: s.fields.filter((_, fIdx) => fIdx !== fi) } : s) }));

  const addType = () =>
    update((d) => ({ ...d, types: [...d.types, { pid: `type_${crypto.randomUUID()}`, name: "", pub: false, value: "" }] }));
  const setType = (i: number, patch: Partial<TypeDef>) =>
    update((d) => ({ ...d, types: d.types.map((t, idx) => (idx === i ? { ...t, ...patch } : t)) }));
  const removeType = (i: number) =>
    update((d) => ({ ...d, types: d.types.filter((_, idx) => idx !== i) }));

  // JSX：三段面板（每段 sectionHeader + addBtn；条目行 itemRow）
  //  Imports：自由文本 input（placeholder "import ..."）+ 删除按钮
  //  Structs：名称 input + pub/nominal 开关 + fields 行（name: Type 两 input + 删除）+ "+ Field"
  //  Types：名称 input + pub 开关 + 类型表达式 input（placeholder 如 "i32" / "Ok | Err(String)"）
  return (
    <div style={S.panel}>
      {/* Imports 段 */}
      <div style={S.sectionHeader}>
        <span>Imports</span>
        <button type="button" style={S.addBtn} onClick={addImport}>+ Import</button>
      </div>
      {def.imports.map((imp, i) => (
        <div key={i} style={S.itemRow}>
          <input style={S.input} value={imp} placeholder="import ..."
            onChange={(e) => setImport(i, e.target.value)} />
          <button type="button" style={S.removeBtn} onClick={() => removeImport(i)}>×</button>
        </div>
      ))}
      {/* Structs 段 */}
      <div style={S.sectionHeader}>
        <span>Structs</span>
        <button type="button" style={S.addBtn} onClick={addStruct}>+ Struct</button>
      </div>
      {def.structs.map((s, si) => (
        <div key={s.pid} style={S.itemRow}>
          <input style={S.input} value={s.name} placeholder="struct name"
            onChange={(e) => setStruct(si, { name: e.target.value })} />
          <label style={{ fontSize: 12 }}>
            <input type="checkbox" checked={s.pub}
              onChange={(e) => setStruct(si, { pub: e.target.checked })} /> pub
          </label>
          <label style={{ fontSize: 12 }}>
            <input type="checkbox" checked={s.nominal}
              onChange={(e) => setStruct(si, { nominal: e.target.checked })} /> nominal
          </label>
          <button type="button" style={S.removeBtn} onClick={() => removeStruct(si)}>×</button>
          {s.fields.map((f, fi) => (
            <div key={fi} style={S.valueRow}>
              <input style={{ ...S.input, width: 72 }} value={f.name} placeholder="field"
                onChange={(e) => setStructField(si, fi, { name: e.target.value })} />
              <span style={S.typeSep}>:</span>
              <input style={S.input} value={f.type} placeholder="Type"
                onChange={(e) => setStructField(si, fi, { type: e.target.value })} />
              <button type="button" style={S.removeBtn} onClick={() => removeStructField(si, fi)}>×</button>
            </div>
          ))}
          <button type="button" style={S.addBtn} onClick={() => addStructField(si)}>+ Field</button>
        </div>
      ))}
      {/* Types 段 */}
      <div style={S.sectionHeader}>
        <span>Types</span>
        <button type="button" style={S.addBtn} onClick={addType}>+ Type</button>
      </div>
      {def.types.map((t, ti) => (
        <div key={t.pid} style={S.itemRow}>
          <input style={{ ...S.input, width: 96 }} value={t.name} placeholder="type name"
            onChange={(e) => setType(ti, { name: e.target.value })} />
          <label style={{ fontSize: 12 }}>
            <input type="checkbox" checked={t.pub}
              onChange={(e) => setType(ti, { pub: e.target.checked })} /> pub
          </label>
          <span style={S.typeSep}>=</span>
          <input style={S.input} value={t.value} placeholder="Ok | Err(String)"
            onChange={(e) => setType(ti, { value: e.target.value })} />
          <button type="button" style={S.removeBtn} onClick={() => removeType(ti)}>×</button>
        </div>
      ))}
    </div>
  );
}
```
（`crypto.randomUUID()` 若项目已有 uid 工具则替换为既有 `uid()`（core/utils/id）——以 StoreMembersPanel 的既有 id 生成方式为准。S 样式对象从 StoreMembersPanel.tsx:1-120 逐项复制。）

- [ ] **Step 4: DesignPage 布局接线**

(a) grid（224-235）加 code 分支：
```ts
          gridTemplateColumns: showDesignPanels
            ? "280px 1fr 300px"
            : showCodeFlowPanel
              ? "280px 1fr 240px"
              : activeTabType === "code"
                ? "280px 1fr"
                : "1fr",
```
(b) 左面板（258 行 showDesignPanels 块之后）加：
```tsx
        {/* Left panel: LibMembersPanel（code tab 常驻，无按钮开关） */}
        {activeTabType === "code" && <LibMembersPanel />}
```
(c) 中心画布 gridColumn（~302）加 code 分支：`activeTabType === "code" ? 2 : ...`（与 showDesignPanels/showCodeFlowPanel 同列的既有条件链并列）。
顶部 import LibMembersPanel。

- [ ] **Step 5: 测试通过 + 回归 + tsc**

Run: `cd poiesis && npm test -- src/components/code/__tests__/LibMembersPanel.test.tsx` → PASS
Run: `cd poiesis && npm test`、`cd poiesis && npx tsc --noEmit`

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/components/code/LibMembersPanel.tsx poiesis/src/components/code/__tests__/LibMembersPanel.test.tsx poiesis/src/pages/DesignPage.tsx
git commit -m "feat(codeflow): LibMembersPanel（imports/structs/types 常驻左面板）"
```

---

## 验证清单（全部任务完成后）

1. `cd poiesis && npm test` 全绿；`cd poiesis && npx tsc --noEmit` 无错误。
2. 手工真机（Victauri）：打开 learn-omniview 项目 → lib 文件 tab 显示左侧 LibMembersPanel + 画布含各 fn 图；进入组件 tab codeflow 模式显示 @effect/@mount 节点与 try/throw/await 节点；编辑后保存 → WASM 门禁通过 → 重开项目内容不丢。
3. learn-omniview 现有文件 WASM 门禁通过为基线（保存不得引入新 critical error）。
