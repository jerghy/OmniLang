# 17 — 反馈与模态

本页覆盖**功能设计级语义域 10「反馈模态」**（spec 2026-08-09-semantic-batch1-feedback-modal-design §2-§4 实施收口）：**模态对话框（`@modal`）**、**命令式通知（`showToast`）**、**加载占位（`@skeleton`）**与**渲染降级（`@errorBoundary`）**。四者均把「平台级手工细节」内建进编译器——焦点陷阱、Esc/遮罩点击、a11y、容器、定时器、降级兜底全部零手写，源码只表达意图，选路是 codegen 职责。

> **语义域登记**：四词汇已登记 vocabulary-registry（`@modal`/`showToast`/`@skeleton`/`@errorBoundary`，来源域 10）；`showToast` 是**内建语义函数保留名**（用户不得以同名声明 fn/store/信号/组件——V8 硬错误）；`dialog` 是模态**专属宿主**元素（裸用硬错误 V3）；`@skeleton` 硬约束于 `@load` loading 分支（V9）；`tooltip` 为**留生态**候选（不进入本批语法与白名单，见语言边界）。

## 模态对话框（@modal）

意图：「`<dialog @modal={openSig}>…</dialog>` 声明一个受 open 信号控制的模态对话框——焦点陷阱、Esc 关闭、遮罩点击关闭、无障碍语义四要素由编译器内建，用户零手写」。

```omni
component App {
  @motion fadeIn {
    from { opacity: 0, translateY: 20 }
    to { opacity: 1, translateY: 0 }
    over: 400ms
    easing: ease-out
  }
  signal isOpen: bool = false
  fn close() { isOpen = false }
  view {
    <dialog @modal={isOpen} @enter={fadeIn}>
      @slot header { <h1>确认删除</h1> }
      @slot body { <p>此操作不可撤销。</p> }
      @slot footer { <button @onPress={close}>取消</button> }
    </dialog>
  }
}
```

编译器生成：受控开闭（open 信号为**唯一真源**——true 打开 / false 关闭，Esc 与遮罩点击均写回该信号）、焦点陷阱（打开时焦点移入对话框，Tab/Shift+Tab 循环不逸出）、Esc 关闭（web 端拦截原生 cancel 事件默认关闭，走受控置位）、遮罩点击关闭（点击非内容区 → 信号置 false；**语言语义统一为「遮罩点击关闭」**，原生 `<dialog>` 不自动关闭的差异由编译器消化）、a11y（`role="dialog"` + `aria-modal="true"` + `aria-labelledby` 指向 dialog 内第一个标题元素）、`@enter` 打开动画（条件变 true 时自动重播）。

约束（违规 = 编译期硬错误，除非注明警告）：

- **宿主元素**：`@modal` 只能绑定在 `dialog` 元素上（`"@modal" 只能用于 dialog 元素`）；`dialog` 元素必须绑定 `@modal`（`dialog 元素必须绑定 "@modal"`——裸 `<dialog>` 无模态语义且 Flutter 端无内联 dialog 概念，三端一致要求下即硬错误）。
- **绑定值**：必须是**用户声明的布尔信号**（读引用）——`"@modal" 的值必须是布尔信号`。
- **重复绑定**：同一元素两个 `@modal` → `元素不能重复绑定 "@modal"`。
- **组合限制**：`@modal` 不能出现在 `@for` 循环内（每项一模态无意义，用索引信号表达）——`"@modal" 不能用在 @for 循环内`。
- **内容投影**：`@slot header`/`@slot body`/`@slot footer` 命名槽（槽名不限，按文档序渲染；匿名子内容兜底渲染进 body 槽）。
- **标题警告（V13，警告级）**：dialog 内无标题元素 → `"@modal" 内容没有标题——生成的 aria-labelledby 将被省略，建议在 dialog 内放置标题元素`。`role: "dialog"` 不在 a11y role 子集白名单内（validate_a11y 子集）——用户**不需要书写 role**，编译器生成是唯一正确通道。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 受控 `<dialog>`：`open={isOpen}` + ref 同步 `useLayoutEffect`（`showModal`/`close`）+ `onCancel`/遮罩点击 → 信号置 false；`role="dialog" aria-modal="true" aria-labelledby={titleId}`；三命名槽投影 |
| Flutter | `showDialog(barrierDismissible: true, builder: FocusTraversalGroup(Dialog(…槽投影)))`；open 信号驱动 Navigator push/pop（幂等 guard）；移动端系统返回键即 Esc（行为对等注记） |
| OmniView | 自绘遮罩 + 对话框容器 + 编译器生成焦点循环/Esc/遮罩点击处理/role（三端中唯一需编译器手写焦点逻辑的目标） |

