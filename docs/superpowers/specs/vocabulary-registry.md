# 词汇登记册（Vocabulary Registry）

**日期**: 2026-08-05
**状态**: Active —— 设计 P1 复发机制（v3 修复计划 Phase 7，Task 7.1）
**依据**: 设计文档 §2 原则 6（词汇语义域封闭）+ §5 每域模板「词汇来源」字段；4.2 事件白名单（`event_names.rs` 共享表）/ 4.3 a11y 词汇（`validate_a11y.rs`）/ 19 属性白名单 / RefOp 10 ops / design token 13 类型 / validate 6 规则名（`validate_form_load.rs`）
**审计**: `scripts/verify-vocabulary.py`（登记册 ↔ 编译器白名单交叉断言，docs-gate 同批）

> 本登记册强制回答一个问题：**每个语义词汇的语义域是什么**（WAI-ARIA / DOM 事件 / CSS 属性 / …）。
> 7 个封闭词汇域：事件 / a11y role / motion 属性 / RefOp / design token 类型 / 表单校验规则 / 反馈模态。
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

W3C UI Events / Pointer Events / Touch Events / CSS Animation & Transition Events——**用户与元素交互的时机与类型化数据**（设计文档 域 7；4.2 事件白名单 = `crates/omnilang-core/src/codegen/shared/event_names.rs` 共享 canonical 表）。本域词汇分三层（spec 2026-08-08 §3.1，原则 8「语义化即切换」）：

1. **DSL 表面 = 14 个语义事件名**（`SEMANTIC_EVENT_NAMES` 常量，见下节「语义事件名（DSL 表面）」）——用户写 `@onPress={...}` 等；被取代的旧 DOM 拼写（`@onClick`/`@click`/`@onTap`/`@tap` → `@onPress`；`@onMouseEnter`/`@mouseenter` → `@onEnter`；`@onMouseLeave`/`@mouseleave` → `@onLeave`）是编译期硬错误 + 迁移提示（`semantic/validate_event_names.rs`，无透传共存期）。
2. **内部解析面 = canonical 名（54）**——编译器 `canonical_event_name` 单表解析目标，即下方「语义词汇」表；14 个语义名只覆盖其中 9 族，其余 40 个 canonical 名仍合法透传（非语义名，未知拼写走未知名 warning）。
3. **平台形状 = 排除词汇**——React 合成事件名（onClick/onKeyDown/…）与 Flutter 组件回调名（onPressed/onChanged/…）不得成为 canonical 语义词汇（借词黑名单，见「排除词汇」表）。

### 语义词汇

> 内部解析面：canonical 名（54）——编译器 `canonical_event_name` 单表解析目标（断言锚点不变）；DSL 表面拼写见下节「语义事件名（DSL 表面）」。

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

### 语义事件名（DSL 表面）

首批语义化事件（spec 2026-08-08 §3.1，`SEMANTIC_EVENT_NAMES` 常量）——14 个事件名/9 语义族；旧 DOM 拼写（`@onClick`/`@onTap`/`@onMouseEnter`/`@onMouseLeave`…）→ 硬错误 + 迁移提示，其余 40 个 canonical 名维持透传。

| 语义事件名（DSL 表面） | 解析关系（canonical_event_name） | 语义 |
|------------------------|----------------------------------|------|
| onPress | → click | 点按（click/tap 别名收编） |
| onInput | → input | 输入中 |
| onChange | → change | 选值变化提交 |
| onSubmit | → submit | 表单提交 |
| onKeyDown | → keydown | 按键按下 |
| onKeyUp | → keyup | 按键抬起 |
| onPointerDown | → pointerdown | 指针按下 |
| onPointerMove | → pointermove | 指针移动 |
| onPointerUp | → pointerup | 指针抬起 |
| onScroll | → scroll | 滚动 |
| onFocus | → focus | 获得焦点 |
| onBlur | → blur | 失去焦点 |
| onEnter | → mouseenter（enter 短拼写同族） | 指针悬停进入 |
| onLeave | → mouseleave（leave 短拼写同族） | 指针悬停离开 |

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

- `bind:checked`：v4 正式化（domain HIGH-3 三方漂移修复）——语义 = 「选中态双向绑定」；Flutter 为部分支持（warning + skip），非便捷形状借词（checked 是 DOM 属性名，非平台形状）

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

## 域 6: 表单校验规则（表单 validate）

### 来源域

