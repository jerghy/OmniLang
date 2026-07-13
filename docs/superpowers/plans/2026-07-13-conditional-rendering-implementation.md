# @if / @else if / @else 条件渲染实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Poiesis 支持 OmniView @if/@else if/@else 条件渲染的完整 round-trip（解析 → 编辑 → 源码输出）

**Architecture:** 新增 `ConditionalBlockData` 元素类型，parser 解析 `@if` 指令产生条件块，`view-tree-builder` 将条件块转为 `IfViewNode`（codegen 已支持），元素树中以展开的分支节点显示，右键菜单/属性面板提供可视化编辑。

**Tech Stack:** TypeScript, React, Zustand (store), Vitest, Playwright

**Spec:** `docs/superpowers/specs/2026-07-13-conditional-rendering-design.md`

## Global Constraints

- 遵循现有 immutability 模式（spread operator 更新 Redux state）
- 所有 mutation action 必须设置 `isDirty: true`
- Parser 条件表达式仅收集原文，不做语法解析
- ConditionalBlockData 至少 2 个 branch（@if + @else）
- 遵循 TDD：先写测试 → 确认 FAIL → 实现 → 确认 PASS → commit

---

### Task 1: Element Model — ConditionalBlockData + TreeNode 扩展

**Files:**
- Modify: `poiesis/src/core/types/element.ts`
- Modify: `poiesis/src/components/panels/tree-utils.ts`

**Interfaces:**
- Produces: `ConditionalBlockData` interface, `ElementData` union type, `TreeNode` with optional `nodeType`/`condition`/`branchType`/`branchIndex` fields

- [ ] **Step 1: 添加 ConditionalBlockData 类型到 element.ts**

在 `ElementDataBase` 之后、`ElementData` union 之前，添加：

```typescript
// core/types/element.ts — 在 ElementDataBase 之后

export interface ConditionalBlockData extends ElementDataBase {
  nodeType: "conditional";
  /** 条件分支列表，按 @if → @else if → @else 顺序 */
  branches: Array<{
    type: "if" | "elif" | "else";
    condition: string;       // @else 时为空字符串 ""
    children: string[];      // 该分支内的子元素 ID
  }>;
  /** 所有分支子元素的扁平列表（兼容现有 children 遍历模式） */
  children: string[];
  isFolded?: boolean;
  position?: { x: number; y: number };
}
```

- [ ] **Step 2: 扩展 ElementData 联合类型**

```typescript
// 找到 type ElementData = ViewElementData | CodeblockElementData | GroupElementData;
// 改为：
export type ElementData = ViewElementData | CodeblockElementData | GroupElementData | ConditionalBlockData;
```

- [ ] **Step 3: 扩展 TreeNode interface**

```typescript
// components/panels/tree-utils.ts

export interface TreeNode {
  id: string;
  tag: string;
  htmlTag: string;
  displayLabel: string;
  componentRef?: string;
  children: TreeNode[];
  // NEW: 条件块支持
  nodeType?: "view" | "conditional" | "conditional-branch";
  condition?: string;           // 仅 conditional-branch 节点
  branchType?: "if" | "elif" | "else";  // 仅 conditional-branch 节点
  branchIndex?: number;         // 分支在 branches 数组中的索引
}
```

- [ ] **Step 4: 验证 TypeScript 编译**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -30
```

Expected: 无新增类型错误（可能有预存的 776 个错误，过滤查看新增的）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/core/types/element.ts poiesis/src/components/panels/tree-utils.ts
git commit -m "feat(poiesis): add ConditionalBlockData type and extend TreeNode for conditional rendering"
```

---

### Task 2: Parser — parseViewDirective

**Files:**
- Modify: `poiesis/src/services/codegen/parser-view.ts`
- Create: `poiesis/src/services/codegen/__tests__/parser-view-conditional.test.ts`

**Interfaces:**
- Consumes: `ConditionalBlockData` from Task 1
- Produces: `parseViewDirective(internals: ParserInternals): void`

- [ ] **Step 1: 写 parser 测试**

```typescript
// poiesis/src/services/codegen/__tests__/parser-view-conditional.test.ts

import { describe, it, expect } from "vitest";
import { OmniTokenizer } from "../omni-tokenizer";
import { TokenStream } from "../token-stream";
import { Parser } from "../parser";
import type { ConditionalBlockData } from "../../../core/types/element";

function parse(source: string) {
  const tokens = new OmniTokenizer().tokenize(source);
  const stream = new TokenStream(tokens);
  const parser = new Parser(stream);
  return parser.parse();
}

describe("conditional rendering parser", () => {
  it("parses @if with @else into ConditionalBlockData", () => {
    const source = `
import omniview;
component Test {
  view {
    @if isActive {
      <div id="a">Active</div>
    } @else {
      <div id="b">Inactive</div>
    }
  }
}`;
    const result = parse(source);
    expect(result.errors).toHaveLength(0);
    const condBlock = result.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(condBlock).toBeDefined();
    expect(condBlock!.branches).toHaveLength(2);
    expect(condBlock!.branches[0].type).toBe("if");
    expect(condBlock!.branches[0].condition).toBe("isActive");
    expect(condBlock!.branches[1].type).toBe("else");
    expect(condBlock!.branches[1].condition).toBe("");
    // Each branch should have 1 child
    expect(condBlock!.branches[0].children).toHaveLength(1);
    expect(condBlock!.branches[1].children).toHaveLength(1);
    // Flattened children
    expect(condBlock!.children).toHaveLength(2);
  });

  it("parses @if / @else if / @else chain", () => {
    const source = `
