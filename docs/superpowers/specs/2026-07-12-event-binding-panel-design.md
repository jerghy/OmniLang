# 事件绑定面板重设计

日期: 2026-07-12 | 状态: approved

## 目标

重设计 Poiesis 右侧面板中的 EventsSection，为所有元素类型（不仅是自定义组件）提供直观的事件绑定交互。

## 当前问题

### UI 问题
1. EventsSection 仅对非 basic 元素显示（`!isBasic` 条件），button/input 等基础元素无法绑定事件
2. 事件列表硬编码 14 个，未使用 componentProtocol 中的 40+ 事件定义
3. handler 只支持手动输入字符串名，无法可视化选择函数/action 或编写内联表达式

### 管线问题（解析/发射往返）
4. **Parser 只解析单个 IDENTIFIER**（`parser-view.ts:298`）：`@onClick={handleClick}` 可以解析，但 `@onClick={() => { ... }}` 或 `@onClick={TodoStore.add("item")}` 无法解析
5. **Emitter 丢失 `on` 前缀**（`source-emitter.ts:648`）：`props.onClick` → 发射为 `@click={handler}`，下次解析时 key 变成 `click`（不以 `on` 开头），事件在二次保存后丢失

## 面板层级顺序

```
属性 (PropsSection)
事件 (EventsSection)      ← 新位置，Class 上面
Class (ClassesSection)
Variant (VariantBindingSection)
样式 (InlineStylePanel)
```

---

## 零、Phase 0：Parser / Emitter 管线修复（先于 UI）

### 0.1 Parser 修复

**文件**：`parser-view.ts` 第 293-301 行

**问题 A**：只解析单个 IDENTIFIER handler

当前：
```typescript
const handlerName = internals.stream.expect("IDENTIFIER").text;
result.events[eventName] = handlerName;
```

修复后：将 `@{...}` 内的内容作为原始表达式字符串读取，支持：
- `@onClick={handleClick}` — 简单标识符
- `@onClick={handleClick()}` — 函数调用
- `@onClick={switchTab("home")}` — 带参数调用
- `@onClick={TodoStore.add(item.id)}` — Store action 调用
- `@onClick={() => { count = count + 1; }}` — 内联闭包

通过括号匹配深度计数收集 `{ }` 内的所有 token，拼接为原始字符串。

**问题 B**：事件名存储时丢失 `on` 前缀

当前存储方式：`@click={handler}` → `props.click = "handler"`（无 `on` 前缀）

修复后：统一存储为 `on` 前缀格式。解析 `@click` 和 `@onClick` 都归一化为 `props.onClick`。与 Rust 端 parser 对齐（Rust parser 保留 `on` 前缀）。

### 0.2 Emitter 修复

**文件**：`source-emitter.ts` 第 646-653 行

**问题 A**：发射时丢失 `on` 前缀（`onClick` → `@click`），导致二次解析后事件 key 不匹配

当前：
```typescript
const evtName = key[2].toLowerCase() + key.slice(3); // onClick → click
parts.push(` @${evtName}={${val}}`);
```

修复后：保留 `on` 前缀，发射 `@onClick={handler}`：
```typescript
parts.push(` @${key}={${val}}`);
```

**问题 B**：fallback 路径只匹配 `on*` 前缀 key

修复后：统一使用 `on*` 前缀规范，无需兼容两种格式。

### 0.3 存储规范确立

| 层 | 事件名格式 | 示例 |
|----|-----------|------|
| `.omni` 源码 | `@onXxx` | `@onClick={handler}` |
| Store `element.props` | `onXxx` | `props.onClick = "handler"` |
| Handler 值 | 原始表达式字符串 | `handleClick`、`switchTab("home")`、`() => { ... }` |

往返保证：
```
.omni: @onClick={handleClick}
  → parse → props.onClick = "handleClick"
  → emit  → @onClick={handleClick}  ← 完全一致
```

---

## 一、事件列表（右侧面板）

### 视觉

