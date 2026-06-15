# OmniDriver v2 E2E Test Plan

## 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `editor/e2e/omni-driver-counter.spec.ts` | **修改** | 迁移到 v2 API |
| `editor/e2e/omni-driver-task-manager.spec.ts` | **修改** | 迁移到 v2 API |
| `build/test/omni-driver/task-manager-app.js` | **修改** | 重写为 v2 API |
| `editor/e2e/omni-driver-code-tab.spec.ts` | **新增** | 测试 Code Tab 管理 API |
| `editor/e2e/omni-driver-customfn.spec.ts` | **新增** | 测试 customFn + return + BFS 验证 |
| `editor/e2e/omni-driver-event-binding.spec.ts` | **新增** | 测试 bindEvent + ParamBinding |
| `build/test/omni-driver/counter-v2.js` | **新增** | v2 Counter 脚本（供 E2E 加载） |

---

## Test 1: Counter v2（修改现有）

**文件**: `editor/e2e/omni-driver-counter.spec.ts`

**变更**:
1. 用 `bindEvent("onPress", "handleIncrement")` 替换 `createEventHandler("onPress", {...})`
2. 新增 code 模式操作：创建 event tab → customFunction → returnNode
3. 移除 `action` codeBlock 引用
4. 验证 `O.listTabs()` 和 `O.listCustomFunctions()` 在 pre-compile 状态中正确

**预期**:
- preCompile elementCount >= 5 (card + heading + button + text + signal + tab elements)
- 无 action codeBlock
- listCustomFunctions 返回 `handleIncrement`

---

## Test 2: Task Manager Pro v2（修改现有）

**文件**: `editor/e2e/omni-driver-task-manager.spec.ts` + `build/test/omni-driver/task-manager-app.js`

**变更**:
1. 所有 `createEventHandler("onPress"|"onChange", { source: ... })` → `bindEvent(eventName, fnName)`
2. 所有 event handler 函数移到 code 模式 event tab（customFn + return）
3. 用 `O.createTab("event", ...)` 创建事件 tab
4. Recalculate element counts（action 节点 → customFn + return 对，数量增加）

**预期**:
- elementCount >= 70（因为每个 handler 多了 return 节点）
- wireCount >= 15 不变
- codeBlock 计数需调整（无 action block，有 customFn）

---

## Test 3: Code Tab Management（新增）

**文件**: `editor/e2e/omni-driver-code-tab.spec.ts`

**测试场景**:

```javascript
test("creates, switches, lists, clones, and removes code tabs", async ({ page }) => {
  // 1. 创建多个 tab
  //    - logic tab "Main Logic"
  //    - event tab "Button Events"
  //    - event tab "Form Events"
  // 2. listTabs() → 验证 3 个 tab
  // 3. switchTab → 验证 active tab 变化
  // 4. addVariable / getVariables → 验证变量 scope 隔离
  // 5. cloneTab → 验证克隆内容一致（不同 id）
  // 6. removeTab → 验证 tab 被删除（最后一个 tab 不能删）
  // 7. exportTab / importTab → 往返无数据丢失
  // 8. getTabElements / getTabWires → 验证非空
});
```

**预期**:
- tab CRUD 完整工作
- 变量 scope 隔离
- clone/export/import 往返无数据丢失
- remove 最后 tab 抛出错误

---

## Test 4: Custom Function Operations（新增）

**文件**: `editor/e2e/omni-driver-customfn.spec.ts`

**测试场景**:

```javascript
test("creates, validates, and discovers custom functions", async ({ page }) => {
  // 1. customFunction("calculateTotal", [{name:"price",type:"f64"},{name:"qty",type:"i32"}], "f64")
  // 2. 验证 port: return(out,f64) + price(in,f64) + qty(in,i32)
  // 3. returnNode(fn.id) → 验证 port: value(in)
  // 4. addParam("discount", "f64") → 验证端口动态添加
  // 5. setReturnType("f64") → 验证 return 端口类型更新
  // 6. validateFunction → 验证 BFS 检查通过（无 return 不通过）
  // 7. listCustomFunctions → 验证函数列表正确
  // 8. findFunctionByName("calculateTotal") → 验证找到
  // 9. checkFunctionNameConflict("calculateTotal") → 验证冲突检测
  // 10. getFunctionSignature → 验证签名摘要正确
});
```