表单字段校验意图——**字段级 `validate: 规则` 属性的值域闭集**（2026-08-09 阶段 2 域 A 收口；`VALIDATE_RULES` = `crates/omnilang-core/src/semantic/validate_form_load.rs:10`，6 规则名闭集）。语义词汇是「校验规则名」，不是平台校验机制名，也不是规则参数值（如 `format: email` 的 `email`/`url` 是 `FORMAT_VALUES` 闭集值，非规则名——见排除词汇）。

> 判定注记（内建信号名不是词汇）：`formErrors`/`formValid`/`formSubmitting`（`@form`）与 `loading`/`error`/`data`/`retry`（`@load`）为**编译器保留名判定**（`validate_form_load.rs` 保留名冲突硬错误），三目标内建信号形态——它们随 `@form`/`@load` 机制自动生成，用户不可自定义，不进本登记册词汇表（域 6 仅收 validate 规则名本体）。

### 语义词汇

| 语义词汇 | 值域 / 约束 | 内建错误消息 | 说明 |
|---------|------------|-------------|------|
| required | 无参 | 此项必填 | 必填校验 |
| format | 闭集名 `FORMAT_VALUES`：email / url | 邮箱格式不正确 / URL 格式不正确 | 格式校验（闭集名字分派；其余值跳过 → 需 custom） |
| min | 数字 | 长度至少为 N（Flutter 此项至少 n 个字符） | 最小长度 |
| max | 数字 | 长度最多为 N（Flutter 此项最多 n 个字符） | 最大长度 |
| pattern | 正则字符串 | 格式不正确 | 正则校验 |
| custom | 组件 fn 名（v0 边界：无字面量自定义消息） | fn 返回消息 | 自定义校验 |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `validateForm` 执行器（校验规则链 → `setFormErrors` map，`formValid = Object.keys(formErrors).length === 0`） |
| Flutter | `TextFormField.validator` 链（`Form` + `_formKey`；错误经 `errorText` 自动展示——语言面 `form.errors` 发射为常量空 map，行为差异注记于 semantic-universe 域 8） |
| OmniView | HTML5 校验属性（`required`/`minLength`/`maxLength`/`pattern`…）+ `{Comp}_validateForm` 自绘错误 |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| validator | Flutter | TextFormField.validator 参数名——平台机制形状 | validate: 规则 |
| regex | JS/HTML | 平台正则形状（`pattern: "..."` 是值不是规则名） | pattern |
| minLength | HTML | HTML5 属性名——平台机制形状 | min |
| maxLength | HTML | 同上 | max |
| email | HTML | `input[type=email]` 机制形状——`format:` 闭集值，非规则名 | format |
| url | HTML | 同上 | format |

---

## 域 7: 导航（navigation）

### 来源域

域 11 导航（semantic-universe 域 11）——**页面切换、路径/查询参数、返回、守卫、嵌套布局、懒加载**（批次 2 设计 spec §5.2，2026-08-10 登记先行 ADR-074）。本域为**机制级词汇**（无编译器词法级白名单常量——语义词汇是组件级指令名/内建语义函数名，交叉断言以存在性检查承担，照域 10 先例）：`@route`/`@layout`/`@query` 组件级指令 + `navigate`/`back` 内建语义函数。**v0 边界**（批次 2 spec §6）：`guard:`/`redirect:`/`lazy:` 为属性级参数不单独立法；页面组件（`@route`）与布局组件（`@layout`）互斥（V3）；动态守卫目标/通配符 404/滚动恢复/预取不进语言。

> 判定注记（编译器保留名判定，非登记词汇）：路径参数注入 prop 名（`:id` → `id`）随路径声明动态产生，为编译器保留名判定——照域 6 先例（formErrors/loading 等内建信号名不登记）。

### 语义词汇

