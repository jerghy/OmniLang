# 语义全集（Semantic Universe）

**日期**: 2026-08-07
**状态**: Active —— 设计 P3 机制（v3 修复计划 Phase 7，Task 7.2）
**依据**: 设计文档 §2 原则 5（已实现范围 vs 设计记录显式区分）+ 白名单教训（19 属性白名单曾是「已实现集」——borderRadius/fontSize 上轮排除理由事实错误、v2 才补回，白名单随实现修正而变动；ArrayMethod 语义 pass 提升到所有后端但 floor 门缺失——3 后端 verbatim 零诊断，v3 Task 4.8 补 E_CODEGEN）
**审计**: `scripts/verify-semantic-universe.py`（语义集条目 → 实现状态矩阵，docs-gate 同批）

> 本全集回答两个问题：**前端功能全集是什么**（语义集——与实现无关）；**每项当前支持哪些**（实现状态——支持/部分/未支持 + 诊断行为）。
> **P3 机制**：语法表面（BNF 白名单）只随语义集变化——「这是不是前端功能」由语义域决定；实现进度单独在状态列跟踪。白名单不再随实现修正而变动，语法表面不再漂移。
> 7 个语义域：可动画属性 / 数组操作 / 事件 / RefOp / design token 类型 / 值绑定（bind）/ 元素指令与绑定（directives）。
> **与登记册（`vocabulary-registry.md`，Task 7.1）互补**：登记册管词汇来源（语义词汇 ↔ 来源域 ↔ 排除词汇——「词从哪来」）；本全集管功能全集（条目 ↔ 实现状态 ↔ 诊断行为——「功能有哪些、支持哪些」）。交集域（motion/事件/RefOp/token）双载：登记册定词汇与借词黑名单，本全集定全集边界与实现状态；登记册独有 a11y role（封闭全集 = 已实现集，无漂移可能）；本全集独有数组操作（语言方法域）与值绑定（bind，域 6，v4 起）——登记册未载。

## 维护约定

- **新增语法必须先在本全集登记**（新条目 + 语义 + 实现状态 + 诊断行为）→ 然后才可进入 BNF/设计文档/编译器白名单——`scripts/verify-semantic-universe.py` 双向断言使漂移即门禁失败（与登记册、docs-gate 对设计文档的强制力同级）。
- **实现状态变更只改状态列，不改语法表面**：属性/操作实现补齐（未支持 → 支持）、目标补齐（部分 → 支持）均不动 BNF；只有语义集变更（新增/删除功能）才动语法。
- 状态值域：**支持**（全目标实现）/ **部分**（部分目标实现或降级，诊断行为列写明缺口）/ **未支持**（语义集成员但未实现——诊断行为列写明当前表面与诊断）。
- 已实现白名单 = 编译器当前白名单（与 BNF/IR 交叉断言）；语义集 ⊇ 白名单，白名单外条目必有「未支持」状态——这就是「语义集 vs 已实现集」的分界。

---

## 域 1: 可动画属性（motion）

### 语义域定义

