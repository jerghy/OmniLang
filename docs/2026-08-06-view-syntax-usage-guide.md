# OmniLang View 层六类语法真实用法详解

> **定位**：Poiesis 设计画布缺口设计的前置研究文档（2026-08-06）。
> **数据来源**：`test/projects/learn-omniview/src`（96 个 .omni 真实文件）+ `docs/omniview/`（05-components / 06-templates / 15-grammar-view / 07-events-and-bindings）。
> **覆盖语法**：① @mount/@unmount/@effect ② @ref + ref 声明 ③ @for key={expr} ④ class="x" 属性形式 ⑤ 裸布尔属性 ⑥ prop={expr} 动态属性。
> 每节末尾附「对设计画布的含义」——这些语法目前在 Poiesis 中的支持状态与缺口。

---

## 总览：六类语法使用频率

| # | 语法 | 使用文件数 | 出现次数 | 代表文件 |
|---|------|-----------|---------|---------|
| 1 | @mount / @unmount / @effect | 7+4+6 | 41 处 | AddTodo / ElementRefDemo / UserSettings / Timer |
| 2 | @ref + `ref name: Element` | 2 | 13+ 处 | ElementRefDemo（12）/ NotificationsPanel |
| 3 | @for key={expr} | 6 | 12 处 | ForDirectiveKeyDemo / DataTable / TodoList |
| 4 | class="x" 属性形式 | 1 | 1 处 | simple.omni |
| 5 | 裸布尔属性（required/disabled 等） | 2 | 8 处 | FormPage / FormControlsDemo |
| 6 | prop={expr} 动态属性 | 遍布 30+ | — | AddTodo / FormControlsDemo / main.omni |

---

## 一、@mount / @unmount / @effect —— 生命周期钩子

### 1.1 语法形式

三个钩子都是**组件 body 级声明**（与 signal/fn/view 平级，不在 view 内）：

```omni
component Counter {
    // 挂载后执行一次（初始化数据、恢复持久化状态、注册全局监听）
    @mount {
        pageLoadCount = pageLoadCount + 1;
        ThemeStore.load();
    }

    // 卸载前清理（计时器、监听器；有 cleanups 时 React 推导 deps）
    @unmount {
        console_log("unmounted");
    }

    // 副作用：追踪指定 signal（三种依赖形态）
    @effect(input) { ... }                          // ① 裸 signal 依赖
    @effect (searchText, setFilter) { ... }          // ② 多依赖（store action 可作依赖）
    @effect(notifications, NotificationStore.unreadCount) { ... }  // ③ store 字段依赖
    @effect { ... }                                  // ④ 无参：每次组件更新执行
}
```

### 1.2 语义要点

| 钩子 | 执行时机 | 关键约束 |
|------|---------|---------|
| `@mount` | 组件挂载到 DOM 后**执行一次** | 常见用途：初始化数据、恢复持久化（storage_get）、自动聚焦 |
| `@unmount` | 组件移除前 | 做清理；**不得写入仍有消费者的 signal**（写入在卸载瞬间无意义，且 React 目标 cleanup 会先执行——见 Timer 注释） |
| `@effect(deps)` | 依赖 signal 变化时执行，**在 DOM 更新后** | deps 可为裸名或 `Store.field`；漏声明依赖报 `E_EFFECT_DEPS_MISSING` |
| `@effect`（无参） | 每次组件更新 | 等价每次渲染后执行 |

### 1.3 真实用例全景（文件 × 用法）