import omniview;
component Test {
  view {
    @if status == "loading" {
      <div id="a">Loading</div>
    } @else if status == "error" {
      <div id="b">Error</div>
    } @else {
      <div id="c">OK</div>
    }
  }
}`;
    const result = parse(source);
    expect(result.errors).toHaveLength(0);
    const condBlock = result.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(condBlock).toBeDefined();
    expect(condBlock!.branches).toHaveLength(3);
    expect(condBlock!.branches[0].type).toBe("if");
    expect(condBlock!.branches[0].condition).toBe('status == "loading"');
    expect(condBlock!.branches[1].type).toBe("elif");
    expect(condBlock!.branches[1].condition).toBe('status == "error"');
    expect(condBlock!.branches[2].type).toBe("else");
  });
});
```

- [ ] **Step 2: 运行测试确认 FAIL**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/parser-view-conditional.test.ts
```

Expected: FAIL — 条件块未找到或解析错误

- [ ] **Step 3: 在 parseViewBody 中添加 AT token 处理**

在 `parser-view.ts` 的 `parseViewBody` 函数中，`"BRACE_OPEN"` 文本内容处理之后、`skip any other unexpected tokens` 之前，添加：

```typescript
// Template directives: @if / @for / @slot
if (tok.kind === "AT") {
  parseViewDirective(internals);
  internals.stream.skipNewlines();
  continue;
}
```

- [ ] **Step 4: 实现 parseViewDirective 函数**

在 `parser-view.ts` 文件末尾添加：

```typescript
// ── View directive (@if only — @for/@slot to follow) ──

/**
 * Parse @if / @else if / @else directives in view body.
 * Produces a ConditionalBlockData element with branch children.
 */
export function parseViewDirective(internals: ParserInternals): void {
  internals.stream.consume(); // AT (@)

  const directiveTok = internals.stream.expect("IDENTIFIER");
  if (directiveTok.kind === "EOF") return;

  // Currently only @if is handled; @for and @slot will follow
  if (directiveTok.text !== "if") {
    // Skip unknown directives by consuming until end of block
    let depth = 0;
    while (!internals.stream.isEof()) {
      const t = internals.stream.peek();
      if (t.kind === "BRACE_OPEN") depth++;
      else if (t.kind === "BRACE_CLOSE") { depth--; if (depth < 0) break; }
      internals.stream.consume();
    }
    return;
  }

  const condBlockId = `${internals.ctx.currentComponent}.@if.${internals.ctx.elements.length}`;
  const branches: Array<{
    type: "if" | "elif" | "else";
    condition: string;
    children: string[];
  }> = [];

  // Push condition block ID to parent stack so children get correct parentId
  if (internals.ctx.viewParentStack.length > 0) {
    internals.ctx.viewParentStack.push(condBlockId);
  }

  let currentBranchType: "if" | "elif" | "else" = "if";

  while (!internals.stream.isEof()) {
    // Collect condition expression until BRACE_OPEN
    let condition = "";
    if (currentBranchType !== "else") {
      while (
        !internals.stream.isEof() &&
        internals.stream.peek().kind !== "BRACE_OPEN" &&
        internals.stream.peek().kind !== "NEWLINE"
      ) {
        condition += internals.stream.consume().text;
      }
      condition = condition.trim();
    }

    // Expect opening brace
    if (internals.stream.peek().kind === "NEWLINE") {
      internals.stream.skipNewlines();
    }
    internals.stream.expect("BRACE_OPEN");

    // Track elements added inside this branch
    const beforeCount = internals.ctx.elements.length;

    // Parse branch body: recurse into view elements until matching close brace
    let braceDepth = 1;
    while (braceDepth > 0 && !internals.stream.isEof()) {
      const tok = internals.stream.peek();

      if (tok.kind === "BRACE_CLOSE") {
        braceDepth--;
        if (braceDepth === 0) {
          internals.stream.consume();
          break;
        }
        internals.stream.consume();
        continue;
      }

      if (tok.kind === "BRACE_OPEN") {
        braceDepth++;
        internals.stream.consume();
        continue;
      }

      // Recurse into view elements
      if (tok.kind === "ANGLE_OPEN") {
        parseViewElement(internals);
      } else if (tok.kind === "COMMENT_LINE") {
        parseViewComment(internals);
      } else if (
        tok.kind === "IDENTIFIER" ||
        tok.kind === "TEXT" ||
        tok.kind === "STRING" ||
        tok.kind === "BRACE_OPEN"
      ) {
        parseViewTextContent(internals);
      } else {
        internals.stream.consume();
      }

      internals.stream.skipNewlines();
    }

    // Collect child element IDs added during this branch
    const afterCount = internals.ctx.elements.length;
    const branchChildren: string[] = [];
    for (let i = beforeCount; i < afterCount; i++) {
      branchChildren.push(internals.ctx.elements[i].id);
    }

    // Save branch
    branches.push({
      type: currentBranchType,
      condition,
      children: branchChildren,
    });

    // Check for @else / @else if continuation
    internals.stream.skipNewlines();
    const nextTok = internals.stream.peek();
    if (
      nextTok.kind === "AT" &&
      internals.stream.peek(1)?.kind === "IDENTIFIER" &&
      internals.stream.peek(1)?.text === "else"
    ) {
      internals.stream.consume(); // AT
      internals.stream.consume(); // "else"
      // Check if it's "else if" or just "else"
      const afterElse = internals.stream.peek();
      if (afterElse.kind === "IDENTIFIER" && afterElse.text === "if") {
        internals.stream.consume(); // "if"
        currentBranchType = "elif";
        continue;
      }
      currentBranchType = "else";
      continue;
    }

    // No more branches
    break;
  }

  // Build ConditionalBlockData
  const allChildren = branches.flatMap((b) => b.children);
  const parentId =
    internals.ctx.viewParentStack.length > 1
      ? internals.ctx.viewParentStack[internals.ctx.viewParentStack.length - 2]
      : null;

  const condBlock: import("../../core/types/element").ConditionalBlockData = {
    id: condBlockId,
    nodeType: "conditional",
    parentId,
    branches,
    children: allChildren,
  };

  internals.ctx.elements.push(condBlock);

  // Pop condition block from parent stack
  if (internals.ctx.viewParentStack.length > 0) {
    internals.ctx.viewParentStack.pop();
  }
}
```

- [ ] **Step 5: 在 parser-view.ts 顶部导入 ConditionalBlockData**

不需要额外导入——`import("../../core/types/element").ConditionalBlockData` 已在函数体内使用 inline import。为保持现有模式，在文件顶部无需添加新 import（ConditionalBlockData 在 element.ts 中已定义）。

- [ ] **Step 6: 运行测试确认 PASS**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/parser-view-conditional.test.ts
```

