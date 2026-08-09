# 批次 1 反馈与模态 设计 spec（2026-08-09-semantic-batch1-feedback-modal-design）

**日期**: 2026-08-09
**状态**: 定稿（详细级）
**上游**: `2026-08-09-semantic-capability-baseline-design.md` §4 批次 1（权威裁决）；车票 `06-stage-scope-feedback-modal.md`（域裁决 + 2026-08-09 翻案注记）；研究 `02-feedback-and-style.md`（平台实证）、`04-design-direction-audit.md`（真人写法审计 + 细化点 1/3）；契约草案 `2026-08-08-semantic-design-coverage-design.md` §4.2
**配套计划路径**: `writing-plans` 生成批次 1 实施计划（本 spec 为计划唯一语法输入源）

---

## 1. 背景与范围

本批次落实基线能力矩阵维度 8「反馈与模态」的进语言项。能力清单与上游裁决引用如下：

| 能力项 | 上游裁决 | 本批次定位 |
|---|---|---|
| modal/dialog（含焦点陷阱/Esc/遮罩点击） | 🔜 进语言（车票 06：双端平台级标准齐全） | 详细语法定稿：`@modal` 元素指令 + @slot 投影 |
| toast/通知 | 🔜 进语言（车票 06：命令式意图三端可对齐） | 详细语法定稿：`showToast` 内建语义函数 |
| 骨架屏 | ⏳ @load 的 loading 形态扩展（不独立成项） | `@skeleton` 元素指令，硬约束于 @load loading 分支 |
| 错误边界 | 🔜 进语言带降级（车票 06 翻案注记 2026-08-09） | `@errorBoundary` 元素指令 + `@slot fallback` 降级槽 |
| tooltip | 🏞 留生态候选 → 属性级指令低优先（批次 5 交互归组评估） | 语言边界记录（§6），不进本批次语法 |
| 空状态/虚拟滚动/懒加载 | 🏞 基线已裁决留生态 | 不重复评估，§6 记录 |

本批次同时完成：semantic-universe 新域 10「反馈模态」登记（ADR-074 登记先行）、vocabulary-registry 新词汇登记、五层门禁验证节、demo 迁移清单。

## 2. 语义意图定义

### 2.1 @modal —— 模态对话框元素指令

**意图一句话**：`<dialog @modal={openSig}>…</dialog>` 声明一个受 open 信号控制的模态对话框——焦点陷阱、Esc 关闭、遮罩点击关闭、无障碍语义四要素由编译器内建，用户零手写（现 Modal.omni 手写 overlay + tabindex + onKeyDown + stopPropagation 的全部逻辑即迁移对象）。

**详细语法形态**：

```omni
<dialog @modal={isOpen} @enter={fadeIn}>
  @slot header {
    <Heading>确认删除</Heading>
  }
  @slot body {
    <Text>此操作不可撤销。</Text>
  }
  @slot footer {
    <Button @onPress={cancel}>取消</Button>
    <Button @onPress={confirm}>删除</Button>
  }
</dialog>
```

规则定稿：

1. **宿主元素**：`@modal` 只能绑定在 `dialog` 元素上；`dialog` 元素必须绑定 `@modal`（双向校验，见 §3 V2/V3）。`dialog` 是模态的专属宿主——裸 `<dialog>` 无模态语义且 Flutter 端无内联 dialog 概念，三端一致要求下裸用即硬错误。`dialog` 元素为新登记元素（§5，ADR-074 登记先行；当前元素解析为通用透传，需在语义层登记识别）。
2. **绑定值**：`@modal={openSig}` 的值必须是用户声明的布尔信号（读引用）。true = 打开，false = 关闭；Esc/遮罩点击关闭即把该信号置 false——**信号是唯一真源**（受控形态）。
3. **内容投影**：`@slot header` / `@slot body` / `@slot footer` 命名槽（迁移 Modal.omni 现形态）；匿名子内容兜底渲染进 body 槽；槽名不限上述三个（按文档序渲染），header/body/footer 为语义提示名。
4. **内建行为**（编译器生成，用户零手写）：
   - 焦点陷阱：打开时焦点移入对话框，Tab/Shift+Tab 循环于对话框内，不逸出
   - Esc 关闭：打开状态下按 Esc → 信号置 false（web 端拦截原生 cancel 事件默认关闭，走受控置位）
   - 遮罩点击关闭：点击遮罩（非内容区）→ 信号置 false。**注意**：原生 `<dialog>` 遮罩点击不自动关闭，本语言语义统一为「遮罩点击关闭」（Modal.omni 现有行为先例 + 三端一致），由编译器实现差异消化
   - 打开动画：复用 `@enter` 指令——条件变 true 时自动重播（docs/omniview/11-motion-animation.md 条件切换重播先例，`<div @enter={fadeIn}>Modal</div>` 文档先例即为此场景）；关闭动画为语言边界（无 @leave 机制，§6）
