# 词汇登记册（Vocabulary Registry）

**日期**: 2026-08-05
**状态**: Active —— 设计 P1 复发机制（v3 修复计划 Phase 7，Task 7.1）
**依据**: 设计文档 §2 原则 6（词汇语义域封闭）+ §5 每域模板「词汇来源」字段；4.2 事件白名单（`event_names.rs` 共享表）/ 4.3 a11y 词汇（`validate_a11y.rs`）/ 19 属性白名单 / RefOp 10 ops / design token 13 类型
**审计**: `scripts/verify-vocabulary.py`（登记册 ↔ 编译器白名单交叉断言，docs-gate 同批）

> 本登记册强制回答一个问题：**每个语义词汇的语义域是什么**（WAI-ARIA / DOM 事件 / CSS 属性 / …）。
> 5 个封闭词汇域：事件 / a11y role / motion 属性 / RefOp / design token 类型。
> **动因（设计 P1）**：a11y 域曾以 Flutter 旗标名（`textField`/`image`）为 role 词汇——出生即违反「语义纯度」原则（web 输出非法 `role="textField"`）；4.3 已修正词汇（Task 4.3 词汇 ARIA 化），本登记册使复发机制化：新域设计时「禁止借词」是硬规则，不是好习惯。

## 登记册结构（条目必含四字段）

| 字段 | 必填 | 说明 |
|------|------|------|
| 语义词汇 | 是 | 该域的封闭语义词汇表（第一列 = 词汇本体） |
| 来源域 | 是 | 词汇的权威语义域（WAI-ARIA / DOM 事件 / CSS 属性 / …） |
| 目标映射 | 是 | React / Flutter / OmniView 三目标生成形状 |
| 排除词汇 | 是 | 平台旗标名——不得成为语义词汇的名字（借词黑名单） |

审计脚本 `scripts/verify-vocabulary.py` 强制四字段存在，并与编译器白名单双向交叉断言：
新增词汇若不进登记册（或登记册词汇不在白名单）即门禁失败；排除词汇与语义词汇重叠（借词）即门禁失败。

## 禁止借词规则（先查登记册，再定词汇）

- **Flutter 旗标名**（`SemanticsFlag`：`textField`/`image`/`button`/…）与 **React 合成事件名**（`onClick`/`onKeyDown`/…）**不得直接成为语义词汇**——它们是平台的机制形状，不是前端意图。
- 新域设计流程：打开本登记册 → 词汇已在册 → 用登记册语义名；词汇不在册 → 若候选词是平台旗标名/合成事件名 → 借词违规，改回语义名（或按 §5 模板「词汇来源」字段论证语义域后扩册）。
- 合法示例（编译器即事实——docs-gate 校验本块可编译）：

```omni
<button a11y: { label: "Close", role: "button" }>×</button>
```

- 非法示例（只可写进本段说明，不得作为代码块教学——docs-gate 会拦截）：

  `a11y: { role: "textField" }`（Flutter 旗标名借词——warning + 建议 `textbox`）；`<div aria-label="Close">`（原生 aria-* 已删除——硬诊断 + 迁移提示）。

---

## 域 1: 事件（Events）

### 来源域

W3C UI Events / Pointer Events / Touch Events / CSS Animation & Transition Events——**用户与元素交互的时机与类型化数据**（设计文档 域 7；4.2 事件白名单 = `crates/omnilang-core/src/codegen/shared/event_names.rs` 共享 canonical 表）。DSL 表面拼写是 `@onXxx={...}`（或裸 `onXxx={...}`），内部 canonical 名即本表词汇——`onXxx` 拼写是平台形状（React 合成事件惯例），语义词汇是事件名本体。

### 语义词汇