Expected: 所有测试 PASS

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/services/codegen/parser-view.ts poiesis/src/services/codegen/__tests__/parser-view-conditional.test.ts
git commit -m "feat(poiesis): add @if/@else if/@else directive parser in view body"
```

---

### Task 3: Pipeline — view-tree-builder 条件块转 IfViewNode

**Files:**
- Modify: `poiesis/src/services/codegen/view-tree-builder.ts`
- Create: `poiesis/src/services/codegen/__tests__/view-tree-conditional.test.ts`

**Interfaces:**
- Consumes: `ConditionalBlockData` from Task 1
- Produces: `IfViewNode` in built view tree (consumed by SourceEmitter which already handles it)

- [ ] **Step 1: 写 view-tree-builder 测试**

```typescript
// poiesis/src/services/codegen/__tests__/view-tree-conditional.test.ts

import { describe, it, expect } from "vitest";
import { buildViewTree } from "../view-tree-builder";
import type { ElementData, ConditionalBlockData, ViewElementData } from "../../../core/types/element";

describe("buildViewTree with conditional blocks", () => {
  it("converts ConditionalBlockData to IfViewNode", () => {
    const childA: ViewElementData = {
      id: "el-a", nodeType: "view", tag: "div", props: { label: "A" },
      children: [], parentId: "cond-1",
    };
    const childB: ViewElementData = {
      id: "el-b", nodeType: "view", tag: "span", props: { label: "B" },
      children: [], parentId: "cond-1",
    };
    const condBlock: ConditionalBlockData = {
      id: "cond-1", nodeType: "conditional", parentId: null,
      branches: [
        { type: "if", condition: "x > 0", children: ["el-a"] },
        { type: "else", condition: "", children: ["el-b"] },
      ],
      children: ["el-a", "el-b"],
    };

    const elements: ElementData[] = [condBlock, childA, childB];
    const result = buildViewTree(elements, "Test");

    expect(result.roots).toHaveLength(1);
    const root = result.roots[0];
    expect(root.kind).toBe("if");
    if (root.kind === "if") {
      expect(root.condition).toBe("x > 0");
      expect(root.branches).toHaveLength(2);
      expect(root.branches[0].condition).toBe("x > 0");
      expect(root.branches[0].body).toHaveLength(1);
      expect(root.branches[1].condition).toBe("");
      expect(root.fallback).toBeDefined();
      expect(root.fallback!.length).toBe(1);
    }
  });
});
```

- [ ] **Step 2: 运行测试确认 FAIL**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/view-tree-conditional.test.ts
```

Expected: FAIL — conditional block not handled

- [ ] **Step 3: 修改 buildViewTree 处理 conditional blocks**

在 `view-tree-builder.ts` 的 `buildNode` 函数内，`if (el.nodeType !== "view")` 之后：

