# 语义级语法能力成熟路线图（wayfinder map）

> Effort: `semantic-capability-baseline` — 制图于 2026-08-09（brainstorming 会话，用户拍板 3 决策）
> 车票是子 issue（`.scratch/semantic-capability-baseline/issues/`）；决议在车票、地图只索引。

## Destination

**《语义级语法能力基线》方向级大 spec 定稿**——「真实复杂前端项目能力全景」× 逐项边界裁决 × 入语言缺口设计方向 × 优先级排序与阶段批次划分。

具体形态（用户 2026-08-09 拍板）：
- 能力全景 = 14 维能力清单终稿（状态/异步/路由/表单/渲染/交互/响应式/反馈/国际化/无障碍/性能/生命周期/媒体文件/样式）
- 边界裁决 = 三判据「跨目标一致表达 + 编译器理解意图 + 真实语料需求」→ 进语言 / 留生态 / 明确不支持
- 设计方向 = 每个入语言缺口 1-2 句方向描述（遵循语义意图优先 → 编译器选路 → 地道目标 codegen 哲学：ADR-054 原则 1 + ADR-071 编译器责任模式）
- 阶段批次 = 按能力批次切，每阶段独立走 spec → plan → subagent-driven
- 基线 spec = 语义全覆盖收官（ROADMAP 1B-1F 重排依据）+ 冻结 ADR 的前置文档

地图完成 = 所有车票决议完毕、路线清晰——能力基线大 spec 可以撰写。之后进入阶段执行（每阶段独立 spec→plan→SDD 循环）。

## Notes

- **领域**：OmniLang 语义级 DSL（OmniView 语义语法）；三目标 React/Flutter/OmniView codegen；五层验证网（tsc/dart analyze/Playwright/docs-gate/注入语料 855 元组）
- **每次会话必读**：`docs/superpowers/specs/semantic-universe.md`（10 语义域登记册）、`vocabulary-registry.md`（词汇来源）、`2026-08-08-semantic-design-coverage-design.md`（域 B/C/D 契约草案）、ROADMAP.md 阶段 1
- **技能**：/grilling、/domain-modeling、/omnilang-usage、omniview-coding-standards、/writing-plans、/subagent-driven-development
- **语言偏好**：中文交流；文档中文
- **常驻偏好**：
  - 三判据边界裁决（用户拍板）；方向级粒度（详细语法设计留给各阶段 spec）
  - 语义化即切换、不留旧残留（ADR-075 原则 8）；诚实降级 > 静默错误（ADR-054）
  - 新增语法先登记 semantic-universe → 才进 BNF/白名单（ADR-074 纪律）
  - 手写体感优化不在范围（poiesis 是作者——用户拍板；语法可表达性是范围）
  - 研究产出落盘 `.scratch/semantic-capability-baseline/research/`（仓库 main-only 约定，不用分支）

## Decisions so far