CSS 动画语义域——**属性值从 A 到 B 平滑过渡**（CSS Transitions/Animations 可插值属性，按语义族划分，与实现无关）。机制词（`transform`/`transition`/`animation` 属性本身）非语义词汇（登记册 域 3 排除词汇）；变换族以**分量**为语义词汇（translateX/scale/rotate…）。全集封闭按本表（2026-08-05 定稿）：未列属性视为未支持（解析期 error），扩充需设计评审 + 登记册 + 三目标验证（borderRadius/fontSize 补入先例，Task 5.2）。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| opacity | 不透明度 | 支持 | 无（三目标动画；Flutter Opacity wrapper） |
| scale | 缩放（三轴统一） | 支持 | 无（三目标；Flutter Transform.scale） |
| translateX | 水平位移 | 支持 | 无（三目标；Flutter Transform.translate） |
| translateY | 垂直位移 | 支持 | 无（三目标） |
| translateZ | 深度位移 | 部分 | React/OmniView 动画；**Flutter 跳过 + [flutter] warning**（无对应 widget 构造） |
| scaleX | 水平缩放 | 部分 | 同上 |
| scaleY | 垂直缩放 | 部分 | 同上 |
| rotate | 旋转（三轴统一） | 支持 | 无（三目标；Flutter Transform.rotate） |
| rotateX | X 轴旋转 | 部分 | React/OmniView 动画；**Flutter 跳过 + [flutter] warning** |
| rotateY | Y 轴旋转 | 部分 | 同上 |
| rotateZ | Z 轴旋转 | 部分 | 同上 |
| skewX | 水平倾斜 | 部分 | React/OmniView 动画；**Flutter 跳过 + [flutter] warning**（SDK 无 Transform.skew） |
| skewY | 垂直倾斜 | 部分 | 同上 |
| backgroundColor | 背景色 | 支持 | 无（三目标；Flutter Tween\<Color\>） |
| color | 前景色 | 支持 | 无（三目标；Flutter Tween\<Color\>） |
| width | 宽度 | 支持 | Flutter 经 SizedBox 动画 + 性能提示（[flutter] prefer scale/transform）；关键字值（auto 等）→ error「keyword value cannot be animated」 |
| height | 高度 | 支持 | 同上 |
| borderRadius | 圆角（v2 补入） | 支持 | 无（三目标；Flutter BorderRadiusTween） |
| fontSize | 字号（v2 补入） | 支持 | 无（三目标；Flutter TextStyle） |
| borderColor | 边框色 | 未支持 | 解析期 error「白名单外属性」（19 项封闭，语义集外实现） |
| outlineColor | 轮廓色 | 未支持 | 同上 |
| textDecorationColor | 文本装饰色 | 未支持 | 同上 |
| boxShadow | 阴影 | 未支持 | 同上 |
| backgroundPosition | 背景定位 | 未支持 | 同上 |
| backgroundSize | 背景尺寸 | 未支持 | 同上 |
| top | 顶偏移 | 未支持 | 同上 |
| right | 右偏移 | 未支持 | 同上 |
| bottom | 底偏移 | 未支持 | 同上 |
| left | 左偏移 | 未支持 | 同上 |
| marginTop | 外边距（上） | 未支持 | 同上 |
| marginLeft | 外边距（左） | 未支持 | 同上 |
| paddingTop | 内边距（上） | 未支持 | 同上 |
| paddingLeft | 内边距（左） | 未支持 | 同上 |
| minWidth | 最小宽度 | 未支持 | 同上 |
| maxHeight | 最大高度 | 未支持 | 同上 |
| borderTopLeftRadius | 圆角长手（左上） | 未支持 | 同上 |
| fontWeight | 字重 | 未支持 | 同上 |
| letterSpacing | 字距 | 未支持 | 同上 |
| lineHeight | 行高 | 未支持 | 同上 |
| textShadow | 文字阴影 | 未支持 | 同上 |
| filter | 滤镜 | 未支持 | 同上 |
| clipPath | 裁剪路径 | 未支持 | 同上 |
| zIndex | 层级 | 未支持 | 同上（integer 插值可动画，但跨目标语义复杂——Flutter z 序由 widget 顺序决定，见 token 域） |
| visibility | 可见性 | 未支持 | 同上（离散插值） |
| flexGrow | 弹性增长 | 未支持 | 同上 |
| order | 弹性/网格顺序 | 未支持 | 同上 |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| opacity | BNF `semantic_anim_prop`（19 项；ADR-053 D3 + v2 Task 5.2） |
| scale | 同上 |
| scaleX | 同上 |
| scaleY | 同上 |
| translateX | 同上 |
| translateY | 同上 |
| translateZ | 同上 |
| rotate | 同上 |
| rotateX | 同上 |
| rotateY | 同上 |
| rotateZ | 同上 |
| skewX | 同上 |
| skewY | 同上 |
| backgroundColor | 同上 |
| color | 同上 |
| width | 同上 |
| height | 同上 |
| borderRadius | 同上 |
| fontSize | 同上 |

> 白名单外条目 = 语义集成员但未支持：**语义集说「CSS 可动画、前端功能存在」；实现状态说「当前不支持」——解析期 error**。borderRadius/fontSize 教训（上轮以「实现未验证」为由排除，理由事实错误）：语义集判断只看语义域（CSS 可插值），不看实现进度。

---

## 域 2: 数组操作（array）

### 语义域定义