| 语义词汇 | 值域 / 约束 | 说明 |
|---------|------------|------|
| @route | `@route STRING (guard: IDENT \| lazy: BOOL \| redirect: STRING)*`——路径以 `/` 开头（V2 硬错误）；参数段 `:id` 注入为 String prop | 页面路径声明——`@route "/todos/:id" guard: isAdmin lazy: true redirect: "/login"` |
| @layout | `@layout STRING`——前缀以 `/` 开头；视图须含 `<@slot/>` 子页面出口（V10 硬错误） | 布局前缀声明——`@layout "/admin"` |
| @query | `@query IDENT : TYPE (= expr)?`——声明查询参数注入 prop（默认值可选） | 查询参数声明——`@query filter: String = "all"` |
| navigate | `navigate(path, { params, query })`——首参须为应用内路径字面量（V12 外链硬错误；动态路径 W1 警告） | 内建语义函数——应用内跳转 |
| back | `back()` 无参 | 内建语义函数——返回上一页 |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | 路由表项 `createBrowserRouter`（React Router v7 data router）+ `useNavigate`（`navigate` → `navigate(path)`；`back` → `navigate(-1)`）+ `useSearchParams`（@query 注入） |
| Flutter | `GoRoute` 表项 + `redirect` 守卫回调 + `ShellRoute`（@layout）+ `context.push`（`navigate`）/`context.pop`（`back`，栈空 no-op 注记）+ `pathParameters`/`uri.queryParameters` 注入 |
| OmniView | hash 路由表项 + 守卫 fn + 嵌套渲染 + `location.hash` 更新（`navigate`）/`history.back()`（`back`） |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| path | React Router | 路由声明词（`path:` 属性名）——目标端声明词（禁止借词） | @route |
| route | 通用 | 通用路由概念词——目标端声明词 | @route |
| Routes | React Router | 路由表组件名——目标端声明词 | @route |
| GoRoute | Flutter | GoRouter 路由类名——目标端声明词 | @route |
| Outlet | React Router | 嵌套出口组件名——目标端词 | @layout |
| ShellRoute | Flutter | 嵌套路由壳类名——目标端词 | @layout |
| Layout | 通用 | 通用布局词——目标端词 | @layout |
| query | 通用 | 通用查询词——目标端词 | @query |
| searchParams | React Router | useSearchParams 返回值名——目标端词 | @query |
| push | Flutter | context.push 方法名——目标端词 | navigate |
| go | GoRouter | context.go 方法名——目标端词 | navigate |
| useNavigate | React Router | 钩子名——目标端词 | navigate |
| Navigate | React Router | 声明式跳转组件名——目标端词 | navigate |
| pop | Flutter | context.pop 方法名——目标端词 | back |
| back() | 通用 | 平台方法形状（括号是 JS/Dart 形状）——语义词汇是 back | back |

---

## 域 10: 反馈模态（feedback & modal）

### 来源域

反馈与模态意图语义域——**命令式短暂通知 + 模态对话框 + 加载占位 + 渲染降级**（批次 1 设计 spec §2，2026-08-09 登记先行 ADR-074）。本域为**机制级词汇**（无编译器词法级白名单常量——语义词汇是内建语义函数名/元素指令名，交叉断言以存在性检查承担，见 verify-vocabulary.py 域 10 断言 Task 0）：`showToast` 内建语义函数（命令式短暂通知）+ `@modal`/`@skeleton`/`@errorBoundary` 元素指令；`dialog` 元素为本域宿主元素（解析透传通道 + @modal 语义处理器接管）。**v0 边界**：`showToast` 无手动关闭按钮、无位置参数；`@modal` 无关闭动画（打开动画由 `@enter` 复用覆盖）；`@skeleton` 形状自适应不进 v0；`@errorBoundary` fallback 槽不暴露错误对象（spec §2/§9）。

> 判定注记：`@modal` 绑定的 open 信号为编译器受控信号（唯一真源判定，不注册词汇）；`showToast` 生成的保留名判定同域 6 先例（formErrors/formValid/formSubmitting 为编译器保留名判定，非注册词汇——本批次 V8 属此类）。

### 语义词汇

