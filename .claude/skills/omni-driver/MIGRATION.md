# OmniDriver Migration Guide: v1 → v2

> **Editor Code Section Redesign**: Pure data-flow architecture replaces the old flow+data dual-type system.

## Summary of Breaking Changes

| v1 API (deprecated) | v2 Replacement | Why |
|---------------------|----------------|-----|
| `el.createEventHandler("onPress", {...})` | `el.bindEvent("onPress", "fnName", {...})` | Events decoupled from code graph; no longer creates action nodes |
| `O.codeBlock("action", ...)` | `O.customFunction("fnName", [...], "retType")` + `O.returnNode(fnId)` | Replaced by customFn + return node pair |
| `O.codeBlock("onPress", ...)` | `el.bindEvent("onPress", "fnName")` | Event source nodes removed |
| `portType: "flow"` connections | `portType: "data"` connections only | FlowEdge deleted; all connections are data edges |
| `ViewEventNode` | _removed_ | Events handled via PropertyPanel binding |
| global undo only | per-tab undo (code) + global undo (design) | Undo stacks split by mode |

## Architecture Change

```
v1 (old):
  Design Canvas: UI elements
  Code Canvas: Event Nodes → Action Nodes → Signal Nodes
               Flow edges (temporal) + Data edges (value)
               Global undo stack

v2 (new):
  Design Canvas: UI elements + Event Binding (PropertyPanel)
  Code Canvas: Multiple tabs, each with customFn + return + data nodes
               Data edges only (no flow edges)
               Per-tab undo stacks (code) + Global undo (design)
```

## Migration Recipes

### Counter App: v1 → v2

**v1 (old):**
```javascript
const O = window.__OMNI__;
O.switchMode("design");
O.clearAll();

const card = O.card({ padding: "24px" });
const btn = O.button("+1", { variant: "primary" }, card.id);
const count = O.codeBlock("signal", "count", {
  typeAnnotation: "i32", source: "signal count: i32 = 0",
});

const { handler, wire } = btn.createEventHandler("onPress", {
  handlerName: "increment",
  source: `action increment(event) { count.set(count.value + 1) }`,
});
handler.connectTo(count, "result", "set");
```

**v2 (new):**
```javascript
const O = window.__OMNI__;
O.help();
O.switchMode("design");
O.clearAll();

const card = O.card({ padding: "24px" });
const btn = O.button("+1", { variant: "primary" }, card.id);
const count = O.codeBlock("signal", "count", {
  typeAnnotation: "i32", source: "signal count: i32 = 0",
});

// Step 1: Design 模式绑定事件
btn.bindEvent("onPress", "handleIncrement");

// Step 2: Code 模式创建处理函数
O.switchMode("code");
const tab = O.createTab("event", "Counter Events", "🔢");
O.switchTab(tab.id);
const fn = O.customFunction("handleIncrement", [], "void");
fn.setSource(`action increment(event) { count.set(count.value + 1) }`);
O.returnNode(fn.id);

O.validate();
```

### Search Filter: v1 → v2

**v1 (old):**
```javascript
const input = O.input("Search...", {}, card.id);
const query = O.codeBlock("signal", "searchQuery", { typeAnnotation: "String" });
const { handler } = input.createEventHandler("onChange", {
  handlerName: "updateSearch",
  source: `action updateSearch(event: ChangeEvent) { searchQuery.set(event.value) }`,
});
handler.connectTo(query, "result", "set");
```

**v2 (new):**
```javascript
const input = O.input("Search...", {}, card.id);
const query = O.codeBlock("signal", "searchQuery", { typeAnnotation: "String" });

// Design 模式绑定事件 + 参数
input.bindEvent("onChange", "updateSearch", {
  params: {
    searchValue: { sourceType: "event", value: "event.value", expectedType: "String" },
  }
});

// Code 模式创建处理函数
O.switchMode("code");
const tab = O.createTab("event", "Search", "🔍");
O.switchTab(tab.id);
const fn = O.customFunction("updateSearch", [
  { name: "searchValue", type: "String" },
], "void");
fn.setSource(`action updateSearch(searchValue: String) { searchQuery.set(searchValue) }`);
O.returnNode(fn.id);
```

### Computed 连线（无变化）

