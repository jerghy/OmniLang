# 语义批次 5 长尾四归组 设计 spec（2026-08-09-semantic-batch5-longtail-design.md）

> 状态：draft ｜ 日期：2026-08-09 ｜ 上游：`2026-08-09-semantic-capability-baseline-design.md` §4 批次 5 / §8 治理（权威，本 spec 不推翻）｜ 车票：`.scratch/semantic-capability-baseline/issues/11-stage-scope-longtail-capabilities.md`（Answer 定案）｜ 研究：`research/03-interaction-and-longtail.md` ｜ 配套计划路径：各归组独立 plan（writing-plans 后接 SDD，可并行）

## 1. 背景与范围

本批次为基线 §4 批次 5「长尾四归组」——四个互相独立的归组，各成章节。范围表（上游裁决引用 = 基线 §4 批次 5 原文 + 能力矩阵）：

| # | 归组 | 能力项 | 上游裁决 | 本 spec 形态决策 |
|---|---|---|---|---|
| 1 | 交互 | 拖拽重排 | 「@for reorder / @reorder → dnd-kit / ReorderableListView」 | **@for 容器 reorder= 选项**（与 key= 并列） |
| 2 | 交互 | 基本手势 | 「@swipe/@longPress/@pinch → GestureDetector 双端」 | 三个元素意图指令（handler schema 定稿） |
| 3 | 交互 | 剪贴板 | 「@copy 意图 → Clipboard API / services」 | `@copy={expr}` 元素指令（v0 无反馈） |
| 4 | 交互 | 快捷键 | 「@key 组合 → React keydown / Flutter Shortcuts+Actions」 | `@key="ctrl+s"={onSave}` 键位串意图 |
| 5 | i18n | @t 文本翻译 | 「ICU 消息语法子集——三端同基座」 | 表达式意图函数 + ICU 子集正式定义 |
| 6 | i18n | @locale 语言切换 | 「@locale 语言切换（随 @t 配套）」 | 组件级指令（信号引用） |
| 7 | i18n | formatDate | 「stdlib 最简形态 formatDate(locale)——参数面留生态」 | stdlib 函数 `formatDate(date, locale?)` |
| 8 | 媒体 | canvas | 「命令式绘制意图——RefOp 扩展或元素指令」 | **RefOp 扩展**（ctx 方法门类，决策见 §2.4） |
| 9 | 媒体 | 文件上传 | 「input file 意图 → input[type=file] / file_picker」 | `@filePick={handler}` 指令（低优先） |
| 10 | 媒体 | 摄像头/录音 | 「权限模型批次 spec 细化」 | `@capture` 指令 + 权限模型草案（低优先） |
| 11 | 生命周期 | @effect 强化 | 「定时器/订阅清理 → useEffect cleanup / State.dispose」 | effect 块内托管函数 timer/interval/subscribe |
| 12 | 生命周期 | @store/@mount/@effect 入册 | 「1B 项并入本归组」 | 新域 17 登记表格草案（见 §5） |

记录边界（§6）：memo（编译器职责）、虚拟化（后议）、tooltip（低优先翻案注记）、懒加载、焦点陷阱、系统文件拖放/图片剪贴板。

## 2. 语义意图定义

### 2.1 交互归组

**① 拖拽重排（@for reorder）**
- 意图：「列表项可拖拽重排，重排后源下标→目标下标通知数据源」——容器级意图（区别于 @drag 的元素级自由位移）。
- 语法形态：@for 容器选项 `reorder=`（与既有 `key=` 选项并列，同上下文）：

```
@for item in visibleItems key={item.id} reorder={handleMove} {
    <TodoItem text={item.text} />
}
action handleMove(from: i32, to: i32) { TodoStore.items.move(from, to) }
```