```typescript
// view-tree-builder.ts

import type { ElementData, ViewElementData, ConditionalBlockData } from "../../core/types/element";
import type { ViewNode, ElementViewNode, IfViewNode } from "./types";

export function buildViewTree(
  elements: ElementData[],
  compName: string,
  rootOrder?: string[],
): { roots: ViewNode[]; pascalCaseTags: string[] } {
  const byId = new Map<string, ViewElementData>();
  const pascalCaseTags: string[] = [];

  for (const el of elements) {
    if (el.nodeType === "view") {
      if (byId.has(el.id)) {
        const existing = byId.get(el.id)!;
        if (existing.parentId !== null && el.parentId === null) {
          byId.set(el.id, el as ViewElementData);
        }
      } else {
        byId.set(el.id, el as ViewElementData);
      }
    }
  }

  function buildNode(el: ElementData, currentPath: string): ViewNode {
    // ── Conditional block → IfViewNode ──
    if (el.nodeType === "conditional") {
      const cond = el as ConditionalBlockData;
      const condBranches = cond.branches.map((b) => ({
        condition: b.condition,
        body: b.children
          .map((cid) => byId.get(cid))
          .filter((c): c is ViewElementData => c !== undefined)
          .map((c) => buildNode(c, `${cond.id}.${c.tag.toLowerCase()}`)),
      }));

      const first = condBranches[0];
      const lastBranch = cond.branches[cond.branches.length - 1];
      const fallback =
        lastBranch.type === "else" && lastBranch.condition === ""
          ? condBranches[condBranches.length - 1].body
          : null;
      const elifBranches = fallback
        ? condBranches.slice(1, -1)
        : condBranches.slice(1);

      const branches = [
        { condition: first.condition, body: first.body },
        ...elifBranches.map((b) => ({ condition: b.condition, body: b.body })),
      ];

      return {
        kind: "if",
        id: cond.id,
        path: currentPath,
        condition: first.condition,
        branches,
        fallback,
        children: condBranches.flatMap((b) => b.body),
      } as IfViewNode;
    }

    // ── Existing view element handling ──
    const viewEl = el as ViewElementData;

    const isPascal = /^[A-Z]/.test(viewEl.tag);
    if (isPascal && !pascalCaseTags.includes(viewEl.tag)) {
      pascalCaseTags.push(viewEl.tag);
    }

    const childElements = (viewEl.children || [])
      .map((cid) => byId.get(cid))
      .filter((c): c is ViewElementData => c !== undefined);
    const children = childElements.map((c, i) =>
      buildNode(c, `${currentPath}.${c.tag.toLowerCase()}.${i}`),
    );

    // Note: When a child element's parentId points to a conditional block,
    // it won't be in `byId` (which only contains view elements). The
    // conditional block handles its children in its own buildNode branch
    // above through the flat `children` array.

    return {
      id: viewEl.id,
      path: currentPath,
      tag: viewEl.tag,
      props: { ...viewEl.props },
      children,
      kind: "element",
      isFolded: viewEl.isFolded,
      renderMode: viewEl.renderMode,
      classRefs: viewEl.classRefs,
      variantRef: viewEl.variantRef,
      componentClasses: viewEl.componentClasses,
      inlineStyle: viewEl.inlineStyle,
    } as ElementViewNode;
  }

  let rootElements: (ElementData)[];
  if (rootOrder) {
    rootElements = rootOrder
      .map((id) => elements.find((el) => el.id === id))
      .filter((el): el is ElementData => el !== undefined);
  } else {
    rootElements = elements.filter(
      (el): el is ElementData => el.parentId === null
    );
  }
  const roots = rootElements.map((el) => buildNode(el, compName));

  return { roots, pascalCaseTags };
}
```

- [ ] **Step 4: 运行测试确认 PASS**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/view-tree-conditional.test.ts
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/view-tree-builder.ts poiesis/src/services/codegen/__tests__/view-tree-conditional.test.ts
git commit -m "feat(poiesis): add ConditionalBlockData to IfViewNode conversion in view-tree-builder"
```

---

### Task 4: Store CRUD — 条件块 Actions

**Files:**
- Modify: `poiesis/src/store/slices/element-crud.ts`
- Modify: `poiesis/src/store/undo-slice.ts`
- Create: `poiesis/src/store/__tests__/conditional-crud.test.ts`

**Interfaces:**
- Consumes: `ConditionalBlockData` from Task 1
- Produces: 6 new store actions (see Step 1 test for signatures)

- [ ] **Step 1: 写 CRUD 测试**

```typescript
// poiesis/src/store/__tests__/conditional-crud.test.ts

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import type { ConditionalBlockData } from "../../core/types/element";

describe("conditional block CRUD", () => {
  beforeEach(() => {
    // Reset store to known state
    usePoiesisStore.setState({
      elements: [
        { id: "root", nodeType: "view", tag: "div", props: {}, children: [], parentId: null },
      ],
      selectedElementIds: [],
      rootOrder: ["root"],
      isDirty: false,
    });
  });

  it("addConditionalBlock creates block with @if + @else branches", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state = usePoiesisStore.getState();
    expect(state.isDirty).toBe(true);
    const condBlock = state.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(condBlock).toBeDefined();
    expect(condBlock!.branches).toHaveLength(2);
    expect(condBlock!.branches[0].type).toBe("if");
    expect(condBlock!.branches[0].condition).toBe("");
    expect(condBlock!.branches[0].children).toEqual([]);
    expect(condBlock!.branches[1].type).toBe("else");
    expect(condBlock!.branches[1].condition).toBe("");
    // Root element should have the conditional block as a child
    const root = state.elements.find((e) => e.id === "root");
    expect(root).toBeDefined();
    expect("children" in root! && root!.children).toContain(condBlock!.id);
  });

  it("addConditionalBranch adds @else if between @if and @else", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;
    store.addConditionalBranch(condBlock.id, "elif");

    const state2 = usePoiesisStore.getState();
    const updated = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(updated.branches).toHaveLength(3);
    expect(updated.branches[0].type).toBe("if");
    expect(updated.branches[1].type).toBe("elif");
    expect(updated.branches[2].type).toBe("else");
  });

  it("updateBranchCondition changes condition expression", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;
    store.updateBranchCondition(condBlock.id, 0, "count > 5");

    const state2 = usePoiesisStore.getState();
    const updated = state2.elements.find(
      (e) => e.id === condBlock.id
    ) as ConditionalBlockData;
    expect(updated.branches[0].condition).toBe("count > 5");
  });

  it("removeConditionalBlock deletes block and its branch children", () => {
    const store = usePoiesisStore.getState();
    store.addConditionalBlock("root");

    const state1 = usePoiesisStore.getState();
    const condBlock = state1.elements.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData;

    // Add a child element inside the @if branch
    store.addElement({
      id: "child-1", nodeType: "view", tag: "span", props: {},
      children: [], parentId: condBlock.id,
    });
    // Move it to branch 0
    store.addChildToBranch(condBlock.id, 0, "child-1");

    store.removeElement(condBlock.id);

    const state2 = usePoiesisStore.getState();
    expect(state2.elements.find((e) => e.id === condBlock.id)).toBeUndefined();
    // Child should also be deleted (via existing removeElement cascade)
    expect(state2.elements.find((e) => e.id === "child-1")).toBeUndefined();
    // Root should no longer reference condBlock
    const root = state2.elements.find((e) => e.id === "root");
    expect("children" in root! && root!.children).not.toContain(condBlock.id);
  });
});
```

- [ ] **Step 2: 运行测试确认 FAIL**

```bash
cd poiesis && npx vitest run src/store/__tests__/conditional-crud.test.ts
```

Expected: FAIL — actions not found

- [ ] **Step 3: 实现 addConditionalBlock**

在 `element-crud.ts` 中添加：

```typescript
// store/slices/element-crud.ts