## 命令式通知（showToast）

意图：「`showToast(msg, {kind, duration})` 是全局可调用的内建语义函数——告诉用户一条短暂消息，三端映射到各自平台通知形态，编译器生成调用点代码，用户不写容器、不写样式、不写定时器」。

```omni
component App {
  fn save() {
    showToast("保存成功", { kind: "success" })       // 默认 duration 3000
    showToast("网络不可用", { kind: "error", duration: 0 })  // 0 = 不自动关闭
  }
  view {
    <button @onPress={save}>保存</button>
  }
}
```

签名：`showToast(msg: string, options?: { kind?: "info" | "success" | "error", duration?: number }): void`。

编译器生成：调用点代码 + 平台通知形态（React/OmniView 自生成容器 + 信号列表 + 定时器，多条**堆叠**展示；Flutter `ScaffoldMessenger` SnackBar，队列单条）；kind 映射三色板（web 类名 `.omni-toast--info|success|error`，默认内建样式 + `.omni-toast` 类名覆盖通道）；duration 默认 3000ms，`0` = 不自动关闭。

约束（违规 = 编译期硬错误）：

- **kind 闭集**：`"info"`（默认）/ `"success"` / `"error"`——非法值 → `未知 toast 类型 "{kind}"——合法类型: info/success/error`。
- **duration 非负**：负数 → `"showToast" 的 duration 不能为负数`。
- **保留名（V8）**：`showToast` 是内建语义函数名——`"showToast" 是内建语义函数，不能用作组件声明名`（fn/store/信号/组件同名均禁）。
- **调用点**：事件处理器、任意函数体——与 std 函数同等待遇，无 JSX 形态；**v0 无手动关闭按钮**（三端一致；`duration: 0` 在 Flutter 端映射为超长时长 + `SnackBarAction("关闭")` 降级注记）。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 根挂载自生成 `ToastProvider`（容器 + 信号列表 + 定时器）；调用点生成 `toast({message, kind, duration})`；容器类 `.omni-toast--{kind}` |
| Flutter | `ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg), duration: …))`；kind → 内容/主题色；**队列单条**（堆叠差异注记） |
| OmniView | 自绘 toast 容器（全局挂载）+ 信号列表 + 定时器 |

## 加载占位（@skeleton）

意图：「骨架屏是加载三态的视觉占位形态，不独立成项——`@skeleton` 指令声明元素为「加载占位」，只能出现在 `@load` 的 loading 分支内，编译器生成占位视觉」。

```omni
component App {
  fn fetchItems() -> Array<Int> {}
  view {
    @load={fetchItems}
    @if loading {
      <div @skeleton />          // 默认 text 形状：段落占位
      <div @skeleton="card" />
    }
    @for item in data {
      <div>{item}</div>
    }
  }
}
```

编译器生成：占位元素（形状值 `text`（段落条，默认）/ `line`（单条）/ `circle`（圆）/ `card`（卡片块））；web 端自带 shimmer 动画（keyframes 渐变扫光，零依赖）；Flutter 端生成**静态占位**灰块/圆（诚实降级注记：动画形态为 web 增强，不引 shimmer 依赖）。

约束（违规 = 编译期硬错误）：

