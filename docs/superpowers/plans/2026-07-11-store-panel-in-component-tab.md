# Store 面板（组件标签页内）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在组件标签页右侧面板加 Store 按钮，点击从右侧滑出全幅抽屉，复用 StorePage 编辑内联/全局 Store。

**Architecture:** 7 个 task，自底向上：类型层 → 解析层 → 发射层 → 状态层 → StorePage 适配 → UI 抽屉组件 → 保存链路。每层独立可测。

**Tech Stack:** React 19 + TypeScript + Zustand 5 + 现有 Poiesis parser/emitter/codegen pipeline

## Global Constraints

- 无新依赖，复用现有 Zustand store 模式
- 组件 omni 文件需支持内联 `store Name { ... }` 语法
- 抽屉动画 250ms cubic-bezier(0.16, 1, 0.3, 1)，与 CodeFlow 动画一致
- 不创建新 tab，面板状态独立
- PID 注释保证往返闭环（parser ↔ emitter）

---

### Task 1: ParseResult 与 Parser 上下文支持内联 Store

**Files:**
- Modify: `poiesis/src/services/codegen/types.ts:281-300`
- Modify: `poiesis/src/services/codegen/parser.ts:64-81` (context)
- Modify: `poiesis/src/services/codegen/parser-finalize.ts:97-114`

**Interfaces:**
- Consumes: `ParseResult` (existing), `ParserInternals` context (existing)
- Produces: `ParseResult.inlineStoreDefs?: Record<string, StoreDefinition>`

- [ ] **Step 1: 在 ParseResult 增加 inlineStoreDefs 字段**

```typescript
// poiesis/src/services/codegen/types.ts 在 storeDef 之后新增:
  /** Parsed inline store definitions (store blocks inside component body). */
  inlineStoreDefs?: Record<string, import("../../core/types/store").StoreDefinition>;
```

- [ ] **Step 2: 在 Parser 上下文增加 inlineStoreDefs 字段**

```typescript
// poiesis/src/services/codegen/parser.ts 在 createContext() 中 storeDef: null 之后新增:
      inlineStoreDefs: {} as Record<string, import("../../core/types/store").StoreDefinition>,
```

并在 `ParserInternals` 子接口中声明类型（`ctx` 属性附近）。

- [ ] **Step 3: 在 parser-finalize.ts 传入 inlineStoreDefs**

```typescript
// poiesis/src/services/codegen/parser-finalize.ts buildParseResult 中:
    inlineStoreDefs: Object.keys(ctx.inlineStoreDefs).length > 0 ? ctx.inlineStoreDefs : undefined,
```

- [ ] **Step 4: 验证类型**

运行: `cd poiesis && npx tsc --noEmit 2>&1 | head -20`
预期: 无新增类型错误（Pre-existing errors 776 以内）

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/parser-finalize.ts
git commit -m "feat: add inlineStoreDefs field to ParseResult and parser context"
```

---

### Task 2: Parser 支持解析组件内联 `store Name { ... }`

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts:344-374` (parseComponentKeyword)
- Modify: `poiesis/src/services/codegen/parser.ts:512-620` (parseStoreBody — 微小适配)

**Interfaces:**
- Consumes: `parseStoreBody()` (existing), `ctx.inlineStoreDefs`, `ctx.currentComponent`
- Produces: 解析后的 inlineStoreDefs 存入 `ctx.inlineStoreDefs[name]`

- [ ] **Step 1: 在 parseComponentKeyword() 的 switch 中增加 case "store"**

```typescript
// poiesis/src/services/codegen/parser.ts:344 parseComponentKeyword()
// 在 switch (kw) 中，case "style" 之后新增:
      case "store":
        this.stream.consume(); // "store"
        const storeNameTok = this.stream.expect("IDENTIFIER");
        if (storeNameTok.kind === "EOF") {
          this.pushError("error", "P005", "Invalid inline store declaration: missing name", storeNameTok);
          break;
        }
        this.ctx.currentStore = storeNameTok.text;
        this.ctx.currentStorePid = "";
        this.ctx.storeSignalCounter = 0;
        this.ctx.storeActionCounter = 0;
        this.ctx.storeGetterCounter = 0;
        this.ctx.parsedStorePid = null;
        this.stream.expect("BRACE_OPEN");
        this.pushState("IN_STORE");
        // Mark: parser state knows we are inside a component — parseStoreBody
        // will store the result to ctx.inlineStoreDefs instead of ctx.storeDef
        this.ctx._parsingInlineStore = true;
        break;
```

- [ ] **Step 2: 在 parseStoreBody() 末尾将 StoreDefinition 存入 inlineStoreDefs**

```typescript
// poiesis/src/services/codegen/parser.ts:parseStoreBody()
// 在 "Close store" 分支 (tok.kind === "BRACE_CLOSE") 中，替换原有的:
//   this.ctx.storeDef = storeDef;
// 为:
        if (this.ctx._parsingInlineStore) {
          // Inline store inside a component
          const key = `${this.ctx.currentComponent}.${storeDef.name}`;
          this.ctx.inlineStoreDefs = { ...this.ctx.inlineStoreDefs, [key]: storeDef };
          this.ctx._parsingInlineStore = false;
        } else {
          this.ctx.storeDef = storeDef;
        }
```

- [ ] **Step 3: 在 Parser 上下文增加 `_parsingInlineStore` 标志**

```typescript
// poiesis/src/services/codegen/parser.ts createContext() 中:
      _parsingInlineStore: false,
```

- [ ] **Step 4: 编写单元测试**