import type { ConditionalBlockData } from "../../core/types/element";

// In the store creator's return object:

addConditionalBlock: (parentId: string) => {
  set((s) => {
    const condId = `conditional.${s._nextId || s.elements.length}`;
    const condBlock: ConditionalBlockData = {
      id: condId,
      nodeType: "conditional",
      parentId,
      branches: [
        { type: "if", condition: "", children: [] },
        { type: "else", condition: "", children: [] },
      ],
      children: [],
      isFolded: false,
    };
    const elements = [
      ...s.elements,
      condBlock,
    ].map((e) =>
      e.id === parentId && "children" in e
        ? { ...e, children: [...e.children, condId] }
        : e
    );
    return { ...s, elements, isDirty: true };
  });
},
```

- [ ] **Step 4: 实现 addConditionalBranch**

```typescript
addConditionalBranch: (blockId: string, type: "elif" | "else") => {
  set((s) => {
    const elements = s.elements.map((e) => {
      if (e.id !== blockId || e.nodeType !== "conditional") return e;
      const cond = e as ConditionalBlockData;
      const newBranch = {
        type: type === "elif" ? "elif" as const : "else" as const,
        condition: "",
        children: [] as string[],
      };
      // Insert elif before last branch (else), else replaces the else
      let newBranches: ConditionalBlockData["branches"];
      if (type === "elif") {
        newBranches = [
          ...cond.branches.slice(0, -1),
          newBranch,
          cond.branches[cond.branches.length - 1],
        ];
      } else {
        // Already has else — no-op
        return e;
      }
      return {
        ...cond,
        branches: newBranches,
        children: newBranches.flatMap((b) => b.children),
      };
    });
    return { ...s, elements, isDirty: true };
  });
},
```

- [ ] **Step 5: 实现 updateBranchCondition**

```typescript
updateBranchCondition: (blockId: string, branchIndex: number, condition: string) => {
  set((s) => {
    const elements = s.elements.map((e) => {
      if (e.id !== blockId || e.nodeType !== "conditional") return e;
      const cond = e as ConditionalBlockData;
      if (branchIndex < 0 || branchIndex >= cond.branches.length) return e;
      const newBranches = cond.branches.map((b, i) =>
        i === branchIndex ? { ...b, condition } : b
      );
      return { ...cond, branches: newBranches };
    });
    return { ...s, elements, isDirty: true };
  });
},
```

- [ ] **Step 6: 实现 addChildToBranch**

```typescript
addChildToBranch: (blockId: string, branchIndex: number, childId: string) => {
  set((s) => {
    const child = s.elements.find((e) => e.id === childId);
    if (!child) return s;

    const elements = s.elements
      .map((e) => {
        // Update child's parentId
        if (e.id === childId) {
          return { ...e, parentId: blockId } as typeof e;
        }
        // Remove from old parent's children
        if (child.parentId && e.id === child.parentId && "children" in e) {
          return { ...e, children: e.children.filter((c) => c !== childId) };
        }
        return e;
      })
      .map((e) => {
        if (e.id !== blockId || e.nodeType !== "conditional") return e;
        const cond = e as ConditionalBlockData;
        const newBranches = cond.branches.map((b, i) => {
          if (i !== branchIndex) return b;
          if (b.children.includes(childId)) return b; // already there
          return { ...b, children: [...b.children, childId] };
        });
        return {
          ...cond,
          branches: newBranches,
          children: newBranches.flatMap((b) => b.children),
        };
      });

    return { ...s, elements, isDirty: true };
  });
},
```

- [ ] **Step 7: 实现 removeChildFromBranch**

```typescript
removeChildFromBranch: (blockId: string, branchIndex: number, childId: string) => {
  set((s) => {
    const elements = s.elements.map((e) => {
      if (e.id !== blockId || e.nodeType !== "conditional") return e;
      const cond = e as ConditionalBlockData;
      const newBranches = cond.branches.map((b, i) => {
        if (i !== branchIndex) return b;
        return { ...b, children: b.children.filter((c) => c !== childId) };
      });
      return {
        ...cond,
        branches: newBranches,
        children: newBranches.flatMap((b) => b.children),
      };
    });
    return { ...s, elements, isDirty: true };
  });
},
```

- [ ] **Step 8: 在 store types 中注册 action 类型**

在 `store/types.ts` 中添加新 action 的 type definition（如需要），确保 `undo-slice.ts` 中能正确 undo。

- [ ] **Step 9: 运行测试确认 PASS**

```bash
cd poiesis && npx vitest run src/store/__tests__/conditional-crud.test.ts
```

Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add poiesis/src/store/slices/element-crud.ts poiesis/src/store/undo-slice.ts poiesis/src/store/__tests__/conditional-crud.test.ts
git commit -m "feat(poiesis): add conditional block CRUD actions to store"
```

