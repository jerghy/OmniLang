# 03 — 能力全景实证：交互/国际化/性能/媒体

Type: research
Status: resolved
Blocked by:

## Question

主流前端框架与生态在**交互、国际化、性能、媒体与文件**四簇能力的全景形态是什么？为边界裁决（三判据）提供事实依据。

覆盖能力项（基线清单草稿）：
- 交互：拖拽排序（dnd-kit/HTML5 DnD/Flutter ReorderableListView）、手势（swipe/pinch/长按）、焦点陷阱/焦点管理、快捷键/键盘导航、剪贴板/拖放文件
- 国际化：文本翻译方案（ICU/MessageFormat/react-intl/Flutter intl）、语言切换、日期数字格式化
- 性能：组件 memo（React.memo/Flutter const）、懒加载/代码分割、列表虚拟化
- 媒体与文件：canvas（React canvas 形态 vs Flutter CustomPaint）、文件上传、摄像头/录音（浏览器 API vs Flutter 插件）

## 产出要求

高可信一手来源（官方文档优先）——不写二手博客。产出 `.scratch/semantic-capability-baseline/research/03-interaction-and-longtail.md`，每项能力记录：
1. 主流框架的标准形态（谁是事实标准库）
2. 生态化程度（有标准库=留生态候选；无标准形态=进语言候选）
3. 跨目标一致性（同一意图在 React/Flutter 两端能否对齐——进语言判据的输入）
4. 对 OmniLang 的启示（1-2 句：进语言/留生态/不支持的初步倾向 + 依据）

## 解析即

研究完成 → 车票加 `## Answer`（要点 + 产出文件链接）+ `Status: resolved` + 地图 Decisions-so-far 追加一行。

## Answer

14 项能力全部一手来源实证（官方文档/npm registry/MDN，curl 直连核实）。**进语言候选 6**：拖拽重排（@for+reorder 意图，三端有标准）、基本手势 @swipe/@longPress/@pinch（Flutter 内置证明平台级概念、React 端无标准恰是进语言动因）、剪贴板文本、文本翻译 @t（唯一三端同语法基座：ICU 消息语法）、语言切换随 @t、列表虚拟化（@for 虚拟化属性）；**随 @modal 进语言 1**：焦点陷阱（附随行为，不单独立法）；快捷键为低优先进语言候选（Flutter Shortcuts/Actions/Intent 声明式同构）。**留生态/不进用户语法 7**：日期数字格式化（参数面爆炸）、memo（编译器责任，React Compiler 先例）、懒加载（跨目标不一致——Flutter 无对等，否决性判据）、canvas（命令式底层）、文件上传、摄像头/录音（权限/硬件强绑定）。清单对照：补 4 项遗漏（系统文件拖放 drop zone、音视频播放、图片剪贴板、屏幕朗读语义归属确认）。

产出：[research/03-interaction-and-longtail.md](research/03-interaction-and-longtail.md)