5. **a11y 编译器生成**：打开态生成 `role="dialog"` + `aria-modal="true"`；`aria-labelledby` 指向 dialog 内第一个 Heading 类元素（编译器自生成 id），无 Heading → 省略并警告（V13）。**用户不需要书写 role**——`role: "dialog"` 不在 a11y role 子集白名单内（validate_a11y.rs FLUTTER_ROLE_SUBSET），编译器生成是唯一正确通道。
6. **组合限制**：`@modal` 不能出现在 `@for` 循环内（每项一模态无意义，用索引信号表达，V5）。

**BNF 产生式草案**：

```
ModalDirective  := '@modal' '=' '{' Ident '}'        ; Ident 为 bool 信号名（解析后校验）
DialogElement   := '<' 'dialog' ModalDirective ViewBody '>'
ViewBody        := (Slot | Element | Text)*          ; 复用既有 view 体
Slot            := '@slot' Ident? '{' ViewBody '}' | '@slot' Ident
Constraint      : DialogElement 必须含 ModalDirective；ModalDirective 仅绑定 DialogElement
```

**冲突检查表**：

| 冲突面 | 检查结论 |
|---|---|
| `@modal` 与既有指令（@if/@for/@slot/@load/@enter/@drag） | 无词汇冲突（新指令名）；组合限制见 V5（@for 内） |
| `dialog` 元素名与既有元素/组件 | 元素透传通道无白名单冲突；`to_html_tag` 无 `dialog` 映射——透传即 `<dialog>`（React 正确），Flutter 端由 @modal 处理器接管 |
| `role: "dialog"` 与 a11y role 子集 | 子集无 dialog——编译器生成通道，用户书写会 warn（不冲突，仅提示） |
| openSig 与 @load 保留名（loading/error/data/retry） | openSig 是用户信号名，若撞 @load 保留名走既有保留名检查 |

### 2.2 showToast —— 内建语义函数（命令式短暂通知）

**意图一句话**：`showToast(msg, {kind, duration})` 是全局可调用的内建语义函数——「告诉用户一条短暂消息」，三端映射到各自平台通知形态（React 自生成轻量 toast / Flutter SnackBar / OmniView 自绘），编译器生成调用点代码，用户不写容器、不写样式、不写定时器。

**详细语法形态**：

```omni
fn save() {
  store.save()
  showToast("保存成功", { kind: "success" })       // 默认 duration 3000
  showToast("网络不可用", { kind: "error", duration: 0 })  // 0 = 不自动关闭
}
```

签名定稿：

```
showToast(msg: string, options?: { kind?: "info" | "success" | "error", duration?: number }): void
```

1. **kind**：`"info"`（默认）/ `"success"` / `"error"`——映射 kind 三色板（§4）；非法值硬错误（V6）。
2. **duration**：默认 **3000ms**（Element Plus ElMessage 先例；Radix 默认 5000 / SnackBar 4000，取中短值更贴「短暂通知」语义）；**0 = 不自动关闭**（ElMessage 先例）；负数硬错误（V7）。
3. **调用点**：事件处理器、任意函数体——与 std 函数同等待遇的内建函数，不在组件声明处使用（无 JSX 形态）。
4. **堆叠**：React/OmniView 同时多条堆叠展示（自生成容器自然支持）；Flutter SnackBar 为队列单条——诚实降级注记（§4，行为差异非功能缺失）。
5. **v0 无手动关闭按钮**（三端一致，YAGNI）；`duration: 0` 在 Flutter 端映射为超长时长 + SnackBarAction「关闭」（降级注记）。
6. **容器样式定案**（细化点 3 配套决策）：默认内建样式（kind 三色板）+ 类名覆盖通道 `.omni-toast` / `.omni-toast--info|success|error`（web 端）；**不新增 design token 语法**——YAGNI，语料驱动后议（未决问题 3）。
7. **保留名**：`showToast` 是内建语义函数名——用户不得以同名声明 fn/store/信号/组件（V8，语义化即切换）。