| 文件 | 钩子 | 用途 |
|------|------|------|
| `AddTodo.omni:26-35` | @mount + @effect(input) | mount 打日志；effect 在输入超 20 字符时告警 |
| `ElementRefDemo.omni:73-76` | @mount | 置 `shouldFocus = true` → `focus={shouldFocus}` 绑定自动聚焦 |
| `DataTable.omni:83` | @mount | `initRows()` 初始化行数据 |
| `SignalPatternsDemo.omni:100-111` | @mount + @effect + @unmount | 三件套齐演示；无参 @effect 计数 |
| `UserSettings.omni:13-34` | @mount + @effect(username) | mount 从 localStorage 读配置（含 null/空串双守卫）；username 变化自动 `saveSettings()` 持久化 |
| `FilterBar.omni:40-46` | @effect (searchText, setFilter) | searchText 变化即写 TodoStore.setFilter —— **用 effect 替代 @onChange 里读旧值**（React 闭包捕获教训） |
| `NotificationsPanel.omni:19-25` | @effect(notifications, NotificationStore.unreadCount) | 多依赖（本地 signal + store getter）；新通知到达时 `scrollContainer.scrollTo(0, 9999)` 滚到底部 |
| `StatsDashboard.omni:38` | @unmount | 卸载清理 |
| `StdLibShowcase.omni:85` | @mount | `runAsyncDemos()` 运行异步动作 |
| `Timer.omni:54-56` | @unmount | 打最终 count 日志（cleanup 读取 count 进 deps，**不得写入 running**——写入会在每次 tick 提前生效） |
| `TypeSystemDemo.omni:20` | @mount | 初始化 |

### 1.4 完整示例摘录

**@mount + @effect 持久化模式（UserSettings.omni）**——生命周期钩子最典型的真实用途：

```omni
component UserSettings {
    signal username: String = "";
    signal savedText: String = "";

    // @mount：组件挂载时从 storage 读取配置
    @mount {
        let raw = storage_get("user-settings");
        if raw != "" && raw != null {          // 必须同时排除 null 和空串
            let parsed = json_parse(raw);
            if parsed != "" && parsed != null {
                username = parsed;
                savedText = "Settings loaded from storage";
            } else {
                savedText = "No saved settings found";
            }
        } else {
            savedText = "No saved settings found";
        }
    }

    // @effect：信号变化时自动保存
    @effect(username) {
        saveSettings();
    }
    ...
}
```

**@effect 显式依赖 + store 写入（FilterBar.omni）**——依赖列表里有 store action：

```omni
// (searchText, setFilter) 显式依赖：React 目标 useEffect 依赖数组的来源
//（setFilter 为 slice 局部动作，exhaustive-deps 必需）；
// OmniView 目标 __omni_effect 自动跟踪信号读取，依赖列表忽略。
@effect (searchText, setFilter) {
    if searchText == "" {
        TodoStore.setFilter("all");
    } else {
        TodoStore.setFilter(searchText);
    }
}
```

**@effect 多依赖（本地 signal + store getter，NotificationsPanel.omni:19-25）**：

```omni
// deps 显式列出 effect 读取的 store 字段（含 getter），字段变化时重跑
@effect(notifications, NotificationStore.unreadCount) {
    if NotificationStore.unreadCount > 0 {
        badgeVisible = true;
        scrollContainer.scrollTo(0, 9999);   // RefOp: .scrollTo
    }
}
```

### 1.5 实战注意事项（来自项目注释，非文档）

1. **React 闭包捕获旧值**：`@onChange` 内读 signal 会捕获旧值 → 过滤逻辑移交 @effect（FilterBar 注释）。
2. **unmount 写入禁令**：卸载时组件已不可见，写入无消费者的 signal 是死写入（SignalPatternsDemo.omni:108-111 注释「7.5b-L5：死写入已移除」）。
3. **deps 泄漏**：cleanup 读取 count 会进 deps → 每次 count 变化先执行上一次 cleanup 再重注册（Timer 注释）。

### 1.6 对设计画布的含义

- **零 UI + 打开即丢**：Poiesis 解析器把 @mount/@unmount/@effect 当「未知指令」吞块（`parser-view.ts:623-633`），没有数据建模；组件 body 解析（`parser.ts`）也不认识它们 → **learn-omniview 全项目 41 处钩子，画布打开→保存全部消失**。
- 设计时需回答：钩子体是「代码块节点」（复用 CodeFlow/CodeCard 编辑器）还是文本？deps 列表如何编辑？三个钩子的 body 语法 = fn 体语法（可复用 code-op 体系）。

---

## 二、@ref + `ref name: Element` 声明 —— 元素引用系统