| 语义词汇（canonical） | 家族 | 别名拼写 | 说明 |
|----------------------|------|---------|------|
| click | click | press / tap | 单击 |
| dblclick | click | doubleclick | 双击 |
| longpress | click | — | 长按（Flutter-only 便捷事件，无 DOM 对应） |
| hover | click | — | 悬停 |
| change | 表单 | — | 值变化提交 |
| input | 表单 | — | 输入中 |
| toggle | 表单 | — | 切换（checkbox 等） |
| submit | 表单 | — | 表单提交 |
| focus | 表单 | — | 获得焦点 |
| blur | 表单 | — | 失去焦点 |
| select | 表单 | — | 文本选择 |
| resize | 表单 | — | 尺寸变化 |
| keydown | 键盘 | — | 键按下 |
| keyup | 键盘 | — | 键抬起 |
| keypress | 键盘 | — | 键按下（字符输入） |
| mouseenter | 鼠标 | — | 指针进入（不冒泡） |
| mouseleave | 鼠标 | — | 指针离开（不冒泡） |
| mousedown | 鼠标 | — | 按下 |
| mouseup | 鼠标 | — | 抬起 |
| mousemove | 鼠标 | — | 移动 |
| mouseover | 鼠标 | — | 指针进入（冒泡） |
| mouseout | 鼠标 | — | 指针离开（冒泡） |
| pointerdown | 指针 | — | 指针按下（Pointer Events） |
| pointerup | 指针 | — | 指针抬起 |
| pointermove | 指针 | — | 指针移动 |
| pointercancel | 指针 | — | 指针取消 |
| scroll | 滚动 | — | 滚动 |
| wheel | 滚动 | — | 滚轮（mousewheel 非标准拼写 → wheel） |
| touchstart | 触摸 | — | 触摸开始 |
| touchend | 触摸 | — | 触摸结束 |
| touchmove | 触摸 | — | 触摸移动 |
| touchcancel | 触摸 | — | 触摸取消 |
| contextmenu | 菜单 | — | 右键菜单 |
| drag | 拖放 | — | 拖拽 |
| dragstart | 拖放 | — | 拖拽开始 |
| dragend | 拖放 | — | 拖拽结束 |
| dragover | 拖放 | — | 拖拽悬停 |
| dragleave | 拖放 | — | 拖拽离开 |
| drop | 拖放 | — | 放置 |
| copy | 剪贴板 | — | 复制 |
| cut | 剪贴板 | — | 剪切 |
| paste | 剪贴板 | — | 粘贴 |
| play | 媒体 | — | 播放 |
| pause | 媒体 | — | 暂停 |
| ended | 媒体 | — | 播放结束 |
| load | 媒体 | — | 加载完成 |
| error | 媒体 | — | 加载错误 |
| animationstart | 动画 | — | 动画开始 |
| animationend | 动画 | — | 动画结束 |
| animationiteration | 动画 | — | 动画迭代 |
| transitionend | 过渡 | — | 过渡结束 |
| transitionrun | 过渡 | — | 过渡运行 |
| transitionstart | 过渡 | — | 过渡开始 |
| transitioncancel | 过渡 | — | 过渡取消 |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 合成事件处理器 `onClick`/`onKeyDown`/…（`onXxx` 拼写 = 平台形状；语义词汇 = canonical 名；`map_event` 五后端单表共享） |
| Flutter | 每事件回调/监听器（`onPressed`/`onChanged`/`GestureDetector` listener/`FocusNode`…） |
| OmniView | DOM 事件名透传（`click`/`keydown`/…） |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| onClick | React | 合成事件处理器名——平台形状；语义词汇是事件名本体 | click |
| onKeyDown | React | 同上 | keydown |
| onPointerDown | React | 同上 | pointerdown |
| onDoubleClick / onDblClick | React | 同上（canonical 是 dblclick） | dblclick |
| onPressed | Flutter | 组件回调名（ButtonStyleButton.onPressed）——非语义词汇 | click |
| onTapDown | Flutter | GestureDetector 回调名 | pointerdown |
| onChanged | Flutter | 表单回调名 | input |
| onSubmitted | Flutter | 表单回调名 | submit |
| mousewheel | DOM | 非标准拼写（标准事件是 wheel） | wheel |
| doubleclick | DOM | 别名拼写（canonical 是 dblclick） | dblclick |

> 规则：任何 `onXxx` 拼写（React 合成事件名）与 Flutter 组件回调名都不进语义词汇——白名单外的 `@onXxx` 产生「未知事件名 warning + 最近合法事件建议」（Task 4.2）。

---

## 域 2: a11y role（可访问性）

### 来源域