**BNF 产生式草案**：

```
ShowToastCall  := 'showToast' '(' Expr ','? ToastOptions? ')'
ToastOptions   := '{' ToastKindOpt? ToastDurationOpt? '}'
ToastKindOpt   := 'kind' ':' StringLiteral        ; "info" | "success" | "error"
ToastDurationOpt := 'duration' ':' NumberLiteral   ; >= 0，0 = 不自动关闭
```

**冲突检查表**：

| 冲突面 | 检查结论 |
|---|---|
| `showToast` 与既有 std 函数/词汇 | 新词汇（vocabulary-registry 登记）；与 `@t`/`formatDate` 等无同名冲突 |
| `kind` 选项键名 | 冒号参数风格与既有词汇一致（over: 400ms 先例） |
| 与现有手写通知形态（NotificationStore/NotificationsPanel） | 语义化即切换（ADR-075）：本批次迁移 demo，旧手写形态不作为双轨保留（§8） |

### 2.3 @skeleton —— @load loading 态占位形态

**意图一句话**：骨架屏是加载三态的视觉占位形态，不独立成项——`@skeleton` 指令声明元素为「加载占位」，只能出现在 `@load` 的 loading 分支内，编译器生成占位视觉（web 端带 shimmer 动画 / Flutter 静态占位降级）。

**详细语法形态**：

```omni
@load fetchItems
@if loading {
  <div @skeleton />                      // 默认 text 形状：段落占位
  <div @skeleton="card" />
} @else {
  <div @for={items}>…</div>
}
```

规则定稿：

1. `@skeleton`（布尔）或 `@skeleton="text"`：形状值 `text`（段落条）/ `line`（单条）/ `circle`（圆）/ `card`（卡片块），默认 `text`；非法形状硬错误（V10）。
2. **硬约束**：`@skeleton` 只能出现在 `@load` 的 loading 分支内（`@if loading` 的直系或嵌套子树上）——脱离加载语义的占位无意义（V9，诚实降级 > 静默错误）。
3. **生成**：web 端生成占位 div + 编译器自带 shimmer 动画（keyframes 渐变扫光，零依赖）；Flutter 端生成静态占位（灰块/圆，无 shimmer 依赖）——**诚实降级注记**：动画形态为 web 增强，Flutter 端静态占位（§4）。
4. 形状自适应的复杂度（真实数据行数推断）不进 v0——语料驱动（未决问题 5）。

**BNF 产生式草案**：

```
SkeletonDirective := '@skeleton' | '@skeleton' '=' StringLiteral   ; text|line|circle|card
Constraint        : 所在元素必须位于 @load loading 分支子树
```

### 2.4 @errorBoundary —— 子树渲染失败降级

**意图一句话**：`@errorBoundary` 声明子树渲染失败时的降级 UI（`@slot fallback`），React 端生成官方模式 ErrorBoundary 组件，Flutter 端因无组件级标准而带诚实降级注记（数据级错误已由 `@load` error 信号覆盖）。

**详细语法形态**：

```omni
<div @errorBoundary>
  @slot fallback {
    <Text>该区域加载失败，请重试</Text>
    <Button @onPress={retry}>重试</Button>
  }
  <UserProfile userId={id} />
</div>
```

规则定稿：

1. **捕获范围**：子树渲染抛错（渲染期错误）；不捕获事件处理器、异步代码（React 官方语义，文档注记）。
2. **fallback 槽**：`@slot fallback` 为降级 UI（SlotDefaultDemo 形态）；无 fallback 槽 → 渲染空内容 + 警告（V12，建议提供降级内容）；`fallback` 槽重复声明硬错误（V14）。
3. **v0 不暴露错误对象给 fallback**（React `error` prop 形态后议，未决问题 4）。
4. **Flutter 降级**：组件级无事实标准（研究 02：仅 ErrorWidget.builder 全局兜底）——生成空壳 + 代码注释注记；数据级错误路径 = `@load` error 信号（编译器提示），全局兜底注记见 §6 触发条件。

**BNF 产生式草案**：

```
ErrorBoundaryDirective := '@errorBoundary'
ErrorBoundaryElement   := Element '@errorBoundary' (Slot 'fallback')? ViewBody
Constraint             : fallback 槽至多一个
```