```typescript
// poiesis/src/services/codegen/__tests__/parser-inline-store.test.ts
import { parseOmniSource } from "../omni-parser";

test("parses inline store inside component", () => {
  const source = `
component TodoApp {
    store CartStore {
        signal items: Array<String> = [];
        getter count: i32 = items.len();
        action add(name: String) {
            // body
        }
    }
    view {
        <div></div>
    }
}`;
  const result = parseOmniSource(source);
  expect(result.inlineStoreDefs).toBeDefined();
  expect(result.inlineStoreDefs!["TodoApp.CartStore"]).toBeDefined();
  const store = result.inlineStoreDefs!["TodoApp.CartStore"];
  expect(store.name).toBe("TodoApp.CartStore");
  expect(store.signals).toHaveLength(1);
  expect(store.signals[0].name).toBe("items");
  expect(store.getters).toHaveLength(1);
  expect(store.getters[0].name).toBe("count");
  expect(store.actions).toHaveLength(1);
  expect(store.actions[0].name).toBe("add");
});

test("parses multiple inline stores", () => {
  const source = `
component App {
    store A { signal x: String = ""; }
    store B { signal y: i32 = 0; }
    view { <div></div> }
}`;
  const result = parseOmniSource(source);
  expect(Object.keys(result.inlineStoreDefs ?? {})).toHaveLength(2);
});

test("inline store does not affect standalone storeDef", () => {
  const source = `
component App {
    store Cart { signal items: Array<String> = []; }
    view { <div></div> }
}`;
  const result = parseOmniSource(source);
  expect(result.storeDef).toBeUndefined();
  expect(result.inlineStoreDefs).toBeDefined();
});
```

- [ ] **Step 5: Run tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/parser-inline-store.test.ts
```
预期: 3 tests PASS

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/__tests__/parser-inline-store.test.ts
git commit -m "feat: parse inline store blocks inside component body"
```

---

### Task 3: SourceEmitter 支持发射内联 Store

**Files:**
- Modify: `poiesis/src/services/codegen/source-emitter.ts:250-339` (emitComponent)
- Modify: `poiesis/src/services/codegen/types.ts` (EmitInput 增加 inlineStores)

**Interfaces:**
- Consumes: `StoreDefinition` (existing), `emitStoreDefinition` (existing private method in SourceEmitter)
- Produces: 内联 store 块嵌入在 component body 中

- [ ] **Step 1: EmitInput 增加 inlineStores 字段**

```typescript
// poiesis/src/services/codegen/types.ts EmitInput 接口:
  /** Inline store definitions to emit inside component body. */
  inlineStores?: import("../../core/types/store").StoreDefinition[];
```

- [ ] **Step 2: emitComponent() 中发射内联 stores**

```typescript
// poiesis/src/services/codegen/source-emitter.ts emitComponent()
// 在 "@store refs" 循环之后、"Props" 之前插入:
    // ── Inline stores ──
    for (const store of input.inlineStores ?? []) {
      // Use a private method to emit the store block without the file header
      this.emitInlineStore(store, w);
      w.blank();
    }
```

- [ ] **Step 3: 新增 emitInlineStore() 私有方法**

```typescript
// poiesis/src/services/codegen/source-emitter.ts SourceEmitter 类中:
  private emitInlineStore(store: StoreDefinition, w: CodeWriter): void {
    w.emit(`// @pid:${store.pid}`);
    w.emit(`store ${store.name} {`);
    w.indent();

    for (const sig of store.signals) {
      const defStr = sig.defaultValue !== undefined && sig.defaultValue !== ""
        ? ` = ${formatDefaultValue(sig.defaultValue, sig.type)}`
        : "";
      w.emit(`signal ${sig.name}: ${sig.type}${defStr}; // @pid:${sig.pid}`);
    }

    if (store.getters.length > 0 && store.signals.length > 0) w.blank();
    for (const g of store.getters) {
      w.emit(`getter ${g.name}: ${g.returnType} = ${g.expression}; // @pid:${g.pid}`);
    }

    for (const a of store.actions) {
      if (store.signals.length + store.getters.length > 0 || a !== store.actions[0]) w.blank();
      const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
      const retStr = a.returnType ? `: ${a.returnType}` : "";
      const asyncStr = a.isAsync ? " async" : "";
      w.emit(`action ${a.name}(${paramsStr})${retStr}${asyncStr} { // @pid:${a.pid}`);

      if (a.elements.length > 0) {
        w.indent();
        const actionFnPid = a.elements.find((e) => e.tag === "fn")?.id;
        if (actionFnPid) {
          const body = compileCodeFlowGraph(a.elements, a.wires, actionFnPid);
          const lines = body.split("\n");
          const innerLines = lines.slice(1, -1);
          let hasContent = false;
          for (const line of innerLines) {
            const trimmed = line.trim();
            if (trimmed.length > 0) {
              w.emit(trimmed);
              hasContent = true;
            }
          }
          if (!hasContent) {
            w.emit(`fn ${a.name}() {`);
            w.emit(`}`);
          }
        } else {
          w.emit(`fn ${a.name}() {`);
          w.emit(`}`);
        }
        w.dedent();
      }
      w.emit(`}`);
    }

    w.dedent();
    w.emit(`}`);
  }
```

- [ ] **Step 4: 编写单元测试（往返）**

```typescript
// poiesis/src/services/codegen/__tests__/inline-store-roundtrip.test.ts
import { parseOmniSource } from "../omni-parser";
import { generateOmniFiles } from "../pipeline";