列表数据处理语义域——**对列表的变换/查询操作**（过滤/映射/计数/排序/查找/长度/增删…）。两层表面：**语言方法层**（`subject.filter(...)` 等，具语言语义 = 跨目标保证，即本矩阵语义集）；**方法调用层**（普通方法调用按目标翻译，无语言语义、无统一 floor 门——不具跨目标保证）。语言化新操作（方法调用层 → 语言方法层）必须先登记本表（状态 未支持 → 支持），语法表面不随实现进度漂移。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| filter | 过滤（保留谓词匹配项） | 支持 | Python/Rust/Dioxus 目标 → **E_CODEGEN**「语言数组方法不支持该目标——用显式循环或 stdlib（array_filter 已弃用）」（v3 Task 4.8 floor 门）；JS 族 `filter` / Dart 族 `where(...).toList()` |
| map | 映射（逐项变换） | 支持 | 同上（JS 族 `map` / Dart 族 `map(...).toList()`） |
| count | 计数（谓词匹配项数） | 支持 | 同上（JS 族 `filter(...).length` / Dart 族 `where(...).length`）；`count()` 无谓词 → error「count 需要一个谓词闭包」 |
| sort | 排序（稳定序） | 未支持 | 当前经普通方法调用表面（JS/Dart/Rust 族比较器翻译，`array_sort` stdlib）；语言化需先登记本表 + 三目标验证 |
| find | 查找首个匹配项 | 未支持 | 当前经 stdlib `array_find`；语言化需先登记 |
| reduce | 归约 | 未支持 | 当前经普通方法调用（无跨目标保证）；语言化需先登记 |
| some | 存在性判定 | 未支持 | 同上 |
| every | 全称判定 | 未支持 | 同上 |
| unique | 去重 | 未支持 | 同上 |
| reverse | 反转 | 未支持 | 同上 |
| flat | 展平 | 未支持 | 同上 |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| filter | `validate_array_methods.rs` `ARRAY_METHODS`（→ `ExprIR::ArrayMethod`；IR `ArrayMethodIR`） |
| map | 同上 |
| count | 同上 |

> **方法调用层（对照，非语义集条目）**：`len`/`get`/`push`/`contains`/`pop` 等经普通方法调用跨目标翻译（JS 族 `length`/`push`/`includes`、Dart 族 `length`/`add`/`removeLast`、Python `len()`/`append`/`in`）——**不具语言语义**（无 floor 门：未翻译操作在后端 verbatim）。ArrayMethod 教训（P3 背景）：语义 pass 曾提升到所有后端但 3 后端 verbatim 零诊断——floor 门（E_CODEGEN）于 v3 Task 4.8 补齐，本表如实记录。

---

## 域 3: 事件（events）

### 语义域定义