## 3. Validation 清单

错误消息风格对齐 `validate_form_load.rs` 既有中文消息（`"@form"` 引号包裹指令名、`——` 分隔）。

| # | 级别 | 消息草案 | 触发 |
|---|---|---|---|
| V1 | 错误 | `"@modal" 的值必须是布尔信号` | 绑定值非信号/非布尔 |
| V2 | 错误 | `"@modal" 只能用于 dialog 元素` | 非 dialog 标签绑定 @modal |
| V3 | 错误 | `dialog 元素必须绑定 "@modal"` | 裸 `<dialog>`（无模态语义，三端无一致映射） |
| V4 | 错误 | `元素不能重复绑定 "@modal"` | 同一元素两个 @modal |
| V5 | 错误 | `"@modal" 不能用在 @for 循环内` | @for 内出现 @modal |
| V6 | 错误 | `未知 toast 类型 "{kind}"——合法类型: info/success/error` | showToast kind 非法 |
| V7 | 错误 | `"showToast" 的 duration 不能为负数` | duration < 0 |
| V8 | 错误 | `"showToast" 是内建语义函数，不能用作组件声明名` | 用户以 showToast 命名 fn/store/信号/组件 |
| V9 | 错误 | `"@skeleton" 只能用在 @load 的 loading 分支内（@if loading）` | @skeleton 脱离 loading 分支 |
| V10 | 错误 | `未知骨架形状 "{shape}"——合法形状: text/line/circle/card` | @skeleton 形状非法 |
| V11 | 错误 | `元素不能重复绑定 "@errorBoundary"` | 同一元素两个 @errorBoundary |
| V12 | 警告 | `"@errorBoundary" 没有提供 fallback 槽——渲染失败时将显示空内容，建议添加 @slot fallback 降级内容` | 无 fallback 槽 |
| V13 | 警告 | `"@modal" 内容没有标题——生成的 aria-labelledby 将被省略，建议在 dialog 内放置标题元素` | dialog 内无 Heading 类元素 |
| V14 | 错误 | `"fallback" 槽重复声明——降级内容只能有一个` | 两个 fallback 槽 |

## 4. 三目标映射表

### 4.1 @modal

| 目标 | 生成代码形态 | 地道性依据 | 降级注记 |
|---|---|---|---|
| React | 受控 `<dialog>`：`open={isOpen}` + ref 同步 `useLayoutEffect(() => { isOpen ? d.showModal() : d.close() }, [isOpen])` + `onCancel={e => { e.preventDefault(); setIsOpen(false) }}` + 遮罩点击 `onClick={e => { if (e.target === ref.current) setIsOpen(false) }}` + `role="dialog" aria-modal="true" aria-labelledby={titleId}`；内容为三个命名槽投影 | **audit 细化点 1 定案**：纯 `open` prop 渲染非模态 top layer（无遮罩/焦点陷阱/Esc）；受控 open + ref 同步 showModal/close 是 React 团队文档化的真实惯用法（Ryan Florence 形态）；焦点/Esc 由浏览器内建承担（研究 02 关键事实——编译器零手写焦点逻辑） | 无（原生 `<dialog>` 全支持） |
| Flutter | `showDialog(context, barrierDismissible: true, builder: (_) => FocusTraversalGroup(child: Dialog(child: …三个槽投影…)))`；打开/关闭由 openSig 信号驱动的 Navigator push/pop（幂等 guard） | `showDialog` + `FocusTraversalGroup` = 官方地道（audit 04）；焦点范围/Esc（pop）为 Navigator 内建 | 移动端无「Esc」键——系统返回键触发 pop（行为对等注记）；桌面端 Esc 实现形态留未决问题 1 |
| OmniView | 自绘遮罩 + 对话框容器 + tabindex 焦点循环（编译器生成）+ Esc/遮罩点击处理 + role 属性 | 无平台原语，自绘是唯一路径；焦点陷阱为编译器生成（研究 02：三端中唯一需手写焦点逻辑的目标） | 无（自绘全可控） |

三端统一语义（语言契约）：打开态焦点陷阱内建、Esc 关闭、遮罩点击关闭、role/aria-modal 生成。Flutter 遮罩点击 = `barrierDismissible: true`（默认即符合语言语义）。

### 4.2 showToast