test("roundtrip: inline store survives parse → emit → parse", () => {
  const source = `
component App {
    store Cart {
        signal items: Array<String> = [];
        getter count: i32 = items.len();
        action add(name: String) {
        }
    }
    view {
        <div></div>
    }
}`;
  // Parse → gets inlineStoreDefs
  const r1 = parseOmniSource(source);
  expect(r1.inlineStoreDefs).toBeDefined();

  // Emit via pipeline
  const genResult = generateOmniFiles({
    designElements: r1.designElements ?? [],
    designWires: r1.designWires ?? [],
    codeTabs: [],
    componentName: "App",
    rootOrder: [],
    filePath: "src/App.omni",
  });
  // Parse emitted source back
  const emittedSource = genResult.value?.["src/App.omni"];
  expect(emittedSource).toBeDefined();
  const r2 = parseOmniSource(emittedSource!);
  // Verify inline store survives
  expect(r2.inlineStoreDefs).toBeDefined();
  expect(r2.inlineStoreDefs!["App.Cart"]).toBeDefined();
});
```

- [ ] **Step 5: Run tests**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/inline-store-roundtrip.test.ts src/services/codegen/__tests__/parser-inline-store.test.ts
```
预期: 4 tests PASS

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/source-emitter.ts poiesis/src/services/codegen/types.ts poiesis/src/services/codegen/__tests__/inline-store-roundtrip.test.ts
git commit -m "feat: emit inline stores inside component body in source-emitter"
```

---

### Task 4: Zustand State 增加 inlineStoreDefs 与 CRUD

**Files:**
- Modify: `poiesis/src/store/types.ts` (PoiesisState 接口)
- Modify: `poiesis/src/store/slices/store-crud.ts` (inline store CRUD)

**Interfaces:**
- Consumes: `StoreDefinition` type (existing)
- Produces: `inlineStoreDefs: Record<string, StoreDefinition>`, CRUD actions

- [ ] **Step 1: PoiesisState 增加 inlineStoreDefs 和 CRUD 方法声明**

```typescript
// poiesis/src/store/types.ts 在 storeDefs 之后新增:
  // ── Inline store definitions (component-scoped, keyed by "ComponentName.StoreName") ──
  inlineStoreDefs: Record<string, StoreDefinition>;
  loadInlineStoreDef: (def: StoreDefinition) => void;
  removeInlineStoreDef: (key: string) => void;
```

- [ ] **Step 2: store-crud.ts 增加 inline store CRUD**

```typescript
// poiesis/src/store/slices/store-crud.ts createStoreCrudSlice() 返回对象中:

    // ── Inline store definitions ──
    inlineStoreDefs: {} as Record<string, StoreDefinition>,

    loadInlineStoreDef: (def: StoreDefinition) => set((s) => ({
      inlineStoreDefs: { ...s.inlineStoreDefs, [def.name]: def },
    })),

    removeInlineStoreDef: (key: string) => set((s) => {
      const { [key]: _removed, ...rest } = s.inlineStoreDefs;
      return { inlineStoreDefs: rest };
    }),

    // ── Inline store Signal CRUD ──
    addInlineStoreSignal: (storeKey: string, signal: StoreSignalDef) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: { ...store, signals: [...store.signals, signal] },
        },
        isDirty: true,
      };
    }),

    updateInlineStoreSignal: (storeKey: string, pid: string, updates: Partial<StoreSignalDef>) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: {
            ...store,
            signals: store.signals.map((sig) => sig.pid === pid ? { ...sig, ...updates } : sig),
          },
        },
        isDirty: true,
      };
    }),

    removeInlineStoreSignal: (storeKey: string, pid: string) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: { ...store, signals: store.signals.filter((sig) => sig.pid !== pid) },
        },
        isDirty: true,
      };
    }),

    // ── Inline store Getter CRUD ──
    addInlineStoreGetter: (storeKey: string, getter: StoreGetterDef) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: { ...store, getters: [...store.getters, getter] },
        },
        isDirty: true,
      };
    }),

    updateInlineStoreGetter: (storeKey: string, pid: string, updates: Partial<StoreGetterDef>) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: {
            ...store,
            getters: store.getters.map((g) => g.pid === pid ? { ...g, ...updates } : g),
          },
        },
        isDirty: true,
      };
    }),

    removeInlineStoreGetter: (storeKey: string, pid: string) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: { ...store, getters: store.getters.filter((g) => g.pid !== pid) },
        },
        isDirty: true,
      };
    }),

    // ── Inline store Action CRUD ──
    addInlineStoreAction: (storeKey: string, action: StoreActionDef) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: { ...store, actions: [...store.actions, action] },
        },
        isDirty: true,
      };
    }),

    updateInlineStoreAction: (storeKey: string, pid: string, updates: Partial<StoreActionDef>) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: {
            ...store,
            actions: store.actions.map((a) => a.pid === pid ? { ...a, ...updates } : a),
          },
        },
        isDirty: true,
      };
    }),

    removeInlineStoreAction: (storeKey: string, pid: string) => set((s) => {
      const store = s.inlineStoreDefs[storeKey];
      if (!store) return s;
      return {
        inlineStoreDefs: {
          ...s.inlineStoreDefs,
          [storeKey]: { ...store, actions: store.actions.filter((a) => a.pid !== pid) },
        },
        isDirty: true,
      };
    }),