**预期**:
- customFn 端口动态生成正确
- validateFunction 检查 return 存在性 + 类型一致性
- 函数发现 API 完整工作

---

## Test 5: Event Binding v2（新增）

**文件**: `editor/e2e/omni-driver-event-binding.spec.ts`

**测试场景**:

```javascript
test("binds events with paramBindings and discovers matching functions", async ({ page }) => {
  // 1. 创建 Button + Form + Input
  // 2. button.bindEvent("onPress", "handleSubmit")
  //     - 验证 element.props.onPress === "handleSubmit"
  // 3. input.bindEvent("onChange", "updateSearch", { params: {
  //      searchValue: { sourceType: "event", value: "event.value", expectedType: "String" }
  //    }})
  //     - 验证 paramBindings 写入 element 数据
  // 4. getAvailableEvents() → 验证返回 ["onPress", "onHover", "onFocus", "onLongPress"]
  // 5. getEventBindings() → 验证返回绑定记录
  // 6. unbindEvent("onPress") → 验证 element.props.onPress 被移除
  // 7. listAllEventBindings() → 验证全局视图
  // 8. findMatchingFunctions(tabId, "onPress") → 验证签名匹配
  //     - 先在 event tab 中创建一个 customFn "handleClick" 并验证被匹配到
});
```

**预期**:
- bindEvent 正确设置 element props
- paramBindings 完整存储
- unbindEvent 正确清理
- listAllEventBindings 跨元素汇总
- findMatchingFunctions 按签名过滤

---

## 测试矩阵

| 测试 | API 覆盖 | 状态 |
|------|---------|------|
| Counter v2 | bindEvent, customFunction, returnNode, listTabs, listCustomFunctions | **需改写** |
| Task Manager v2 | All convenience methods + validate + file ops | **需改写** |
| Code Tab | createTab, switchTab, removeTab, listTabs, getActiveTab, getTab, cloneTab, exportTab, importTab, addVariable, getVariables, removeVariable, getTabElements, getTabWires | **新增** |
| Custom Functions | customFunction, returnNode, addParam, setReturnType, setDescription, validateFunction, listCustomFunctions, findFunctionByName, checkFunctionNameConflict, getFunctionSignature | **新增** |
| Event Binding | bindEvent, unbindEvent, getAvailableEvents, getEventBindings, listAllEventBindings, findMatchingFunctions | **新增** |

## 撤销测试

以下测试确认 v1 API 正确被移除/报错：

```javascript
test("createEventHandler throws deprecation error in v2", async ({ page }) => {
  // btn.createEventHandler("onPress") → throws Error with deprecation message
});

test("codeBlock('action', ...) throws unknown block type error", async ({ page }) => {
  // O.codeBlock("action", "test") → throws Error
});

test("codeBlock('onPress', ...) throws unknown block type error", async ({ page }) => {
  // O.codeBlock("onPress", "test") → throws Error
});
```

## 运行前提

```bash
# Terminal 1: Backend
python -m editor.server.main --dir build/test --port 8000

# Terminal 2: Frontend
cd editor/client && npm run dev  # → http://localhost:5174

# Terminal 3: Run tests
npx playwright test editor/e2e/omni-driver-counter.spec.ts
npx playwright test editor/e2e/omni-driver-code-tab.spec.ts
npx playwright test editor/e2e/omni-driver-customfn.spec.ts
npx playwright test editor/e2e/omni-driver-event-binding.spec.ts
npx playwright test editor/e2e/omni-driver-task-manager.spec.ts
```