```javascript
// computed 用法在 v2 中保持不变
const total = O.codeBlock("computed", "total", { typeAnnotation: "f64" });
price.connectTo(total, "value", "deps");
quantity.connectTo(total, "value", "deps");
```

## Port Reference Changes

| Node Tag | v1 Ports | v2 Ports |
|----------|---------|---------|
| `signal` | value(out), set(in) ✅ 不变 | value(out), set(in) |
| `computed` | value(out), deps(in) ✅ 不变 | value(out), deps(in) |
| `store` | items(out) ✅ 不变 | items(out) |
| `action` | call(flow,in), result(data,out) | 🚫 **节点已移除** |
| `onPress` / `onChange` | flowOut + event(data) | 🚫 **节点已移除**（事件绑定在 PropertyPanel） |
| `customFn` | _(无)_ | return(data,out) + 动态输入参数 |
| `return` | _(无)_ | value(data,in) |

## E2E Test Migration

### Files to update:

1. **`editor/e2e/omni-driver-counter.spec.ts`**
   - Replace `createEventHandler("onPress", { source: ... })` with `bindEvent("onPress", "handleIncrement")`
   - Add code mode steps: `createTab("event", ...)`, `customFunction(...)`, `returnNode(...)`
   - Remove `action` code block references

2. **`editor/e2e/omni-driver-task-manager.spec.ts`**
   - Replace all `createEventHandler` calls
   - Update `codeBlock("action", ...)` to `customFunction(...)` + `returnNode(...)`
   - Recalculate element counts (action nodes → customFn + return pairs)

3. **`build/test/omni-driver/task-manager-app.js`**
   - Full rewrite to v2 API
   - New canonical script template

## API That Is REMOVED

| Removed API | Notes |
|-------------|-------|
| `el.createEventHandler(eventName, options)` | Returns `{ handler, wire }`. Use `el.bindEvent()` instead. |
| `O.codeBlock("action", ...)` | action block type no longer registered |
| `O.codeBlock("onPress", ...)` / `O.codeBlock("onChange", ...)` | Event source nodes removed |
| `O.codeBlock("runCodeBlock", ...)` | Replaced by customFn + return |
| FlowEdge / DataEdge distinction | All edges are DataEdge now |

## API That Is NEW

| New API | Description |
|---------|-------------|
| `el.bindEvent(eventName, fnName, options?)` | Design-mode event binding (sets event prop) |
| `el.unbindEvent(eventName)` | Remove event binding |
| `el.getAvailableEvents()` | List available event names for this element |
| `el.getEventBindings()` | Get current bindings on this element |
| `O.findMatchingFunctions(tabId, eventName)` | Find customFn matching event signature |
| `O.listAllEventBindings()` | Global view of all event bindings |
| `O.createTab(type, label?, icon?)` | Create code tab ("event" or "logic") |
| `O.switchTab(tabId)` | Switch active code tab |
| `O.removeTab(tabId)` | Delete a code tab |
| `O.listTabs()` | List all code tabs with metadata |
| `O.getActiveTab()` | Get current active tab |
| `O.getTab(tabId)` | Get tab by ID |
| `O.cloneTab(tabId, newLabel?)` | Deep clone a tab |
| `O.exportTab(tabId)` | Export tab to JSON |
| `O.importTab(json)` | Import tab from JSON |
| `O.addVariable(tabId, name, type, default?, desc?)` | Add variable to tab scope |
| `O.removeVariable(tabId, name)` | Remove variable from tab scope |
| `O.getVariables(tabId)` | List variables in tab scope |
| `O.getTabElements(tabId)` | Get elements in a specific tab |
| `O.getTabWires(tabId)` | Get wires in a specific tab |
| `O.customFunction(fnName, fnParams, fnReturnType)` | Create customFn node |
| `O.returnNode(parentFnId)` | Create return node for a customFn |
| `fn.addParam(name, type, default?)` | Add parameter to customFn |
| `fn.setReturnType(type)` | Change return type |
| `fn.setDescription(desc)` | Set function description |
| `O.validateFunction(fnId)` | BFS type consistency check |
| `O.listCustomFunctions()` | List all customFn nodes |
| `O.findFunctionByName(fnName)` | Find customFn by name |
| `O.checkFunctionNameConflict(fnName)` | Check for name conflicts |
| `O.getFunctionSignature(fnId)` | Get signature summary |