```

- [ ] **Step 3: PoiesisState 声明所有新增方法的类型签名**

在 `poiesis/src/store/types.ts` 中追加完整接口声明（与 Step 2 同名方法）。

- [ ] **Step 4: 验证类型**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep -c "error TS"
```
预期: ≤ 776 (pre-existing)

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/store/types.ts poiesis/src/store/slices/store-crud.ts
git commit -m "feat: add inlineStoreDefs state and CRUD actions to Zustand store"
```

---

### Task 5: StorePage 适配抽屉模式（虚拟 tabId）

**Files:**
- Modify: `poiesis/src/pages/StorePage.tsx`

**Interfaces:**
- Consumes: `inlineStoreDefs`, `inlineStoreDefs` CRUD actions (from Task 4)
- Produces: StorePage 接受可选 `inlineMode` prop

- [ ] **Step 1: 增加 inlineMode prop 和适配逻辑**

```typescript
// poiesis/src/pages/StorePage.tsx Props:
interface Props {
  storeName: string;
  tabId: string;
  /** When true, reads/writes inlineStoreDefs instead of storeDefs. */
  inlineMode?: boolean;
}
```

- [ ] **Step 2: 在 StorePage 内部根据 inlineMode 切换数据源**

```typescript
// poiesis/src/pages/StorePage.tsx
// 将 `const store = usePoiesisStore((s) => s.storeDefs[storeName]);`
// 替换为:
  const globalStore = usePoiesisStore((s) => s.storeDefs[storeName]);
  const inlineStore = usePoiesisStore((s) => s.inlineStoreDefs[storeName]);
  const store = inlineMode ? inlineStore : globalStore;

// 类似地，在 createDefaultStore、loadStoreDef 中适配:
// inlineMode 时 loadInlineStoreDef 而非 loadStoreDef
```

- [ ] **Step 3: StoreMembersPanel 适配**

StoreMembersPanel 当前直接读写 `storeDefs[storeName]`。抽屉模式下，需要适配为根据 `inlineMode` 调用不同的 CRUD actions。最简单方案：StoreMembersPanel 不感知 inlineMode，由 StorePage 包装一层 props 映射。

```typescript
// poiesis/src/pages/StorePage.tsx — 传递给 StoreMembersPanel 的 on* props 路由:
  const addSignal = inlineMode
    ? (name: string, sig: StoreSignalDef) => addInlineStoreSignal(storeName, sig)
    : (name: string, sig: StoreSignalDef) => addStoreSignal(storeName, sig);
  // ... 同样处理其他 CRUD 方法
```

> **简化方案（推荐）：** 不修改 StoreMembersPanel 签名。StorePage 通过 Zustand selector 将正确的 store 传递给 StoreMembersPanel，StoreMembersPanel 内部不感知 inline/global 差异，因为 `usePoiesisStore((s) => s.storeDefs[storeName])` 的 `storeName` 在 inline 模式下是 `"ComponentName.StoreName"` —— 但 inline stores 存在 `inlineStoreDefs`，不是 `storeDefs`。

> **最终方案：** 让 StorePage 维护一个"统一视图"。创建衍生对象 `effectiveStore` = `inlineMode ? inlineStoreDefs[name] : storeDefs[name]`。将所有 CRUD 操作通过 callback 代理到正确的 slice 方法。StoreMembersPanel 通过 props 接收 store 数据和回调（它已经这样做了 — 只是通过 Zustand selector 读状态）。

实际上当前 StoreMembersPanel **自己**通过 `usePoiesisStore((s) => s.storeDefs[storeName])` 读数据。最简单的适配是：
- 在 inline 模式下，StorePage 先 `loadInlineStoreDef(def)` 把数据加载到 inlineStoreDefs
- 新增一个 `effectiveStoreDefs` selector 或让 StorePage 在 inline 模式下 proxy 数据
- **或者更简单的：** 让 StoreMembersPanel 接受可选的 `stores` prop override

```typescript
// StoreMembersPanel Props 增加:
  /** Override store lookup (for inline stores in drawer mode).
   *  When provided, used instead of s.storeDefs[storeName]. */
  storeOverride?: StoreDefinition;
```

StoreMembersPanel 中 `const store = storeOverride ?? usePoiesisStore((s) => s.storeDefs[storeName]);`

CRUD 操作同样通过新增 props 覆盖：

```typescript
  // Store CRUD callbacks (when storeOverride is set, these must also be provided)
  onAddSignal?: (signal: StoreSignalDef) => void;
  onUpdateSignal?: (pid: string, updates: Partial<StoreSignalDef>) => void;
  onRemoveSignal?: (pid: string) => void;
  onAddGetter?: (getter: StoreGetterDef) => void;
  onUpdateGetter?: (pid: string, updates: Partial<StoreGetterDef>) => void;
  onRemoveGetter?: (pid: string) => void;
  onAddAction?: (action: StoreActionDef) => void;
  onRemoveAction?: (pid: string) => void;
```

- [ ] **Step 4: 验证无 TypeScript 错误**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep "StorePage\|StoreMembers" | head -20
```
预期: 无错误

- [ ] **Step 5: Commit**

```bash
git add poiesis/src/pages/StorePage.tsx poiesis/src/components/store/StoreMembersPanel.tsx
git commit -m "feat: adapt StorePage and StoreMembersPanel for inline store drawer mode"
```

---

### Task 6: Store 抽屉 UI 组件 + DesignPage 集成

**Files:**
- Create: `poiesis/src/components/design/StoreDrawer.tsx` (新文件)
- Modify: `poiesis/src/pages/DesignPage.tsx:1-55` (imports + store reads)
- Modify: `poiesis/src/components/panels/ContextInspector.tsx:38-56` (Store 按钮)
- Modify: `poiesis/src/core/types/design-page.ts:47-57` (DesignPageState)

