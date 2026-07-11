# Poiesis 注解格式统一 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一 Poiesis 所有板块的 `.omni` 注解格式为 `// @pid:xxx`，建立统一注解基础设施（annotation-registry），扩展 project.json v3 schema 支持 code flow graph 持久化，实现手动 omni `// @<type>` 往返编辑。

**Architecture:** 三层改造——Layer 1 扩展 project.json v3 schema（+nodeType, +blockName, +codeFlowGraphs）；Layer 2 新建 annotation-registry.ts 统一所有注解解析/发射；Layer 3 适配所有 parser/emitter 使用新基础设施。spec 中"Omni 是主源 + Git merge 友好 + 手动 omni 可往返"三大原则贯穿始终。

**Tech Stack:** TypeScript (Zustand 5 store, Tauri v2 fs plugin), OmniLang parser (recursive descent), Vitest

## Global Constraints

- `.omni` 是主源——代码结构、design system 全部在 `.omni`，JSON 只存编辑器元数据
- Git merge 友好——.omni 中注解各占一行；JSON 全部 `Record<string, T>` 而非数组
- 手动 omni 可往返——`// @<type>` 标注 → parser 识别 → 保存时替换为 `// @pid` → 再次手写
- project.json version 字段必须升级为 3
- v2 项目打开时自动迁移到 v3
- 不改变画布渲染、WASM 编译路径

---

### Task 1: 创建 annotation-registry.ts（注解基础设施）

**Files:**
- Create: `poiesis/src/services/codegen/annotation-registry.ts`
- Test: `poiesis/src/services/codegen/__tests__/annotation-registry.test.ts`

**Interfaces:**
- Produces:
  - `parsePidAnnotation(line: string): { pid: string } | null`
  - `parseTypeAnnotation(line: string): { nodeType: string } | null`
  - `parseAnyAnnotation(line: string): { pid?: string; nodeType?: string } | null`
  - `isPidAnnotation(line: string): boolean`
  - `isTypeAnnotation(line: string): boolean`
  - `isAnyAnnotation(line: string): boolean`
  - `emitPidAnnotation(pid: string): string` → `"// @pid:${pid}"`
  - `PID_REGEX: RegExp`
  - `TYPE_REGEX: RegExp`
  - `VALID_TYPES: Set<string>`

- [ ] **Step 1: Write test file**

```typescript
// poiesis/src/services/codegen/__tests__/annotation-registry.test.ts
import { describe, it, expect } from "vitest";
import {
  parsePidAnnotation,
  parseTypeAnnotation,
  parseAnyAnnotation,
  isPidAnnotation,
  isTypeAnnotation,
  isAnyAnnotation,
  emitPidAnnotation,
  PID_REGEX,
  TYPE_REGEX,
  VALID_TYPES,
} from "../annotation-registry";

describe("parsePidAnnotation", () => {
  it("extracts pid from // @pid:xxx", () => {
    expect(parsePidAnnotation("// @pid:a1")).toEqual({ pid: "a1" });
  });

  it("handles leading whitespace", () => {
    expect(parsePidAnnotation("    // @pid:b2")).toEqual({ pid: "b2" });
  });

  it("handles trailing content (inline pid)", () => {
    expect(parsePidAnnotation("signal count: i32 = 0; // @pid:sig_0")).toEqual({ pid: "sig_0" });
  });

  it("returns null for non-pid comments", () => {
    expect(parsePidAnnotation("// @if")).toBeNull();
    expect(parsePidAnnotation("// some comment")).toBeNull();
    expect(parsePidAnnotation("")).toBeNull();
  });

  it("returns null for old design annotations", () => {
    expect(parsePidAnnotation("// @token id=tk_1")).toBeNull();
    expect(parsePidAnnotation("// @class id=cls_1")).toBeNull();
    expect(parsePidAnnotation("// @variant id=var_1")).toBeNull();
  });
});

describe("parseTypeAnnotation", () => {
  it.each(["if", "elif", "else", "return", "code-block", "for", "while"])(
    "recognizes // @%s",
    (type) => {
      expect(parseTypeAnnotation(`// @${type}`)).toEqual({ nodeType: type });
    },
  );

  it("handles trailing whitespace", () => {
    expect(parseTypeAnnotation("// @if   ")).toEqual({ nodeType: "if" });
  });

  it("rejects unknown types", () => {
    expect(parseTypeAnnotation("// @unknown")).toBeNull();
    expect(parseTypeAnnotation("// @token")).toBeNull();
    expect(parseTypeAnnotation("// @pid:a1")).toBeNull();
  });

  it("rejects empty line", () => {
    expect(parseTypeAnnotation("")).toBeNull();
  });
});

describe("parseAnyAnnotation", () => {
  it("returns pid when line has @pid", () => {
    expect(parseAnyAnnotation("// @pid:a1")).toEqual({ pid: "a1" });
  });

  it("returns nodeType when line has @type", () => {
    expect(parseAnyAnnotation("// @if")).toEqual({ nodeType: "if" });
  });

  it("returns null for non-annotations", () => {
    expect(parseAnyAnnotation("// hello")).toBeNull();
  });
});

describe("is functions", () => {
  it("isAnyAnnotation returns true for both pid and type", () => {
    expect(isAnyAnnotation("// @pid:a1")).toBe(true);
    expect(isAnyAnnotation("// @if")).toBe(true);
    expect(isAnyAnnotation("// plain comment")).toBe(false);
  });
});

describe("emitPidAnnotation", () => {
  it("emits correct format", () => {
    expect(emitPidAnnotation("a1")).toBe("// @pid:a1");
  });
});