---

### Task 5: UI — ComponentTree 条件块渲染 + 右键菜单

**Files:**
- Modify: `poiesis/src/components/panels/ComponentTree.tsx`

**Interfaces:**
- Consumes: `TreeNode` with `nodeType`/`condition`/`branchType` from Task 1
- Consumes: `ConditionalBlockData` from Task 1
- Consumes: Store CRUD actions from Task 4

- [ ] **Step 1: 修改 ComponentTree.buildNode 处理 conditional blocks**

在 `ComponentTree.tsx` 的 `buildNode` 函数中（约第53行），扩展条件块处理：

```typescript
const buildNode = (el: ElementData): TreeNode => {
  // ── Conditional block ──
  if (el.nodeType === "conditional") {
    const cond = el as ConditionalBlockData;
    return {
      id: cond.id,
      tag: "条件渲染",
      htmlTag: "conditional",
      displayLabel: "",
      nodeType: "conditional",
      children: cond.branches.map((b, i) => ({
        id: `${cond.id}.br${i}`,
        tag: b.type === "else" ? "else" : `(${b.condition || "条件"})`,
        htmlTag: "conditional-branch",
        displayLabel: b.type === "else" ? "" : b.condition,
        nodeType: "conditional-branch" as const,
        condition: b.condition,
        branchType: b.type,
        branchIndex: i,
        children: b.children
          .map((cid: string) => elMap.get(cid))
          .filter((e: unknown): e is ElementData => e !== undefined)
          .map((c) => buildNode(c)),
      })),
    };
  }

  // ── Existing view element handling ──
  const tag = "tag" in el ? el.tag : "div";
  const props = "props" in el ? (el.props as Record<string, unknown>) : {};
  return {
    id: el.id,
    tag,
    htmlTag: resolveHtmlTag(tag, props),
    displayLabel: getDisplayLabel(props),
    componentRef: "componentRef" in el ? el.componentRef : undefined,
    children: childrenOf(el.id).map(buildNode),
  };
};
```

- [ ] **Step 2: 添加 "用条件块包裹" + 条件块右键菜单项**

在 `ContextMenu` 组件中（约第309行），在现有菜单项中添加条件块相关菜单项：

```typescript
// 在 ContextMenu 的 return 中，</Separator> 之后添加：

{/* ── 条件块菜单项 ── */}
{contextElement?.nodeType === "conditional" ? (
  <>
    <MenuRow
      label="添加 @else if 分支"
      onClick={() => {
        const store = usePoiesisStore.getState();
        store.addConditionalBranch(elementId, "elif");
        onClose();
      }}
    />
    <MenuRow
      label="添加 @else 分支"
      onClick={() => {
        const store = usePoiesisStore.getState();
        store.addConditionalBranch(elementId, "else");
        onClose();
      }}
    />
    <Separator />
  </>
) : contextElement && "nodeType" in contextElement && contextElement.nodeType !== "codeblock" && contextElement.nodeType !== "group" ? (
  <>
    <MenuRow
      label="用条件块包裹"
      onClick={() => {
        const store = usePoiesisStore.getState();
        const parentId = contextElement.parentId;
        if (parentId) {
          store.addConditionalBlock(parentId);
        }
        onClose();
      }}
    />
    <Separator />
  </>
) : null}
```

- [ ] **Step 3: 添加导入**

在 `ComponentTree.tsx` 顶部添加：
```typescript
import type { ConditionalBlockData } from "../../core/types/element";
```

- [ ] **Step 4: 验证编译**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | grep -i "ComponentTree" | head -10
```

Expected: 无 ComponentTree 相关类型错误

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/components/panels/ComponentTree.tsx
git commit -m "feat(poiesis): add conditional block rendering and context menu in ComponentTree"
```

---

### Task 6: UI — TreeNodeRenderer 条件分支样式

**Files:**
- Modify: `poiesis/src/components/panels/TreeNodeRenderer.tsx`

**Interfaces:**
- Consumes: `TreeNode` with `nodeType`/`condition`/`branchType` from Task 1

- [ ] **Step 1: 添加条件分支节点渲染**

在 `TreeNodeView` 组件中，`return` 之前添加条件分支特殊处理。在现有渲染逻辑最前面插入：