| 目标 | 生成代码形态 | 地道性依据 | 降级注记 |
|---|---|---|---|
| React | 根挂载自生成 `ToastProvider`（容器 + 信号列表 + 定时器），调用点生成 `toast({ message, kind, duration })`；容器类 `.omni-toast--{kind}` | **audit 细化点 3 定案**：零依赖自生成——「不引依赖的真人手写路径」是真人写法的合法子集（Sonner/Radix 是引库路线，audit 04 原则：自生成原始写法亦达标） | 无（自生成全可控，含堆叠） |
| Flutter | `ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg), duration: …))`；kind → SnackBar 内容/主题色 | `ScaffoldMessenger.showSnackBar` = 官方地道（audit 04；研究 02：Flutter 无 toast 概念，SnackBar 是平台标准） | **队列单条**（同时只显示一条，后续排队）——堆叠差异注记；`duration: 0` 无等价 → 映射超长时长 + `SnackBarAction("关闭")` |
| OmniView | 自绘 toast 容器（全局挂载）+ 信号列表 + 定时器 | 无平台原语，自绘是唯一路径 | 无（同 React 全可控） |

### 4.3 @skeleton

| 目标 | 生成代码形态 | 地道性依据 | 降级注记 |
|---|---|---|---|
| React | 占位元素 + 编译器生成 shimmer keyframes（渐变扫光动画） | 无官方标准（研究 02：Suspense fallback 仅近似）——自生成占位是零依赖真人路径 | 无 |
| Flutter | 静态占位灰块/圆（Container 灰底） | 无内建（shimmer 是第三方事实标准，pub.dev） | **动画降级**：静态占位（不引 shimmer 依赖）；闪烁动画形态为 web 增强注记 |
| OmniView | 同 React（web 通道 CSS 动画） | 同 React | 无 |

### 4.4 @errorBoundary

| 目标 | 生成代码形态 | 地道性依据 | 降级注记 |
|---|---|---|---|
| React | 自生成 `ErrorBoundary` class（`getDerivedStateFromError` + `componentDidCatch`，fallback 槽 → error UI） | React 官方错误边界模式（研究 02）；react-error-boundary 同形态，零依赖自生成 = 真人原始写法 | 捕获范围注记：不捕获事件处理器/异步错误（官方语义） |
| Flutter | 空壳 + 生成代码注释注记（数据级错误请走 @load error 信号）；全局 `ErrorWidget.builder` 兜底为文档指引 | Flutter 无组件级标准（研究 02）——**诚实降级**（ADR-054），不静默 | 组件级错误边界留生态，触发条件见 §6 |
| OmniView | try 兜底渲染 fallback 槽（ErrorHandlingDemo 先例） | 自绘环境，运行时 catch 渲染错误 | 无 |

## 5. 登记册扩展

### 5.1 semantic-universe 新域 10「反馈模态」（登记先行，ADR-074）

**语义集条目**（4 列表格：条目 | 语义 | 实现状态 | 诊断行为）：

| 条目 | 语义 | 实现状态 | 诊断行为 |
|---|---|---|---|
| showToast | 命令式短暂通知内建函数（kind/duration/堆叠） | 支持（v0：无手动关闭、无位置参数） | kind/duration 非法硬错误（V6/V7）；保留名硬错误（V8） |
| @modal | dialog 元素指令：open 信号控制 + 焦点陷阱/Esc/遮罩点击/a11y 编译器内建 | 支持（v0：无关闭动画） | 值非信号（V1）、非 dialog 标签（V2）、裸 dialog（V3）、重复（V4）、@for 内（V5）硬错误；无标题警告（V13） |
| 焦点陷阱 | @modal 附随行为（不单独立法） | 支持（随 @modal 内建） | 随 @modal 诊断 |
| @skeleton | @load loading 态占位形态（text/line/circle/card） | 支持 | 非 loading 分支硬错误（V9）；形状非法硬错误（V10） |
| @errorBoundary | 子树渲染失败降级（@slot fallback） | 支持（Flutter 降级注记） | 重复硬错误（V11）；无 fallback 警告（V12）；fallback 重复硬错误（V14） |
| tooltip | 属性级修饰（悬停/长按说明） | 留生态（基线 §8 状态值域扩展） | 语言边界注记 + 触发条件（§6） |

**已实现白名单**（2 列表格：条目 | 编译器事实源）：