### 2.1 语法形式

```omni
component ElementRefDemo {
    // ── ref 声明（组件 body 级，类型 Element 或 HtmlElement）──
    ref inputBox: Element;
    ref scrollContainer: HtmlElement;

    view {
        <!-- 方式 A：冒号简写（原语） -->
        <input ref: myInput bind:value={text} />

        <!-- 方式 B：@ref={name} 事件风格（推荐，与 @onClick 一致性高） -->
        <div @ref={scrollContainer}>...</div>
    }
}
```

两种绑定语法**等价**，官方推荐 `@ref={name}`。

### 2.2 Ref API 表面（跨后端安全子集，ADR-052/053）

| API | 用途 | 真实示例（ElementRefDemo） |
|-----|------|---------------------------|
| `.focus()` / `.blur()` | 聚焦/失焦 | `inputBox.focus()` / `inputBox.blur()` |
| `.scrollTo(x, y)` / `.scrollBy(dx, dy)` | 定点/增量滚动 | `scrollArea.scrollTo(0, 200)`、`scrollArea.scrollBy(0, 100)` |
| `.select()` | 全选文本 | `selectInput.select()` |
| `.setSelection(start, end)` | 选区控制 | `rangeInput.setSelection(0, 3)` |
| `.measure()` | 位置/尺寸查询（返回 rect） | `let rect = rectBox.measure()`；`.measure().width` |
| `.capturePointer()` / `.releasePointer()` | 指针捕获（**无参语义**，有参编译错误） | `pointerBox.capturePointer()` |
| `.value`（只读） | 读输入值 | `let val = mirrorInput.value` |

**不暴露**：`innerHTML`、`addEventListener`、`querySelector`、`parentElement`、`ref.style.*`——破坏跨后端抽象（已删除）。

### 2.3 语义绑定（配合 @ref 使用的元素属性）

| 绑定 | 值语义 | 编译行为 |
|------|--------|----------|
| `focus={sig}` | bool | `el.focus()` / `el.blur()` |
| `scroll-y={sig}` | 数字 | `el.scrollTop = sig` |
| `scroll-x={sig}` | 数字 | `el.scrollLeft = sig` |

约束：值必须是**裸 signal**（或 `Store.sig`）；元素必须同时带 `@ref={name}`，否则语义检查报错（E0006）；非裸 signal 表达式回退为普通属性。

### 2.4 真实用例全景（ElementRefDemo.omni，399 行全主题文件）

| ref | 绑定位置 | 用途 |
|-----|---------|------|
| `inputBox` | `<input @ref={inputBox}>` | .focus()/.blur() 焦点管理 |
| `selectInput` | `<input @ref={selectInput} bind:value>` | .select() 全选 |
| `rangeInput` | `<input @ref={rangeInput} @onSelect>` | .setSelection() 选区 + SelectEvent(start/end/text) 回显 |
| `focusSigInput` | `<input @ref={focusSigInput} focus={shouldFocus}>` | 语义绑定焦点（@mount 置 true 自动聚焦） |
| `scrollArea` | `<div scroll-y={scrollTarget} @ref={scrollArea}>` | 语义滚动绑定 + .scrollBy/.scrollTo 命令式互补 |
| `rectBox` | `<div @ref={rectBox}>` | .measure() 位置查询 |
| `sizeBox` | `<div @ref={sizeBox} style: {width, height}>` | .measure() 尺寸查询 + 信号驱动样式绑定 |
| `mirrorInput` | `<input @ref={mirrorInput}>` | .value 读值 → 镜像文本 |
| `pointerBox` | `<div @ref={pointerBox} @onPointerDown/@onPointerUp>` | .capturePointer/.releasePointer 指针捕获 |

**高频组合模式**：`@ref` + `@drag-horizontal` 触发器（ElementRefDemo:370-388）——拖拽由触发器 + @motion 声明式驱动，最终值用 signal 保存；`ref.style.*` 已删除，改用 `style: { width: sig }` 绑定。

### 2.5 边界情况（文档明示）