- BNF 草案：`for_directive ::= '@for' <ident> 'in' expr ('key=' '{' expr '}')? ('reorder=' '{' <ident> '}')?`
- 与既有语法关系：handler 约束同 @drag（组件 fn 名，禁内联箭头/调用）；**与 @drag 同元素互斥**（位移 vs 重排语义二选一）。触发器差异（移动端长按 vs 桌面 handle）与拖拽视觉代理为编译器选路细节，不入语法（研究 03 ①④）。
- 冲突检查：`reorder` 无既有词汇；与事件域 54 名无碰撞。

**② 基本手势（@swipe / @longPress / @pinch）**
- 意图：「滑动（含 dismiss 语义）/ 长按 / 双指缩放的触发时机回调」——最简三意图；复杂自定义手势（多指组合、边界判定）留生态（研究 03 ②④）。
- 语法形态：元素意图指令，handler 为组件 fn 名：

```
<Card @swipe={handleSwipe} />         // handler 收 { dir, x, y }  dir: "left"|"right"|"up"|"down"
<Button @longPress={handleLongPress} /> // handler 收 { x, y }
<Image @pinch={handlePinch} />        // handler 收 { scale, x, y }  scale 相对增量（1.0 = 无变化）
```

- BNF 草案：`gesture_prop ::= '@swipe' '=' '{' <ident> '}' | '@longPress' '=' '{' <ident> '}' | '@pinch' '=' '{' <ident> '}'`（并入 `element_prop` 分派，与 drag_prop 同机制）
- 冲突表（重点）：

| 新词汇 | 既有词汇 | 处理 |
|---|---|---|
| @swipe / @pinch | 无（54 事件名无 swipe/pinch） | 无冲突 |
| @longPress | 事件域 `longpress`（@onLongpress，透传态，Flutter 交付 / React 不交付） | **语义化即切换（ADR-075 原则 8）**：@longPress 落地时 `@onLongpress` 旧形态硬诊断 + 迁移提示「使用 @longPress 意图指令」；cut/paste 类同 |
| @copy | 事件域 `copy`（@onCopy，透传态，Flutter 未映射） | 同上：@copy 落地时 @onCopy 硬化迁移 |
| @key | keydown/keyup/keypress 事件 + @for `key=` 选项 | @-前缀指令名与 @onX 事件拼写不同形（@key vs @onKeydown）；@for key= 是选项上下文无碰撞；事件 schema 字段 key 为值语义无碰撞 |

**③ 剪贴板（@copy）**
- 意图：「用户点击即把给定值写入系统剪贴板」——文本剪贴板三端原生一致；图片/富媒体剪贴板留生态（研究 03 ⑤③）。
- 语法形态：`@copy={expr}`（expr = 字符串字面量/信号/表达式）：

```
<Button @copy={TodoStore.shareLink}>复制链接</Button>
```

- BNF 草案：`clipboard_prop ::= '@copy' '=' expr`
- 与事件关系：@copy 是写意图（点击即复制），事件域 copy 是被动事件——语义化即切换后旧事件形态硬化（见冲突表）。v0 无复制成功反馈（不注 toast——YAGNI，见未决 P1）。

**④ 快捷键（@key 组合）**
- 意图：「按键组合触发动作」——采用研究 03 ④的极简两层（键位→handler）；三层（键位→意图→动作）列入未决 P2（Flutter Intent 模型对齐为编译器内部结构，不要求用户写三层）。
- 语法形态：键位串属性 + handler：

```
<Page @key="ctrl+s"={onSave}>…</Page>
<Page @key="ctrl+k"={onSearchFocus}>…</Page>
```

- BNF 草案：`keymap_prop ::= '@key' '=' <key_combo> '=' '{' <ident> '}'`；`key_combo ::= (('ctrl'|'alt'|'shift'|'meta') '+')* <key_name>`（key_name 复用域 3 keydown 语义键名枚举——单源常量）
- 约束：键位串必须含修饰符（裸键为键盘事件语义，由 @onKeydown 承担）；同元素多 @key 合法（各组合独立）。

### 2.2 i18n 归组