WAI-ARIA 1.2——**可访问名称（accname）/ 角色（role）/ 辅助技术隐藏 / live region / 图像替代**（设计文档 域 10；4.3 词汇 ARIA 化 = `crates/omnilang-core/src/semantic/validate_a11y.rs`）。**教训（设计 P1）**：本域曾以 Flutter Semantics 旗标名为词汇（`textField`/`image`）——web 输出非法 `role="textField"`；4.3 改为 WAI-ARIA 规范名，旗标名降为内部映射 + 借词黑名单。

### 语义词汇

| 语义词汇 | 值域 / 约束 | 说明 |
|---------|------------|------|
| label | string（字面量） | 可访问名称 |
| role | WAI-ARIA 规范名（Flutter 可映射子集见下） | 角色 |
| hidden | expr（动态） | 辅助技术隐藏 |
| live | "polite" / "assertive"（见 live 值域） | 实时区域 |
| alt | expr（动态） | 图像替代文本 |

### role 子集

| role 子集（WAI-ARIA 规范名） | Flutter 内部映射 | 说明 |
|------------------------------|------------------|------|
| button | button: true | 按钮 |
| textbox | textField: true | 文本框（内部映射，非借词） |
| link | link: true | 链接 |
| checkbox | checkbox: true | 复选框 |
| radio | radio: true | 单选 |
| img | image: true | 图像（内部映射，非借词） |

### live 值域

| live 值域 | 说明 |
|-----------|------|
| polite | aria-live="polite" |
| assertive | aria-live="assertive" |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `aria-label`/`role`/`aria-hidden`/`aria-live`/`alt` |
| Flutter | `Semantics(label:)` + flags / `ExcludeSemantics` / `liveRegion` / `semanticLabel`（不可映射 role → warning + 生成注释） |
| OmniView | aria-* 透传 |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| textField | Flutter SemanticsFlag | 旗标名 ≠ 语义词汇（4.3 教训：曾直接作 role 词汇，web 输出非法 `role="textField"`） | textbox |
| image | Flutter SemanticsFlag | 同上 | img |

> 原生 `aria-*` 属性（`aria-label`/`aria-hidden`/`aria-live`/…）为**已删除形式**（硬诊断 + 迁移提示，docs-gate 删除家族拦截）——一律改用 `a11y: { ... }` 块；`alt=` 保持普通 DOM 属性（非本域表面）。

---

## 域 3: motion 属性（动画）

### 来源域

CSS 动画属性语义——**属性值从 A 到 B 的平滑过渡**（设计文档 域 2；19 属性白名单 ADR-053 D3 + v2 新增 borderRadius/fontSize，BNF spec `semantic_anim_prop`）。白名单封闭：白名单外属性解析期报错；语义词汇是「属性名」而非「变换组合机制」。

### 语义词汇

| 语义词汇 | 说明 |
|---------|------|
| opacity | 不透明度 |
| scale | 缩放（三轴统一） |
| scaleX | 水平缩放 |
| scaleY | 垂直缩放 |
| translateX | 水平位移 |
| translateY | 垂直位移 |
| translateZ | 深度位移 |
| rotate | 旋转（三轴统一） |
| rotateX | X 轴旋转 |
| rotateY | Y 轴旋转 |
| rotateZ | Z 轴旋转 |
| skewX | 水平倾斜 |
| skewY | 垂直倾斜 |
| backgroundColor | 背景色 |
| color | 前景色 |
| width | 宽度（Flutter diagnostic） |
| height | 高度（Flutter diagnostic） |
| borderRadius | 圆角（v2 新增，三目标已验证） |
| fontSize | 字号（v2 新增，三目标已验证） |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | WAAPI keyframes（`el.animate(...)`） |
| Flutter | Tween（`Transform.scale`/`BorderRadiusTween`/`TextStyle.lerp`…；width/height → diagnostic） |
| OmniView | runtime transition |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| transform | CSS | 组合机制名（`transform: translate(...)`）——语义词汇是分量名 | translateX / scale / rotate |
| transition | CSS | 过渡机制属性（transition-property 等）——语义词汇是「触发器 + 属性值」 | — |
| animation | CSS | 动画机制属性（animation-name 等） | — |
| blur | CSS | 白名单外属性（19 项封闭；解析期报错） | — |
| brightness | CSS | 同上 | — |
| forward / reverse | Flutter | AnimationController 方法名——平台机制 | — |