- **@for 内 @ref**：绑定到循环的**最后一个元素**（Phase 2 解决语义）——已知未定义行为。
- **自定义组件 ref**：`<MyComponent @ref={x}>` 不支持（需 React forwardRef 等价物）。
- **类型两种**：`ref x: Element`（通用）与 `ref x: HtmlElement`（NotificationsPanel 用法，语义等价可互换）。

### 2.6 对设计画布的含义

- **零 UI + 打开即丢**：`ref` 声明不在组件 body 解析表（parser.ts 只认 signal/computed/prop/event/fn），`@ref={x}` 作为元素属性未被 parser-view 识别 → 保存即丢。
- 设计时需回答：ref 声明放哪（ComponentDefPanel 定义 tab？）；`@ref` 作为元素属性如何编辑；`.focus()` 等 RefOp 调用在 fn 体/CodeFlow 里是普通 call（已可表达）；`focus={sig}`/`scroll-y={sig}` 语义绑定是**特殊属性**，需要独立 UI 或属性白名单扩展。

---

## 三、@for key={expr} —— 列表身份追踪

### 3.1 语法形式

```omni
@for item in items key={item.id} {
    <li class: list-item>#{item.id} — {item.name}</li>
}
@empty {
    <li class: list-item empty>No items</li>
}

<!-- 完整形态（含可选 [scroll] 提示，仅 ScrollStoreDemo 1 处） -->
@for item in ScrollStore.items [scroll] { ... }
```

### 3.2 语义要点

- `key={...}` 提供**稳定的 DOM 身份标识**，支持增量更新和 FLIP 动画（React key / Flutter ValueKey / OmniView keyed 克隆复用）。
- 增删/重排时**保留列表项状态**（输入框内容、滚动位置、动画状态等）。
- **对象项建议提供** key=；标量项可直接用项本身（`key={item}`）。
- `@empty` 是 @for 的可选空列表后缀。

### 3.3 真实用例全景

| 文件 | 行 | key 形态 | 列表内容 |
|------|----|---------|---------|
| `ForDirectiveKeyDemo.omni:54` | `key={item.id}` | 对象字段（`Array<Fruit>`，struct 含 id）—— 主题演示文件，配 Add/Remove/Move-to-Front 三操作 |
| `DataTable.omni:113` | `key={row.id}` | 对象字段（表格行） |
| `TodoList.omni:40` | `key={item.id}` | 对象字段（可见项列表） |
| `SignalPatternsDemo.omni:166` | `key={item}` | **标量项自身**（`Array<String>`）——key 不限于字段 |
| `ScrollStoreDemo.omni` | `[scroll]` | `@for item in ScrollStore.items [scroll]` ——滚动列表提示（全项目唯一） |

补充：TodoStore.omni:14 注释「自增 id——删除后 id 不复用，保证 @for key={item.id} 唯一」——**key 唯一性由数据层纪律保证**。

### 3.4 完整示例（ForDirectiveKeyDemo.omni 核心段）

```omni
struct Fruit { id: i32, name: String }

component ForDirectiveKeyDemo {
    signal items: Array<Fruit> = [
        Fruit { id: 1, name: "Apple" },
        Fruit { id: 2, name: "Banana" },
        Fruit { id: 3, name: "Cherry" },
    ];
    signal nextId: i32 = 4;

    fn addItem() {
        items = [...items, Fruit { id: nextId, name: "Fruit-" + nextId.to_string() }];
        nextId = nextId + 1;
    }
    fn removeLast() { ... }
    fn moveLastToFront() { ... }   // 重排操作——正是 key 价值所在

    view {
        <ul class: List>
            @for item in items key={item.id} {
                <li class: list-item>#{item.id} — {item.name}</li>
            }
            @empty {
                <li class: list-item empty>No items</li>
            }
        </ul>
    }
}
```

### 3.5 对设计画布的含义

- **编辑器现状**：`keyExpr` **已解析**（`ForBlockData.keyExpr`，parser-view.ts:838-859，2026-08-06 随 @for 支持新增），ForBlockEditor 有 `keyExpr` 输入框（ForBlockEditor.tsx）——**但发射端不输出 key**（source-emitter.ts:537-554）→ 保存后 key 消失。解析↔发射不对称（已实现的缺口，改动小）。
- 设计时需回答：key 表达式用什么编辑器（纯文本输入已够用？）；`[scroll]` 提示是否建模。