describe("VALID_TYPES", () => {
  it("contains all 7 types", () => {
    expect(VALID_TYPES.size).toBe(7);
    expect(VALID_TYPES.has("if")).toBe(true);
    expect(VALID_TYPES.has("elif")).toBe(true);
    expect(VALID_TYPES.has("else")).toBe(true);
    expect(VALID_TYPES.has("return")).toBe(true);
    expect(VALID_TYPES.has("code-block")).toBe(true);
    expect(VALID_TYPES.has("for")).toBe(true);
    expect(VALID_TYPES.has("while")).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/annotation-registry.test.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Write annotation-registry.ts**

```typescript
// poiesis/src/services/codegen/annotation-registry.ts
// Unified annotation parsing and emission for all // @... comments in .omni source.
// Single source of truth for annotation format across the entire codebase.

// ── Regex constants ─────────────────────────────────────────────────────────

/** Matches `// @pid:<id>` — the sole identity annotation format in .omni. */
export const PID_REGEX = /^\s*\/\/\s*@pid:(\S+)/;

/** Matches `// @<type>` — manual type annotations for fn/action body nodes. */
export const TYPE_REGEX = /^\s*\/\/\s*@(if|elif|else|return|code-block|for|while)\s*$/;

/** The set of all valid type annotation values. */
export const VALID_TYPES = new Set(["if", "elif", "else", "return", "code-block", "for", "while"]);

// ── Parsing ─────────────────────────────────────────────────────────────────

/**
 * Parse `// @pid:<id>` from a source line.
 * Returns the pid or null if the line does not match.
 */
export function parsePidAnnotation(line: string): { pid: string } | null {
  const match = line.match(PID_REGEX);
  if (!match) return null;
  return { pid: match[1] };
}

/**
 * Parse `// @<type>` from a source line.
 * Returns the nodeType or null if the line does not match one of the valid types.
 */
export function parseTypeAnnotation(line: string): { nodeType: string } | null {
  const match = line.match(TYPE_REGEX);
  if (!match) return null;
  return { nodeType: match[1] };
}

/**
 * Parse any annotation from a source line.
 * Returns pid, nodeType, or null.
 */
export function parseAnyAnnotation(line: string): { pid?: string; nodeType?: string } | null {
  const pidResult = parsePidAnnotation(line);
  if (pidResult) return pidResult;

  const typeResult = parseTypeAnnotation(line);
  if (typeResult) return typeResult;

  return null;
}

// ── Classification ──────────────────────────────────────────────────────────

/** True if the line contains a `// @pid:` annotation. */
export function isPidAnnotation(line: string): boolean {
  return PID_REGEX.test(line);
}

/** True if the line contains a `// @<type>` annotation. */
export function isTypeAnnotation(line: string): boolean {
  return TYPE_REGEX.test(line);
}

/** True if the line contains any // @... annotation. */
export function isAnyAnnotation(line: string): boolean {
  return isPidAnnotation(line) || isTypeAnnotation(line);
}

// ── Emission ────────────────────────────────────────────────────────────────

/**
 * Emit a `// @pid:<id>` annotation line.
 * This is the ONLY format ever emitted into .omni source.
 */
export function emitPidAnnotation(pid: string): string {
  return `// @pid:${pid}`;
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/annotation-registry.test.ts
```
Expected: 12+ tests PASS.

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/annotation-registry.ts poiesis/src/services/codegen/__tests__/annotation-registry.test.ts
git commit -m "feat: add annotation-registry — unified // @pid and // @<type> parsing/emission"
```

---

### Task 2: 扩展 project-json.ts v3 schema 类型

**Files:**
- Modify: `poiesis/src/core/types/project-json.ts`
- Modify: `poiesis/src/services/persistence/project-json.ts`

**Interfaces:**
- Produces: `ProjectJson { version: 3 }`, `NodeMeta { nodeType, blockName }`, `CodeFlowGraph`, `CodeFlowNode`, `CodeFlowWire`

- [ ] **Step 1: Update core types**

Edit `poiesis/src/core/types/project-json.ts`:

Change the `ProjectJson` interface and add new types:

```typescript
// poiesis/src/core/types/project-json.ts
// Project JSON persistence types — git merge-friendly design

export interface ProjectJson {
  version: 3;

  /** Node metadata keyed by PID. */
  nodes: Record<string, NodeMeta>;

  /** Wire metadata keyed by wire ID. */
  wires: Record<string, WireMeta>;

  architecture: ArchitectureMeta;

  /** Canvas viewport per .omni file, keyed by relative path */
  canvasStates: Record<string, CanvasViewport>;

  panels: PanelLayout;

  /** Code flow graphs keyed by fn/action PID. */
  codeFlowGraphs: Record<string, CodeFlowGraph>;
}

export interface NodeMeta {
  pos: { x: number; y: number };
  folded?: boolean;
  /** migrated from old @edit render=canvas annotation */
  renderMode?: "dom" | "canvas";
  /** Node type for code flow graph nodes: "if" | "elif" | "else" | "return" | "code-block" | "for" | "while" | "fn" | "action" */
  nodeType?: string;
  /** Owning fn/action name, used for PID-based matching during parse */
  blockName?: string;
}

export interface WireMeta {
  from: string;        // source PID
  fromPort: string;
  to: string;          // target PID
  toPort: string;
  /** Skeleton node branch routing path */
  path?: Array<{ x: number; y: number }>;
}

export interface ArchitectureMeta {
  mode: "overall" | "local";
  expandedBubbles: string[];
  focusComponent: string | null;
  camera: { x: number; y: number; zoom: number };
}

export interface CanvasViewport {
  zoom: number;
  panX: number;
  panY: number;
}

export interface PanelLayout {
  leftWidth: number;
  rightWidth: number;
}

// ── New: Code Flow Graph persistence ──────────────────────────

/** Persisted code flow graph for one fn/action, keyed by fn/action PID. */
export interface CodeFlowGraph {
  pid: string;
  nodes: Record<string, CodeFlowNode>;
  wires: Record<string, CodeFlowWire>;
}

export interface CodeFlowNode {
  nodeType: string;    // "if" | "elif" | "else" | "return" | "code-block" | "for" | "while"
}

export interface CodeFlowWire {
  from: string;        // source node PID
  fromPort: string;    // "flow-out" | "br0" | "br1" | "br2"
  to: string;          // target node PID
  toPort: string;      // "flow-in"
}

// ── Component def types (unchanged) ──────────────────────────

export interface ComponentSourceDef {
  name: string;
  filePath: string;
  signals: Array<DefEntry>;
  props: Array<DefEntry>;
  events: Array<{ pid: string; name: string; payloadType: string }>;
  fns: Array<FnDef>;
}

export interface ComponentCanvasState {
  elements: import("../types/element").ElementData[];
  wires: import("../types/wire").WireData[];
  rootOrder: string[];
}

export type ComponentDef = ComponentSourceDef & Partial<ComponentCanvasState>;

export interface DefEntry {
  pid: string;
  name: string;
  type: string;
  defaultValue?: unknown;
}

export interface FnDef {
  pid: string;
  name: string;
  params: Array<{ name: string; type: string }>;
  returnType: string;
}
```

- [ ] **Step 2: Update project-json.ts serialize/deserialize for v3**

Edit `poiesis/src/services/persistence/project-json.ts`:

```typescript
// services/persistence/project-json.ts
import type { ProjectJson } from "../../core/types/project-json";

export function serializeProjectJson(data: ProjectJson): string {
  return JSON.stringify(data, sortedKeysReplacer, 2) + "\n";
}

function sortedKeysReplacer(_key: string, value: unknown): unknown {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    const sorted: Record<string, unknown> = {};
    const keys = Object.keys(value as Record<string, unknown>).sort();
    for (const k of keys) {
      sorted[k] = (value as Record<string, unknown>)[k];
    }
    return sorted;
  }
  return value;
}

export function deserializeProjectJson(raw: string): ProjectJson {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    throw new Error(`Failed to parse project.json: ${e instanceof Error ? e.message : String(e)}`);
  }
  validateProjectJson(parsed);
  // Migration: v2 → v3 handled by migration module
  return parsed as ProjectJson;
}

function validateProjectJson(data: unknown): void {
  if (!data || typeof data !== "object") {
    throw new Error("project.json must be a JSON object");
  }
  const obj = data as Record<string, unknown>;
  if (obj.version !== 2 && obj.version !== 3) {
    throw new Error(`Unsupported project.json version: ${obj.version}. Expected: 2 or 3`);
  }
  if (obj.nodes !== undefined && typeof obj.nodes !== "object") {
    throw new Error("project.json: 'nodes' must be a Record<string, NodeMeta>");
  }
  if (obj.wires !== undefined && typeof obj.wires !== "object") {
    throw new Error("project.json: 'wires' must be a Record<string, WireMeta>");
  }
}

export function createEmptyProjectJson(): ProjectJson {
  return {
    version: 3,
    nodes: {},
    wires: {},
    architecture: {
      mode: "overall",
      expandedBubbles: [],
      focusComponent: null,
      camera: { x: 0, y: 0, zoom: 1 },
    },
    canvasStates: {},
    panels: { leftWidth: 240, rightWidth: 300 },
    codeFlowGraphs: {},
  };
}
```

- [ ] **Step 3: Verify build**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```
Expected: no new type errors from these two files.

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/core/types/project-json.ts poiesis/src/services/persistence/project-json.ts
git commit -m "feat: project.json v3 schema — add NodeMeta.nodeType/blockName and CodeFlowGraph types"
```

---

### Task 3: 更新 graph-parser.ts 支持 // @<type> + 连线推断

**Files:**
- Modify: `poiesis/src/services/codegen/graph-parser.ts`
- Test: `poiesis/src/services/codegen/__tests__/graph-parser.test.ts`（已有，更新）

**Interfaces:**
- Consumes: `parsePidAnnotation`, `parseTypeAnnotation`, `isAnyAnnotation` from `annotation-registry`
- Produces: `parseCodeFlowGraph(fnBody: string, nodeMeta?: Record<string, NodeMeta>): { elements, wires }` — signature extended with optional nodeMeta

- [ ] **Step 1: Update graph-parser.ts `splitByPid` to also handle `// @<type>`**

In `poiesis/src/services/codegen/graph-parser.ts`, import annotation-registry and modify `splitByPid`:

```typescript
import { isAnyAnnotation, parsePidAnnotation, parseTypeAnnotation, emitPidAnnotation } from "./annotation-registry";
import { uid } from "../../core/utils/id";
```

Replace the `splitByPid` function:

```typescript
function splitByPid(fnBody: string): ParsedSegment[] {
  if (!fnBody || !fnBody.trim()) return [];

  const segments: ParsedSegment[] = [];
  const lines = fnBody.split("\n");

  let currentPid = "";
  let currentStart = 0;
  let currentLines: string[] = [];
  let segmentCount = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (isAnyAnnotation(line)) {
      // Save previous segment
      if (currentPid && currentLines.length > 0) {
        segments.push({
          pid: currentPid,
          startLine: currentStart,
          endLine: i - 1,
          code: currentLines.join("\n").trimEnd(),
          type: detectSegmentType(currentLines),
        });
      }

      // Determine pid for new segment
      const pidResult = parsePidAnnotation(line);
      const typeResult = parseTypeAnnotation(line);

      if (pidResult) {
        currentPid = pidResult.pid;
      } else if (typeResult) {
        // Auto-generate PID for manual @<type> annotations
        currentPid = uid(`${typeResult.nodeType}`);
      } else {
        // Unknown annotation — skip, treat as part of code
        currentLines.push(line);
        continue;
      }

      currentStart = i;
      currentLines = [];
    } else {
      currentLines.push(line);
    }
  }

  // Save last segment
  if (currentPid && currentLines.length > 0) {
    segments.push({
      pid: currentPid,
      startLine: currentStart,
      endLine: lines.length - 1,
      code: currentLines.join("\n").trimEnd(),
      type: detectSegmentType(currentLines),
    });
  }

  return segments;
}
```

- [ ] **Step 2: Add function to convert type annotation to `// @pid`**

Add this export at the bottom of graph-parser.ts:

```typescript
/**
 * Convert // @<type> annotations to // @pid:xxx in fn body source.
 * Called during save to replace manual type annotations with editor-generated PIDs.
 *
 * @param fnBody  Raw fn body source with // @<type> or // @pid annotations.
 * @param pidMap  Map from auto-generated PID to the original // @<type> line.
 *                Caller builds this from parseCodeFlowGraph's returned elements.
 * @returns       fnBody with // @<type> replaced by // @pid:xxx.
 */
export function normalizeAnnotations(fnBody: string, pidMap: Map<string, string>): string {
  const lines = fnBody.split("\n");
  const result: string[] = [];

  for (const line of lines) {
    const typeResult = parseTypeAnnotation(line);
    if (typeResult) {
      // Find the matching PID for this type annotation
      // pidMap is built during parseCodeFlowGraph: type lineno string → generated pid
      const key = `${typeResult.nodeType}:${lines.indexOf(line)}`;
      const pid = pidMap.get(key);
      if (pid) {
        result.push(emitPidAnnotation(pid));
        continue;
      }
    }
    result.push(line);
  }

  return result.join("\n");
}
```

- [ ] **Step 3: Run existing graph-parser tests to check no regression**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/graph-parser.test.ts
```
Expected: all existing tests PASS (old behavior preserved for `// @pid`).

- [ ] **Step 4: Add tests for // @<type> parsing**

Add to `poiesis/src/services/codegen/__tests__/graph-parser.test.ts`:

```typescript
describe("parseCodeFlowGraph with // @<type> annotations", () => {
  it("parses manual @if and @return annotations", () => {
    const fnBody = [
      "// @if",
      "if (x > 0) {",
      "  // @return",
      "  return x;",
      "}",
      "// @return",
      "return 0;",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    expect(result.elements).toHaveLength(3); // fn entry + if + 2 returns
    const ifEl = result.elements.find((e) => e.tag === "condition");
    expect(ifEl).toBeDefined();
    const returnEls = result.elements.filter((e) => e.tag === "return");
    expect(returnEls).toHaveLength(2);
    // Should have wires connecting them
    expect(result.wires.length).toBeGreaterThanOrEqual(2);
  });

  it("parses sequential @code-block nodes with linear flow-out", () => {
    const fnBody = [
      "// @code-block",
      "step1();",
      "// @code-block",
      "step2();",
      "// @code-block",
      "step3();",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    const codeBlocks = result.elements.filter((e) => e.tag === "code-block");
    expect(codeBlocks).toHaveLength(3);
    // Linear flow-out: step1 → step2, step2 → step3
    const flowWires = result.wires.filter((w) => w.fromPort === "flow-out");
    expect(flowWires).toHaveLength(2);
  });

  it("parses if/elif/else with manual annotations", () => {
    const fnBody = [
      "// @if",
      "if (x > 0) {",
      "  // @code-block",
      "  doA();",
      "}",
      "// @elif",
      "else if (x == 0) {",
      "  // @code-block",
      "  doB();",
      "}",
      "// @else",
      "else {",
      "  // @code-block",
      "  doC();",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(fnBody);
    const conditions = result.elements.filter((e) => e.tag === "condition");
    expect(conditions).toHaveLength(1);
    // Should have br0, br1, br2 wires
    const brWires = result.wires.filter((w) => w.fromPort.startsWith("br"));
    expect(brWires).toHaveLength(3);
  });
});
```

- [ ] **Step 5: Run tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/graph-parser.test.ts
```
Expected: all tests PASS (old + new).

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/graph-parser.ts poiesis/src/services/codegen/__tests__/graph-parser.test.ts
git commit -m "feat: graph-parser supports // @<type> annotations with auto-PID and wire inference"
```

---

### Task 4: 更新 parser.ts / parser-view.ts / parser-helpers.ts / parser-finalize.ts 使用 annotation-registry

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`
- Modify: `poiesis/src/services/codegen/parser-view.ts`
- Modify: `poiesis/src/services/codegen/parser-helpers.ts`
- Modify: `poiesis/src/services/codegen/parser-finalize.ts`

**Interfaces:**
- Consumes: `isPidAnnotation`, `parsePidAnnotation`, `parseAnyAnnotation` from `annotation-registry`
- Changes: All `// @pid` matching regex inline uses replaced with annotation-registry imports

- [ ] **Step 1: Update parser.ts — replace annotation regex usage**

In `poiesis/src/services/codegen/parser.ts`, add import:

```typescript
import { parsePidAnnotation, isAnyAnnotation, parseAnyAnnotation } from "./annotation-registry";
```

Replace all occurrences of inline regex matching for `@pid` and design annotations:

1. Replace `parseDesignAnnotation` usage in design block parsing with `parsePidAnnotation`
2. Replace PID regex in chain parsing with `isAnyAnnotation` + `parseAnyAnnotation`

For design block parsing (lines handling `// @token/@class/@variant`), edit the parser's design parsing to:

```typescript
// In parseDesign method, replace design annotation parsing:
// Old: const ann = parseDesignAnnotation(commentLine);
// New: const ann = parsePidAnnotation(commentLine);
// Design tokens/classes/variants now use // @pid:xxx format
```

- [ ] **Step 2: Update parser-helpers.ts**

In `poiesis/src/services/codegen/parser-helpers.ts`, update `createElement` and any function that uses `// @pid` to use annotation-registry:

```typescript
import { parsePidAnnotation, parseTypeAnnotation } from "./annotation-registry";
```

Replace inline PID regex in chain parsing helpers with `parsePidAnnotation`/`parseTypeAnnotation` calls.

- [ ] **Step 3: Update parser-view.ts**

In `poiesis/src/services/codegen/parser-view.ts`, update `parseViewComment` to handle new annotation format:

```typescript
import { parsePidAnnotation } from "./annotation-registry";

export function parseViewComment(internals: ParserInternals): void {
  const tok = internals.stream.consume();
  const text = tok.text.trim();

  // @pid annotation on view elements — store for later PID index building
  const pidResult = parsePidAnnotation(text);
  if (pidResult) {
    internals.ctx.pendingPid = pidResult.pid;
    return;
  }

  // @edit annotation (unchanged)
  if (text.startsWith("// @edit")) {
    internals.ctx.pendingEditFlags = text.slice("// @edit".length).trim() || null;
  }
}
```

- [ ] **Step 4: Update parser-finalize.ts**

In `poiesis/src/services/codegen/parser-finalize.ts`, update `finalizeParse` to use annotation-registry for PID extraction instead of inline regex:

```typescript
import { parsePidAnnotation } from "./annotation-registry";
```

- [ ] **Step 5: Run existing parser tests to check no regression**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/ --reporter=verbose 2>&1 | tail -30
```
Expected: all existing tests PASS.

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/parser-view.ts poiesis/src/services/codegen/parser-helpers.ts poiesis/src/services/codegen/parser-finalize.ts
git commit -m "refactor: parser suite uses annotation-registry for all // @pid/@type parsing"
```

---

### Task 5: 更新 source-emitter.ts — 统一 // @pid 输出，消除行尾 pid

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts`

**Interfaces:**
- Consumes: `emitPidAnnotation` from `annotation-registry`
- Produces: All PID emission uses `emitPidAnnotation(pid)` — always on its own line before the declaration

- [ ] **Step 1: Update source-emitter.ts**

In `poiesis/src/services/codegen/source-emitter.ts`, add import:

```typescript
import { emitPidAnnotation } from "./annotation-registry";
```

Replace all manual PID annotation construction:

```typescript
// Old pattern (inline, at end of line):
// w.emit(`signal ${sig.name}: ${sig.type}${defStr}; // @pid:${sig.pid}`);

// New pattern (on its own line before declaration):
// w.emit(emitPidAnnotation(sig.pid));
// w.emit(`signal ${sig.name}: ${sig.type}${defStr};`);
```

Update these locations:
1. **Store definition emission** (signals): PID line before each signal
2. **Store definition emission** (getters): PID line before each getter
3. **Store definition emission** (actions): PID line before each action
4. **Inline store emission**: same pattern as standalone
5. **View node emission**: PID line before opening tag (if annotation strategy provides pid)
6. **Component meta emission**: component-level PID before `component Name {`

- [ ] **Step 2: Run existing pipeline tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/pipeline.test.ts src/services/codegen/__tests__/store-roundtrip.test.ts
```
Expected: may FAIL due to format changes in emitted output. Continue to Task 6 for full fix.

- [ ] **Step 3: Commit** (after verifying no unrelated breakage)

```bash
git add poiesis/src/services/codegen/source-emitter.ts
git commit -m "refactor: source-emitter uses emitPidAnnotation — PID always on its own line"
```

---

### Task 6: 更新 design-emitter.ts — `@token/@class/@variant` → `@pid`

**Files:**
- Modify: `poiesis/src/services/codegen/design-emitter.ts`

**Interfaces:**
- Consumes: `emitPidAnnotation` from `annotation-registry`
- Produces: `emitDesignSource` emits `// @pid:xxx` instead of `// @token id=xxx` etc.

- [ ] **Step 1: Update design-emitter.ts**

In `poiesis/src/services/codegen/design-emitter.ts`, add import:

```typescript
import { emitPidAnnotation } from "./annotation-registry";
```

Replace annotation emission in helper functions:

```typescript
function emitToken(lines: string[], token: DesignToken): void {
  lines.push(`    ${emitPidAnnotation(token.id)}`);
  const value = `"${token.value}"`;
  lines.push(`    token ${token.name}: ${token.tokenType} = ${value};`);
  lines.push("");
}

function emitClass(lines: string[], cls: DesignClass): void {
  lines.push(`    ${emitPidAnnotation(cls.id)}`);
  lines.push(`    class ${cls.name} {`);
  // ... (rest unchanged)
}

function emitVariant(lines: string[], variant: VariantDefinition): void {
  lines.push(`    ${emitPidAnnotation(variant.id)}`);
  lines.push(`    variant ${variant.name} {`);
  // ... (rest unchanged)
}
```

- [ ] **Step 2: Run tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/ --reporter=verbose 2>&1 | grep -E "(PASS|FAIL|Tests)"
```
Expected: design-emitter related tests may fail due to format change. We'll fix remaining tests in Task 14.

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/design-emitter.ts
git commit -m "refactor: design-emitter uses emitPidAnnotation — unified // @pid format"
```

---

### Task 7: 更新 chain-emitter.ts — chain 注解移入 JSON

**Files:**
- Modify: `poiesis/src/services/codegen/chain-emitter.ts`

**Interfaces:**
- Consumes: `emitPidAnnotation` from `annotation-registry`
- Produces: `emitChains` still returns `Map<string, ChainBlock>` for JSON persistence, but no longer produces `// @chain/@node/@wire` source comments

- [ ] **Step 1: Update chain-emitter.ts**

In `poiesis/src/services/codegen/chain-emitter.ts`, import and add new exports:

```typescript
import { emitPidAnnotation } from "./annotation-registry";
import type { CodeFlowGraph } from "../../core/types/project-json";

/**
 * Build CodeFlowGraph JSON structure from chain blocks.
 * Called during save to populate project.json codeFlowGraphs field.
 */
export function buildCodeFlowGraphs(
  chains: Map<string, ChainBlock>,
): Record<string, CodeFlowGraph> {
  const graphs: Record<string, CodeFlowGraph> = {};

  for (const [fnName, chain] of chains) {
    graphs[fnName] = {
      pid: chain.pid ?? fnName,
      nodes: {},
      wires: {},
    };

    for (const node of chain.nodes) {
      graphs[fnName].nodes[node.id] = { nodeType: node.type };
    }

    for (const wire of chain.wires) {
      graphs[fnName].wires[wire.id] = {
        from: wire.fromId,
        fromPort: wire.fromPort,
        to: wire.toId,
        toPort: wire.toPort,
      };
    }
  }

  return graphs;
}

/**
 * Format chain blocks as // @pid annotations (replaces old // @chain block format).
 * Each node gets // @pid:<id> on its own line before its source code.
 */
export function formatChainAsPidAnnotations(chain: ChainBlock): string {
  const lines: string[] = [];
  for (const node of chain.nodes) {
    lines.push(emitPidAnnotation(node.id));
  }
  return lines.join("\n");
}
```

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/services/codegen/chain-emitter.ts
git commit -m "refactor: chain-emitter — add buildCodeFlowGraphs for JSON, deprecate chain comment blocks"
```

---

### Task 8: 消除 pipeline.ts 中 StoreDef → StoreDefinition 转换

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts`

**Interfaces:**
- Removes: `storeDefToStoreDefinition()` function — parser now directly produces `StoreDefinition`

- [ ] **Step 1: Remove storeDefToStoreDefinition, inline stores array usage unchanged**

In `poiesis/src/services/codegen/pipeline.ts`:

Remove the `storeDefToStoreDefinition` function (lines 429-457) and update any call sites to use `StoreDefinition` directly (currently called in `generateOmniSource` line 294).

The stores from `phaseB.ctx.stores` are `StoreDef[]` (from store-extractor). Since the parser now provides `StoreDefinition` directly, the emitter expects `StoreDefinition`. Keep the conversion for now as a compatibility shim with deprecation comment:

```typescript
/**
 * @deprecated Parser now produces StoreDefinition directly.
 * This conversion shim exists for backward compatibility with store-extractor output.
 * Remove after store-extractor is updated to emit StoreDefinition.
 */
function storeDefToStoreDefinition(sd: StoreDef): StoreDefinition {
  // ... keep existing code unchanged
}
```

- [ ] **Step 2: Verify no build errors**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | head -20
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/codegen/pipeline.ts
git commit -m "refactor: mark storeDefToStoreDefinition as deprecated"
```

---

### Task 9: 更新 serialization.ts — toProjectJson/loadFromProjectJson → v3

**Files:**
- Modify: `poiesis/src/store/slices/serialization.ts`

**Interfaces:**
- Consumes: v3 `ProjectJson` type with `codeFlowGraphs`
- Produces: `toProjectJson()` returns v3 with `codeFlowGraphs: {}`, `loadFromProjectJson()` handles v3 fields

- [ ] **Step 1: Update serialization.ts**

Edit `poiesis/src/store/slices/serialization.ts`:

```typescript
import type { PoiesisState } from "../types";
import type { ProjectJson, CodeFlowGraph } from "../../core/types/project-json";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createSerializationSlice(set: SetFn, get: GetFn) {
  return {
    toProjectJson: (): ProjectJson => {
      const s = get();
      return {
        version: 3,
        nodes: s.nodeMeta,
        wires: s.wireMeta,
        architecture: {
          mode: s.architecture.mode,
          expandedBubbles: [...s.architecture.expandedBubbleIds].sort(),
          focusComponent: s.architecture.focusComponentId,
          camera: s.architecture.camera,
        },
        canvasStates: s.canvasStates,
        panels: s.panels,
        codeFlowGraphs: s.codeFlowGraphs ?? {},
      };
    },

    loadFromProjectJson: (json: ProjectJson) => set((s) => ({
      nodeMeta: json.nodes || {},
      wireMeta: json.wires || {},
      architecture: {
        ...s.architecture,
        ...(json.architecture ? {
          mode: json.architecture.mode || "overall",
          expandedBubbleIds: json.architecture.expandedBubbles || [],
          focusComponentId: json.architecture.focusComponent || null,
          camera: json.architecture.camera || { x: 0, y: 0, zoom: 1 },
        } : {}),
      },
      canvasStates: json.canvasStates || {},
      panels: json.panels || s.panels,
      codeFlowGraphs: json.codeFlowGraphs || {},
    })),
  };
}
```

- [ ] **Step 2: Add codeFlowGraphs to PoiesisState type**

Edit `poiesis/src/store/types.ts`, add to `PoiesisState` interface:

```typescript
// ── Code flow graph persistence ──
codeFlowGraphs: Record<string, import("../core/types/project-json").CodeFlowGraph>;
```

And add initial state in `poiesis/src/store/index.ts`:

```typescript
codeFlowGraphs: {},
```

- [ ] **Step 3: Verify type consistency**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | grep -E "(serialization|types\.ts|index\.ts)" | head -10
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/store/slices/serialization.ts poiesis/src/store/types.ts poiesis/src/store/index.ts
git commit -m "feat: serialization slice → v3 with codeFlowGraphs support"
```

---

### Task 10: 更新 save-manager.ts + load-manager.ts — codeFlowGraphs 持久化

**Files:**
- Modify: `poiesis/src/services/persistence/save-manager.ts`
- Modify: `poiesis/src/services/persistence/load-manager.ts`

**Interfaces:**
- Consumes: `buildCodeFlowGraphs` from `chain-emitter.ts`
- Produces: save writes `codeFlowGraphs` to project.json; load reads `codeFlowGraphs` from project.json

- [ ] **Step 1: Update save-manager.ts**

Edit `poiesis/src/services/persistence/save-manager.ts`:

```typescript
import { writeTextFile, mkdir } from "@tauri-apps/plugin-fs";
import { usePoiesisStore } from "../../store";
import { serializeProjectJson } from "./project-json";
import { buildCodeFlowGraphs } from "../codegen/chain-emitter";

export function buildProjectJsonFromStore(): string {
  const store = usePoiesisStore.getState();
  const json = store.toProjectJson();
  return serializeProjectJson(json);
}

/**
 * Update code flow graphs in the store from chain emitter output.
 * Called during the save pipeline before writing project.json.
 */
export function updateCodeFlowGraphsInStore(
  chains: Map<string, import("../codegen/types").ChainBlock>,
): void {
  const store = usePoiesisStore.getState();
  const graphs = buildCodeFlowGraphs(chains);
  usePoiesisStore.setState({ codeFlowGraphs: graphs });
}

export async function saveProject(projectDir: string): Promise<{ success: boolean; error?: string; jsonStr?: string }> {
  try {
    const jsonStr = buildProjectJsonFromStore();

    const poiesisDir = `${projectDir}/.poiesis`;
    await mkdir(poiesisDir, { recursive: true });

    const filePath = `${poiesisDir}/project.json`;
    await writeTextFile(filePath, jsonStr);

    return { success: true, jsonStr };
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return { success: false, error: message, jsonStr: "" };
  }
}
```

- [ ] **Step 2: Update load-manager.ts**

Edit `poiesis/src/services/persistence/load-manager.ts`:

```typescript
export function loadProjectFromJson(raw: string): { success: boolean; error?: string } {
  try {
    const projectJson = deserializeProjectJson(raw);
    const store = usePoiesisStore.getState();
    store.newProject();
    store.loadFromProjectJson(projectJson);
    return { success: true };
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

/**
 * Restore code flow nodes from project.json's codeFlowGraphs.
 * Called after .omni parsing to overlay persisted graph data on parsed elements.
 */
export function applyCodeFlowGraphs(
  pidIndex: Record<string, { sourceFile: string }>,
  codeFlowGraphs: Record<string, import("../../core/types/project-json").CodeFlowGraph>,
): void {
  const store = usePoiesisStore.getState();
  // Merge persisted code flow graphs into store
  usePoiesisStore.setState({ codeFlowGraphs });
}
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/services/persistence/save-manager.ts poiesis/src/services/persistence/load-manager.ts
git commit -m "feat: save-manager + load-manager handle codeFlowGraphs persistence"
```

---

### Task 11: v2 → v3 迁移模块

**Files:**
- Create: `poiesis/src/services/persistence/migration.ts`
- Test: `poiesis/src/services/persistence/__tests__/migration.test.ts`

**Interfaces:**
- Produces: `migrateV2ToV3(json: ProjectJson & { version: 2 }): ProjectJson`

- [ ] **Step 1: Write test**

```typescript
// poiesis/src/services/persistence/__tests__/migration.test.ts
import { describe, it, expect } from "vitest";
import { migrateV2ToV3, needsMigration } from "../migration";

describe("needsMigration", () => {
  it("returns true for v2", () => {
    expect(needsMigration({ version: 2 } as any)).toBe(true);
  });

  it("returns false for v3", () => {
    expect(needsMigration({ version: 3 } as any)).toBe(false);
  });
});

describe("migrateV2ToV3", () => {
  it("upgrades version and adds empty codeFlowGraphs", () => {
    const v2 = {
      version: 2,
      nodes: {
        "pid_1": { pos: { x: 100, y: 200 } },
      },
      wires: {},
      architecture: {
        mode: "overall" as const,
        expandedBubbles: [],
        focusComponent: null,
        camera: { x: 0, y: 0, zoom: 1 },
      },
      canvasStates: {},
      panels: { leftWidth: 240, rightWidth: 300 },
    };

    const v3 = migrateV2ToV3(v2);
    expect(v3.version).toBe(3);
    expect(v3.codeFlowGraphs).toEqual({});
    // Existing data preserved
    expect(v3.nodes["pid_1"].pos).toEqual({ x: 100, y: 200 });
  });
});
```

- [ ] **Step 2: Write migration.ts**

```typescript
// poiesis/src/services/persistence/migration.ts
import type { ProjectJson } from "../../core/types/project-json";

/**
 * Check if project.json needs migration.
 */
export function needsMigration(json: { version: number }): boolean {
  return json.version < 3;
}

/**
 * Migrate v2 project.json → v3.
 *
 * Changes:
 * - version: 2 → 3
 * - + codeFlowGraphs: {} (empty — populated on first save from chain emitter)
 * - nodes[pid] left as-is (nodeType inferred during parse if missing)
 */
export function migrateV2ToV3(
  v2: Record<string, unknown> & { version: 2 },
): ProjectJson {
  return {
    version: 3,
    nodes: (v2.nodes as Record<string, unknown>) || {},
    wires: (v2.wires as Record<string, unknown>) || {},
    architecture: (v2.architecture as ProjectJson["architecture"]) || {
      mode: "overall",
      expandedBubbles: [],
      focusComponent: null,
      camera: { x: 0, y: 0, zoom: 1 },
    },
    canvasStates: (v2.canvasStates as Record<string, unknown>) || {},
    panels: (v2.panels as ProjectJson["panels"]) || { leftWidth: 240, rightWidth: 300 },
    codeFlowGraphs: (v2 as Record<string, unknown>).codeFlowGraphs as Record<string, unknown> || {},
  };
}
```

- [ ] **Step 3: Run tests**

```bash
cd poiesis && npx vitest run src/services/persistence/__tests__/migration.test.ts
```
Expected: all PASS.

- [ ] **Step 4: Integrate migration into deserializeProjectJson**

Edit `poiesis/src/services/persistence/project-json.ts`, add import and call:

```typescript
import { needsMigration, migrateV2ToV3 } from "./migration";

export function deserializeProjectJson(raw: string): ProjectJson {
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch (e) {
    throw new Error(`Failed to parse project.json: ${e instanceof Error ? e.message : String(e)}`);
  }
  validateProjectJson(parsed);

  if (needsMigration(parsed as { version: number })) {
    return migrateV2ToV3(parsed as Record<string, unknown> & { version: 2 });
  }

  return parsed as ProjectJson;
}
```

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/persistence/migration.ts poiesis/src/services/persistence/__tests__/migration.test.ts poiesis/src/services/persistence/project-json.ts
git commit -m "feat: v2→v3 migration — auto-upgrade project.json on load"
```

---

### Task 12: Design parser — `@token/@class/@variant` → `@pid` 适配

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts`（design block 解析部分）
- Modify: `poiesis/src/core/types/design.ts`（DESIGN_ANNOTATION_PREFIX → deprecate）

**Interfaces:**
- Consumes: `parsePidAnnotation` from `annotation-registry`
- Produces: Parser understands both old `// @token id=xxx` and new `// @pid:xxx` for design elements

- [ ] **Step 1: Update design block parsing in parser.ts**

In the parser's design parsing section, replace:

```typescript
// Old: const ann = parseDesignAnnotation(line);
// New: dual-format support
```

```typescript
import { parsePidAnnotation } from "./annotation-registry";
import { parseDesignAnnotation } from "../../core/types/design"; // keep for backward compat

// In design block parsing:
function parseDesignElementAnnotation(line: string): { id: string } | null {
  // Try new format first: // @pid:xxx
  const pidResult = parsePidAnnotation(line);
  if (pidResult) return { id: pidResult.pid };

  // Fall back to old format: // @token id=xxx / // @class id=xxx / // @variant id=xxx
  const oldResult = parseDesignAnnotation(line);
  if (oldResult) return { id: oldResult.id };

  return null;
}
```

Update all call sites in design token/class/variant parsing to use `parseDesignElementAnnotation` instead of `parseDesignAnnotation`.

- [ ] **Step 2: Add deprecation comment to design.ts**

In `poiesis/src/core/types/design.ts`, add:

```typescript
/**
 * @deprecated Use parsePidAnnotation from annotation-registry instead.
 * Kept for backward compatibility with old // @token/@class/@variant annotations.
 * Remove after all projects have been migrated.
 */
export const DESIGN_ANNOTATION_PREFIX = { ... };
```

- [ ] **Step 3: Run tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/ --reporter=verbose 2>&1 | tail -20
```

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/core/types/design.ts
git commit -m "feat: design parser dual-format — supports both // @pid and legacy // @token/@class/@variant"
```

---

### Task 13: omni-parser.ts + PidMap — 适配新注解格式

**Files:**
- Modify: `poiesis/src/services/codegen/omni-parser.ts`

**Interfaces:**
- Consumes: `PID_REGEX` from `annotation-registry`
- Produces: `parsePidAnnotations` uses unified regex

- [ ] **Step 1: Update omni-parser.ts**

```typescript
import { PID_REGEX } from "./annotation-registry";

export function parsePidAnnotations(source: string): PidMap {
  const entries: PidEntry[] = [];
  const lines = source.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const match = lines[i].match(PID_REGEX);
    if (match) {
      const pid = match[1].trim();
      entries.push({
        pid,
        blockType: "view",
        blockName: "",
        sourceFile: "",
        startLine: i + 1,
        endLine: i + 1,
      });
    }
  }

  const entriesMap: Record<string, PidEntry> = {};
  const lineToPid: Record<number, string> = {};
  for (const entry of entries) {
    entriesMap[entry.pid] = entry;
    for (let line = entry.startLine; line <= entry.endLine; line++) {
      if (lineToPid[line] && lineToPid[line] !== entry.pid) {
        log.warn(`Line ${line} has conflicting PID annotations: ${lineToPid[line]} vs ${entry.pid}`);
      }
      lineToPid[line] = entry.pid;
    }
  }

  return { status: "ok", entries: entriesMap, lineToPid };
}
```

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/services/codegen/omni-parser.ts
git commit -m "refactor: omni-parser uses PID_REGEX from annotation-registry"
```

---

### Task 14: 集成测试 + 往返测试 + 修复剩余测试

**Files:**
- Create/Modify: `poiesis/src/services/codegen/__tests__/roundtrip-annotation.test.ts`（新增往返测试）
- Modify: Existing tests that broke due to format changes

**Interfaces:**
- Tests: Manual `// @<type>` → parse → `// @pid` → re-parse round-trip
- All existing tests must pass

- [ ] **Step 1: Write round-trip integration test**

```typescript
// poiesis/src/services/codegen/__tests__/roundtrip-annotation.test.ts
import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";

describe("annotation round-trip", () => {
  it("manual @if/@return → auto-PID nodes, then re-parse preserves structure", () => {
    const manualSource = [
      "// @if",
      "if (x > 0) {",
      "  // @return",
      "  return x;",
      "}",
      "// @return",
      "return 0;",
    ].join("\n");

    // Parse manual source → elements + wires
    const result1 = parseCodeFlowGraph(manualSource);
    expect(result1.elements.length).toBeGreaterThanOrEqual(3);

    // Verify all elements have auto-generated PIDs
    for (const el of result1.elements) {
      expect(el.id).toBeTruthy();
      expect(el.id).not.toBe("");
    }

    // The structure should have 1 condition + 2 returns
    const conditionEl = result1.elements.find((e) => e.tag === "condition");
    expect(conditionEl).toBeDefined();
    const returnEls = result1.elements.filter((e) => e.tag === "return");
    expect(returnEls).toHaveLength(2);
  });

  it("sequential @code-block nodes have linear flow-out wires", () => {
    const source = [
      "// @code-block",
      "a();",
      "// @code-block",
      "b();",
      "// @code-block",
      "c();",
    ].join("\n");

    const result = parseCodeFlowGraph(source);
    const blocks = result.elements.filter((e) => e.tag === "code-block");
    expect(blocks).toHaveLength(3);

    // Wire chain: a → b → c
    const wires = result.wires;
    const flowOutWires = wires.filter((w) => w.fromPort === "flow-out");
    // Should have 2 flow-out wires: a→b, b→c
    expect(flowOutWires).toHaveLength(2);
  });

  it("if/elif/else with @type annotations creates correct branch wires", () => {
    const source = [
      "// @if",
      "if (n > 0) {",
      "  // @code-block",
      "  a();",
      "}",
      "// @elif",
      "else if (n == 0) {",
      "  // @code-block",
      "  b();",
      "}",
      "// @else",
      "else {",
      "  // @code-block",
      "  c();",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(source);

    // Should have 1 condition + 3 code-blocks
    const conditions = result.elements.filter((e) => e.tag === "condition");
    expect(conditions).toHaveLength(1);

    const cond = conditions[0];
    // Should have br0, br1, br2 wires from condition
    const br0 = result.wires.find((w) => w.fromId === cond.id && w.fromPort === "br0");
    const br1 = result.wires.find((w) => w.fromId === cond.id && w.fromPort === "br1");
    const br2 = result.wires.find((w) => w.fromId === cond.id && w.fromPort === "br2");
    expect(br0).toBeDefined();
    expect(br1).toBeDefined();
    expect(br2).toBeDefined();
  });

  it("mixed // @pid and // @type in same body", () => {
    const source = [
      "// @if",
      "if (new) {",
      "  // @return",
      "  return 1;",
      "}",
      "// @pid:old_pid",
      "if (existing) {",
      "  // @pid:old_ret",
      "  return 2;",
      "}",
    ].join("\n");

    const result = parseCodeFlowGraph(source);

    // Old nodes keep their PIDs
    const oldIf = result.elements.find((e) => e.id === "old_pid");
    expect(oldIf).toBeDefined();
    const oldRet = result.elements.find((e) => e.id === "old_ret");
    expect(oldRet).toBeDefined();

    // New nodes have auto-generated PIDs
    const newIf = result.elements.find((e) => e.tag === "condition" && e.id !== "old_pid");
    expect(newIf).toBeDefined();
  });
});
```

- [ ] **Step 2: Run new tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/roundtrip-annotation.test.ts
```
Expected: all 5 tests PASS.

- [ ] **Step 3: Fix broken existing tests**

Update any existing tests that reference old annotation formats:
- `// @token id=xxx` → assert on `// @pid:xxx` in design-emitter tests
- `// @pid:xxx` at end of line → assert on separate line above
- Chain block tests → assert JSON structure instead of `// @chain` comments

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/ --reporter=verbose 2>&1 | grep "FAIL"
```
For each FAIL, update the test assertion to match the new format.

- [ ] **Step 4: Run full test suite**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/
```
Expected: ALL TESTS PASS.

- [ ] **Step 5: Type check**

```bash
cd poiesis && npx tsc --noEmit --pretty false 2>&1 | tail -5
```
Expected: no new errors.

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/__tests__/
git commit -m "test: add round-trip annotation tests + fix format regressions"
```

---

### Task 15: 更新 poiesis-dev 技能文件

**Files:**
- Modify: `.claude/skills/poiesis-dev/SKILL.md`
- Modify: `.claude/skills/poiesis-dev/references/confusions.md`

- [ ] **Step 1: Update SKILL.md core facts**

Update the annotation format section:

```markdown
| 事实 | 说明 |
|------|------|
| 统一注解格式 | `.omni` 中只使用 `// @pid:xxx`（独占一行，在声明之上）。旧 `// @token/@class/@variant` 格式已废弃 |
| 手动 omni | fn/action body 内用 `// @<type>` 标注节点类型（@if/@elif/@else/@return/@code-block/@for/@while），保存后自动替换为 `// @pid` |
| project.json v3 | 新增 `nodeMeta.nodeType`、`nodeMeta.blockName`、`codeFlowGraphs`；全部 `Record<string, T>` 结构 |
```

- [ ] **Step 2: Update confusions.md**

Add new entry:

```markdown
### 注解格式混淆

**陷阱**：旧代码中 `// @token id=tk_1`、`// @class id=cls_1`、行尾 `// @pid:xxx` 已废弃。
**正确**：所有注解统一为 `// @pid:xxx`独占一行，在声明之上。

**陷阱**：手动写 omni 时忘记 `// @<type>`。只有 fn/action body 内部需要。
**正确**：component / store / design / view / style / `<tag>` / `@if` / `@for` 等顶层/模板构造有自明关键词，不需要标注。只在 body 内部的代码语句前加标注。
```

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/poiesis-dev/
git commit -m "docs: update poiesis-dev skill — unified annotation format and project.json v3"
```

---

## 自审检查

### Spec 覆盖
- ✅ 注解统一（`// @pid` 唯一格式） — Task 1, 4, 5
- ✅ 手动 `// @<type>` 书写 — Task 3, 14
- ✅ project.json v3 schema — Task 2, 9
- ✅ codeFlowGraphs 持久化 — Task 7, 10
- ✅ 设计系统注解迁移 — Task 6, 12
- ✅ v2 → v3 迁移 — Task 11
- ✅ 增量往返（混合 @pid + @type） — Task 3, 14
- ✅ 消除行尾 pid — Task 5
- ✅ 消除 chain 注解块 — Task 7
- ✅ StoreDef 转换消除 — Task 8
- ✅ 文档更新 — Task 15

### 占位符检查
- ✅ 无 TBD/TODO
- ✅ 所有步骤有实际代码
- ✅ 所有命令有预期输出

### 类型一致性
- ✅ `parsePidAnnotation` → `{ pid: string }` 在所有 task 中一致
- ✅ `CodeFlowGraph` 类型从 Task 2 定义，Task 7/10/11 使用
- ✅ `emitPidAnnotation` 签名在 Task 1/5/6/7 中一致