---

## 域 4: RefOp（元素命令式操作）

### 来源域

DOM 命令式元素 API 的语义意图——**焦点 / 滚动 / 测量 / 选择 / 指针捕获**（设计文档 域 1；RefOpIR 10 ops = `crates/omnilang-core/src/ir/types.rs`）。语义词汇是「对元素的一次性命令式操作」，不是 DOM 方法形状。

### 语义词汇

| 语义词汇 | 说明 |
|---------|------|
| focus | 移动焦点到元素 |
| blur | 移除焦点 |
| scrollTo | 绝对滚动位置 |
| scrollBy | 相对滚动增量 |
| measure | 读元素包围盒（{ width, height, x, y }） |
| select | 选中元素文本 |
| setSelection | 选中文本区间（start, end） |
| value | 读元素当前值（input/textarea）——命令式写已删除 |
| capturePointer | 锁定后续指针事件到本元素（无参数） |
| releasePointer | 释放指针捕获 |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `ref.current.focus()`/`scrollTo()`/`getBoundingClientRect()`/`setPointerCapture()`… |
| Flutter | `FocusNode`/`ScrollController`/`TextEditingController`/…（capturePointer → error 诊断，floor 外） |
| OmniView | runtime 活动 pointerId / DOM 方法 |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| setPointerCapture() | DOM | 平台方法形状——语义是 capturePointer() 无参数（D3：多指针锁定不支持） | capturePointer |
| scrollIntoView() | DOM | 平台方法形状 | scrollTo |
| getBoundingClientRect() | DOM | 平台方法形状 | measure |
| focus() | DOM | 平台方法形状（括号是 JS 形状）——语义是 focus | focus |
| ref.style.* | DOM API | 已删除形式（硬诊断 + 迁移提示「use signal + style binding instead」） | — |
| ref.setValue(x) / ref.value = x | 命令式写 | 已删除（用 bind:value 双向绑定） | bind:value |
| capturePointer(id) | DOM | 已删除参数形式（D3：无参数） | capturePointer |

---

## 域 5: design token 类型（设计值）

### 来源域

设计系统类型化设计值——**CSS 值语义（颜色/尺寸/字距/时长/缓动/…）**（设计文档 域 3；13 类型，BNF spec `token_type`，parser 接受小写拼写）。语义词汇是「类型名」，不是 CSS 属性名，也不是平台类型名。

### 语义词汇

| 语义词汇（parser 拼写） | 设计文档拼写 | 说明 |
|------------------------|-------------|------|
| color | Color | 颜色 |
| size | Dimension | 尺寸 |
| radius | Radius | 圆角 |
| fontFamily | FontFamily | 字体族 |
| fontSize | FontSize | 字号 |
| fontWeight | FontWeight | 字重 |
| lineHeight | LineHeight | 行高 |
| letterSpacing | LetterSpacing | 字距 |
| shadow | Shadow | 阴影 |
| duration | Duration | 时长 |
| easing | Easing | 缓动 |
| opacity | Opacity | 不透明度 |
| zIndex | ZIndex | 层级 |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | CSS 变量 + `setProperty`（动态 token） |
| Flutter | 运行时 Values（既有） |
| OmniView | CSS |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| font-size | CSS | 属性名 ≠ 类型名——语义词汇是类型 size | size |
| letter-spacing | CSS | 同上 | letterSpacing |
| z-index | CSS | 同上 | zIndex |
| Color | Dart/Flutter | 平台类型名——语义词汇是小写类型名 color | color |
| TextStyle | Flutter | 平台类型名 | — |
| CSSStyleDeclaration | JS | 平台类型名 | — |

---

## 维护约定

- 编译器白名单变更（新增事件 canonical 名 / a11y 角色 / 动画属性 / RefOp / token 类型）必须同步本登记册**同 commit**——`scripts/verify-vocabulary.py` 双向断言使漂移即门禁失败（与 docs-gate 对设计文档的强制力同级）。
- 新语义域设计时，§5 模板「词汇来源」字段必填：写清词汇来源域 + 排除词汇，并确保本登记册收录后方可定稿。