---

## 四、class="x" 属性形式

### 4.1 语法形式

```omni
<div class="app">   <!-- 属性形式（EQUALS） -->
<div class: "bg-blue-100 border rounded-lg">   <!-- 冒号字符串形式 -->
<div class: LearnDesign.Card>                  <!-- 设计类引用 -->
```

### 4.2 语义

`class="x"` 是**普通 HTML 属性形式的类名绑定**，与 `class: "x"` 冒号形式语义等价（空白分隔静态类名）。类名限 `[a-zA-Z0-9_-]`（E0006）。

### 4.3 真实使用情况（重要）

**全项目仅 1 处**：`simple.omni:7` `<div class="app">`（最原始的入门文件）。其余 68 个使用类名的文件全部采用 `class:` 冒号形式（1383 处）。

```omni
// simple.omni —— 唯一使用属性形式的文件
component SimpleApp {
  signal count: i32 = 0;
  fn increment() { count = count + 1; }
  view {
    <div class="app">
      <h1>Hello</h1>
      <button @onPress={increment}>Click: {count}</button>
    </div>
  }
  style {
    .app { padding: 20px; }
  }
}
```

### 4.4 对设计画布的含义

- **低频但致命**：Poiesis 解析器对 `attrName === "class"` 的 EQUALS 属性**静默跳过**（parser-view.ts:378-390）→ 打开 simple.omni 保存后 `<div class="app">` 变裸 `<div>`，样式全丢。
- 设计时：**必须支持**（等价 `class:` 字符串形式，建模为 classRefs 即可），优先级低于其他语法但成本极低——属于「解析时改写为 class: 形式」或「补建模」二选一。

---

## 五、裸布尔属性 required/disabled/readonly 等

### 5.1 语法形式（15 个布尔关键字属性）

```omni
<!-- 裸形式：属性存在即 true -->
<input disabled readonly required>

<!-- 动态形式：{expr} 布尔表达式 -->
<input type="checkbox" checked={agreed} />
<button disabled={isLoading}>Submit</button>
<input readonly={isReadonly} required={isRequired} />

<!-- 双向形式 -->
<input type="checkbox" bind:checked={isChecked} />
```

15 个布尔属性全集：`disabled / checked / readonly / required / selected / autofocus / loop / muted / hidden / autocomplete / autoplay / multiple / open / …`（文档列 15 个关键字，含 form 语义）。

### 5.2 语义要点

- 裸形式 = 常量 true；`{expr}` 形式 = 响应式布尔（值可为**任意表达式**，不限于信号——`disabled={false}` 是合法常量表达式）。
- `checked` 有三种表面：裸/动态 `checked={agreed}`（单向）/ `bind:checked={isChecked}`（双向，ViewAttributesDemo 唯一用例）。
- 已有 `E_VALUE_BINDING_REMOVED` 诊断：`value={sig}` 裸信号形式已删除，但**布尔属性无此限制**——`disabled={sig}` 合法（单项语义无歧义）。

### 5.3 真实用例全景

| 文件 | 行 | 形式 | 用途 |
|------|----|------|------|
| `FormPage.omni:131` | `disabled`（裸） | 演示禁用输入框 |
| `FormPage.omni:144` | `readonly`（裸） | 演示只读输入框（配合静态 `value="Read-only value"`） |
| `FormPage.omni:180` | `disabled={false}` | Submit 按钮（常量表达式） |
| `FormPage.omni:198` | `disabled={false}` | 条件提交按钮（演示用常量，真实场景为 `disabled={!isFormValid()}` 形态） |
| `FormControlsDemo.omni:67` | `checked={agreed}`（动态） | checkbox 单向受控 |
| `ViewAttributesDemo.omni:28` | `bind:checked={isChecked}`（双向） | checkbox 双向绑定 |
| `FormControlsDemo.omni:56-58` | `<option value="option1">` | select 的 value 是**普通静态属性**（非布尔） |

