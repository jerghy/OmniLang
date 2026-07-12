# 事件绑定面板重设计

日期: 2026-07-12 | 状态: approved

## 目标

重设计 Poiesis 右侧面板中的 EventsSection，为所有元素类型（不仅是自定义组件）提供直观的事件绑定交互。

## 当前问题

1. EventsSection 仅对非 basic 元素显示（`!isBasic` 条件），button/input 等基础元素无法绑定事件
2. 事件列表硬编码 14 个，未使用 componentProtocol 中的 40+ 事件定义
3. handler 只支持手动输入字符串名，无法可视化选择函数/action 或编写内联表达式

## 面板层级顺序

```
属性 (PropsSection)
事件 (EventsSection)      ← 新位置，Class 上面
Class (ClassesSection)
Variant (VariantBindingSection)
样式 (InlineStylePanel)
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
| `EventsSection.tsx` | 完全重写：紧凑行列表 + 弹窗触发 |
| `ContextInspector.tsx` | 移除 `!isBasic` 条件（已做），调整面板顺序（已做） |

---

## 七、待后续

- 自定义事件（component event `emit`）的声明和绑定 — 等基础事件面板完成后再讨论
- SignalSlashPicker 集成到参数输入框 — 未来可加 `/` 触发信号补全
- 事件处理器重复检查 — 同一个事件名只允许绑定一次（通过下拉去重实现）