```
┌──────────────────────────────────┐
│ 事件                       [+ 添加]│
│                                  │
│ ┌──────────┬──────────────┬──┐  │
│ │ onClick ▼│ handleClick  │✕ │  │
│ └──────────┴──────────────┴──┘  │
│ ┌──────────┬──────────────┬──┐  │
│ │ onInput ▼│ () => { ... }│✕ │  │
│ └──────────┴──────────────┴──┘  │
└──────────────────────────────────┘
```

### 行为

| 操作 | 行为 |
|------|------|
| 点击「+ 添加」 | 列表末尾新增空事件行，默认事件名 `onPress`，handler 为空 |
| 事件下拉 | `<select>` + `<optgroup>` 分组显示 componentProtocol 事件（点击/输入/键盘/焦点/滚动/拖拽/...），已绑定事件不出现在选项中 |
| handler 区域 | 只读展示，点击打开浮层弹窗；空值时显示灰色占位「点击编辑...」 |
| ✕ 按钮 | 删除该事件绑定，设置 `prop[name] = ""` |
| 可滚动 | 事件列表项超过一定数量时滚动 |

### 事件分组建议

```
点击        onPress, onClick, onDoubleClick, onContextMenu
输入        onChange, onInput
焦点        onFocus, onBlur
键盘        onKeyDown, onKeyUp
表单        onSubmit
滚动        onScroll
鼠标        onMouseEnter, onMouseLeave, onMouseDown, onMouseUp, onMouseMove
拖拽        onDragStart, onDragOver, onDragEnd, onDrop
触摸        onTouchStart, onTouchEnd, onTouchMove
滚轮        onWheel
媒体        onLoad, onError, onPlay, onPause, onEnded
动画        onAnimationStart, onAnimationEnd, onAnimationIteration, onTransitionEnd
切换        onToggle
剪贴板      onCopy, onCut, onPaste
```

---

## 二、浮层弹窗 — 整体结构

### 触发

点击事件列表中的 handler 区域（已绑定或占位文字）。

### 视觉

```
┌──────────────────────────────────────────┐
│ 编辑 onClick 处理器                 [✕]  │
├──────────────────────────────────────────┤
│  ● 绑定函数    ○ 写表达式                 │
├──────────────────────────────────────────┤
│                                          │
│   (根据 tab 显示不同内容)                  │
│                                          │
├──────────────────────────────────────────┤
│                        [取消]    [确认]   │
└──────────────────────────────────────────┘
```

### 行为

- 标题动态显示事件名
- tab 切换用圆形指示器；空值默认「绑定函数」，已有表达式默认「写表达式」
- 半透明遮罩 + 居中浮层（类似 CodeEditorModal）
- 尺寸：宽 560px，高自适应（约 420px），z-index 在画布之上
- Esc 关闭

---

## 三、绑定函数 Tab

### 视觉

```
┌──────────────────────────────────────────┐
│ 编辑 onClick 处理器                 [✕]  │
├──────────────────────────────────────────┤
│  ● 绑定函数    ○ 写表达式                 │
├──────────────────────────────────────────┤
│                                          │
│  函数/Action                             │
│  ┌────────────────────────────────────┐  │
│  │ handleClick                    ▼  │  │
│  └────────────────────────────────────┘  │
│                                          │
│  参数                                    │  ← 仅当选中函数有参数时显示
│  ┌ item ─────────────────────────────┐  │
│  │ todo.id                           │  │
│  └───────────────────────────────────┘  │
│                                          │
├──────────────────────────────────────────┤
│                        [取消]    [确认]   │
└──────────────────────────────────────────┘
```

### 函数/Action 选择下拉

点击函数输入框后展开：

```
┌── 🔍 搜索... ─────────────────────────┐
│ 组件函数                                │
│  handleClick()                         │
│  handleInput(e: InputEvent)            │
│  reset()                               │
│ Store Actions                          │
│  TodoStore.add(item: String)           │
│  TodoStore.remove(id: I32)            │
│  ThemeStore.toggle()                   │
└────────────────────────────────────────┘
```

### 交互流程