交互时机语义域——**用户与元素交互时触发处理器**（W3C UI Events / Pointer Events / Touch Events / CSS Animation & Transition Events，54 canonical 名，封闭）。语义词汇 = canonical 事件名本体（`onXxx` 拼写是平台形状，登记册 域 1 排除词汇）；DSL 表面 `@onXxx={...}`（或裸 `onXxx={...}`）。全集封闭：白名单外事件名 → 未知事件名 warning + 最近合法事件建议（Task 4.2）。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| click | 单击 | 支持 | 无（全目标；Flutter onPressed/onTap 依组件） |
| dblclick | 双击 | 支持 | 无（全目标；`map_event` 单表复用） |
| longpress | 长按 | 部分 | **Flutter 便捷事件（onLongPress）；React/OmniView 无 DOM 对应——处理器名透传不触发** |
| hover | 悬停 | 部分 | **Flutter MouseRegion.onHover 交付；React/OmniView 无 DOM 对应——处理器名透传不触发（v4：编译 warning + 跳过死处理器，见代码生成诚实降级）** |
| change | 值变化提交 | 支持 | 无（全目标） |
| input | 输入中 | 支持 | 无（全目标；Flutter onChanged） |
| toggle | 切换 | 部分 | **Flutter onChanged 交付（checkbox 便捷形状）；React/OmniView 在 `<details>` 上交付（React onToggle 合成事件 / OmniView addEventListener("toggle")——details 条件性，Task 6.1b）；其他元素 v4：编译 warning + 跳过死处理器** |
| submit | 表单提交 | 支持 | 无（全目标） |
| focus | 获得焦点 | 支持 | 无（全目标） |
| blur | 失去焦点 | 支持 | 无（全目标） |
| select | 文本选择 | 支持 | 无（全目标） |
| resize | 尺寸变化 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| keydown | 键按下 | 支持 | 无（全目标；KeyEvent 稳定映射 keyCode/keyLabel） |
| keyup | 键抬起 | 支持 | 无（全目标） |
| keypress | 键按下（字符） | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| mouseenter | 指针进入（不冒泡） | 支持 | 无（全目标） |
| mouseleave | 指针离开（不冒泡） | 支持 | 无（全目标） |
| mousedown | 按下 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| mouseup | 抬起 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| mousemove | 移动 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| mouseover | 指针进入（冒泡） | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| mouseout | 指针离开（冒泡） | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| pointerdown | 指针按下 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| pointerup | 指针抬起 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| pointermove | 指针移动 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| pointercancel | 指针取消 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| scroll | 滚动 | 支持 | 无（全目标） |
| wheel | 滚轮 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| touchstart | 触摸开始 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| touchend | 触摸结束 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| touchmove | 触摸移动 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| touchcancel | 触摸取消 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| contextmenu | 右键菜单 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| drag | 拖拽 | 支持 | 无（全目标） |
| dragstart | 拖拽开始 | 支持 | 无（全目标） |
| dragend | 拖拽结束 | 支持 | 无（全目标） |
| dragover | 拖拽悬停 | 支持 | 无（全目标） |
| dragleave | 拖拽离开 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| drop | 放置 | 支持 | 无（全目标） |
| copy | 复制 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| cut | 剪切 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| paste | 粘贴 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| play | 播放 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| pause | 暂停 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| ended | 播放结束 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| load | 加载完成 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| error | 加载错误 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| animationstart | 动画开始 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| animationend | 动画结束 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| animationiteration | 动画迭代 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| transitionend | 过渡结束 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| transitionrun | 过渡运行 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| transitionstart | 过渡开始 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |
| transitioncancel | 过渡取消 | 部分 | **Flutter 未映射——[flutter] warning「not supported; skipped」（flutter/view.rs:1697-1723 诚实降级）；React/OmniView 依赖合成事件/DOM 对应（以目标为准）** |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| click | `event_names.rs` 共享 canonical 表（54 canonical；五后端单表，`canonical_event_name`） |
| dblclick | 同上 |
| longpress | 同上 |
| hover | 同上 |
| change | 同上 |
| input | 同上 |
| toggle | 同上 |
| submit | 同上 |
| focus | 同上 |
| blur | 同上 |
| select | 同上 |
| resize | 同上 |
| keydown | 同上 |
| keyup | 同上 |
| keypress | 同上 |
| mouseenter | 同上 |
| mouseleave | 同上 |
| mousedown | 同上 |
| mouseup | 同上 |
| mousemove | 同上 |
| mouseover | 同上 |
| mouseout | 同上 |
| pointerdown | 同上 |
| pointerup | 同上 |
| pointermove | 同上 |
| pointercancel | 同上 |
| scroll | 同上 |
| wheel | 同上 |
| touchstart | 同上 |
| touchend | 同上 |
| touchmove | 同上 |
| touchcancel | 同上 |
| contextmenu | 同上 |
| drag | 同上 |
| dragstart | 同上 |
| dragend | 同上 |
| dragover | 同上 |
| dragleave | 同上 |
| drop | 同上 |
| copy | 同上 |
| cut | 同上 |
| paste | 同上 |
| play | 同上 |
| pause | 同上 |
| ended | 同上 |
| load | 同上 |
| error | 同上 |
| animationstart | 同上 |
| animationend | 同上 |
| animationiteration | 同上 |
| transitionend | 同上 |
| transitionrun | 同上 |
| transitionstart | 同上 |
| transitioncancel | 同上 |

> 事件域语义集 = 已实现集（54 canonical 全支持，longpress 部分）：事件全集为封闭域（登记册 域 1 同载），无漂移空间——新增事件 = 语义集扩充，先登记本表 + 登记册再入编译器。

---

## 域 4: RefOp（元素命令式操作）

### 语义域定义

