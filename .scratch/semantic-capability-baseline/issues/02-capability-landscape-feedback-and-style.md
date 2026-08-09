# 02 — 能力全景实证：反馈/渲染/样式

Type: research
Status: resolved
Blocked by:

## Question

主流前端框架与生态在**反馈与模态、渲染组合、样式**三簇能力的全景形态是什么？为边界裁决（三判据）提供事实依据。

覆盖能力项（基线清单草稿）：
- 反馈：toast/通知（形态、kind/duration、堆叠）、modal/dialog（焦点陷阱/Esc/遮罩）、tooltip、骨架屏
- 渲染：空状态、错误边界、虚拟滚动、懒加载、keep-alive
- 样式：复杂选择器（兄弟/属性/:nth-child/后代伪类）、用户级 @media 断点、容器查询、CSS 作用域/模块化、深色模式

## 产出要求

高可信一手来源（官方文档优先）——不写二手博客。产出 `.scratch/semantic-capability-baseline/research/02-feedback-and-style.md`，每项能力记录：
1. 主流框架的标准形态（谁是事实标准：Headless UI/Radix/dnd-kit…；CSS Modules/Tailwind/vanilla-extract…）
2. 生态化程度（有标准库=留生态候选；无标准形态=进语言候选）
3. 跨目标一致性（同一意图在 React/Flutter 两端能否对齐——进语言判据的输入；如 dialog：React 原生 `<dialog>` vs Flutter showDialog）
4. 对 OmniLang 的启示（1-2 句：进语言/留生态/不支持的初步倾向 + 依据）

## 解析即

研究完成 → 车票加 `## Answer`（要点 + 产出文件链接）+ `Status: resolved` + 地图 Decisions-so-far 追加一行。

## Answer

14 项能力全实证（一手官方文档 + MDN BCD 基线验证，产出 `research/02-feedback-and-style.md`）：**进语言候选 4 项**——modal/dialog（双端平台级标准对齐：原生 `<dialog>.showModal` vs `showDialog`，焦点/Esc/遮罩均内建）、toast/通知（React 无标准形态靠 Sonner/Radix、Flutter 无 toast 只有 SnackBar → 命令式意图可映射）、深色模式（prefers-color-scheme/Tailwind dark 变体 vs ThemeMode，且是现有 token 体系的自然扩展）、错误边界（弱——React/Vue 可对齐但 Flutter 无组件级标准，须诚实降级）；**@load 形态扩展 1 项**——骨架屏（三端无标准，作域 9 @load 的 loading 视觉形态）；**留生态 9 项**——tooltip/空状态/虚拟滚动/懒加载/keep-alive/复杂选择器/@media 断点/容器查询/样式模块化（样式四能力共性：CSS 原生语法面透传即达，进语言=重造 CSS；虚拟滚动/懒加载与车票 01 性能类裁决一致）。清单无遗漏；补充发现 Svelte 过渡体系为形态参数非独立能力。