- **硬约束（V9）**：`@skeleton` 只能出现在 `@load` 的 loading 分支内（`@if loading` 的直系或嵌套子树上）——脱离加载语义的占位无意义：`"@skeleton" 只能用在 @load 的 loading 分支内（@if loading）`。
- **形状闭集（V10）**：非法形状 → `未知骨架形状 "{shape}"——合法形状: text/line/circle/card`。
- **v0 边界**：形状自适应（真实数据行数推断占位条数）不进 v0——语料驱动。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 占位元素 + 编译器生成 shimmer keyframes（渐变扫光动画，零依赖） |
| Flutter | 静态占位灰块/圆（`Container` 灰底）——**动画降级注记**（shimmer 为第三方事实标准，不引入依赖） |
| OmniView | 同 React（web 通道 CSS 动画） |

## 渲染降级（@errorBoundary）

意图：「`@errorBoundary` 声明子树渲染失败时的降级 UI（`@slot fallback`），React 端生成官方模式 ErrorBoundary 组件，Flutter 端因无组件级标准而带诚实降级注记（数据级错误已由 `@load` error 信号覆盖）」。

```omni
component App {
  fn retry() {}
  view {
    <div @errorBoundary>
      @slot fallback {
        <p>加载失败</p>
        <button @onPress={retry}>重试</button>
      }
      <div>正常内容</div>
    </div>
  }
}
```

编译器生成：React 自生成 `ErrorBoundary` class（`getDerivedStateFromError` + `componentDidCatch`，fallback 槽 → 降级 UI）；OmniView try 兜底渲染 fallback 槽（ErrorHandlingDemo 先例）；Flutter 端生成**空壳 + 代码注释注记**（数据级错误请走 `@load` error 信号；全局 `ErrorWidget.builder` 兜底为文档指引）——诚实降级（ADR-054），不静默。

约束（违规 = 编译期硬错误，除非注明警告）：

- **捕获范围**：子树**渲染期**错误；不捕获事件处理器、异步代码（React 官方语义）。
- **fallback 槽（V12，警告级）**：无 fallback 槽 → 渲染空内容 + `"@errorBoundary" 没有提供 fallback 槽——渲染失败时将显示空内容，建议添加 @slot fallback 降级内容`（警告不翻转编译 success）。
- **重复声明（V14）**：两个 fallback 槽 → `"fallback" 槽重复声明——降级内容只能有一个`；同一元素两个 `@errorBoundary` → `元素不能重复绑定 "@errorBoundary"`。
- **v0 不暴露错误对象**给 fallback（React `error` prop 形态后议）。

### 三目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 自生成 `ErrorBoundary` class（官方错误边界模式，零依赖）；fallback 槽 → error UI |
| Flutter | 空壳 + 生成代码注释注记（数据级错误走 `@load` error 信号；`ErrorWidget.builder` 全局兜底为文档指引） |
| OmniView | try 兜底渲染 fallback 槽（运行时 catch 渲染错误） |

## 语言边界（v0 defer）

- **tooltip**：属性级指令低优先（批次 5 交互归组评估）——React 无标准形态、触发模型跨目标差异大（悬停 vs 长按）、语义面窄；触发条件：真实语料高频 + React 端形态稳定后重评。
- **关闭动画**：无 `@leave` 机制，`@modal` 关闭过渡不可表达（打开动画已由 `@enter` 复用覆盖）；触发条件：动画归组批次引入 `@leave`。
- **Flutter 组件级错误边界**：留生态——`@errorBoundary` 在 Flutter 端生成空壳 + 数据级错误走 `@load` error 信号 + `ErrorWidget.builder` 全局兜底文档指引；触发条件：Flutter 生态出现组件级事实标准。
- **showToast 手动关闭按钮**：v0 不提供（三端一致；`duration: 0` 场景 Flutter 端以 `SnackBarAction` 降级）；触发条件：真实语料需求。
- **`@modal` 关闭时内容保留**：受控信号为唯一真源——关闭即信号置 false，内容不卸载（React dialog 语义内建；Flutter pop 销毁）——行为差异以 Flutter 注记呈现。
- **空状态/虚拟滚动/懒加载/复杂选择器**：基线已裁决留生态，不重复评估。