元素命令式操作语义域——**对元素的一次性动作**（焦点/滚动/测量/选择/指针捕获；DOM 命令式 API 的语义意图，与实现无关）。语义词汇 = 操作名本体（`focus` 等，非 `focus()` DOM 形状——登记册 域 4 排除词汇）。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| focus | 移动焦点到元素 | 支持 | 无（三目标；Flutter FocusNode） |
| blur | 移除焦点 | 支持 | 无（三目标） |
| scrollTo | 绝对滚动位置 | 支持 | 无（三目标；平滑度由目标默认决定——React 瞬时/Flutter 300ms easeInOut，滚动契约） |
| scrollBy | 相对滚动增量 | 支持 | 无（三目标） |
| measure | 读包围盒 | 支持 | 无（三目标；{ width, height, x, y }，Flutter 安全 cast） |
| select | 选中元素文本 | 支持 | 无（三目标） |
| setSelection | 选中文本区间 | 支持 | 无（三目标；(start, end)） |
| value | 读元素当前值 | 支持 | 无（三目标；**命令式写已删除**——`ref.value = x` → error「用 bind:value 双向绑定」） |
| capturePointer | 锁定后续指针事件 | 部分 | React/OmniView 活动指针捕获；**Flutter → error「Flutter 不支持指针捕获，用 GestureDetector」**（floor 外） |
| releasePointer | 释放指针捕获 | 部分 | 同上 |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| focus | `RefOpIR` 10 variants（`crates/omnilang-core/src/ir/types.rs`；`validate_ref_ops` 10 ops） |
| blur | 同上 |
| scrollTo | 同上 |
| scrollBy | 同上 |
| measure | 同上 |
| select | 同上 |
| setSelection | 同上 |
| value | 同上 |
| capturePointer | 同上 |
| releasePointer | 同上 |

> capturePointer/releasePointer 语义集成员（多指针显式锁定 = floor 外硬诊断，D3）——Flutter 目标 floor 外（error 诊断）：「部分」= 目标缺口，语法表面不动。

---

## 域 5: design token 类型（设计值）

### 语义域定义

设计值类型语义域——**类型化设计值**（颜色/尺寸/字距/时长/缓动…；CSS 值语义，与实现无关）。语义词汇 = 类型名（parser 小写拼写，登记册 域 5；`Color` 等平台类型名 = 排除词汇）。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| color | 颜色 | 支持 | 无（三目标） |
| size | 尺寸 | 支持 | 无（三目标） |
| radius | 圆角 | 支持 | 无（三目标） |
| fontFamily | 字体族 | 支持 | 无（三目标） |
| fontSize | 字号 | 支持 | 无（三目标） |
| fontWeight | 字重 | 支持 | 无（三目标） |
| lineHeight | 行高 | 支持 | 无（三目标） |
| letterSpacing | 字距 | 支持 | 无（三目标） |
| shadow | 阴影 | 支持 | 无（三目标） |
| duration | 时长 | 支持 | 无（三目标） |
| easing | 缓动 | 支持 | 无（三目标；Flutter 不可映射 easing → warning + 降级） |
| opacity | 不透明度 | 支持 | 无（三目标） |
| zIndex | 层级 | 部分 | React/OmniView CSS z-index；**Flutter → warning「Flutter z 序由 widget 顺序决定」**（降级） |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| color | BNF `token_type`（13 类型；parser 接受小写拼写；登记册 域 5 同载） |
| size | 同上 |
| radius | 同上 |
| fontFamily | 同上 |
| fontSize | 同上 |
| fontWeight | 同上 |
| lineHeight | 同上 |
| letterSpacing | 同上 |
| shadow | 同上 |
| duration | 同上 |
| easing | 同上 |
| opacity | 同上 |
| zIndex | 同上 |

> 不可映射设计值（box-shadow 组合等）→ warning + 生成代码降级注释（warnings 通道，Task 4.3/4.4）——值级诊断，类型语义集不动。

---

## 域 6: 值绑定（bind）

### 语义域定义

声明式双向绑定语义域——**控件状态 ↔ 信号双向一致**（`bind:checked` 选中态；`bind:value` 文本值沿 v2 值绑定收敛登记于设计文档 域 1 BNF `value_binding` 产生式 + 映射表）。语义词汇 = 绑定名本体（`bind:checked` 等——非平台事件形状；登记册 域 4 借词判定注记承载）。v4 正式化（domain HIGH-3）：此前实现存在（React/OmniView 双向、Flutter warning + 跳过）但权威文档宣称「bind:value 唯一双向绑定」——本域收敛该漂移。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| bind:checked | 选中态双向绑定 | 部分 | **React/OmniView 双向（onChange/change 回写）；Flutter warning + 跳过（TextField 无 checked 概念）——v4 正式化（此前实现存在但文档宣称「bind:value 唯一双向绑定」）** |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| bind:checked | 设计文档 域 1 BNF `value_binding` 产生式；React `"value" \| "checked"` onChange 回写 `e.target.checked`（`omnilang/src/codegen/react/view.rs` ~:396）；OmniView two-way bindings（`omniview/view/element.rs` ~:351）；Flutter `write_back_eligible = bind_name == "value"`——checked warning + 跳过（`flutter/view.rs` ~:2084；warning 文本 ~:2107） |