**Interfaces:**
- Consumes: `storePanelVisible`, `storePanelActiveStore` (DesignPageState)
- Produces: StoreDrawer 组件 (imported by DesignPage)

- [ ] **Step 1: DesignPageState 增加 storePanelVisible 和 storePanelActiveStore**

```typescript
// poiesis/src/core/types/design-page.ts DesignPageState:
  /** Store drawer panel state */
  storePanelVisible: boolean;
  storePanelActiveStore: string | null;
```

并在 Zustand store 初始化中设置默认值 `storePanelVisible: false, storePanelActiveStore: null`。

- [ ] **Step 2: 创建 StoreDrawer 组件**

```typescript
// poiesis/src/components/design/StoreDrawer.tsx
import { useState, useCallback, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { StorePage } from "../../pages/StorePage";
import type { StoreDefinition } from "../../core/types/store";
import { uid } from "../../core/utils/id";

export function StoreDrawer() {
  const visible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const activeStore = usePoiesisStore((s) => s.designPage.storePanelActiveStore);
  const setStorePanelVisible = usePoiesisStore((s) => s.setStorePanelVisible);
  const setStorePanelActiveStore = usePoiesisStore((s) => s.setStorePanelActiveStore);
  const globalStoreDefs = usePoiesisStore((s) => s.storeDefs);
  const inlineStoreDefs = usePoiesisStore((s) => s.inlineStoreDefs);
  const loadInlineStoreDef = usePoiesisStore((s) => s.loadInlineStoreDef);
  const removeInlineStoreDef = usePoiesisStore((s) => s.removeInlineStoreDef);
  const activeTab = usePoiesisStore((s) => {
    const id = s.designPage.activeTabId;
    return id ? s.designPage.openTabs.find(t => t.id === id) : null;
  });
  const openTab = usePoiesisStore((s) => s.openTab);

  const [showStoreSwitcher, setShowStoreSwitcher] = useState(false);
  const [showCreateModal, setShowCreateModal] = useState(false);

  // All stores (global + inline), deduplicated
  const allStores = [
    ...Object.entries(globalStoreDefs).map(([name, def]) => ({ key: name, name, def, type: "global" as const })),
    ...Object.entries(inlineStoreDefs).map(([key, def]) => ({ key, name: key, def, type: "inline" as const })),
  ];

  // ESC key to close
  useEffect(() => {
    if (!visible) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setStorePanelVisible(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [visible, setStorePanelVisible]);

  // Auto-select first store on open
  useEffect(() => {
    if (visible && !activeStore && allStores.length > 0) {
      setStorePanelActiveStore(allStores[0].key);
    }
  }, [visible, activeStore, allStores, setStorePanelActiveStore]);

  const handleClose = useCallback(() => setStorePanelVisible(false), [setStorePanelVisible]);

  const handleSelectStore = useCallback((key: string) => {
    setStorePanelActiveStore(key);
    setShowStoreSwitcher(false);
  }, [setStorePanelActiveStore]);

  const handleDeleteStore = useCallback((key: string, type: "global" | "inline") => {
    if (type === "global") {
      usePoiesisStore.getState().removeStoreDef(key);
    } else {
      removeInlineStoreDef(key);
    }
    // If deleting active store, switch to first remaining
    if (key === activeStore) {
      const remaining = Object.keys(type === "global" ? globalStoreDefs : inlineStoreDefs)
        .filter(k => k !== key);
      setStorePanelActiveStore(remaining[0] ?? null);
    }
  }, [activeStore, globalStoreDefs, inlineStoreDefs, removeInlineStoreDef, setStorePanelActiveStore]);

  const handleCreateStore = useCallback((name: string, type: "global" | "inline") => {
    const sanitized = name.trim();
    if (!sanitized) return;
    const pid = uid("s");
    const key = type === "inline"
      ? `${activeTab?.componentName ?? "main"}.${sanitized}`
      : sanitized;
    const def: StoreDefinition = {
      pid,
      name: key, // inline stores use "ComponentName.StoreName" as name
      filePath: type === "global" ? `src/stores/${sanitized}.omni` : (activeTab?.filePath ?? ""),
      imports: ["import omniview;"],
      signals: [],
      getters: [],
      actions: [],
    };
    if (type === "global") {
      usePoiesisStore.getState().loadStoreDef(def);
    } else {
      loadInlineStoreDef(def);
    }
    setStorePanelActiveStore(key);
    setShowCreateModal(false);
  }, [activeTab, loadInlineStoreDef, setStorePanelActiveStore]);

  const handleOpenInTab = useCallback(() => {
    if (!activeStore) return;
    // Determine if this store is global or inline
    const isGlobal = activeStore in globalStoreDefs;
    if (isGlobal) {
      const def = globalStoreDefs[activeStore];
      openTab(def.filePath, activeStore, "store");
    } else {
      // Inline stores must be promoted to global — or open a component tab
      // For now, just open the parent component tab
      const tab = activeTab;
      if (tab) openTab(tab.filePath, tab.componentName, "component");
    }
    setStorePanelVisible(false);
  }, [activeStore, globalStoreDefs, activeTab, openTab, setStorePanelVisible]);

  if (!visible) return null;

  const currentStore = allStores.find(s => s.key === activeStore);

  return (
    <div style={{
      position: "absolute",
      top: 0, right: 0, bottom: 0, left: 0,
      zIndex: 10,
      display: "flex",
      flexDirection: "column",
      background: "var(--ol-surface, #fff)",
      borderLeft: "1px solid #e2e8f0",
      boxShadow: "-4px 0 20px rgba(0,0,0,0.08)",
      animation: "storeDrawerIn 250ms cubic-bezier(0.16, 1, 0.3, 1)",
    }}>
      <style>{`
        @keyframes storeDrawerIn {
          from { transform: translateX(100%); }
          to { transform: translateX(0); }
        }
      `}</style>

      {/* Header */}
      <div style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "6px 12px",
        borderBottom: "1px solid #e2e8f0",
        background: "#f8fafc",
        minHeight: 36,
      }}>
        {/* Store switcher */}
        <div style={{ position: "relative" }}>
          <button
            onClick={() => setShowStoreSwitcher(!showStoreSwitcher)}
            style={{
              display: "flex", alignItems: "center", gap: 6,
              padding: "3px 10px",
              fontSize: 12, fontWeight: 600,
              border: "1px solid #e2e8f0", borderRadius: 6,
              background: "#fff", cursor: "pointer",
              color: "#1e293b",
              fontFamily: "inherit",
            }}
          >
            <span style={{ color: "#6366f1", fontSize: 10 }}>●</span>
            {activeStore ?? "选择 Store"}
          </button>

          {showStoreSwitcher && (
            <>
              <div
                onClick={() => setShowStoreSwitcher(false)}
                style={{ position: "fixed", inset: 0, zIndex: 99 }}
              />
              <div style={{
                position: "absolute", top: "100%", left: 0,
                marginTop: 4,
                background: "#fff",
                border: "1px solid #e2e8f0",
                borderRadius: 8,
                boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
                zIndex: 100,
                minWidth: 220,
                overflow: "hidden",
              }}>
                {allStores.map(s => (
                  <div key={s.key} style={{
                    display: "flex", alignItems: "center",
                    padding: "8px 14px",
                    cursor: "pointer",
                    fontSize: 12,
                    color: s.key === activeStore ? "#6366f1" : "#374151",
                    fontWeight: s.key === activeStore ? 600 : 400,
                    borderBottom: "1px solid #f1f5f9",
                  }}
                    onClick={() => handleSelectStore(s.key)}
                    onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = "#f8fafc"; }}
                    onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                  >
                    <span style={{ marginRight: 8 }}>
                      {s.key === activeStore ? "●" : "○"}
                    </span>
                    <span style={{ flex: 1 }}>{s.key}</span>
                    <button
                      onClick={e => { e.stopPropagation(); handleDeleteStore(s.key, s.type); }}
                      style={{
                        border: "none", background: "transparent",
                        cursor: "pointer", fontSize: 14,
                        color: "#94a3b8", padding: "0 4px",
                      }}
                    >✕</button>
                  </div>
                ))}
                <div style={{ borderTop: "1px solid #e2e8f0" }}>
                  <button
                    onClick={() => { setShowStoreSwitcher(false); setShowCreateModal(true); }}
                    style={{
                      width: "100%", padding: "8px 14px",
                      border: "none", background: "transparent",
                      cursor: "pointer", fontSize: 12,
                      color: "#6366f1", fontWeight: 600,
                      textAlign: "left", fontFamily: "inherit",
                    }}
                  >
                    + 新建 Store
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Right buttons */}
        <div style={{ display: "flex", gap: 4 }}>
          <button
            onClick={handleOpenInTab}
            title="在标签页中打开"
            style={{
              padding: "2px 8px", fontSize: 13,
              border: "1px solid #e2e8f0", borderRadius: 4,
              background: "transparent", cursor: "pointer",
            }}
          >📋</button>
          <button
            onClick={handleClose}
            style={{
              padding: "2px 8px", fontSize: 16,
              border: "none", background: "transparent",
              cursor: "pointer", color: "#94a3b8",
            }}
          >✕</button>
        </div>
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        {currentStore ? (
          <StorePage
            storeName={currentStore.key}
            tabId="__store_panel__"
            inlineMode={currentStore.type === "inline"}
          />
        ) : (
          <EmptyStoreGuide onCreate={() => setShowCreateModal(true)} />
        )}
      </div>

      {/* Create Store Modal */}
      {showCreateModal && (
        <CreateStoreModal
          onClose={() => setShowCreateModal(false)}
          onCreate={handleCreateStore}
        />
      )}
    </div>
  );
}

function EmptyStoreGuide({ onCreate }: { onCreate: () => void }) {
  return (
    <div style={{
      display: "flex", flexDirection: "column",
      alignItems: "center", justifyContent: "center",
      height: "100%", color: "#94a3b8", gap: 12,
    }}>
      <div style={{ fontSize: 36 }}>📦</div>
      <div style={{ fontSize: 14, fontWeight: 600 }}>还没有 Store</div>
      <div style={{ fontSize: 12 }}>使用 Store 管理状态、信号和操作</div>
      <button
        onClick={onCreate}
        style={{
          marginTop: 8,
          padding: "6px 16px", fontSize: 12, fontWeight: 600,
          border: "none", borderRadius: 6,
          background: "#6366f1", color: "#fff",
          cursor: "pointer", fontFamily: "inherit",
        }}
      >+ 新建 Store</button>
    </div>
  );
}

function CreateStoreModal({
  onClose, onCreate,
}: {
  onClose: () => void;
  onCreate: (name: string, type: "global" | "inline") => void;
}) {
  const [storeType, setStoreType] = useState<"inline" | "global">("inline");
  const [name, setName] = useState("");

  return (
    <>
      <div onClick={onClose} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)", zIndex: 1000 }} />
      <div style={{
        position: "fixed", top: "50%", left: "50%",
        transform: "translate(-50%, -50%)",
        zIndex: 1001,
        background: "#fff", borderRadius: 12,
        padding: "24px 28px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
        minWidth: 320,
      }}>
        <div style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, color: "#1e293b" }}>
          新建 Store
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, cursor: "pointer" }}>
            <input type="radio" checked={storeType === "inline"}
              onChange={() => setStoreType("inline")} />
            <div>
              <div style={{ fontWeight: 600 }}>内联 Store</div>
              <div style={{ color: "#94a3b8", fontSize: 11 }}>保存在当前组件 .omni 文件中</div>
            </div>
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, cursor: "pointer" }}>
            <input type="radio" checked={storeType === "global"}
              onChange={() => setStoreType("global")} />
            <div>
              <div style={{ fontWeight: 600 }}>全局 Store</div>
              <div style={{ color: "#94a3b8", fontSize: 11 }}>独立文件 src/stores/xxx.omni</div>
            </div>
          </label>
        </div>

        <input
          autoFocus
          placeholder="Store 名称"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => {
            if (e.key === "Enter" && name.trim()) onCreate(name, storeType);
            if (e.key === "Escape") onClose();
          }}
          style={{
            width: "100%", padding: "6px 10px", fontSize: 13,
            border: "1px solid #d1d5db", borderRadius: 6,
            boxSizing: "border-box", marginBottom: 16,
            fontFamily: "inherit",
          }}
        />

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button onClick={onClose} style={{
            padding: "5px 14px", fontSize: 12,
            border: "1px solid #d1d5db", borderRadius: 6,
            background: "#fff", cursor: "pointer", fontFamily: "inherit",
          }}>取消</button>
          <button onClick={() => { if (name.trim()) onCreate(name, storeType); }} style={{
            padding: "5px 14px", fontSize: 12, fontWeight: 600,
            border: "none", borderRadius: 6,
            background: "#6366f1", color: "#fff",
            cursor: "pointer", fontFamily: "inherit",
          }}>创建</button>
        </div>
      </div>
    </>
  );
}
```