**① @t 文本翻译（ICU 消息语法子集）**
- 意图：「消息文案随 locale 翻译，支持插值/复数/选择」——三端唯一「同语法基座」点（研究 03 ⑥③：react-intl / vue-i18n / dart intl 均 ICU 消息语法），编译器映射损耗最小。
- 语法形态：**表达式意图函数**（先例：域 D `bp("md")` 设计表达式函数——@t 同层）：文本插值 `{}` 内与属性值 `label:` 位置均可：

```
{@t("Hello {name}", {name: user.name})}
{@t("{count, plural, =1 {1 item} other {{count} items}}", {count: cart.count})}
<label text: @t("Submit") />
```

- BNF 草案：`i18n_expr ::= '@t' '(' icu_message (',' '{' (ident ':' expr) (',' ident ':' expr)* '}')? ')'`；参数对象为命名实参映射（键 = 占位符名，值 = 表达式/信号）。
- **ICU 子集定义（v0 封闭）**：

```
icu_message ::= (text | placeholder)*          // text = 原文字面量
placeholder ::= '{' arg '}'                                      // 插值
              | '{' arg ',' 'number' '}'                         // 数字格式化（按 locale）
              | '{' arg ',' ('plural'|'select') cases '}'        // 复数/选择
cases       ::= (case_body ',' icu_message)+ 'other' ',' icu_message   // 闭集：plural 接受 =0/=1/整数/one/few/many/other；select 接受字面量/other
```

- **不含（显式排除 → 硬错误）**：offset 语法、`#` 编号简写、`date`/`time` 复杂格式、ordinal、嵌套深度 >3（递归计数）、花括号未闭合/逃逸缺失。CLDR 复数类别按目标运行时判定（Intl.PluralRules / dart intl plural——同一 CLDR 数据源）。
- 消息源：v0 内联字面量（编译器提取为三目标消息资源是工程面——react-intl messages 对象 / gen_l10n ARB / OmniView 消息表；外部键表/回退/构建插件留生态，见未决 P3）。
- 冲突检查：`@t` 无既有词汇（文本插值 `{}` 与事件 54 名无碰撞）；与 store 信号命名空间不同层。

**② @locale 语言切换**
- 意图：「子树文案语言随信号切换」——locale 是全局状态，与状态域天然关联（研究 03 ⑦③）。
- 语法形态：组件级指令 `@locale={lang}`（lang 为信号引用，约束同 focus= 裸信号引用）：

```
component App {
    signal lang: String = "zh"
    view { <div @locale={lang}> <span>{@t("Hello")}</span> </div> }
}
```

- 编译器职责：React IntlProvider locale / Flutter Localizations.override + supportedLocales 声明 / OmniView locale 状态 → 全子树 @t/formatDate 随信号重渲染。BNF 草案：`locale_prop ::= '@locale' '=' '{' <ident> '}'`。

**③ formatDate(locale) stdlib 最简形态**
- 意图：「按当前/指定 locale 格式化日期」——参数面（模式串/时区/日历）留生态（基线裁决；研究 03 ⑧：参数爆炸映射损耗 > 收益，故只做最简）。
- 语法形态：stdlib 内建函数（与 debounce/throttle 同层候选）：

```
{formatDate(order.createdAt)}              // 用 @locale 上下文 locale（默认）
{formatDate(order.createdAt, "en-US")}     // 显式 locale
```

- BNF 草案：`fmt_expr ::= 'formatDate' '(' expr (',' expr)? ')'`；返回值 = 按 locale 的日期字符串（默认格式；模式串留生态）。

### 2.3 媒体归组

**① canvas（RefOp 扩展——形态决策）**
- 决策：**RefOp 扩展，非元素指令**。理由（研究 03 ⑫④）：绘制是命令式序列（moveTo/lineTo/fill），声明式元素指令只能表达「有 canvas」不能表达「画什么」；RefOp（域 4）本就是元素命令式操作门类——加 ctx 方法门类与既有架构同构。
- 语法形态：`<canvas @ref={c} width={w} height={h} />` + 方法级白名单：