| 条目 | 编译器事实源 |
|---|---|
| dialog 元素 | 元素解析透传通道（parser/view）+ @modal 语义处理器（本批次新增）；`to_html_tag` 透传 `<dialog>`（React） |
| showToast 调用 | 内建函数表（语义函数分派）+ 三目标 codegen |
| @modal/@skeleton/@errorBoundary | 元素指令语义化（语义层登记 + codegen 分派） |

**verify 锚点方案**：`verify-semantic-universe.py` 新增域 10 断言——条目存在性（showToast/@modal/焦点陷阱/@skeleton/@errorBoundary/tooltip）+ tooltip 状态值域「留生态」断言（基线 §8 状态值域新增值同步脚本）。

### 5.2 vocabulary-registry 新词汇

| 语义词汇 | 来源域 | 目标映射 | 排除词汇 |
|---|---|---|---|
| @modal | 域 10 反馈模态 | React 受控 `<dialog>` + showModal() 同步 / Flutter showDialog / OmniView 自绘 | dialog、showModal、showDialog、Modal、ModalDialog（禁止借词） |
| showToast | 域 10 反馈模态 | React 自生成 toast / Flutter ScaffoldMessenger.showSnackBar / OmniView 自绘 | toast、SnackBar、Toast、Sonner、ElMessage（禁止借词） |
| @skeleton | 域 10（@load 形态） | 占位生成（web shimmer / Flutter 静态） | skeleton、shimmer、Shimmer（禁止借词） |
| @errorBoundary | 域 10 | React ErrorBoundary / Flutter 数据级错误 + 全局兜底注记 / OmniView try 兜底 | ErrorBoundary、error-boundary（禁止借词） |

**判定注记**：`@modal` 绑定的 open 信号为编译器受控信号（唯一真源判定，不注册词汇）；`showToast` 生成的保留名判定同域 6 先例（formErrors/formValid/formSubmitting 为编译器保留名判定，非注册词汇——本批次 V8 属此类）。

## 6. 语言边界与触发条件

| 能力/形态 | 边界裁决 | 触发条件 |
|---|---|---|
| tooltip | 属性级指令低优先（批次 5 交互归组评估）——React 无标准形态、触发模型跨目标差异大（悬停 vs 长按）、语义面窄（车票 06） | 真实语料高频 + React 端形态稳定后重评 |
| 关闭动画 | 语言边界——无 `@leave` 机制，@modal 关闭过渡不可表达（打开动画已由 @enter 复用覆盖） | 动画归组批次引入 @leave 机制 |
| Flutter 组件级错误边界 | 留生态（诚实降级：@errorBoundary 在 Flutter 端生成空壳 + 数据级错误走 @load error 信号 + ErrorWidget.builder 全局兜底文档指引） | Flutter 生态出现组件级事实标准 |
| showToast 手动关闭按钮 | v0 不提供（三端一致；duration: 0 场景 Flutter 端以 SnackBarAction 降级） | 真实语料需求 |
| 空状态/虚拟滚动/懒加载/复杂选择器 | 基线已裁决留生态（本批次不重复评估） | 基线 §3.3 记录 |
| `@modal` 关闭时内容保留 | 受控信号为唯一真源——关闭即信号置 false，内容不卸载（React dialog 语义内建；Flutter pop 销毁） | 无（行为差异以 Flutter 注记呈现） |

## 7. 门禁与验证

五层门禁逐项：

| 门禁 | 验收口径 |
|---|---|
| tsc | verify-react GATE PASSED（零新增豁免——新语法样例全量进入 React 生成校验） |
| dart analyze | ≤ 基线 warning 数（零新增；Flutter 降级注记以代码注释呈现，不产生 warning） |
| Playwright | 既有 46+ 用例全绿 + 本批次新增浏览器行为断言（见下） |
| docs-gate | 248+ blocks 0 failed（learn-omniview 文档随 §8 迁移同步） |
| 注入语料 | 855 元组扩展（见下）+ verify-semantic-universe.py 域 10 断言 + verify-vocabulary.py 新词汇断言 + verify-demo-coverage.py 新 demo 接入 |

**浏览器行为断言清单**（新增，Playwright）：