- [ ] **Step 3: ContextInspector 标题栏加 Store 按钮**

```typescript
// poiesis/src/components/panels/ContextInspector.tsx
// 在 ContextInspector 组件中，将标题栏（无论有无选中元素）修改为:

export function ContextInspector() {
  const selectedIds = useSelectedElementIds();
  const elements = useElements();
  const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const setStorePanelVisible = usePoiesisStore((s) => s.setStorePanelVisible);

  const el = selectedIds.length > 0 ? elements.find((e) => e.id === selectedIds[0]) : undefined;

  return (
    <div className="context-inspector" style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* Title bar */}
      <div style={{
        display: "flex", alignItems: "center", justifyContent: "space-between",
        padding: "6px 12px", borderBottom: "1px solid #e2e8f0",
        minHeight: 32,
      }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: "#64748b" }}>
          {el ? ("tag" in el ? `属性 · <${el.tag}>` : "属性") : "属性面板"}
        </span>
        <button
          onClick={() => setStorePanelVisible(!storePanelVisible)}
          title={storePanelVisible ? "关闭 Store 面板" : "打开 Store 面板"}
          style={{
            display: "flex", alignItems: "center", gap: 4,
            padding: "2px 10px", fontSize: 11, fontWeight: 600,
            border: storePanelVisible ? "1px solid #6366f1" : "1px solid #e2e8f0",
            borderRadius: 4,
            background: storePanelVisible ? "#eef2ff" : "transparent",
            cursor: "pointer",
            color: storePanelVisible ? "#6366f1" : "#64748b",
            fontFamily: "inherit",
          }}
        >
          📦 Store
        </button>
      </div>

      {/* Body — existing logic */}
      {!el ? (
        <div style={{ padding: 16, color: "#94a3b8", fontSize: 13, textAlign: "center", flex: 1 }}>
          选择元素以编辑属性
        </div>
      ) : (
        // ... existing inspector content
      )}
    </div>
  );
}
```