```
fn draw() {
    c.ctx.clearRect(0, 0, w, h)
    c.ctx.beginPath()
    c.ctx.moveTo(10, 10)
    c.ctx.lineTo(90, 90)
    c.ctx.stroke()
}
```

- RefOp 方法门类（v0 闭集）：`ctx.` 下 `beginPath/moveTo/lineTo/closePath/stroke/fill/arc/rect/fillRect/strokeRect/clearRect/translate/rotate/scale/save/restore` + 属性 `fillStyle/strokeStyle/lineWidth`；canvas 级 `toDataURL()`。绘制时机：组件函数内调用（事件/@effect 内），编译器保证 context 就绪。
- 三目标：React/OmniView `getContext('2d')` 直写；**Flutter 诚实降级注记**：CustomPaint + CustomPainter 重绘模型——命令式序列在画布上重放（paint() 内执行同一序列），运行时语义对等但非逐语句同步；动态绘制经 ValueNotifier 触发 repaint。此降级为显式注记，非静默（ADR-054）。

**② 文件上传（@filePick）**
- 意图：「触发系统文件选择，回传文件元数据」——选择是意图，上传是网络操作（数据域，不新增语义，研究 03 ⑬④）。
- 语法形态：`@filePick={onPick}` 元素指令（input[type=file] 或 button 均可）；handler 收 `{files: [{name, size, type, content}]}`（content 为文本文件内容，v0 限文本；二进制留生态）：

```
<input type="file" @filePick={handlePick} />
fn handlePick(files: File[]) { … }
```

- BNF 草案：`filepick_prop ::= '@filePick' '=' '{' <ident> '}'`；编译器：React 隐藏 input + click 代理 / Flutter file_picker 插件 / OmniView input[type=file] + File API。

**③ 摄像头/录音（@capture）**
- 意图：「打开摄像头拍照 / 录音」——权限模型为本归组的细化核心（基线矩阵 13「权限模型批次 spec 细化」）。
- 语法形态（低优先草案）：`@capture="camera"={onShot}` / `@capture="audio"={onRecorded}`，handler 收 `{data}`（URL/Blob）；**权限模型草案**：组件级 `@permission="camera"` / `@permission="microphone"` 声明（意图化权限），编译器承担：web getUserMedia 运行时询问 + secure context 降级硬诊断 / Flutter 静态声明（Info.plist/AndroidManifest）+ 运行时询问——静态声明需目标工程清单，属工程面注记。录音 Flutter 端无官方插件（研究 03 ⑭②）→ `@capture="audio"` Flutter 端诚实降级注记（第三方插件或未映射诊断）。

### 2.4 生命周期归组

**① @effect 强化（定时器/订阅清理）**
- 意图：「effect 作用域内的定时器/订阅在卸载时自动清理」——编译器生成 useEffect cleanup / State.dispose / OmniView 生命周期（基线裁决原文）。
- 语法形态：既有 `@effect (deps)? block` 不变，块内新增**生命周期托管函数**（仅可在 @effect 块内使用，否则语义硬错误）：

```
@effect {
    interval(1000, () => tick())          // 循环定时，effect 清理时自动取消
    timer(3000, () => autoClose())        // 单次定时，同上
    subscribe(TodoStore.count, (n) => onCount(n))   // 信号订阅，同上
}
```

- BNF 草案：`lifecycle_fn ::= 'timer' '(' expr ',' fn_ref ')' | 'interval' '(' expr ',' fn_ref ')' | 'subscribe' '(' sig_ref ',' fn_ref ')'`；fn_ref 为组件函数名或箭头（与事件 handler 不同——此处为表达式上下文，允许内联）。
- 与现有 @effect 关系：纯扩展（deps 语义、运行时机不变）；现有手写清理模式（Timer.omni 手动步进 + @unmount 清理）不破坏——@unmount 保留（显式清理），interval 等为自动托管形态，两者可并存（手动 = 显式；托管 = 声明式）。
- 与既有 std 关系：`std/async` 的 sleep 类保留（一次性异步睡眠非生命周期资源）；timer/interval 是新 stdlib 成员（std/timer）。