<!-- 每决议一行：车票名（链接）+ 一句话要点；决议只存车票，地图不重述 -->
- [01 — 能力全景实证：数据/异步/路由](issues/01-capability-landscape-data-and-routing.md) — 16 项全实证：仅加载三态已进语言（域 9 @load 先例）；路由切换/参数/返回进语言（域 C 方向定）、深链进语言+降级边界（floor 界定核心）；守卫/嵌套/轮询/乐观为候选；分页/缓存/竞态/懒加载留生态
- [02 — 能力全景实证：反馈/渲染/样式](issues/02-capability-landscape-feedback-and-style.md) — 14 项全实证：进语言候选 4（modal/dialog 双端原生对齐、toast 命令式意图、深色模式 token 扩展、错误边界弱候选取舍留批）；骨架屏作 @load 形态扩展；留生态 9（样式四能力 CSS 透传、tooltip/空状态/keep-alive 及性能类与 01 裁决一致）
- [03 — 能力全景实证：交互/国际化/性能/媒体](issues/03-capability-landscape-interaction-and-longtail.md) — 14 项全实证：进语言候选 6（拖拽重排 @for+reorder、基本手势 @swipe/@longPress/@pinch、剪贴板文本、文本翻译 @t——唯一三端 ICU 同语法基座、语言切换随 @t、列表虚拟化）；焦点陷阱随 @modal 进语言；快捷键低优先候选；留生态/不进语法 7（日期格式化、memo 编译器责任、懒加载跨目标不一致否决、canvas、文件上传、摄像头/录音）；清单补 4 遗漏项
- [04 — 能力清单终稿（14 维定案）](issues/04-capability-list-final.md) — 14 维清单终稿定案：样式维加「深色模式」（已支持——design @themes 实证）；虚拟化/表单深化用户拍板后议（先成熟再谈）；相邻能力 12 项全入清单；状态图例 ✅/🔜/⏳/🏞
- [05 — 边界裁决细则与语言完整性原则](issues/05-boundary-ruling-and-integrity-principles.md) — 裁决细则定稿：三判据判例 + 四段裁决模板；半实现信号=标 v0 边界随批次补齐；留生态=登记册状态值域扩展+文档指引；类型检查归 1F
- [06 — 批次 1 域裁决：反馈与模态](issues/06-stage-scope-feedback-modal.md) — @modal+showToast 进语言（方向级）；焦点陷阱随 @modal；骨架屏随 @load；错误边界留生态（真人写法测试：Flutter 无组件级惯用法，@load error 已覆盖数据级）；tooltip 留生态；真人写法审计 3 细化点
- [07 — 批次 2 域裁决：导航与路由](issues/07-stage-scope-navigation-routing.md) — 标准校准：前端功能跨框架通用、语料只影响排期、留生态收窄为机制语法面+编译器职责；域 C 全量进语言（切换/参数/返回/深链进 floor/守卫 v0 含/嵌套独立语法）
- [08 — 批次 3 域裁决：响应式断点](issues/08-stage-scope-responsive-breakpoints.md) — bp() 逻辑断点+breakpoint token 进语言；@media 机制语法面透传+Flutter 映射策略批次细化；容器查询低优先候选
- [09 — 批次 4 域裁决：数据与异步能力](issues/09-stage-scope-data-pagination.md) — @load 深化（多例/参数/Reset/分页/轮询/无限滚动触发）+ DataFetching 迁移定案；乐观更新远期；防抖节流 stdlib；缓存策略配置面留生态
- [10 — 批次 5 域裁决：样式系统](issues/10-stage-scope-styling-selectors.md) — 选择器静默跳过缺陷修复定案（诊断先行）；复杂选择器机制语法面透传；深色模式/作用域已支持
- [11 — 批次 6 域裁决：长尾能力](issues/11-stage-scope-longtail-capabilities.md)
- [12 — 批次排序、验收形态与基线落盘](issues/12-stage-priority-and-acceptance.md) — 收口定案：批次排序 5 批次（反馈模态→域C→响应式+样式→数据→长尾归组）；验收=矩阵全绿+基准应用；冻结=收官冻结；基线 spec 落盘路径定；地图 12/12 清空 — 全量进语言归组：交互（重排/手势/剪贴板/快捷键）·i18n（@t ICU/@locale/格式化）·媒体（canvas/文件/摄像头）·生命周期（@effect 强化+1B 入册）；memo 编译器职责

## Not yet specified

- **阶段 2 基准应用具体选择**——验收形态已定（矩阵全绿+基准应用）；具体应用在基线 spec 定稿后定义（ROADMAP 决策 14）
- **冻结 ADR 条款定稿**——基线 spec 含冻结清单草案；收官时定稿（收官宣言 + 语法面冻结清单 + 硬规则）
- **半实现信号处置清单**——裁决原则在车票 05，具体到 form.submitting/@load v0 边界的逐项清单随各批次
- **新增语义域登记扩展方式**——verify-semantic-universe.py / verify-vocabulary.py 改造随各阶段 spec

## Out of scope

- **编写体感优化**（样板减少/缩略语法/词法体感）——用户拍板：poiesis 是作者，手写体验不优先；语法可表达性才在范围
- **后端/服务端能力**（数据库、鉴权服务端、实时同步服务）——前端 DSL 边界
- **Vue/Dioxus 目标实现**——#87 触发制，维持 floor 诊断（ADR-052:94 契约）
- **既有 1F 已知缺陷**（cache 性能 15.6×、跨文件验证空洞、C2 决策等）——归 ROADMAP 1F 检查点，不并入本努力（类型检查已裁归 1F，见车票 05）