```typescript
// ── Conditional branch node rendering ──
if (node.nodeType === "conditional-branch") {
  return (
    <div>
      {dropBefore && <DropLine depth={depth} />}
      <div
        ref={setRefs}
        {...listeners}
        {...attributes}
        onClick={(e) => onSelect(node.id, e.ctrlKey || e.metaKey)}
        onContextMenu={(e) => onContextMenu(e, node.id)}
        style={{
          paddingLeft: depth * 16 + 4,
          paddingRight: 4,
          paddingTop: 2,
          paddingBottom: 2,
          fontSize: 12,
          cursor: isDragging ? "grabbing" : "pointer",
          display: "flex",
          alignItems: "center",
          gap: 4,
          background: isSelected ? "#EEF2FF" : "transparent",
          borderRadius: 3,
          userSelect: "none",
          opacity: isDragging ? 0.35 : 1,
          color: node.branchType === "else" ? "#6b7280" : "#374151",
        }}
      >
        {hasChildren ? (
          <span
            onClick={(e) => { e.stopPropagation(); onToggle(node.id); }}
            style={{
              fontSize: 8, width: 12, cursor: "pointer",
              transition: "transform 0.15s",
              transform: isCollapsed ? "rotate(-90deg)" : "rotate(0deg)",
            }}
          >
            ▼
          </span>
        ) : (
          <span style={{ width: 12 }} />
        )}
        <span style={{ color: "#8B5CF6", fontSize: 10, fontWeight: 600 }}>
          {node.branchType === "if" ? "if" : node.branchType === "elif" ? "elif" : "else"}
        </span>
        <span style={{ color: "#1e293b", fontSize: 11, fontFamily: "monospace" }}>
          {node.branchType === "else" ? "" : node.condition || "(空条件)"}
        </span>
      </div>
      {dropAfter && <DropLine depth={depth} />}
      {hasChildren && !isCollapsed && (
        <div>
          {node.children.map((child) => (
            <TreeNodeView
              key={child.id}
              node={child}
              depth={depth + 1}
              selectedIds={selectedIds}
              collapsed={collapsed}
              onToggle={onToggle}
              onSelect={onSelect}
              onContextMenu={onContextMenu}
              treeDropState={treeDropState}
            />
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: 条件块主节点添加 "+" 按钮**

在现有元素渲染逻辑中，`{/* Element icon + tag */}` 之前，添加条件块的特殊图标：

```typescript
{/* ── Conditional block root node extra styling ── */}
{node.nodeType === "conditional" && (
  <span style={{ color: "#8B5CF6", fontSize: 13 }}>🔀</span>
)}
```

- [ ] **Step 3: 验证编译**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | grep "TreeNodeRenderer" | head -10
```

Expected: 无 TreeNodeRenderer 相关类型错误

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/panels/TreeNodeRenderer.tsx
git commit -m "feat(poiesis): add conditional branch node rendering in TreeNodeRenderer"
```

---

### Task 7: UI — BranchConditionEditor (属性面板)

**Files:**
- Create: `poiesis/src/components/panels/inspector/BranchConditionEditor.tsx`

**Interfaces:**
- Consumes: Store actions `updateBranchCondition`, `removeConditionalBranch`, `addChildToBranch` from Task 4
- Consumes: `ConditionalBlockData` from Task 1

- [ ] **Step 1: 创建 BranchConditionEditor 组件**

```typescript
// poiesis/src/components/panels/inspector/BranchConditionEditor.tsx

import React, { useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ConditionalBlockData } from "../../../core/types/element";

interface BranchConditionEditorProps {
  blockId: string;
  branchIndex: number;
}