**② @store/@mount/@effect 入册（1B 项）**：语义已实现但未入册（基线矩阵 12），本归组给登记表格草案（§5 域 17）；@store 语法本体为顶层 `store_ref`/`store_inline`（grammar-view §1.7），入册时注记语法位。

## 3. Validation 清单（V1..Vn，错误消息草案中文）

| # | 级别 | 错误消息草案 | 触发 |
|---|---|---|---|
| V1 | 硬错误 | 「@reorder 只能用于 @for 容器上」 | reorder= 出现在非 @for 上下文 |
| V2 | 硬错误 | 「@reorder 的 handler 必须是组件函数名（禁止内联箭头/调用）」 | reorder= 值非 fn 名（同 @drag 约束） |
| V3 | 硬错误 | 「@reorder 与 @drag 不能在同一元素（重排与位移二选一）」 | 同元素两者兼有 |
| V4 | 硬错误 | 「@swipe/@longPress/@pinch 的 handler 必须是组件函数名」 | 手势 handler 非 fn 名 |
| V5 | 硬错误（迁移） | 「@onLongpress 已弃用——使用 @longPress 意图指令」 | @longPress 落地后旧事件形态 |
| V6 | 硬错误（迁移） | 「@onCopy 已弃用——使用 @copy 意图指令」 | @copy 落地后旧事件形态 |
| V7 | 硬错误 | 「@key 键位串语法错误：修饰符须为 ctrl/alt/shift/meta，键名不在语义键名枚举」 | 键位串非法 |
| V8 | 硬错误 | 「@key 键位串必须含至少一个修饰符（裸键请用 @onKeydown）」 | 裸键组合 |
| V9 | 硬错误 | 「@t 消息语法错误：花括号未闭合/占位符名缺失」 | ICU 子集词法错误 |
| V10 | 硬错误 | 「@t 消息不支持 offset/#/date/time——参数面留生态」 | 子集外语法 |
| V11 | 警告 | 「@t 参数 {n} 在消息中未使用」 / 「消息占位符 {n} 缺少参数」 | 参数与占位符不匹配 |
| V12 | 硬错误 | 「@locale 的值必须是裸信号引用」 | 非信号值 |
| V13 | 硬错误 | 「timer/interval/subscribe 只能在 @effect 块内使用」 | 生命周期托管函数越界 |
| V14 | 硬错误 | 「canvas ctx 方法不在白名单：{name}」 | RefOp ctx 方法名非法 |
| V15 | 硬错误 | 「@capture 需要 @permission 声明」 | 使用 @capture 未声明权限 |
| V16 | 硬错误 | 「@t 消息嵌套深度超过 3」 | 嵌套超限 |
| V17 | 警告（Flutter） | 「@capture="audio" Flutter 端无官方实现——未映射」 | 录音意图 Flutter 目标（诚实降级） |

## 4. 三目标映射表