| 语义词汇 | 值域 / 约束 | 说明 |
|---------|------------|------|
| @modal | 绑定 dialog 元素（双向校验）；值 = 用户布尔信号（读引用，受控形态——信号是唯一真源） | 焦点陷阱/Esc 关闭/遮罩点击关闭/`role="dialog"` + `aria-modal` a11y 编译器内建；V1-V5 硬错误 + V13 警告（spec §2.1/§3） |
| showToast | `showToast(msg, { kind, duration })`——kind: "info"\|"success"\|"error"（默认 info）；duration ≥ 0（默认 3000，0 = 不自动关闭） | 内建语义函数——用户不得以同名声明 fn/store/信号/组件（V8 保留名硬错误）；V6/V7 硬错误（spec §2.2/§3） |
| @skeleton | `@skeleton` 或 `@skeleton="text\|line\|circle\|card"`（默认 text） | 只能出现在 @load 的 loading 分支内（V9 硬错误）；形状非法 V10 硬错误（spec §2.3/§3） |
| @errorBoundary | 无参元素指令 | 子树渲染失败降级（`@slot fallback` 至多一个——V14 硬错误）；重复 V11 硬错误 + 无 fallback V12 警告（spec §2.4/§3） |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `@modal` → 受控 `<dialog>` + showModal() 同步；`showToast` → 自生成 toast 容器；`@skeleton` → 占位元素 + shimmer keyframes；`@errorBoundary` → 自生成 ErrorBoundary class |
| Flutter | `@modal` → showDialog + FocusTraversalGroup；`showToast` → ScaffoldMessenger.showSnackBar；`@skeleton` → 静态占位（动画降级注记）；`@errorBoundary` → 空壳 + 数据级错误走 @load error 信号注记 |
| OmniView | `@modal` → 自绘遮罩 + 焦点循环；`showToast` → 自绘 toast 容器；`@skeleton` → 同 React（CSS 动画）；`@errorBoundary` → try 兜底渲染 fallback 槽 |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| dialog | DOM | 原生元素名——@modal 的宿主元素（透传通道），非指令词汇 | @modal |
| showModal | DOM | 原生 dialog 方法名——平台机制形状 | @modal |
| showDialog | Flutter | showDialog 函数名——平台机制形状 | @modal |
| Modal | React | Modal 组件名（Ant Design 等）——平台组件形状 | @modal |
| ModalDialog | 组件库 | 组件命名形状 | @modal |
| toast | 通用 | 通用词/组件名——showToast 语义词汇本体 | showToast |
| SnackBar | Flutter | SnackBar 组件名——平台机制形状 | showToast |
| Toast | 通用 | 通用词大写拼写 | showToast |
| Sonner | 组件库 | Sonner 库名（引库路线）——自生成原始写法是合法子集，库名不借 | showToast |
| ElMessage | 组件库 | Element Plus 通知 API 名——库 API 形状 | showToast |
| skeleton | 通用 | 通用词——@skeleton 语义词汇本体 | @skeleton |
| shimmer | 通用 | 占位动画效果名——编译器生成形态非词汇 | @skeleton |
| Shimmer | 组件库 | shimmer 库名——引库路线 | @skeleton |
| ErrorBoundary | React | React 错误边界组件名——平台组件形状 | @errorBoundary |
| error-boundary | React | kebab-case 组件命名形状 | @errorBoundary |

---

## 域 8: 响应式（responsive）

### 来源域

域 12 响应式（semantic-universe 域 12）——**命名断点声明 + 断点条件表达式 + 媒体查询透传**（批次 3 设计 spec §2.1/§2.2，2026-08-11 登记先行 ADR-074；域 7 已被导航占用，响应式取域 8——两文档编号体系独立）。本域为**机制级词汇**（无编译器词法级白名单常量——bp 是表达式函数形态、breakpoint 是声明关键字，交叉断言以存在性检查承担，照域 10/7 先例）：`breakpoint` 断点声明（design 块）+ `bp("name")` 断点条件表达式（min-width 语义）。**v0 边界**（批次 3 spec §2/§6）：bp() 参数须为字符串字面量（编译期静态查表，动态参数 E-405）；未声明断点名 E-401 硬错误；断点名与同块 token/class/variant 名互斥（E-402）；值 ≤ 0 硬错误（E-403）。

> 判定注记：`@media`/`@container` 为机制透传关键字（CSS 原生词汇），不登记为语义词汇（at-rule 形态面；语义集条目见 semantic-universe 域 12）。

### 语义词汇

| 语义词汇 | 值域 / 约束 | 说明 |
|---------|------------|------|
| bp | `bp("name")`——参数须为字符串字面量；语义 = min-width（视口宽度 ≥ 断点值） | 内建断点条件表达式——`@if bp("md")` / `{}` 绑定 / 三元均可；编译期静态查表 |
| breakpoint | `breakpoint <名>: <无单位正整数>;`（design 块内声明） | 命名断点数值声明——`breakpoint md: 768;` |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `const bp_768 = useMediaQuery('(min-width: 768px)')`（本地 SSR-safe hook）+ `__design/breakpoints.ts` BP 常量表 |
| Flutter | `MediaQuery.sizeOf(context).width >= 768`（表达式）+ `static const bpMd = 768.0;`（design.dart 常量） |
| OmniView | `matchMedia('(min-width: 768px)').matches`（内联表达式，无常量文件） |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| useMediaQuery | React | react-use/MUI 事实标准 hook 名——库 API 名（禁止借词） | bp |