1. 点击函数/Action 输入框 → 下方展开选择面板
2. 搜索框实时过滤，高亮匹配
3. 选中的函数在面板中高亮，点击后关闭面板，回填输入框
4. 如有参数 → 下方动态显示参数输入行（每行 `label = 参数名`，value 为表达式输入框）
5. 确认 → 生成最终字符串：
   - 无参：`handleClick`
   - 有参：`switchTab("home")` 或 `TodoStore.add(todo.id)`

### 数据来源

| 来源 | Store 字段 | 显示格式 |
|------|-----------|---------|
| 组件 fn | `componentDefs[name].fns` | `fnName(params)` |
| Store action | `storeDefs[name].actions`（限被当前组件引用的 store） | `StoreName.actionName(params)` |

---

## 四、写表达式 Tab

### 视觉

```
┌──────────────────────────────────────────┐
│ 编辑 onClick 处理器                 [✕]  │
├──────────────────────────────────────────┤
│  ○ 绑定函数    ● 写表达式                 │
├──────────────────────────────────────────┤
│                                          │
│  ┌────────────────────────────────────┐  │
│  │ () => {                           │  │
│  │   count = count + 1;              │  │  ← Monaco 编辑器
│  │   log = "clicked";                │  │
│  │ }                                 │  │
│  └────────────────────────────────────┘  │
│                                          │
├──────────────────────────────────────────┤
│                        [取消]    [确认]   │
└──────────────────────────────────────────┘
```

### 行为

- 复用 Monaco 编辑器初始化逻辑（从 `CodeEditorModal.tsx` 或共享 hook）
- 语言模式：`omnilang`
- 编辑器高度约 260px，不显示 minimap
- 新事件默认模板：`() => {\n  \n}`
- 切换 tab 保留当前编辑内容
- 确认后 handler 值 = 编辑器内容（不额外包装）

---

## 五、数据流

```
右侧面板 EventsSection
  │
  ├─ 读取: element.props 中 key.startsWith("on") 的条目
  │        + getComponentEvents(tag) 获取可用事件
  │
  ├─ 添加事件: setProp(id, "onPress", "")  (handler 为空)
  │
  ├─ 点击 handler → 打开 HandlerModal
  │
  └─ 确认 → setProp(id, eventName, handlerString)
```

### handlerString 格式

| 来源 | 示例 |
|------|------|
| 无参 fn | `handleClick` |
| 有参 fn | `switchTab("home")` |
| 无参 store action | `TodoStore.toggle` |
| 有参 store action | `TodoStore.add(todo.id)` |
| 内联闭包 | `() => { count = count + 1; }` |

---

## 六、文件变更

### 新建

| 文件 | 职责 |
|------|------|
| `poiesis/src/components/panels/inspector/HandlerModal.tsx` | 浮层弹窗主组件：tab 切换 + 确认/取消 |
| `poiesis/src/components/panels/inspector/FunctionPicker.tsx` | 函数/Action 选择面板（搜索 + 列表 + 参数输入） |
| `poiesis/src/components/panels/inspector/ExpressionEditor.tsx` | Monaco 编辑器包装（写表达式 tab） |
| `poiesis/src/components/panels/inspector/useHandlerItems.ts` | Hook：收集当前组件可用的 fn/action 列表 |

### 修改

| 文件 | 变更 |
|------|------|
| `parser-view.ts` | 修复事件解析：支持任意表达式 handler + 归一化 `on` 前缀 |
| `source-emitter.ts` | 修复事件发射：保留 `on` 前缀 |
| `EventsSection.tsx` | 完全重写：紧凑行列表 + 弹窗触发 |
| `ContextInspector.tsx` | 移除 `!isBasic` 条件（已做），调整面板顺序（已做） |

---

## 七、待后续

- 自定义事件（component event `emit`）的声明和绑定 — 等基础事件面板完成后再讨论
- SignalSlashPicker 集成到参数输入框 — 未来可加 `/` 触发信号补全
- 事件处理器重复检查 — 同一个事件名只允许绑定一次（通过下拉去重实现）