| 能力 | React | Flutter | OmniView | 降级注记 |
|---|---|---|---|---|
| @for reorder | dnd-kit：DndContext + SortableContext + useSortable（onDragEnd → from/to） | `ReorderableListView` + onReorder（SDK 内置） | HTML DnD 排序合成（draggable + drop 判定） | 键盘重排 dnd-kit 内置 / Flutter 无内置——键盘可达性为编译器补足项（研究 03 边界注记） |
| @swipe | @use-gesture `onSwipe`（事实标准库） | `Dismissible`（滑动删除官方 cookbook）/ GestureDetector onPan 判定 | Pointer Events 位移阈值判定 | dir 判定阈值编译器统一；dismiss 语义 = 滑动超阈值触发回调，具体删除动作用户 handler |
| @longPress | use-gesture onLongPress | `onLongPress`（SDK 内置） | 指针按下计时判定 | — |
| @pinch | use-gesture onPinch | `onScale`（SDK 内置） | Pointer Events 多指距离比 | React 端经库选路（语言层统一是进语言动因，研究 03 ②④） |
| @copy | `navigator.clipboard.writeText`（点击 handler 内） | `Clipboard.setData`（services，SDK 内置） | `navigator.clipboard.writeText` | web 非 secure context → execCommand 兜底（编译器生成，注记） |
| @key | window keydown 监听 + 修饰符检查（React 官方惯用法） | `Shortcuts` + `Actions` + `Intent`（SDK 声明式，最地道——键位→意图→动作分离） | keydown 监听 + 修饰符检查 | Flutter 端编译器将键位串映射为 Shortcuts 绑定；行为对等 |
| @t | react-intl `FormattedMessage`/`formatMessage` + ICU 消息对象 | gen_l10n + ARB（官方工具链；dart intl 消息 ICU 风格） | 内部消息表 + Intl API | 三端同 ICU 子集（研究 03 最强点）；ARB 提取为编译器职责 |
| @locale | `IntlProvider locale`（react-intl） | `Localizations.override` + `supportedLocales` 声明 | locale 状态 + 消息重绑 | 动态刷新机制为编译器责任（provider 重建 / Localizations 重绑定） |
| formatDate | `Intl.DateTimeFormat(locale).format` | `DateFormat`（intl 包） | `Intl.DateTimeFormat` | 参数面（模式串/时区）留生态——硬诊断引导 |
| canvas | `<canvas>` + `getContext('2d')` 直写（官方 API） | CustomPaint + CustomPainter 重放模型 | `<canvas>` + ctx 直写 | **Flutter 降级注记**：命令式序列在 paint() 重放，非逐语句同步；动态绘制 ValueNotifier 触发 repaint |
| @filePick | 隐藏 `<input type="file">` + click 代理 + FileReader | `file_picker` 插件（第三方事实标准） | `<input type="file">` + File API | v0 限文本 content；二进制/多文件超集留生态 |
| @capture | `getUserMedia` + MediaRecorder（运行时询问） | `camera` 插件（官方）；录音无官方 | `getUserMedia` | 权限：web 运行时询问 + secure context 硬诊断；Flutter 静态声明为工程面注记；录音 Flutter 未映射警告（V17） |
| @effect 强化 | `useEffect` + cleanup（取消定时器/退订） | `State.dispose` / Timer.cancel（SDK 惯用法） | effect 生命周期 + 清理 | timer/interval/subscribe 编译器展开为各自清理通道 |

## 5. 登记册扩展

**新域登记（semantic-universe，延续基线 §8 治理的域 10-13 编号）：**

**域 14: 交互意图（interaction）**——语义词汇：`reorder`（@for 选项）/ `swipe` / `longPress` / `pinch` / `copy` / `key`。域定义：手势/重排/剪贴板/快捷键意图层（区别于域 3 事件的被动触发层——意图含编译器选路责任）。条目表草案列：词汇本体 | 语义（一句话） | 实现状态（待实施） | 诊断行为（V1-V8 引用）。白名单事实源列：`for_directive` reorder 分派（parser/view/for.rs）/ `parse_event_prop` 手势分派（element.rs，drag 分派先例 ~:681）/ `@copy` 分派 / `@key` 分派。

**域 15: 国际化（i18n）**——语义词汇：`@t` / `@locale` / `formatDate`。域定义：文案翻译与 locale 感知（ICU 消息子集 v0 封闭，子集外语法硬诊断 V10）。事实源列：表达式解析 @t 分派 / locale_prop / stdlib formatDate。