1. @modal 打开：点击触发按钮 → dialog 出现在 top layer，页面背景不可交互
2. 焦点陷阱：打开后焦点在 dialog 内；Tab 循环不逸出
3. Esc 关闭：按 Esc → dialog 关闭，焦点回触发按钮（React 原生行为）
4. 遮罩点击关闭：点击遮罩 → dialog 关闭（语言语义：遮罩点击关闭）
5. a11y：打开态 dialog 具 role="dialog" + aria-modal="true" + aria-labelledby 指向标题
6. showToast 出现：触发 → toast 文本可见、kind 样式类 `.omni-toast--success` 命中
7. showToast 自动消失：约 3000ms 后消失
8. showToast 堆叠：连续触发多条 → 同时可见多条
9. @skeleton：@load loading 态显示占位（text/card 形状断言），loaded 后替换真实内容
10. @errorBoundary：故障组件子树渲染 → fallback 槽内容可见

**注入语料 855 元组扩展点**（新增解析/校验用例清单）：@modal 合法 5 形态（三槽/无槽/嵌套/@enter 组合/信号开闭）+ 非法 6 形态（V1-V5 + 裸 dialog）；showToast 合法 4（默认 kind/三 kind/duration 0/省略 options）+ 非法 3（V6/V7/V8）；@skeleton 合法 2（布尔/形状）+ 非法 2（V9/V10）；@errorBoundary 合法 2（有/无 fallback 槽）+ 非法 2（V11/V14）+ 警告 1（V12）——合计约 27 元组。

## 8. demo 迁移清单（learn-omniview）

| 用点 | 现形态 | 迁移后 |
|---|---|---|
| `src/components/Modal.omni` | 手写 overlay + tabindex + @onKeyDown Esc + stopPropagation 遮罩点击（开/关函数 + isOpen 信号） | `@modal={isOpen}` + @slot header/body/footer；删除全部手写焦点/Esc/遮罩逻辑（编译器内建）；保留 @enter 打开动画用点 |
| `src/stores/NotificationStore.omni` | 手写通知 store（push/remove/定时器） | **删除**（语义化即切换）——showToast 内建 |
| `src/components/NotificationsPanel.omni` | 手写通知面板（渲染 store 队列） | **删除**（showToast 容器由编译器生成） |
| `src/pages/UiComponents.omni` | 接线 Modal/NotificationsPanel 用点 | showToast 按钮直接调用；Modal 触发沿用 |
| `src/pages/MotionDemo.omni` | @enter 演示 | 增加 @modal + @enter 组合用点（打开动画） |
| `src/components/LoadDemo.omni` | @load 三态 | loading 分支增加 `@skeleton` 用点（骨架屏形态） |
| `src/components/ErrorHandlingDemo.omni` | 数据级错误演示 | 增加 @errorBoundary + @slot fallback 用点（渲染级降级） |
| `src/main.omni` / `design.omni` | 引用旧 Modal/NotificationsPanel | 引用随删除同步更新 |
| docs/omniview 语法文档 | — | 新语法文档随本批次（docs-gate 覆盖） |

## 9. 未决问题

| # | 问题 | 建议方向（不擅自裁决） |
|---|---|---|
| 1 | Flutter 桌面端 Esc 关闭实现形态（平台按键映射 vs 依赖平台 pop 行为） | plan 阶段细化，不阻塞语法定稿 |
| 2 | @skeleton 形状自适应用例（真实数据行数推断占位条数） | v0 不推断；语料驱动 |
| 3 | showToast 位置参数（top/bottom）与关闭按钮是否进 v0 | 默认不进（YAGNI，容器样式已定案为类名覆盖通道）；语料驱动 |
| 4 | 错误边界 fallback 槽是否暴露错误对象（React error prop 形态） | v0 不暴露；后议 |
| 5 | 骨架屏是否放开非 @load 场景（如 @load 嵌套的局部 loading） | 保持 v0 硬约束（V9）；语料驱动后放开 |
| 6 | dialog 内容关闭时是否保留 DOM（React 保留 / Flutter pop 销毁）差异是否需语言级语义统一 | 建议保持目标原生行为（差异已注记），用户拍板 |

---

**自查记录**：占位符清零；节间矛盾清零（受控形态在 §2.1/§4.1/§5.2 一致；@skeleton 硬约束在 §2.3/§3/§4.3 一致）；与基线 spec 无矛盾（细化点 1/3 均在基线「阶段 spec 决策」授权范围内，未推翻上游裁决）；未决问题均不阻塞 plan。
