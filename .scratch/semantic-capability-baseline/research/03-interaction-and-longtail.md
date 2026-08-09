# 03 研究 — 能力全景实证：交互/国际化/性能/媒体与文件

> 车票：[03-capability-landscape-interaction-and-longtail](../issues/03-capability-landscape-interaction-and-longtail.md)
> 日期：2026-08-09 ｜ 类型：research ｜ 目的：为三判据边界裁决（跨目标一致表达 + 编译器理解意图 + 真实语料需求）提供事实依据
> 方法：一手来源（官方文档/官方 npm registry 元数据/MDN）为主；WebSearch/WebFetch 在本环境被网络策略拦截，全部改用 curl 直连官方站点核实。引用 URL 见文末来源清单。

---

## 簇 A：交互（Interaction）

### 1. 拖拽排序

**① 标准形态**
- React：**[dnd-kit](https://docs.dndkit.com/)** 为事实标准。官方文档自述："The modern toolkit for building drag and drop interfaces"；"Batteries included: Drag, drop, sort, and reorder in any layout or direction"；"Fully extensible: Plugins, sensors, and modifiers"；"Production ready: Built for performance, accessibility, and reliability"。传感器支持 "mouse, touch, and keyboard"（键盘可达性内置）。新一代文档称 framework-agnostic：React/Vue/Svelte/Solid/TS 一等支持。老牌 react-dnd（HTML5 backend）仍在用但已非主流。
- Vue：**[vuedraggable](https://www.npmjs.com/package/vuedraggable)**（"draggable component for vue"，latest 标签 2.24.3 为 Vue 2，`next` 标签 4.1.0 支持 Vue 3）本质是 **[SortableJS](https://www.npmjs.com/package/sortablejs)**（"JavaScript library for reorderable drag-and-drop lists on modern browsers and touch devices"）的封装，v4.1.0 dependencies 即 sortablejs。
- Flutter：**SDK 内置** [ReorderableListView](https://api.flutter.dev/flutter/material/ReorderableListView-class.html)（material 库）："A list whose items the user can interactively reorder by dragging"。平台差异内置：desktop 平台自动加拖拽 handle，mobile 平台"long presses anywhere on the item it will start moving"（长按触发）。提供 proxyDecorator（拖拽中的代理视觉）、onReorder/onReorderItem 回调；要求"All list items must have a key"。低层还有 ReorderableList / ReorderableDragStartListener。

**② 生态化程度**：三端均有成熟标准形态（dnd-kit / vuedraggable+SortableJS / SDK 内置）→ 有标准库，**留生态候选**成立。

**③ 跨目标一致性**：意图「列表项可拖拽重排」三端语义一致且全部标准实现。差异在触发与细节：移动端长按 vs 桌面 handle（Flutter 内置自动区分；dnd-kit 需 sensors 配置）；拖拽视觉代理（Flutter proxyDecorator vs dnd-kit DragOverlay）；键盘重排（dnd-kit 内置 keyboard sensor，Flutter 无内置键盘重排）。**意图层对齐度：高**。

**④ 对 OmniLang 的启示**：**进语言候选**。意图「列表可重排」（`@for` + reorder 语义）跨目标一致、编译器理解明确（重排事件 = 源下标→目标下标），可映射 ReorderableListView / dnd-kit Sortable / vuedraggable。触发方式与视觉代理属于编译器选路细节，不应入语法。

### 2. 手势（swipe / pinch / 长按）

**① 标准形态**
- Web/React：**无框架内置手势系统**。平台层是 [Pointer Events](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events)（MDN："Pointer events address that need"——统一 mouse/pen/touch 输入）；React 无内置 onSwipe/onPinch，事实标准库为 **[@use-gesture/react](https://www.npmjs.com/package/@use-gesture/react)**（"React target for @use-gesture"，当前 10.3.1）。
- Vue：**[VueUse](https://vueuse.org/)** 官方函数列表含 useSwipe / usePointerSwipe / useDraggable / useLongPress 类（已核实在 vueuse.org/functions 页面存在 useSwipe、usePointerSwipe、useDraggable）。
- Flutter：**SDK 内置** [GestureDetector](https://api.flutter.dev/flutter/widgets/GestureDetector-class.html)（"A widget that detects gestures"，onTap/onLongPress/onPan/onScale 等）；官方 gestures 教程逐项定义 Long press（onLongPress："A pointer has remained in contact with the screen at the same location for a long period of time"）、Pan（onPanUpdate）等；swipe 删除有内置 [Dismissible](https://docs.flutter.dev/cookbook/gestures/dismissible)（官方 cookbook："Flutter makes this task easy by providing the Dismissible widget"）。缩放手势即 onScale。

**② 生态化程度**：Flutter 全内置；web 端无标准形态（依赖生态库或裸 Pointer Events）→ **半生态半语言候选**（这正是跨目标不对称点）。

**③ 跨目标一致性**：意图「滑动删除 / 长按弹出 / 双指缩放」语义两端一致（onLongPress ↔ onLongPress，Dismissible ↔ 滑动删除库）；实现层 Flutter 内置、React 端无标准 → 语义层可对齐、实现层不对称。

**④ 对 OmniLang 的启示**：**进语言候选（基本意图）**。`@swipe`（含 dismiss 语义）、`@longPress`、`@pinch` 是最简手势意图，三端语义一致且 Flutter 官方证明其为平台级稳定概念；React 端无标准库恰是进语言的动因（语言层统一 → 编译器选路 use-gesture / Pointer Events）。复杂自定义手势（多指组合、边界判定）留生态。

### 3. 焦点陷阱 / 焦点管理

**① 标准形态**
- React：**无内置**。[focus-trap](https://www.npmjs.com/package/focus-trap)（"Trap focus within a DOM node."，8.2.2）为事实标准（配套 focus-trap-react）；react-aria 提供 FocusScope 及完整可访问组件（React Aria 官方："Over 50 components with built-in behavior, adaptive interactions, top-tier accessibility, and internationalization out of the box"）。
- Vue：VueUse 提供 useFocusTrap（vueuse.org/functions 已核实）。
- Flutter：**SDK 内置** [FocusTraversalGroup](https://api.flutter.dev/flutter/widgets/FocusTraversalGroup-class.html)（"A widget that describes the inherited focus policy for focus traversal for its descendants, grouping them into a separate traversal group"）+ FocusScope/FocusNode。

**② 生态化程度**：React/Vue 靠生态、Flutter 内置 → 形态不对称。

**③ 跨目标一致性**：意图「模态打开时焦点圈定在对话框内、Tab 不逃逸」两端语义一致（focus-trap ↔ FocusScope+FocusTraversalGroup）。但这是**模态/对话框的附随行为**，几乎总是与 @modal/@dialog 同时出现，几乎从不单独表达。

**④ 对 OmniLang 的启示**：**随 @modal/@dialog 进语言**（编译器生成焦点圈定行为），**不单独立法**。三判据中「真实语料需求」几乎不会出现孤立焦点陷阱语法；它是模态语义的编译器责任。

### 4. 快捷键 / 键盘导航

**① 标准形态**
- React：**无内置**。react-aria useKeyboard/useKeyDown、hotkeys-js 等生态库。
- Vue：VueUse 提供 useHotkeys（官方 Browser 类目工具）。
- Flutter：**SDK 内置声明式系统**：[Shortcuts](https://api.flutter.dev/flutter/widgets/Shortcuts-class.html) + Actions + Intent——"A widget that creates key bindings to specific actions for its descendants"；官方文档强调其"separates key bindings and their implementations"，键位→Intent→Action 映射**可随焦点上下文适配**（同一删除键在文本框删字符、在文件列表删文件）。键盘导航另有 FocusTraversalGroup（见上）。

**② 生态化程度**：Flutter 内置且为声明式；web 端生态库 → 不对称。

**③ 跨目标一致性**：意图「Ctrl+S 触发保存」两端语义一致；Flutter 的 Shortcuts/Actions/Intent 三元组与语义 DSL 的「意图」模型高度同构（键位声明 → 意图 → 动作实现），React 端生态实现（react-aria 也走 useKeyDown + handler）。

**④ 对 OmniLang 的启示**：**进语言候选（低优先级）**。`@shortcut`/`@keymap` 意图清晰、Flutter 端存在天然声明式映射目标；但真实语料中快捷键多与编辑器类应用相关，需求密度待实证。倾向：若入语言，采用「键位→意图→动作」三层而非裸键位绑定（对齐 Flutter Intent 模型）。

### 5. 剪贴板 / 拖放文件

**① 标准形态**
- Web 剪贴板：[Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API) 平台 API（MDN："The Clipboard API provides the ability to respond to clipboard commands (cut, copy, and paste), as well as to asynchronously read from and write to the system clipboard"，secure context 限制）。
- Web 文件拖放：[HTML Drag and Drop API](https://developer.mozilla.org/en-US/docs/Web/API/HTML_Drag_and_Drop_API)（"HTML Drag and Drop interfaces enable applications to use drag-and-drop features in browsers"）+ [File API](https://developer.mozilla.org/en-US/docs/Web/API/File_API)（"The File API enables web applications to access files and their contents"）。React/Vue 无标准库，直接平台 API（VueUse useClipboard/useDropZone 为封装，已核实存在）。
- Flutter 剪贴板：**SDK 内置** [Clipboard](https://api.flutter.dev/flutter/services/Clipboard-class.html)（services 库："Utility methods for interacting with the system's clipboard"，setData/getData/hasStrings/kTextPlain）。
- Flutter 文件拖放：**无官方系统级实现**。app 内拖拽有 Draggable/DragTarget widget，但系统文件拖入桌面/Web 应用依赖第三方（desktop_drop 等，非官方）。

**② 生态化程度**：剪贴板文本两端原生；**系统文件拖放 Flutter 端无官方标准** → 不对称。

**③ 跨目标一致性**：复制/粘贴文本意图两端一致（Clipboard API ↔ Clipboard.setData）；**文件拖放不一致**（web 平台 API vs Flutter 无官方）→ 跨目标对齐度：剪贴板高、文件拖放低。

**④ 对 OmniLang 的启示**：剪贴板文本读写**进语言候选**（`@clipboard.copy`/`@clipboard.paste` 意图两端原生一致、编译器理解明确）；**文件拖放留生态**（跨目标不一致 + Flutter 端无标准映射目标）。

---

## 簇 B：国际化（i18n）

### 6. 文本翻译方案（ICU / MessageFormat / 工具链）

**① 标准形态**
- React：**[FormatJS / react-intl](https://formatjs.github.io/docs/react-intl/)** 为事实标准（npm 官方元数据："Internationalize React apps. This library provides React components and an API to format dates, numbers, and strings, including pluralization and handling translations."，当前 10.1.20）；底层 [intl-messageformat](https://www.npmjs.com/package/intl-messageformat)（"Formats ICU Message strings with number, date, plural, and select placeholders to create localized messages."，11.2.13）——**语法基座即 ICU MessageFormat**。另一主流为 react-i18next（"Internationalization for react done right. Using the i18next i18n ecosystem"，17.0.11，i18next 生态）。
- Vue：**[vue-i18n](https://vue-i18n.intlify.dev/)**（官方："Internationalization plugin for Vue.js"；"In addition to simple translation, support localization such as pluralization, number, datetime"）。
- Flutter：**官方内置工具链**：gen_l10n（l10n.yaml + ARB 文件，`generate: true` 配置，生成 AppLocalizations 类型安全类）+ [flutter_localizations](https://docs.flutter.dev/ui/internationalization) + [intl 包](https://pub.dev/packages/intl)（0.20.3，官方元数据："Provides internationalization and localization facilities, including message translation, plurals and genders, date/number formatting and parsing, and bidirectional text"）。ARB（App Resource Bundle）为资源源文件；dart intl 的消息占位符语法同样基于 ICU（plural/gender）。

**② 生态化程度**：三端全有标准形态 → **留生态候选**成立；但注意 Flutter 的"标准"是官方工具链而非第三方库。

**③ 跨目标一致性**：意图「文案翻译 + 复数/性别」三端语义一致；**且语法基座共同收敛于 ICU 消息语法**（react-intl 直接 ICU；vue-i18n 复数/数字；dart intl 消息同样 ICU 风格）→ 对齐度：高，且是生态中罕见的"三端同语法"点。

**④ 对 OmniLang 的启示**：**进语言候选**。`@t("...")`/message 意图跨目标一致 + 编译器可映射（react-intl / vue-i18n / gen_l10n+ARB）；若语言层直接采用 ICU 消息片段为语法子集，三端映射损耗最小。完整 i18n 工程（locale 资源管理、回退、构建插件）留生态。

### 7. 语言切换 / 动态文案

**① 标准形态**
- React：react-intl 的 `<IntlProvider locale>` 运行时切换。
- Vue：vue-i18n 的 `locale` ref 响应式切换。
- Flutter：intl 包全局 `Intl.defaultLocale`（官方："The package has a single current locale, called defaultLocale"），局部覆盖 `Intl.withLocale`（"the argument to withLocale will supercede the defaultLocale while the operation is active"）；UI 层官方机制为 `Localizations.override`（Flutter i18n 官方文档："Localizations.override is a factory constructor for the Localizations widget that allows for overriding the locale"）+ MaterialApp `supportedLocales`/`localizationsDelegates` 声明式配置。

**② 生态化程度**：三端均有标准机制；声明式（supportedLocales 列表）与命令式（全局 locale 状态）并存。

**③ 跨目标一致性**：意图「切换语言 → 全部文案/日期数字随 locale 更新」三端语义一致；locale 是全局状态，与状态管理域（@store 类）天然关联。

**④ 对 OmniLang 的启示**：**进语言候选（极简形态）或随状态域**。`@locale` 声明（supportedLocales + 当前值）两端可对齐；动态文案刷新机制（provider 重建 / Localizations 重绑定）是编译器责任。倾向：作为 @t 的配套能力随翻译语法一起裁决，不单独立法。

### 8. 日期数字格式化

**① 标准形态**
- Web：**ECMA-402 [Intl API](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl) 平台内置**（MDN："the ECMAScript Internationalization API, which provides language sensitive string comparison, number formatting, date and time formatting, and more"）；react-intl 的 FormattedDate/FormattedNumber 即其封装。
- Flutter：intl 包 [DateFormat / NumberFormat](https://pub.dev/packages/intl)（官方："This library also defines the DateFormat, NumberFormat, and BidiFormatter classes"）；locale 数据需异步初始化。

**② 生态化程度**：web 端是**语言/平台级标准**（浏览器内置），Flutter 端官方包 → 有标准形态。

**③ 跨目标一致性**：意图「按 locale 格式化日期/数字」两端一致；且两端数据同源（CLDR）。但格式化参数面极大（模式串、日历、时区、小数位、货币…），语义层只适合表达最简意图。

**④ 对 OmniLang 的启示**：**留生态候选**。编译器对「把 DateFormat 模式串 / Intl.DateTimeFormat 选项翻译成 DSL」的价值低（参数爆炸、映射损耗 > 收益）；除非真实语料出现高频最简形态（如 `formatDate(locale)`），否则不进语言。

---

## 簇 C：性能（Performance）

### 9. 组件 memo（React.memo / useMemo / Flutter const）

**① 标准形态**
- React：React.memo / useMemo / Suspense 均为[官方 API](https://react.dev/reference/react/lazy)（react.dev 19.2 reference 确认）；且官方已推出 **React Compiler**（react.dev React Compiler 配置项存在）——自动 memo 化正在成为编译器责任。
- Vue：computed 自动依赖追踪，无需 memo。
- Flutter：官方[性能最佳实践](https://docs.flutter.dev/perf/best-practices)："Use `const` constructors on widgets as much as possible, since they allow Flutter to short-circuit most of the rebuild work"；"Avoid using constructors with a concrete List of children ... if most of the children are not visible"（引导用 builder 构造）。

**② 生态化程度**：三端均为框架内置能力/惯例 → 有标准形态。

**③ 跨目标一致性**：意图「避免无谓重渲染」三端一致，但机制各异（memo/不可变引用 ↔ 自动依赖追踪 ↔ const 编译期常量）。**关键事实：三端都在把它变成编译器/框架责任**（React Compiler 先例 + Flutter const lint 提示）。

**④ 对 OmniLang 的启示**：**不进用户语法**。memo 是编译器/框架优化领域（对应 ROADMAP 1F cache 性能、codegen 自动 const/memo 生成）；若 OmniLang 做性能语义，方向是"编译策略"而非"用户语法"。三判据中「编译器理解意图」成立但「真实语料需求」低（用户几乎不会手写 memo 意图）。

### 10. 懒加载 / 代码分割

**① 标准形态**
- React：**[React.lazy + Suspense](https://react.dev/reference/react/lazy)** 官方 API + 动态 `import()`。
- Vue：defineAsyncComponent + `<Suspense>` 官方。
- Flutter：**无代码分割**（单二进制）。Dart 语言级 [deferred loading](https://dart.dev/language/libraries)（"Deferred loading (also called lazy loading) allows a web app to load a library on demand"）——官方明示"The dart tool doesn't support deferred loading for targets other than web"；Flutter 侧只有 Android deferred components（专项、低频）。

**② 生态化程度**：web 端框架内置标准；Flutter 端无对等形态 → **不对称**。

**③ 跨目标一致性**：**低**。"路由级懒加载"在 React/Vue 是标配实践，Flutter 无通用对应 → 同一意图无法三端对齐表达。

**④ 对 OmniLang 的启示**：**不进语言**（跨目标不一致为否决性判据）；React/Vue 端由编译器策略生成 lazy/asyncComponent（web-only 优化），Flutter 端无操作。若未来补，也只作为 web 端编译优化而非语义语法。

### 11. 列表虚拟化

**① 标准形态**
- React：**[TanStack Virtual](https://tanstack.com/virtual/latest/docs/introduction)**（官方："a headless UI utility for virtualizing long lists of elements in JS/TS, React, Vue, Svelte, Solid, Lit, and Angular"）为当前主流；老牌 react-window / react-virtualized。注意其 headless 形态：需要测量元素尺寸、自行渲染容器。
- Vue：TanStack Virtual / VueUse useVirtualList（已核实官方函数列表）。
- Flutter：**SDK 内置**。ListView.builder（[ListView 官方文档](https://api.flutter.dev/flutter/widgets/ListView-class.html)："the builder is called only for those children that are actually visible"，itemExtent/prototypeItem 提供尺寸预知优化）——虚拟化是 Sliver 滚动框架的内置能力，无需第三方。

**② 生态化程度**：Flutter 内置；React 端第三方（headless 库）→ 半对称。

**③ 跨目标一致性**：意图「长列表只构建可见项」两端语义一致（ListView.builder ↔ TanStack Virtual）；Flutter 端是框架默认（builder 构造即虚拟化），React 端需显式引入库并处理尺寸测量 → 语义层对齐度高、实现成本不对称（编译器可消化）。

**④ 对 OmniLang 的启示**：**进语言候选（@for 的虚拟化属性）**。`@for` + 虚拟化/固定尺寸意图 → Flutter 选 ListView.builder(+itemExtent)、React 选 TanStack Virtual/react-window；尺寸测量这类 headless 配置复杂度正是编译器价值所在。需真实语料确认虚拟化需求密度。

---

## 簇 D：媒体与文件（Media & Files）

### 12. Canvas

**① 标准形态**
- Web：[Canvas API](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API) 平台 API（MDN："The Canvas API provides a means for drawing graphics via JavaScript and the HTML `<canvas>` element... animation, game graphics, data visualization, photo manipulation, and real-time video processing"）。React 无标准库（react-konva、fabric.js 等生态各自为政）。
- Flutter：**SDK 内置** [CustomPaint](https://api.flutter.dev/flutter/widgets/CustomPaint-class.html)（"A widget that provides a canvas on which to draw during the paint phase"，painter 实现 CustomPainter 子类）+ dart:ui Canvas。

**② 生态化程度**：两端均为**命令式底层 API**（web 平台 API / Flutter 内置）；React 端无事实标准封装。

**③ 跨目标一致性**：意图「声明式绘制图形」——两端本质是命令式绘制上下文（moveTo/lineTo/fillRect ↔ drawLine/drawRect），API 风格差异大；画布内容通常由数据驱动代码生成而非声明式 UI 树表达 → 语义层对齐价值低。

**④ 对 OmniLang 的启示**：**不支持/留生态**。canvas 是命令式低层能力，声明式 DSL 化损耗大（映射到两套不同的绘制上下文），且真实语料中"画布绘制"需求由组件库/数据可视化库（konva/fabric/echarts）承接。

### 13. 文件上传（选择文件 / 上传）

**① 标准形态**
- Web：`<input type="file">` + [File API](https://developer.mozilla.org/en-US/docs/Web/API/File_API)（"The File API enables web applications to access files and their contents"）+ fetch/XHR 上传；uppy、dropzone 等封装库。
- Flutter：官方插件 [image_picker](https://pub.dev/packages/image_picker)（1.2.3，flutter.dev 官方维护："Flutter plugin for selecting images from the Android and iOS image library, and taking new pictures with the camera"，支持 Android/iOS/Linux/macOS/Web/Windows）；任意文件用第三方 file_picker；上传本身是网络操作（http/dio，属数据域）。

**② 生态化程度**：web 端平台 API、Flutter 端官方插件 → 有标准形态。

**③ 跨目标一致性**：意图「选择文件/图片」两端有（input file ↔ image_picker/file_picker）；「上传」属网络域（与 @store/@http 数据语义重叠）。

**④ 对 OmniLang 的启示**：**留生态**。文件选择是平台能力直通（意图价值低），上传与既有数据域重叠（不新增语义）；至多 `@file.pick` 极简意图，非优先。

### 14. 摄像头 / 录音

**① 标准形态**
- Web：[MediaDevices.getUserMedia](https://developer.mozilla.org/en-US/docs/Web/API/MediaDevices/getUserMedia)（MDN："The getUserMedia() method of the MediaDevices interface prompts the user for permission to use a media input which produces a MediaStream"，Baseline Widely available、secure context 限定）+ [MediaRecorder](https://developer.mozilla.org/en-US/docs/Web/API/MediaRecorder)（"The MediaRecorder interface of the MediaStream Recording API provides functionality to easily record media"）。
- Flutter：官方插件 [camera](https://pub.dev/packages/camera)（0.12.0+2："A Flutter plugin for iOS, Android and Web allowing access to the device cameras"；预览/拍照/录像/图像流，Android/iOS/web）；**录音无官方插件**（record 等第三方）。

**② 生态化程度**：摄像头两端有标准（平台 API vs 官方插件）；录音 Flutter 端无官方 → 半对称。权限声明（iOS Info.plist / AndroidManifest）是两端强绑定流程。

**③ 跨目标一致性**：意图「打开摄像头/拍照/录像」两端一致；录音不一致（Flutter 无官方）。权限模型差异大（web 运行时询问 vs 原生静态声明+运行时询问）。

**④ 对 OmniLang 的启示**：**留生态**。硬件/权限深度绑定，编译器意图价值低（语义只是"打开摄像头"一行，背后是权限、预览 UI、生命周期）；且录音跨目标不一致。不进语言（至多极简 @camera capture 意图，视真实语料）。

---

## 跨目标一致性总览（进语言判据输入）

| 能力 | 标准形态 | 生态化 | 跨目标一致性 | 初步倾向 |
|---|---|---|---|---|
| 拖拽排序 | dnd-kit / vuedraggable(SortableJS) / ReorderableListView 内置 | 三端有标准 | 高（触发细节两端不同） | **进语言候选**（@for+reorder 意图） |
| 手势（swipe/长按/scale） | React 无标准（use-gesture）/ Flutter GestureDetector+Dismissible 内置 | 不对称 | 高（基本意图） | **进语言候选**（@swipe/@longPress/@pinch 最简意图） |
| 焦点陷阱 | focus-trap / react-aria / FocusTraversalGroup 内置 | 不对称 | 高（但附随模态） | **随 @modal 进语言**，不单独立法 |
| 快捷键/键盘导航 | React 生态 / Flutter Shortcuts+Actions 内置（声明式） | 不对称 | 高 | **进语言候选（低优先）**，三层键位→意图→动作 |
| 剪贴板/文件拖放 | Clipboard API / HTML DnD / Clipboard 内置；Flutter 无系统文件拖放 | 剪贴板对称、拖放不对称 | 文本高、文件拖放低 | 剪贴板**进语言候选**；文件拖放留生态 |
| 文本翻译 | react-intl(ICU) / vue-i18n / gen_l10n+ARB(官方) | 三端有标准 | **高（共同 ICU 消息语法基座）** | **进语言候选**（@t；工程留生态） |
| 语言切换 | IntlProvider / vue-i18n locale / Intl.defaultLocale+Localizations.override | 三端有标准 | 高 | 随 @t/状态域，不单独立法 |
| 日期数字格式化 | ECMA-402 Intl 平台内置 / DateFormat/NumberFormat | 有标准（web 是语言级） | 高（但参数面爆炸） | **留生态候选**（编译器映射价值低） |
| memo | React.memo/React Compiler / Flutter const 惯例 | 框架内置 | 机制各异，且正转为编译器责任 | **不进用户语法**（编译策略） |
| 懒加载/代码分割 | React.lazy+Suspense / defineAsyncComponent / Flutter 无 | 不对称 | **低（Flutter 无对等）** | **不进语言**（web-only 编译优化） |
| 列表虚拟化 | TanStack Virtual / VueUse useVirtualList / ListView.builder 内置 | 半对称 | 高 | **进语言候选**（@for 虚拟化属性） |
| canvas | Canvas API 平台 / CustomPaint 内置 | 两端命令式 | 低（声明式化价值低） | **不支持/留生态** |
| 文件上传 | input file+File API / image_picker 官方插件 | 有标准 | 中（与数据域重叠） | **留生态** |
| 摄像头/录音 | getUserMedia+MediaRecorder / camera 官方插件（录音无官方） | 半对称 | 中（录音不一致） | **留生态**（权限/硬件强绑定） |

**汇总规律**：① 交互簇「意图简单 + 三端有稳定语义」的项（拖拽重排、基本手势、剪贴板文本）是进语言的主力候选；② 国际化簇翻译语法是唯一「三端同语法基座（ICU）」的项，进语言损耗最小；③ 性能簇整体是编译器/框架责任而非用户语法（React Compiler 先例）；④ 媒体簇几乎全部留生态——平台 API 直通 + 权限/硬件绑定，声明式化价值低。

## 清单对照

基线清单覆盖检查（拖拽排序/手势/焦点陷阱/快捷键/剪贴板/翻译/语言切换/日期数字格式化/memo/懒加载/虚拟化/canvas/文件上传/摄像头）：

- ✅ 已覆盖且核实：全部 14 项均有官方一手来源（见各节）。
- ⚠️ **清单遗漏项 1：系统级文件拖放（drop zone）**——「拖拽排序」之外的文件拖入（桌面文件拖进 web 页面）是独立能力：web 端 HTML DnD+DataTransfer 平台 API 成熟，Flutter 端无官方实现（第三方 desktop_drop）→ **跨目标不一致**，应单列于清单（倾向：留生态）。
- ⚠️ **清单遗漏项 2：音视频播放**——「媒体」簇只列了 canvas/摄像头/录音，缺播放能力：web `<video>/<audio>` 平台元素 vs Flutter 官方插件 [video_player](https://pub.dev/packages/video_player)（first-party）→ 有标准形态、跨目标中高一致，建议补入媒体簇清单（倾向：留生态或极简 @video 意图）。
- ⚠️ **清单遗漏项 3：图片/富媒体剪贴板**——Clipboard API 支持异步 ClipboardItem（图片等），Flutter 官方 Clipboard 仅文本 → 跨目标不一致，属剪贴板项的边界注记。
- ⚠️ **清单遗漏项 4：屏幕朗读语义（ARIA/Semantics）**——焦点管理仅是「无障碍」维的子集；14 维清单已单列「无障碍」维，本车票确认：焦点陷阱/键盘导航归属本簇，其余无障碍语义由无障碍维车票覆盖，无重复。
- ℹ️ 边界注记：拖拽排序的键盘可达性（dnd-kit 内置 keyboard sensor vs Flutter 无内置键盘重排）是交互×无障碍交叉点，入语言设计时需编译器补足。

## 来源清单（一手）

- dnd-kit 官方文档（docs.dndkit.com，2026-08 抓取）
- Flutter API 参考：ReorderableListView / GestureDetector / FocusTraversalGroup / Shortcuts / Clipboard / ListView / CustomPaint（api.flutter.dev）
- Flutter 官方指南：Internationalization（docs.flutter.dev/ui/internationalization，含 gen_l10n/l10n.yaml/ARB）、Performance best practices（docs.flutter.dev/perf/best-practices，const 建议）、Implement swipe to dismiss（docs.flutter.dev/cookbook/gestures/dismissible）、Gestures 教程（docs.flutter.dev/ui/interactivity/gestures）
- pub.dev 官方包页：intl 0.20.3、camera 0.12.0+2、image_picker 1.2.3
- npm registry 官方元数据（registry.npmjs.org）：react-intl 10.1.20、intl-messageformat 11.2.13、react-i18next 17.0.11、vuedraggable（latest 2.24.3 / next 4.1.0）、sortablejs 1.15.7、@use-gesture/react 10.3.1、react-aria 3.51.0、focus-trap 8.2.2、react-window 2.3.0、@tanstack/react-virtual 3.14.9
- MDN：Clipboard API、HTML Drag and Drop API、File API、Canvas API、Pointer events、MediaDevices.getUserMedia、MediaRecorder、Intl（ECMA-402）
- 其它官方：react.dev（React 19.2 reference：lazy/memo/Suspense）、vue-i18n.intlify.dev、vueuse.org（functions 列表）、tanstack.com/virtual、react-spectrum.adobe.com/react-aria、dart.dev/language/libraries（deferred loading）

> 注：环境网络策略拦截 WebSearch/WebFetch，以上全部经 curl 直连官方站点核实原文；docs.dndkit.com 与格式相关页面（formatjs.io）原域名已迁移（formatjs.github.io），react-intl 元数据以 npm registry 官方字段为准。