### 5.4 对设计画布的含义

- **裸形式解析即丢**（parser-view.ts:343-448：非 EQUALS/COLON 即 break）；`{expr}` 形式花括号丢失（同第六类问题）→ 表单类组件保存后语义退化。
- 设计时：裸布尔属性需补建模（`disabled` → `props.disabled = true` 或专门布尔字段）；`checked={expr}` 需保真表达式。

---

## 六、prop={expr} 动态属性

### 6.1 语法形式

```omni
<!-- 组件实例属性 -->
<Button label={buttonText} disabled={isLoading} />
<UserCard name={user.name} role="admin" />

<!-- 元素属性 -->
<input value={inputText.to_string()} placeholder={hintText} />
<button disabled={isLoading}>Submit</button>
<a href={dynamicUrl}>Link</a>
```

### 6.2 语义要点

- `prop={expr}` 是**普通单向 prop**：显示表达式计算结果，无任何魔法（值绑定收敛 D2）。
- **`value={sig}` 裸 signal 形式已删除**——写它会编译报错「用 bind:value 双向绑定，或普通 value prop 单向」（`E_VALUE_BINDING_REMOVED`）。动态属性里的表达式**可以是任意表达式**：`value={inputText.to_string()}`（方法调用）、`placeholder={hint}`（computed）、`disabled={isLoading}`（signal）、`checked={agreed}`（signal）。
- 与事件处理器 `@onPress={fn}` 区分：属性是值表达式，事件是函数引用/调用。

### 6.3 真实用例全景（遍布 30+ 文件，抽样）

| 文件 | 行 | 属性 | 表达式形态 |
|------|----|------|-----------|
| `main.omni:47` | `activeTab={activeTab}` | signal 直传 |
| `AddTodo.omni:50` | `placeholder={hint}` | computed 引用 |
| `DataFetching.omni:67-68` | `bind:value={url}`、`class: UrlInput` | 双向 + 静态混排 |
| `FormControlsDemo.omni:67` | `checked={agreed}` | signal 布尔 |
| `FormPage.omni:180` | `disabled={false}` | 常量表达式 |
| `ElementRefDemo.omni:315` | `style: { width: boxWidthPx, height: boxHeightPx }` | 样式对象内插值 |
| `EasingFunctionsDemo.omni` 等 | `@enter={easeInMove}` | 触发器（同类花括号语法，motion 专题） |

### 6.4 对设计画布的含义（全缺口报告最普遍的一处）

- **花括号丢失（动态→静态退化）**：parser-view.ts:371-381 属性插值只认**单 IDENTIFIER**，且丢弃 `{}`——`value={count}` 保存后变 `value="count"`；非标识符表达式（`{inputText.to_string()}`）连解析都失败。
- 这是使用面**最广**的缺口（任何带动态属性的文件都会踩）：设计时需回答——属性值表达式如何建模（`{expr}` 文本字段？与静态字符串区分标记）；表达式编辑器复用 ExpressionEditor（已有 Monaco omnilang 组件）。

---

## 附：六类语法与 Poiesis 支持状态对照

| 语法 | 解析 | 发射 | UI | 打开→保存后果 |
|------|------|------|----|-------------|
| @mount/@unmount/@effect | ❌ 吞块 | ❌ | ❌ | **钩子整体消失**（41 处） |
| ref 声明 + @ref | ❌ | ❌ | ❌ | **ref 与绑定消失**（13+ 处） |
| @for key={expr} | ✅ | ❌ | ✅（输入框） | **key 表达式消失**（12 处） |
| class="x" | ❌ 静默丢弃 | ✅（class: 形式） | ❌ | **类名消失**（1 处） |
| 裸布尔属性 | ❌ 静默丢弃 | ✅（BOOLEAN_ATTRS） | ❌ | **属性消失**（FormPage 等） |
| prop={expr} | ⚠️ 单标识符且丢花括号 | ✅（静态形） | ❌ | **动态退化为静态**（遍布） |