---

## 域 9: 数据（data）

### 来源域

域 13 数据深化 + 域 9 数据三态（semantic-universe 域 13/9）——**分页/无限滚动/轮询/时间节流**（批次 4 设计 spec §5.3，2026-08-11 登记先行 ADR-074）。本域为**词法级词汇**——本批登记先行已建词法级白名单常量层 `load_contract.rs`（`LOAD_PARAM_WORDS`/`TIMING_FUNCS`/`PAGE_TYPE_NAME`），交叉断言经 `rust_str_array` 双向断言承担（verify-vocabulary.py 域 9 断言块）：`paginated`/`scroll`/`interval` @load 局部参数词 + `debounce`/`throttle` std/timing 时间函数 + `Page<T>` 内建分页信封类型。**v0 边界**（批次 4 spec §6）：分页仅页码模式（无游标/无键集）；轮询与分页不组合；无限滚动触发仅列表容器（`@for` 宿主）；乐观更新为远期候选；缓存策略留生态（不进语言）。

> 判定注记：`loading`/`error`/`data`/`retry`/`reset` 为编译器保留名（匿名实例派生信号 `LOAD_RESERVED_NAMES`），不登记为词汇——照域 6 先例（formErrors/loading 等内建信号名不登记）；`scroll` 与既有 `@in-view`/`@onScroll` 词汇不同词汇面（@load 参数面 vs 事件面），不冲突；两文档编号体系独立——universe 域 13 数据深化对应本域 9。

### 语义词汇

| 语义词汇 | 来源域 | 目标映射 | 说明 |
|---------|--------|---------|------|
| debounce | std/timing（stdlib 语言函数） | setTimeout/clearTimeout 闭包（web）/ Dart Timer 闭包（Flutter） | 时间节流原语——非视图意图，stdlib 通道（async_sleep 先例） |
| throttle | std/timing（stdlib 语言函数） | setTimeout/clearTimeout 闭包（web）/ Dart Timer 闭包（Flutter） | 时间节流原语——非视图意图，stdlib 通道（async_sleep 先例） |
| paginated | 域 13 数据深化（@load 参数面） | 页码状态机生成 | @load 局部参数词——机制级条目（存在性断言） |
| scroll（@load 参数） | 域 13 数据深化 | IntersectionObserver / ScrollController 阈值 | 装载触发参数——与 @in-view/@onScroll 既有词汇区分（不同词汇面） |
| interval（@load 参数） | 域 13 数据深化 | setInterval / Timer.periodic | 冒号参数形态（motion over: 先例） |
| Page\<T\> | 域 13 数据深化（内建类型） | TS interface / Dart class | 分页信封——与 Array\<T\> 并列 |

### 目标映射

| 目标 | 生成形状 |
|------|---------|
| React | `debounce`/`throttle` → setTimeout/clearTimeout 闭包；`paginated` → 页码状态机（page/hasMore/加载中/加载更多）；`scroll` → IntersectionObserver 阈值；`interval` → setInterval；`Page<T>` → TS interface |
| Flutter | `debounce`/`throttle` → Dart Timer 闭包；`paginated` → PageController/ScrollController 阈值；`scroll` → ScrollController 阈值；`interval` → Timer.periodic；`Page<T>` → Dart class |
| OmniView | 同 React 形态（web 通道——IntersectionObserver/setInterval 原生） |

### 排除词汇

| 排除词汇 | 平台 | 不得成为语义词汇的原因 | 正确语义词汇 |
|----------|------|----------------------|-------------|
| refetchInterval | TanStack Query | 库参数名——平台/库形状（禁止借词规则） | interval |
| refreshInterval | SWR | 库参数名——平台/库形状（禁止借词规则） | interval |
| infinite_scroll_pagination | 库 | 包名——平台/库形状（禁止借词规则） | paginated |

---

## 维护约定

- 编译器白名单变更（新增事件 canonical 名 / a11y 角色 / 动画属性 / RefOp / token 类型 / 表单校验规则）必须同步本登记册**同 commit**——`scripts/verify-vocabulary.py` 双向断言使漂移即门禁失败（与 docs-gate 对设计文档的强制力同级）。
- 新语义域设计时，§5 模板「词汇来源」字段必填：写清词汇来源域 + 排除词汇，并确保本登记册收录后方可定稿。