- [ ] **Step 4: 在 DesignPage 中渲染 StoreDrawer**

```typescript
// poiesis/src/pages/DesignPage.tsx
// 在 import 中添加:
import { StoreDrawer } from "../components/design/StoreDrawer";

// 在 Zustand selector 中添加:
const storePanelVisible = usePoiesisStore((s) => s.designPage.storePanelVisible);

// 在右侧面板 div 之后，或者覆盖在画布区域上，添加:
{/* Store Drawer — overlays center area when visible */}
{storePanelVisible && activeTabType === "component" && (
  <div style={{
    gridRow: 3,
    gridColumn: showSidePanels ? 2 : "1 / -1",
    overflow: "hidden",
    position: "relative",
    zIndex: 10,
  }}>
    <StoreDrawer />
  </div>
)}
```

- [ ] **Step 5: 在 Zustand store 中实现 setStorePanelVisible / setStorePanelActiveStore**

在 `poiesis/src/store/slices/` 中的 design-page 相关 slice 里实现:

```typescript
setStorePanelVisible: (visible: boolean) => set((s) => ({
  designPage: { ...s.designPage, storePanelVisible: visible },
})),
setStorePanelActiveStore: (name: string | null) => set((s) => ({
  designPage: { ...s.designPage, storePanelActiveStore: name },
})),
```