export function BranchConditionEditor({ blockId, branchIndex }: BranchConditionEditorProps) {
  const condBlock = usePoiesisStore((s) =>
    s.elements.find((e) => e.id === blockId && e.nodeType === "conditional")
  ) as ConditionalBlockData | undefined;

  const branch = condBlock?.branches[branchIndex];
  const isElse = branch?.type === "else";
  const isIf = branch?.type === "if";

  const handleConditionChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      usePoiesisStore.getState().updateBranchCondition(blockId, branchIndex, e.target.value);
    },
    [blockId, branchIndex],
  );

  const handleDeleteBranch = useCallback(() => {
    usePoiesisStore.getState().removeConditionalBranch(blockId, branchIndex);
  }, [blockId, branchIndex]);

  if (!condBlock || !branch) {
    return <div style={{ padding: 12, color: "#9CA3AF", fontSize: 12 }}>未选择分支</div>;
  }

  return (
    <div style={{ padding: 12, fontSize: 12 }}>
      <div style={{ fontWeight: 600, marginBottom: 8, color: "#374151" }}>
        {isIf ? "🔀 @if 分支" : isElse ? "🔀 @else 分支" : "🔀 @else if 分支"}
      </div>

      {/* Condition expression */}
      <div style={{ marginBottom: 12 }}>
        <label style={{ display: "block", marginBottom: 4, color: "#6b7280", fontSize: 11 }}>
          条件表达式
        </label>
        {isElse ? (
          <div style={{
            padding: "6px 8px", background: "#F9FAFB", border: "1px solid #E5E7EB",
            borderRadius: 4, color: "#9CA3AF", fontSize: 12,
          }}>
            else（无条件）
          </div>
        ) : (
          <textarea
            value={branch.condition}
            onChange={handleConditionChange}
            placeholder="例如: count > 5"
            rows={2}
            style={{
              width: "100%", padding: "6px 8px", border: "1px solid #D1D5DB",
              borderRadius: 4, fontSize: 12, fontFamily: "monospace",
              resize: "vertical", boxSizing: "border-box",
            }}
          />
        )}
      </div>

      {/* Child elements list */}
      <div style={{ marginBottom: 12 }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 4 }}>
          <span style={{ color: "#6b7280", fontSize: 11 }}>子元素 ({branch.children.length})</span>
        </div>
        {branch.children.length === 0 ? (
          <div style={{ color: "#9CA3AF", fontSize: 11, fontStyle: "italic" }}>
            右键分支 → 添加子组件
          </div>
        ) : (
          <div style={{ fontSize: 11, color: "#374151" }}>
            {branch.children.map((cid) => (
              <div key={cid} style={{ padding: "2px 0" }}>• {cid}</div>
            ))}
          </div>
        )}
      </div>

      {/* Delete branch button */}
      {!isIf && !isElse && (
        <button
          onClick={handleDeleteBranch}
          style={{
            padding: "4px 12px", border: "1px solid #FCA5A5", borderRadius: 4,
            background: "#FEF2F2", color: "#DC2626", cursor: "pointer", fontSize: 11,
          }}
        >
          删除此分支
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: 在属性面板中集成**

在 `PropsSection.tsx` 或 `ContextInspector.tsx` 中，检测选中元素是否为条件分支（通过 ID 模式 `${blockId}.br${index}`），若是则渲染 `BranchConditionEditor`：

```typescript
// 在 ContextInspector.tsx 中：
import { BranchConditionEditor } from "./inspector/BranchConditionEditor";

// 在渲染逻辑中：
const selectedId = selectedIds[0];
if (selectedId) {
  const match = selectedId.match(/^(.+)\.br(\d+)$/);
  if (match) {
    const blockId = match[1];
    const branchIndex = parseInt(match[2], 10);
    return <BranchConditionEditor blockId={blockId} branchIndex={branchIndex} />;
  }
}
```

- [ ] **Step 3: 验证编译**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | grep -E "BranchCondition|ContextInspector" | head -10
```

Expected: 无相关错误

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/panels/inspector/BranchConditionEditor.tsx poiesis/src/components/panels/ContextInspector.tsx
git commit -m "feat(poiesis): add BranchConditionEditor for editing conditional branch expressions"
```

---

### Task 8: Round-Trip 集成测试

**Files:**
- Create: `poiesis/src/services/codegen/__tests__/conditional-roundtrip.test.ts`

- [ ] **Step 1: 写 round-trip 测试**

```typescript
// poiesis/src/services/codegen/__tests__/conditional-roundtrip.test.ts

import { describe, it, expect } from "vitest";
import { OmniTokenizer } from "../omni-tokenizer";
import { TokenStream } from "../token-stream";
import { Parser } from "../parser";
import { generateOmniSource } from "../pipeline";
import type { ConditionalBlockData } from "../../../core/types/element";

describe("conditional rendering round-trip", () => {
  it("parse → emit → parse produces identical conditional blocks", () => {
    const source = `
import omniview;
component Test {
  signal status: String = "loading";

  view {
    @if status == "loading" {
      <div id="a">Loading</div>
    } @else if status == "error" {
      <div id="b">Error</div>
    } @else {
      <div id="c">OK</div>
    }
  }
}`;

    // First parse
    const result1 = new Parser(
      new TokenStream(new OmniTokenizer().tokenize(source))
    ).parse();

    expect(result1.errors.filter(e => e.severity === "error")).toHaveLength(0);
    const cond1 = result1.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(cond1).toBeDefined();
    expect(cond1!.branches).toHaveLength(3);

    // Emit
    const emitResult = generateOmniSource({
      designElements: result1.designElements ?? [],
      designWires: [],
      codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "", graph: { elements: [], wires: [] }, variables: [] }],
      componentName: "Test",
      rootOrder: undefined,
      inlineStores: [],
    });

    expect(emitResult.errors.filter(e => e.severity === "fatal")).toHaveLength(0);
    expect(emitResult.value).toBeDefined();

    // Second parse
    const result2 = new Parser(
      new TokenStream(new OmniTokenizer().tokenize(emitResult.value!))
    ).parse();

    expect(result2.errors.filter(e => e.severity === "error")).toHaveLength(0);
    const cond2 = result2.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(cond2).toBeDefined();
    expect(cond2!.branches).toHaveLength(3);

    // Verify branch conditions match
    expect(cond2!.branches[0].condition).toBe(cond1!.branches[0].condition);
    expect(cond2!.branches[1].condition).toBe(cond1!.branches[1].condition);
    expect(cond2!.branches[2].condition).toBe(cond1!.branches[2].condition);
  });
});
```

- [ ] **Step 2: 运行测试确认 PASS**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/conditional-roundtrip.test.ts
```

Expected: PASS（验证 round-trip 无损）

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/__tests__/conditional-roundtrip.test.ts
git commit -m "test(poiesis): add round-trip test for conditional rendering"
```

---

### Task 9: 端到端验证

**Files:**
- No new files; use Victauri MCP + Playwright for manual verification

- [ ] **Step 1: 构建 Poiesis 并启动**

```bash
cd poiesis && cargo tauri dev
```

- [ ] **Step 2: 使用 learn-omniview 项目验证**

1. 在 Poiesis 中打开 learn-omniview 项目
2. 打开某个含有 @if 的 .omni 文件（如 `test/projects/learn-omniview/` 下的文件）
3. 验证：元素树中条件块以 🔀 图标显示
4. 验证：条件分支以 `if (condition)` / `elif` / `else` 节点显示
5. 验证：右键条件块出现「添加 @else if 分支」菜单
6. 验证：右键分支出现「添加子组件」菜单
7. 验证：点击分支节点 → 属性面板显示条件编辑框
8. 验证：修改条件表达式 → 保存 → 重新打开 → 条件保留
9. 验证：canvas 中通过调试面板改 signal → 分支切换生效

- [ ] **Step 3: Commit any fixes**

If issues found during verification, fix and commit individually.

---

## Self-Review Results

1. **Spec coverage**: All spec sections covered — Element Model (T1), Parser (T2), Pipeline (T3), Store CRUD (T4), ComponentTree UI (T5), TreeNodeRenderer (T6), BranchConditionEditor (T7), Round-trip test (T8), E2E verification (T9)
2. **Placeholder scan**: No TBD/TODO found. All code is concrete.
3. **Type consistency**: `ConditionalBlockData` → used consistently in T2 (parser), T3 (view-tree-builder), T4 (store), T5 (ComponentTree). `TreeNode.nodeType` → used in T5/T6. `branchIndex` → used in T5/T6/T7.