> 域 6 为 v4 新增（domain HIGH-3）——本全集此前无 bind 语义域；bind 无编译器白名单常量，事实源以代码锚点记录（既有 5 域交叉断言不含本域）。

---

## 域 7: 元素指令与绑定（directives）

### 语义域定义

元素属性层语义域——**声明在元素上的指令与绑定**（元素可拖拽 / 元素可被引用 / 声明式 DOM 副作用 / 动态样式 / 属性形态），与事件（域 3）和命令式 RefOp（域 4）互补：指令表达意图（`@drag` 拖拽意图——指针捕获由编译器按目标选路）、绑定表达声明（`@ref` 引用、`focus=`/`scroll-x=`/`scroll-y=` 语义绑定驱动原生 DOM 副作用）、样式与属性表达形态（`style: { prop: sig }` 动态条目经直写通道落目标、裸布尔属性三态）。语义词汇 = 指令/绑定名本体（`drag`/`ref`/`focus` 等——`@` 前缀与 `=` 后缀是 DSL 表面形态）。全集封闭按本表（2026-08-07 登记）：未列元素属性层功能视为未登记，扩充需设计评审 + 登记册 + 三目标验证（@drag 入册先例，ADR-073）。

### 语义集条目

| 语义集条目 | 语义 | 实现状态 | 诊断行为 |
|-----------|------|---------|---------|
| drag | 拖拽意图指令（指针捕获编译器选路：React setPointerCapture / OmniView capturePointer 组合 / Flutter GestureDetector pan） | 支持 | 无（三目标）；**语义验证硬错误**：需同元素 @ref、handler 必须为组件函数名（禁止内联箭头/调用）、参数 ≤1、@dragEnd 不可单独出现、二者不可重复（validate_drag.rs 6 验证项） |
| dragEnd | 拖拽结束回调（指针抬起/取消） | 支持 | 同上 |
| ref | 元素引用绑定（`@ref={name}` 或 `ref: name`，等价） | 支持 | 无（三目标；@ref 声明见域 4 RefOp 关联） |
| focus | 声明式焦点绑定（`focus={sig}`，true → .focus() / false → .blur()） | 支持 | 无（三目标）；绑定值须裸 signal 引用（`{sig}` 或 `{Store.sig}`），其他表达式为普通 prop |
| scroll-x | 声明式水平滚动绑定 | 支持 | 同上（SEMANTIC_BINDINGS 常量成员） |
| scroll-y | 声明式垂直滚动绑定 | 支持 | 同上 |
| style 动态绑定 | `style: { prop: sig }` 动态条目——信号驱动样式经直写通道（D1/D2 域分类）落目标：React useRef+DOM 直写（零重渲染）/ Flutter ValueNotifier+ListenableBuilder（零 setState 整树重建）/ OmniView el.style.setProperty | 支持 | 无（三目标）；**Flutter 编译期诊断**：不可映射值（opacity/color 等）error_at 显式报错；数值 dimension（width/height/padding/margin）React/OmniView 自动 px 模板 |
| 布尔属性 | 裸布尔属性形态（`disabled`、`checked` 等 15 关键字，`<input checked />` 与 `checked={sig}` 均合法） | 支持 | 无（三目标）；parser 布尔关键字集合 = peek_is_boolean_attr_keyword（当前 15 项——实现细节，以代码为准）；OmniView codegen HTML_BOOLEAN_ATTRS 13 项共享 |

### 已实现白名单

| 条目 | 编译器事实源 |
|------|-------------|
| drag | `parse_event_prop` drag 分派（`crates/omnilang-core/src/parser/view/element.rs` ~:681）+ `validate_drag.rs` + `DragBindingIR`（ir/types.rs） |
| dragEnd | 同上（merge 步骤 push_element_prop 成对合并） |
| ref | ElementIR `ref_binding` + `RefDeclIR`（ir/types.rs） |
| focus | `SEMANTIC_BINDINGS` 常量（parser/view/element.rs:89） |
| scroll-x | 同上 |
| scroll-y | 同上 |

> 注：**style 动态绑定** 与 **布尔属性** 为机制级条目（非词法级），白名单交叉断言以存在性检查承担（见 verify-semantic-universe.py 域 7 断言 Task 2）；本白名单表只列 6 个词法级条目。