- [ ] **Step 6: 验证编译**

```bash
cd poiesis && npx tsc --noEmit 2>&1 | grep -c "error TS"
```
预期: ≤ 776 (pre-existing)

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/components/design/StoreDrawer.tsx poiesis/src/pages/DesignPage.tsx poiesis/src/components/panels/ContextInspector.tsx poiesis/src/core/types/design-page.ts
git commit -m "feat: add Store drawer UI with switcher, create modal, and DesignPage integration"
```

---

### Task 7: 保存链路 — pipeline 与 useSaveProject 支持内联 Store

**Files:**
- Modify: `poiesis/src/services/codegen/pipeline.ts` (传入 inlineStores 到 emitter)
- Modify: `poiesis/src/hooks/useSaveProject.ts` (component tab 保存时包含 inlineStores)

**Interfaces:**
- Consumes: `inlineStoreDefs` from Zustand, `EmitInput.inlineStores` (from Task 3)
- Produces: 内联 stores 写入组件 .omni 文件

- [ ] **Step 1: pipeline.ts generateOmniFiles 支持 inlineStores**

```typescript
// poiesis/src/services/codegen/pipeline.ts GenerateOmniSourceOptions:
  /** Inline store definitions to emit inside the component body. */
  inlineStores?: StoreDefinition[];

// 在构建 EmitInput 时:
  const emitInput: EmitInput = {
    imports,
    stores: [], // standalone stores handled separately
    inlineStores: options.inlineStores ?? [], // NEW
    component: { ... },
    // ... rest
  };
```

- [ ] **Step 2: pipeline.ts — 内联 Store 不生成独立文件**

当 `options.filePath` 不是 `src/stores/` 路径时（即 component 文件），跳过独立 store 文件生成。现有逻辑在 `generateOmniFiles` 的 store 分支中 — 确保 inlineStores 已经通过 emitInput 走入 emitComponent 路径。

- [ ] **Step 3: useSaveProject.ts component tab 保存时传入 inlineStores**

```typescript
// poiesis/src/hooks/useSaveProject.ts handleSave()
// 在 component tab 路径（非 design、非 store）中，增加 inlineStores:

      // 收集与当前组件关联的 inlineStoreDefs
      const componentName = activeTab?.componentName ?? "main";
      const inlineStores = Object.entries(store.inlineStoreDefs)
        .filter(([key]) => key.startsWith(`${componentName}.`))
        .map(([, def]) => def);

      const genResult = generateOmniFiles({
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName,
        rootOrder: store.rootOrder,
        filePath: activeTab?.filePath,
        inlineStores, // NEW
        designName: store.activeDesign?.name && ... ? store.activeDesign.name : undefined,
      });
```

- [ ] **Step 4: 解析 .omni 文件后将 inlineStoreDefs 加载到 Zustand**

在 `DesignTabBar.tsx` 的 `handleOpenFile` 和关联的 .omni 加载逻辑中:

```typescript
// 在 parseOmniSource 结果中:
if (result.inlineStoreDefs) {
  const { loadInlineStoreDef } = usePoiesisStore.getState();
  for (const def of Object.values(result.inlineStoreDefs)) {
    loadInlineStoreDef(def);
  }
}
```

- [ ] **Step 5: 编写集成测试**

```typescript
// poiesis/src/services/codegen/__tests__/save-inline-store.test.ts
// (如已有 pipeline 测试套件则追加)
test("pipeline includes inline stores in component output", () => {
  const inlineStore: StoreDefinition = {
    pid: "s1",
    name: "Test.CartStore",
    filePath: "src/Test.omni",
    imports: [],
    signals: [{ pid: "s1_n1", name: "items", type: "Array<String>", defaultValue: "[]" }],
    getters: [],
    actions: [],
  };
  const result = generateOmniFiles({
    designElements: [],
    designWires: [],
    codeTabs: [],
    componentName: "Test",
    rootOrder: [],
    filePath: "src/Test.omni",
    inlineStores: [inlineStore],
  });
  const source = result.value?.["src/Test.omni"] ?? "";
  expect(source).toContain("store CartStore {");
  expect(source).toContain("signal items: Array<String> = [];");
});
```

- [ ] **Step 6: 运行全部 codegen 测试**

```bash
cd poiesis && npx vitest run src/services/codegen/__tests__/
```
预期: 所有测试 PASS

- [ ] **Step 7: Commit**

```bash
git add poiesis/src/services/codegen/pipeline.ts poiesis/src/hooks/useSaveProject.ts poiesis/src/services/codegen/__tests__/save-inline-store.test.ts
git commit -m "feat: integrate inline stores into save pipeline and file loading"
```

---

## Post-Implementation Checklist

- [ ] 打开 Poiesis → 组件标签页 → 右侧面板有 `📦 Store` 按钮
- [ ] 点击按钮 → 抽屉从右滑入（250ms 动画）
- [ ] 左上角小球切换 Store，弹出浮层列出所有 Store
- [ ] 新建内联 Store → 显示在列表中
- [ ] 新建全局 Store → 显示在列表中（带独立文件）
- [ ] 编辑 Signal/Getter/Action → 正常 CRUD
- [ ] 点击 ✕ 关闭抽屉
- [ ] 再次打开 → 记住上次选择的 Store
- [ ] 保存 → 内联 Store 写入组件 .omni 文件
- [ ] 关闭 Tab → 重新打开 → 内联 Store 正确解析回内存
- [ ] Esc 关闭抽屉
- [ ] 📋 按钮打开标签页