**域 16: 媒体（media）**——语义词汇：canvas（RefOp ctx 门类）/ `@filePick` / `@capture` / `@permission`。域定义：媒体元素意图与命令式绘制门类（元素本体 img/audio/video 归 HTML 元素层不入册）。事实源列：RefOp 方法白名单（`ref_op` 分派）/ filepick_prop / capture_prop。

**域 17: 生命周期（lifecycle，1B 入册）**——语义词汇：`@mount` / `@unmount` / `@effect` / `@store`（@store 注记：语法位 = 顶层 store_ref/store_inline，grammar-view §1.7）。域定义：组件生命周期意图与资源托管（timer/interval/subscribe 为 @effect 内子词汇）。事实源列：grammar-view §1.11 `lifecycle_hook` 产生式（现有）→ 入册后补 registry 交叉断言。

**vocabulary-registry 新词汇登记**（四字段：语义词汇 | 来源域 | 目标映射 | 排除词汇）：`reorder`（@for 选项，源 = 列表重排意图）｜`swipe/longPress/pinch`（源 = 手势意图；排除：swipeleft/swiperight 等平台旗标名不入词汇）｜`copy`（源 = 剪贴板意图；排除：clipboard）｜`key`（源 = 键位组合；排除：keydown/keyup/keypress 已登记事件）｜`t` / `locale` / `formatDate`（源 = ICU/CLDR）｜`filePick` / `capture` / `permission`（源 = 平台媒体权限）｜`timer` / `interval` / `subscribe`（源 = 生命周期托管）。

**verify 脚本锚点方案**：`verify-semantic-universe.py` 按域扩展 Task——域 14-17 各新增「词汇存在性断言」（reorder 在 for_directive 解析、手势/@copy/@key 在 parse_event_prop 分派、@t 表达式解析、formatDate stdlib 表、RefOp ctx 白名单、@mount/@effect 白名单交叉断言——域 7 既有断言 Task 2 模式复用）；`verify-vocabulary.py` 按新词汇行登记断言。

## 6. 语言边界与触发条件（本批次未进语言项）

| 项 | 状态 | 记录 | 触发条件 |
|---|---|---|---|
| 组件 memo | 🏞 编译器职责 | React Compiler 先例——用户不写意图，编译器自动优化（研究 03 ⑨）；对应 ROADMAP 1F cache 性能 | 编译策略调研启动时 |
| 懒加载/代码分割 | 🏞 编译器职责 | Flutter 无对等形态，跨目标不一致否决（研究 03 ⑩）；web-only 编译优化 | — |
| 虚拟化 | 🏞 后议（用户拍板） | 基线矩阵 5 显式「先成熟再谈」；研究 03 ⑪建议 @for 虚拟化属性（itemExtent/尺寸测量）——**不擅自翻案**，留此注记 | 语料出现长列表需求时重评 |
| tooltip | 🔜 低优先（批次 1 翻案注记） | 基线矩阵 8 已标 🔜；批次 1 未收（@modal/showToast 优先），本批次记录：属性级指令（双端映射存在——React 无标准 / Flutter Tooltip 内置），形态待独立批次细化 | 独立低优先批次 |
| 焦点陷阱 | 🔜 随 @modal（批次 1） | 模态附随行为，不单独立法（研究 03 ③④） | 批次 1 实施时 |
| 系统文件拖放 / 图片剪贴板 | 🏞 随剪贴板批次评估 | Flutter 无官方系统文件拖放 / Clipboard 仅文本——跨目标不一致（研究 03 ⑤④/清单遗漏 1/3） | 剪贴板批次实施时评估 |
| 复杂自定义手势 | 留生态 | 多指组合/边界判定——编译器意图价值低（研究 03 ②④） | 语料驱动 |
| 完整 i18n 工程 | 留生态 | locale 资源管理/回退/构建插件（研究 03 ⑥④） | 工程需求出现 |
| formatDate 参数面 | 留生态 | 模式串/时区/日历（基线裁决） | — |

## 7. 门禁与验证（五层）

