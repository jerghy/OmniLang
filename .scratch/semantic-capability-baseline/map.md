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

## Not yet specified

- **阶段 2 基准应用形态**——依赖能力基线定稿后能力边界清晰才可定义（验收形态决策先行于车票 12）
- **各阶段详细语法设计**——批次裁决（06-11）后逐阶段在各自 spec 细化（showToast 签名/@modal 语法/@route 契约/分页形态…）
- **冻结 ADR 条款细节**——基线 spec 定稿后起草（收官宣言 + 语法面冻结清单 + 硬规则）
- **半实现信号处置清单**——裁决原则在车票 05，具体到 form.submitting/@load v0 边界的逐项清单随各批次
- **新增语义域登记扩展方式**——verify-semantic-universe.py / verify-vocabulary.py 改造随各阶段 spec

## Out of scope

- **编写体感优化**（样板减少/缩略语法/词法体感）——用户拍板：poiesis 是作者，手写体验不优先；语法可表达性才在范围
- **后端/服务端能力**（数据库、鉴权服务端、实时同步服务）——前端 DSL 边界
- **Vue/Dioxus 目标实现**——#87 触发制，维持 floor 诊断（ADR-052:94 契约）
- **既有 1F 已知缺陷**（cache 性能 15.6×、跨文件验证空洞、C2 决策等）——归 ROADMAP 1F 检查点，不并入本努力（类型检查已裁归 1F，见车票 05）
