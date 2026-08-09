# 11 — 批次 6 域裁决：长尾能力

Type: grilling
Status: resolved
Blocked by: 04, 05

## Question

**长尾能力簇**的边界裁决与设计方向——按 05 定稿的裁决模板执行（其余批次未覆盖项的收容批次）。

覆盖能力项：
- 渲染组合：骨架屏、空状态、错误边界、虚拟滚动
- 交互：拖拽排序（@drag 延伸）、手势（swipe/pinch/长按）、焦点陷阱、快捷键
- 国际化：文本翻译、语言切换、日期数字格式化（#85 触发条件重评估）
- 性能：组件 memo、懒加载
- 生命周期：@effect 强化（定时器/订阅清理）、@mount/@unmount 入册（1B 项）
- 媒体与文件：canvas、文件上传、摄像头

## 决议内容

1. 逐项边界裁决（进语言/留生态/不支持 + 三判据证据；研究 03 的生态全景输入）
2. 入语言项的 1-2 句设计方向
3. **本批次的内部排期**：裁决为「进语言」的项如何归组（可能拆分为多个阶段 spec——本批次是收容批次，非单阶段承诺）
4. 明确「留生态」项的文档承接（语言边界记录 + 目标生态映射指引）

## 解析即

决议记录 `## Answer` → `Status: resolved` → 地图 Decisions-so-far 追加一行 → 入语言项归组成未来阶段（各自独立 spec→plan→SDD）。

## Answer

**批次 6（长尾能力）域裁决定案**（2026-08-09，校准后标准——全部进语言，按批次归组，语料只影响排期）。

| 能力项 | 结论 | 三目标映射方向 | 归组 |
|---|---|---|---|
| 拖拽重排 | 🔜 进语言（@for reorder 或 @reorder 指令） | dnd-kit / ReorderableListView | 交互批次 |
| 基本手势 | 🔜 进语言（@swipe/@longPress/@pinch 指令） | GestureDetector 双端内建 | 交互批次 |
| 剪贴板 | 🔜 进语言（@copy 意图或 stdlib） | Clipboard API / services Clipboard | 交互批次（低优先） |
| 快捷键 | 🔜 进语言（@key 组合快捷键意图） | React keydown 处理 / Flutter Shortcuts+Actions | 交互批次（低优先） |
| @t 文本翻译 | 🔜 进语言（**ICU 消息语法子集** + 语言切换 @locale——三端同基座，研究 03 最强点） | react-intl / vue-i18n / gen_l10n+ARB | i18n 批次 |
| 日期数字格式化 | 🔜 stdlib 最简形态（formatDate(locale)）——参数面（模式串/时区）留生态 | Intl API / intl 包 | i18n 批次 |
| 组件 memo | 🏞 编译器职责（React Compiler 先例——用户不写意图，编译器自动优化） | — | — |
| canvas | 🔜 进语言（低优先——命令式绘制意图，RefOp 扩展或元素指令） | canvas 2D / CustomPaint | 媒体批次 |
| 文件上传 | 🔜 进语言（低优先——input file 意图） | input[type=file] / file_picker 插件 | 媒体批次 |
| 摄像头/录音 | 🔜 进语言（低优先——权限模型批次 spec 细化） | mediaDevices.getUserMedia / camera 插件 | 媒体批次 |
| @effect 强化（定时器/订阅清理）+ @store/@mount/@effect 入册 | 🔜 进语言（生命周期批次——1B 项并入） | useEffect cleanup / State.dispose | 生命周期批次 |

**归组终稿**：交互批次（拖拽重排/手势/剪贴板/快捷键）、i18n 批次（@t/@locale/格式化）、媒体批次（canvas/文件/摄像头）、生命周期批次（@effect 强化 + 1B 入册）。各归组独立 spec→plan→SDD（可并行或串行，排序见 12）。