1. **tsc**：React 生成物（dnd-kit/use-gesture/react-intl 接线、@copy 剪贴板）类型检查绿。
2. **dart analyze**：Flutter 生成物（ReorderableListView/Shortcuts/GestureDetector/CustomPaint 重放、gen_l10n 集成）零告警（V17 降级路径含编译期诊断验证）。
3. **Playwright**：浏览器行为断言清单——① @for reorder 拖拽后数据源顺序变化 + 重渲染；② @swipe 滑动触发 handler（dir 判定）；③ @copy 点击后剪贴板文本断言（clipboard.readText）；④ @key ctrl+s 触发 handler 且输入框内 ctrl+s 不冲突（焦点上下文）；⑤ @locale 切换后 @t 文案与 formatDate 输出同步变化；⑥ canvas 绘制断言（toDataURL 像素非空）；⑦ @filePick 选择文件回传元数据；⑧ @effect interval 组件卸载后无泄漏（定时器不再触发，console 断言）。
4. **docs-gate**：本 spec → grammar-view 生命周期/指令产生式更新 + 域 14-17 登记 + vocabulary-registry 同步（同 commit 全生态迁移硬规则）。
5. **注入语料（855 元组扩展）**：新增点——@t 插值/复数/选择元组（含子集外语法负例）、reorder schema（from/to）元组、手势 schema（dir/scale）元组、@key 键位串正负例、@effect 托管函数正负例、RefOp ctx 白名单正负例；V1-V17 每条至少 1 正 1 负。

## 8. demo 迁移清单（learn-omniview 用点）

| 语料位 | 变更 |
|---|---|
| `components/TodoList.omni`（@for key={item.id} 既有） | 加 `reorder={handleMove}`——拖拽重排 demo（重排后 store 顺序更新） |
| `components/Timer.omni`（手动步进 + @unmount 清理） | 保留原 demo（显式清理合法形态）；新增 ClockDemo：@effect { interval(1000, tick) } 自动托管形态对照 |
| `pages/DataFetching.omni` | 检查定时器类轮询手写——@effect + timer/interval 迁移用点（如无则跳过） |
| 新 demo：LanguagePage 拆分 | i18n 新 demo：@locale 切换 + @t 复数 + formatDate（复用现有 LanguagePage 页位或新增 i18n 页） |
| 新 demo：CanvasDrawDemo | <canvas @ref> + ctx 命令式绘制 |
| 新 demo：ShortcutDemo | @key="ctrl+s" 保存意图（TodoStore 保存接线） |
| 新 demo：SwipeDemo | 列表项 @swipe 滑动删除（TodoItem 接线） |
| 新 demo：ClipboardDemo | 分享链接 @copy 按钮 |

## 9. 未决问题（需主 agent/用户拍板，不擅自裁决）

- **P1**：@copy 是否需要内建复制成功反馈（toast 自动注出）——v0 草案无反馈（YAGNI），若用户要反馈则挂批次 1 showToast。
- **P2**：@key 采用极简两层（键位→handler）vs 研究 03 ④倾向的三层（键位→意图→动作，对齐 Flutter Intent 模型）——v0 草案极简，Flutter 编译器内部映射为 Intent；三层用户面语法需拍板。
- **P3**：@t 消息源 v0 内联字面量（ARB/键表提取为编译器职责）；是否 v0 就要外部键表（键引用 `@t("key")` + 消息文件）需拍板。
- **P4**：摄像头/录音权限模型——组件级 `@permission` 声明（草案）vs 编译器隐式生成（@capture 出现即生成权限请求）；Flutter 静态清单注入的工程面承接（poiesis 产物工程）需拍板。
- **P5**：@longPress/@copy 落地时事件域 longpress/copy 的硬化范围——硬化仅这两个 vs 同族（cut/paste）一并处理。
- **P6**：canvas Flutter 重放模型的保真度标准（paint() 内重放 vs 逐帧同步）——影响 Playwright 断言口径，需与实施批次对齐。
