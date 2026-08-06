# ADR-067: 语义 DSL v4 行为/注入修复轮（18 项处置验收表 + 3 defer-with-trigger + v5 已记录缺口）

## Status

Implemented

## Context

v2 语法达成度终审评审（`.architecture/reviews/feature-semantic-dsl-v2-syntax-review.md`，2026-08-06，12 位成员）给出 **14 HIGH + 23 MEDIUM** 级发现，总体 **Adequate**（架构层强、行为/台账层系统性缺口），三类交叉主题：

1. **「宣称 vs 事实」漂移**（6 位成员独立报告，最高频）：semantic-universe 域 3 事件行过度宣称、`theme="dark"` 文档勾选不存在行为、`bind:checked` 已实现无文档、docs/README.md 与 docs/stdlib.md 陈旧且不在门禁覆盖
2. **「exit 0 但生成代码损坏/静默分歧」类**（5 位成员）：数组方法在 ref-op 参数内逃逸、`scrollTo(x,y)` 静默丢 x、`longpress`/`hover`/`toggle` 死处理器零诊断、React 事件处理器写后读陈旧闭包、OmniView keyFn `String()` 塌缩对象 key——全部是门禁 fixture 覆盖不到的**组合形态**
3. **注入防线在运行时/动态面缺口**（2 个独立 HIGH）：React 生产后端动态 URL 无运行时消毒（`javascript:` 可构造，防御与 OmniView 倒挂）、Flutter 两条文本发射路径绕过共享 `$` 转义（已修 CRITICAL 类缺陷回归）

**Critical Actions 3 条**：① 闭合注入防线两缺口（React 动态 URL 消毒 + Flutter `$` 转义回归）；② 修复 React 行为层三缺陷（H1 陈旧闭包线程化、H2 悬空 setter DCE、H3 数字 bind:value）；③ 校准「宣称 vs 事实」台账。**Pragmatic Enforcer**：Necessity 7/10、Complexity 6/10、**Ratio 0.86**（目标 <1.5，Approve with simplifications）——比值增量几乎全部来自 `velocity_max`（零消费者全链路实现）与双 verify 脚本（120 行逐字重复 + 锚点钉在自声明 Superseded 的 BNF 上）；Success Metric 7 要求修复后复审 **Ratio ≤ 0.75**。

用户批准单一总计划 `docs/superpowers/plans/2026-08-06-semantic-syntax-v4-fixes.md`（27 任务，6 阶段：0 测试基建 → 1 注入 → 2 React 行为 → 3 exit-0 族 → 4 跨目标 → 5 台账 → 6 终验）。**用户 2026-08-06 范围裁定**：27 → 24 执行（Task 4.2 仅 lint 编译期半）+ 4.2 运行时半 cut + 3 defer（Task 1.1/1.3 → #86、Task 4.1 → #87）。2026-08-06 以 subagent-driven development 执行完毕（每任务 RED→GREEN + review Approved 录于 progress ledger；终验 Task 6.1 全量门禁跑批 + Task 6.1b 修复 2 个终验阻断项 F1/F2）。

本 ADR 记录该修复轮的**验收表逐项勾选（18 项）**、**defer-with-trigger（#84-#87）**、**已记录缺口（留 v5 的评审 MEDIUM 清单）**与**门禁终态数字**，作为 v2 语法达成度评审闭环的决策记录。

## Decision Drivers

* **评审发现必须逐条处置（四选一）**：fix-now / defer-with-trigger（带触发条件）/ 诚实降级（warning+skip）/ 范围裁定 cut——零「open-undispositioned」发现（每阶段结束检查）
* **门禁是行为正确性的护栏而非终点**：组合形态缺陷（ref-op×数组、事件处理器写后读、双轴 scrollTo）必须由组合 fixture / 行为断言 / 浏览器套件抓取——门禁 fixture 结构性不可见
* **注入防线闭合优先**（Phase 1 先于行为）：修复不能引入新注入面；`$` 载荷（含多子节点形态）与 `javascript:` 动态 URL 是已修 CRITICAL 类缺陷的回归高危面
* **诚实降级 > 静默错误**：无目标映射的 canonical 事件（longpress/hover/toggle）发编译 warning + 跳过死处理器，台账同步「部分」化
* **修复必须删豁免（有根因）**：TS2304×2/TS2345×2/TS2554×6 删除后 tsc 87 → 77，「豁免债 = 运行时 bug 类」（H2 先例）不可新增
* **YAGNI 兑现（评审 Success Metric 7）**：velocity_max 零消费者移除 + 双 verify 脚本合并/锚点重指向——Pragmatic Ratio 0.86 → ≤0.75 复审预期

## Decision

**v4 行为/注入修复轮完成**：18 项验收表逐项勾选（15 项处置完成 + 2 项 defer-with-trigger（#86/#87）+ 1 项部分处置（perf HIGH 编译期半））；defer-with-trigger 台账注册 #84-#87；7 项评审 MEDIUM 缺口登记留 v5；终验门禁全绿（Task 6.1/6.1b）。

### 验收表（18 项处置，证据锚点 = ledger commit 链 + 门禁实测）

| # | 评审发现 | 处置 | 证据（commit / 门禁数字） |
|---|---------|------|--------------------------|
| 1 | **security HIGH-1**（React 动态 URL XSS） | **defer-with-trigger #86**（用户 2026-08-06 范围裁定：Task 1.1/1.3 → #86） | 无 1.1/1.3 commit（范围裁定录于 progress.md）；React 主向量已由 react-dom `isJavaScriptProtocol` 原生阻断（javascript:/vbscript: 不执行），残余面 = data: URL + 非 React 目标；触发条件登记 `.architecture/deferrals.md:397` |
| 2 | **rust HIGH**（Flutter `$` 转义回归） | **修复（Task 1.2）** | `4df21d8e`（TextSpan/拼接两路径改共享转义）+ `08287f53`（Dart 扫描器插值扫描修复）；失败测试先行 `4b63c9e1`（rawtext-multi 语料点）；双路径快照断言 |
| 3 | **TS/React H1**（写后读陈旧闭包） | **修复（Task 2.1，3 commits + 2 修复轮）** | `0a4f15b5`（事件处理器写后读线程化）+ `db5e72f7`（+= 隐式读）+ `a1e9bee3`（位置敏感化 + rename 范围收敛）；浏览器行为断言 4/4（semantic-behavior 套件）；15 场景复审推演全 ADDRESSED |
| 4 | **TS/React H2**（悬空 setter 崩溃） | **修复（Task 2.2）** | `ba0d2474`（只写信号保留 useState——有可达 mutation 不剪枝）；H2 崩溃断言（pageerror 断言）绿；TS2304×2 豁免删除（tsc 85）；deferral #72 关闭 |
| 5 | **TS/React H3**（数字 bind:value） | **修复（Task 2.3）** | `c5a41da1`（数字类型信号发射 `Number(e.target.value)`）；形状断言 + TS2345×2 豁免删除（tsc 85→83）；deferral #64 子类关闭 |
| 6 | **TS/React M3**（伪 `handler({})`） | **修复（Task 2.4）** | `e34f010a`（setter 分支空事件特判 + `{call}();` 形状修正）；TS2554×6 豁免删除（tsc 83→77）；deferral #62 关闭 |
| 7 | **compiler HIGH-1**（ref-op×数组 walker） | **修复（Task 3.1）** | `bb05cd00`（9 处 flutter walker 经单一入口 `map_ref_op_args` 统一下钻，grep 无漏接）；组合 fixture（ref-op×数组）`dart analyze` 绿 + omnilang-core 1032/1032 |
| 8 | **compiler HIGH-2**（scrollTo 双轴） | **修复（Task 3.2）** | `1e97131e`（scrollTo 双轴警告断言；基线 671→673 受支持例外注记——触发 demo 调用点）；零新遍历复用 collect_ref_usage |
| 9 | **compiler HIGH-3 + domain HIGH-1**（longpress/hover/toggle） | **修复（Task 3.3）+ 台账（Task 5.1）** | `ad83aa7e`（React/OmniView warning + 跳过死处理器；10 合成事件 canonical 表逐项实证）+ `be53bc83`（域 3 事件行 33 行部分化重分类，与 flutter/utils.rs 1:1 断言）；6.1b 修正：toggle 在 `<details>` 恢复发射（`ef6bd0ba`，真实 DOM 事件条件映射） |
| 10 | **systems HIGH-1**（Vue/Dioxus floor） | **defer-with-trigger #87**（用户范围裁定：Task 4.1 → #87） | 无 4.1 commit；ADR-052:94 契约违约已记录（deferrals.md:398）；触发「vue/dioxus 真实生产使用」时按 Task 4.1 正文执行（E_CODEGEN 门 + floor_matrix 行） |
| 11 | **perf HIGH**（keyFn 对象塌缩） | **修复（编译期半，Task 4.2）+ 运行时半 cut（范围裁定）** | `38324a71`（for_key.rs 对象类型 key 表达式编译期警告——三条件组合 + 负向全覆盖）；运行时兜底（typeof object → JSON.stringify/_idx 退化）按用户范围裁定 cut，未实施 |
| 12 | **domain HIGH-2**（theme="dark"） | **warning + 文档修订（Task 5.2）** | `98993481`/`39835f42`（theme= 已知 DOM 标签 warning——自定义组件豁免结构性成立；14+4 别名归一化全覆盖）；validate_props 警告测试；设计文档 :365/:374 逐字修订 |
| 13 | **domain HIGH-3**（bind:checked） | **正式化（Task 5.3）** | `8161c2e0`（BNF/语义全集/登记册三方收敛）；域 6 新建；docs-gate 0 failed + verify 双脚本 0 problem（域 6 遍历指针由 Task 5.6 处置） |
| 14 | **compiler HIGH-4**（value={sig}） | **判别器修复（Task 5.4）** | `22a6898b`（按 @ref 判别：带 @ref 仍 error，单向 prop 恢复合法 + 提示修正）；双向测试（1044/0）；slot 场景零误伤实证 |
| 15 | **documentation HIGH-1/2**（docs/README + stdlib 入口） | **修复（Task 5.5/5.5b）** | `eedcdb05`（README 五处事实修订 + stdlib 弃用横幅 + `omni/*`→`std/*` 全替换 + check_index_links 链接断言入门禁）+ `bbe3eaad`（v1→v2 迁移指南 17 项删除对照表 + 诊断码索引）；docs-gate 248 块 0 failed |
| 16 | **pragmatic MEDIUM-1**（velocity_max） | **移除（Task 5.8）+ 重入触发登记 #84** | `531c19a5`（27 文件三层零残留零误伤——constraints/elastic/threshold 原样保留；store_slices 陷阱清除）；RED 测试用真实 `@drag-horizontal` 语法锁定硬诊断；motion 53/0；白名单零改动 |
| 17 | **pragmatic MEDIUM-2**（双 verify 脚本重复/锚点） | **合并 + 重指向（Task 5.6）** | `46981059`（verify 脚本锚点重指向 parser 常量（rust_str_array）+ BNF 单向检查 + 域 6 遍历扩展 range(1,7)——三文档不再自洽同步漂移）；spec_whitelists.py；verify 双脚本 0 problem |
| 18 | **implementation-strategist MEDIUM**（台账陈旧/ADR-056 注记） | **关闭纪律（Task 5.7）** | `b7094d40`（收口节七条关闭行 + #42 触发重写 + #84-#87 注册逐字 + ADR-056 Negative 正文划线改写）+ `61f8950d`（#64 收口行改子类关闭——与同文件自证矛盾消除） |

**处置统计**：18 项中 **15 项处置完成**（修复/移除/正式化/文档/台账）+ **2 项 defer-with-trigger**（#86 React 动态 URL 消毒、#87 Vue/Dioxus floor，均为用户范围裁定）+ **1 项部分处置**（#11 perf HIGH：编译期 lint 半完成，运行时兜底半按范围裁定 cut）。评审 **Critical 5 批**（security HIGH-1、rust HIGH、H1/H2/H3）4 修复 + 1 defer-with-trigger——Critical Actions ① ② ③ 全闭环（① 中 React 动态 URL 半侧以「React 原生阻断主向量」+ 触发登记闭环）。

### defer-with-trigger（3 项计划 defer + 1 项移除重入注册）

| # | 条目 | 触发条件 | 处置日/证据 |
|---|------|---------|------------|
| **#86** | React 动态 URL 运行时消毒（security HIGH-1，用户范围裁定 defer）——React 原生 `isJavaScriptProtocol` 阻断 javascript:/vbscript: 主向量已关闭；残余面 = data: URL + 非 React 目标（**Vue/Dioxus 消毒镜像 = 本行触发条款**，届时按 Task 1.1/1.3 正文执行：sanitizeUrl helper + 六属性包裹 + node 行为 harness） | 「真实用户输入流进 URL 绑定」或「Vue/Dioxus 生产使用」 | 2026-08-06 登记（b7094d40 随 5.7）；deferrals.md:397 |
| **#87** | Vue/Dioxus 语义 floor 诊断（systems HIGH-1，用户范围裁定 defer）——非目标后端；ADR-052:94 契约违约已记录 | 「vue/dioxus 真实生产使用」（届时 E_CODEGEN 门 + floor_matrix 行，按 Task 4.1 正文） | 2026-08-06 登记；deferrals.md:398 |
| **#85** | a11y 动态可访问名称（domain MEDIUM-1）——label/role 当前限字符串字面量 | 「真实应用需要 i18n 动态 label」 | 2026-08-06 登记；deferrals.md:396 |
| **#84** | velocity_max（pragmatic MEDIUM-1 的**移除重入注册**——非 defer 未做，而是已移除后保留重入路径） | 「demo 或用户需要 fling 速度物理」；git 历史保留完整实现；syntax-lifecycle 门禁为自然重入路径 | 2026-08-06 登记（5.8 移除 531c19a5 + 5.7 注册）；deferrals.md:395 |

### 已记录缺口（未修复，评审 MEDIUM 级，留 v5）

| 缺口 | 来源 | 现状 |
|------|------|------|
| TS/React M1/M6 类型发射 | 评审 TS/React M1（隐式 any，TS7006×44）+ M6（`e.target.*` duck-typing，TS2339×10） | **54/87 豁免**——0.4 基线（747978a9）已锁定，v5 处置（codegen 内部闭包打类型 + 显式合成事件参数类型可一次折叠两个豁免类） |
| M2 useDrag hook | 评审 TS/React M2 | 拖拽手势逻辑 3× 内联（ElementRefDemo.tsx ~75 行 ×3）+ 模块级 `__omni_activePointerId` 共享全局——v5 抽 hook |
| M4 getters selector | 评审 TS/React M4 | 派生值物化进 store 状态（`__getters` 每次 `__set` 重算全部含无关字段）——v5 改 selector 推导 |
| 半增量编译 | 评审 perf MEDIUM-4 | 缓存路径仍全量跑验证/lint/IR 重建/全部文件 codegen（analyse.rs 依赖图已存在未用于增量）——v5 |
| suggestion 字段政策 | 评审 AI Engineer MEDIUM-1 | 结构化 suggestion 填充不一致（部分验证器修复提示埋进消息文本）——AI 工具化政策 + 测试 v5 |
| finish_pipeline 编排合并 | 评审 Maintainability MEDIUM-1 | 合并后管线阶段序列三处复制粘贴（run_pipeline/analyse_multi/analyse_multi_cached）——提取 finish_pipeline 辅助 v5 |
| token IR 类型化 | 评审 Systems MEDIUM-1 | 设计 token 身份以 CSS 字符串穿过 IR（`ClassIR.properties: FxHashMap<String,String>`）——`DesignValueIR { Literal \| TokenRef \| Mixed }` 类型化 v5 |

### 门禁终态（Task 6.1 跑批 + 6.1b 修复后全绿）

| 门禁 | 终态数字 |
|------|---------|
| React 门禁（verify-react.sh） | **GATE PASSED**：tsc **77/77 全豁免**（0.4 基线 87 − TS2304×2 − TS2345×2 − TS2554×6 = 77）+ eslint **4/4**（6.1b F2 后回到全豁免） |
| Flutter（generated_code_validation 全量 dart analyze） | warnings **≤ 673**（`FLUTTER_WARNINGS_BASELINE = 673`，含 Task 3.2 双轴警告 671→673 受支持例外注记）；11/11 绿 |
| docs-gate（verify-docs.py --check） | **248 omni blocks, 0 failed**（173 semantic notes） |
| verify 双脚本 | verify-vocabulary.py **0 problem**；verify-semantic-universe.py **0 problem** |
| 浏览器全量（Playwright） | **46 passed + 1 skipped（B4 既有文档化 parity-gap skip）+ 0 failed**；semantic-behavior **5/5**（H1 四断言 + H2 崩溃断言） |
| cargo workspace | `--exclude poiesis --no-fail-fast` **全绿**（poiesis.exe 被并行会话进程锁定 = 环境不处置，F3 先例延续）；per-backend react/omniview/vue/flutter 矩阵全绿（6.1b F1 门控修复后） |
| 可复现性 | release omnic 重建输出与已提交 harness **逐字节一致**（0 diff） |

## Consequences

### Positive

- **v2 语法达成度评审闭环**：18 项验收表逐项勾选（15 完成 + 2 defer-with-trigger + 1 部分），零「open-undispositioned」发现；Critical Actions ①②③ 全闭环
- **行为层修复落地**：React 写后读线程化（5 处实例语义修正）、悬空 setter 崩溃消除、数字 bind:value 类型谎言消除——浏览器行为套件 5/5 入 CI 持续守护
- **exit-0 族组合缺陷闭合**：ref-op×数组 walker 下钻、scrollTo 双轴警告、longpress/hover/toggle 诚实降级（toggle 经 6.1b 按 `<details>` 条件化修正 3.3 过度跳过）——组合 fixture 入测试
- **豁免净减 10 条**（87 → 77）且每类删除有根因修复（H2/H3/M3 各自对应代码修复 + 台账关闭 #62/#64/#72）
- **台账诚实**：universe 域 3 部分化（33 行）、theme= warning、bind:checked 正式化、README/stdlib 入口修复 + 链接断言、verify 锚点重指向 parser 常量、域 6 入门禁遍历环
- **YAGNI 兑现**：velocity_max 全链路移除（27 文件零残留）+ 脚本合并/锚点重指向——Pragmatic 复审预期 Ratio 0.86 → ≤0.75
- **治理新鲜度**：deferral 台账 #62/#64/#72/#42 收口 + #84-#87 注册逐字；ADR-056 Negative 节正文划线改写（非注记）

### Negative

- **安全 HIGH-1 半侧 defer**：React 动态 URL 消毒依赖 react-dom 原生 `isJavaScriptProtocol`（javascript:/vbscript: 主向量关闭）——data: URL 与 Vue/Dioxus 目标在触发条件前无自有消毒器（触发：「真实用户输入流进 URL 绑定」或「Vue/Dioxus 生产使用」）
- **perf HIGH 运行时兜底 cut**：OmniView keyFn 对象塌缩仅有编译期 lint 警告，运行时兜底（JSON.stringify/_idx 退化）未实施——lint 已使 `key={item}` 直接编译期可见，静默塌缩被消除
- **Vue/Dioxus floor defer**：#87 触发前 ADR-052:94 契约违约保持已记录状态（非目标后端不投资）
- **留 v5 缺口 7 项**（M1/M6 类型发射 54 豁免、M2 useDrag、M4 getters、半增量编译、suggestion 政策、finish_pipeline、token IR 类型化）——均为评审 MEDIUM 级，非行为缺陷
- **poiesis workspace 环境阻断**（F3 先例延续，用户进程运行中）——`--exclude poiesis` 验证全绿，正式命令待用户会话退出后闭环

### Neutral

- 6.1b 修正 3.3 的 toggle 过度跳过——诚实降级与真实 DOM 事件（details）的边界由元素条件映射 + 双向 4 测试锁定，universe 台账同步
- 诊断语言中文政策延续（v4 全部诊断以中文 verbatim 落地，与 ADR-056 政策一致）

## Implementation Strategy

### Blast Radius

**Impact Scope**: 中——6 阶段顺序执行（0 测试基建 → 1 注入 → 2 React 行为 → 3 exit-0 族 → 4 跨目标 → 5 台账 → 6 终验），每任务独立 commit 范围可回退；治理层（ADR-067 + deferrals #84-#87）随本 ADR 收口。

**Affected Components**:
- 注入：Flutter 文本发射两路径共享转义（1.2）+ rawtext-multi 语料点（0.1）
- codegen：React 事件发射线程化/只写信号保留/Number 强制（2.1-2.3）、setter 空事件特判（2.4）、toggle 元素条件映射（6.1b）；Flutter walker 下钻（3.1）、scrollTo 双轴警告（3.2）；React/OmniView 死处理器 warn+skip（3.3）；OmniView for-key lint（4.2）
- 语义/验证：theme= warning（5.2）、value 判别器按 @ref（5.4）、verify 脚本锚点重指向（5.6）
- 文档/台账：universe 部分化（5.1）、bind:checked 正式化（5.3）、README/stdlib 入口（5.5）、迁移指南（5.5b）、deferral 台账 + ADR-056 改写（5.7）
- 移除：velocity_max 27 文件全链路（5.8）

**User Impact**: 三目标生成代码行为层修复（React 5 处写后读实例、悬空 setter 崩溃、数字转换）；用户区（poiesis/CodeFlowMidChain.omni）零触碰；注入语料/断言全绿。

**Risk Mitigation**: 每任务 RED→GREEN + review Approved（24 任务全录 progress ledger，含 6.1b 终验阻断项 F1/F2 修复）；终验 7 门禁全绿独立复核；验证网（行为套件 5/5 + 组合 fixture + 双后端警告断言）持续守护。

### Reversibility

**Reversibility Level**: 高——各阶段独立 commit 可回退；velocity_max 移除经 deferral #84 保留完整重入路径（git 历史 + syntax-lifecycle 门禁自然重入）；defer #86/#87 触发条件即恢复执行入口。

**Commitments Made**: 门禁-行为-台账三轨一致是语义 DSL 时代的稳定性基础；诚实降级（warning+skip）为无目标映射 canonical 事件的契约（toggle 的 details 例外已元素条件化）。

### Sequencing & Timing

**Prerequisites**: 门禁基线先行（0.4 锁定 87+4）——修复无度量不开始；注入（Phase 1）先于行为（Phase 2）——修复不能引入新注入面；行为（Phase 2）先于组合缺陷（Phase 3）——组合 fixture 依赖 walker 语义稳定。

**Readiness Assessment**: 已完成——终验 7 门禁全绿（6.1b 修复 F1/F2 后）；per-backend 矩阵全绿；工作树仅剩并行会话未跟踪文件（本 ADR 提交只 add 目标文件）。

### Social Cost

**Learning Curve**: 低——决策沿 v3 既有模式（硬诊断/诚实降级/触发条件注册/豁免删除有根因），无新抽象。

**Documentation Needs**: 本 ADR + review v4 处置记录 + deferrals #84-#87 为闭环证据链；v5 缺口清单（7 项）为后续计划输入。

## Alternatives Considered

1. **React 动态 URL 消毒本轮实现（Task 1.1/1.3 原计划）**——拒绝（用户 2026-08-06 范围裁定 defer #86）：react-dom `isJavaScriptProtocol` 已原生阻断 javascript:/vbscript: 主向量（事实防线）；残余面（data: URL + 非 React 目标）无真实用户输入流进 URL 绑定的现实验证场景，YAGNI 下登记触发条件。
2. **Vue/Dioxus floor 本轮实现（Task 4.1 原计划）**——拒绝（用户范围裁定 defer #87）：非目标后端，触发「真实生产使用」前不投资（ADR-052:94 契约违约保持已记录）。
3. **perf HIGH 运行时兜底本轮实施（JSON.stringify/_idx 退化）**——拒绝（用户范围裁定 cut）：编译期 lint 已使 `key={item}` 塌缩在编译期可见（修复前需先触发 lint 才踩中静默塌缩），运行时半留待真实场景。
4. **velocity_max 保留（评审「Approve with simplifications」的备选）**——拒绝：零消费者全链路实现 + React slice 特例排除为纯复杂度，移除 + 重入触发登记（#84）满足 YAGNI。
5. **toggle 保持 3.3 的 warn+skip（不修 F2）**——拒绝（6.1b 实测）：toggle 是 `<details>` 真实 DOM 事件（React 合成 onToggle / OmniView addEventListener），过度跳过导致 EventsDemo 处理器被丢 + eslint 非豁免回归；元素条件映射（details 发射、非 details warn+skip）双向 4 测试锁定。
6. **3.2 scrollTo 警告并入既有警告通道（不更新基线）**——拒绝：基线 671→673 为受支持例外（触发 demo 调用点 `jumpTo(200)` 保留），与 ADR-066 诚实下限一致，注记而非掩盖。

## Validation

- **验收标准 7 项逐项勾选**（Task 6.1/6.2）：
  1. **注入面** ✓：`$` 载荷（含多子节点形态）经 rawtext-multi 语料点 + 双路径共享转义修复（4df21d8e/08287f53）全断言；React 动态 URL 消毒 = defer #86（React 原生阻断主向量，触发登记）
  2. **React 行为语义** ✓：浏览器行为断言 5/5（H1 四断言 + H2 崩溃断言）；tsc 77 ≤ 81 门禁且豁免删除（TS2304×2/TS2345×2/TS2554×6）各有根因修复（2.2/2.3/2.4）
  3. **exit-0 族** ✓：组合 fixture（ref-op×数组）生成 Dart 可编译（3.1）；scrollTo 双轴有警告（3.2）；longpress/hover/toggle 双后端 warning + 跳过（3.3），toggle 经 6.1b details 条件化后 Flutter 正常 + web 在 details 交付
  4. **跨目标契约** ✓（部分）：Vue/Dioxus floor = defer #87（契约违约已记录 + 触发条件）；OmniView keyFn 对象 key 不塌缩 = 编译期 lint 警告（4.2，运行时半按范围裁定 cut）
  5. **台账诚实** ✓：semantic-universe 域 1-6 全行与 Flutter 事实一致（脚本断言 0 problem，5.1/5.6）；theme="dark"（5.2）/bind:checked（5.3）/value={sig}（5.4）文档与行为一致；README/stdlib 入口真实 + 链接断言入门禁（5.5）；verify 脚本锚点 = parser 常量（5.6）
  6. **治理新鲜度** ✓：deferral 台账无「已修复仍开行」（5.7 补关 #62/#64/#72/#42 + 修复轮 1 子类关闭）；ADR-056 Negative 节改写（划线正文）；velocity_max 零引用（5.8 断言翻转 + 27 文件 grep）
  7. **全量绿** ✓：`cargo test --workspace --exclude poiesis` 全绿（poiesis.exe 环境锁 F3 例外）；verify-react GATE PASSED（77/77 + eslint 4/4）；Flutter dart analyze ≤673；docs-gate 248/0；verify 双脚本 0 problem；Playwright 46+1/0（semantic-behavior 5/5）
- **commit 链完整性**：验收表 18 行证据 SHA 全部在 `git log` 实测存在（30 个 ledger SHA 逐一核验，0 缺失）
- **处置闭环**：review v4 处置记录（18 项验收表 + 7 项缺口清单）/ deferrals #84-#87 登记确认（deferrals.md:395-398）

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: 修复轮由 v2 语法达成度评审 14 HIGH + 23 MEDIUM 驱动；18 项处置中 15 项沿既有模式（诚实降级/触发条件注册/豁免删除有根因）展开，无新抽象；本轮自身的两处简化（velocity_max 移除、verify 脚本合并/锚点重指向）恰为评审 Pragmatic 裁决点（Ratio 0.86 增量来源）。

**复审预期注记**（评审 Success Metric 7）：velocity_max 移除 + 脚本合并落地后，复审 Ratio 预期 **0.86 → ≤0.75**——两条增量来源均已兑现（5.8 移除 27 文件零残留、5.6 锚点重指向 parser 常量 + 合并半确认）；实际复审评分由 Pragmatic Enforcer 按 ADR-067 处置记录复核执行。

**Necessity Assessment**: 9/10
- **Current need**: 评审闭环（18 项零悬空承诺）+ 行为层修复（写后读/悬空 setter/数字类型谎言均有用户可见后果）+ 注入防线闭合
- **Future need**: ADR-067 是后续评审的判定基线（tsc 77 豁免 / warnings 673 / 行为套件 5/5 的度量口径）；v5 缺口清单为下一轮输入
- **Cost of waiting**: 不修则行为层缺陷持续（H1 排序与标签相反、H2 点击即崩被豁免掩盖）
- **Evidence of need**: 评审实测（5 处写后读实例、点击 ReferenceError、`javascript:` 可构造）+ 终验门禁数字

**Complexity Assessment**: 4/10
- **Added complexity**: 零新依赖；唯一新机制为 toggle 元素条件映射（归一化标签判定，双向 4 测试）；线程化（2.1）为本轮最复杂修复但经 2 修复轮收敛
- **Maintenance burden**: 组合 fixture + 行为套件入 CI 持续守护；deferral 触发条件为后续处置唯一依据
- **Learning curve**: 低
- **Dependencies introduced**: 零

**Alternative Analysis**: 不记录 ADR（备选 0）会让 18 项验收表 + #84-#87 注册失去唯一参照（评审闭环承诺）；三项用户范围裁定（#86/#87/运行时 cut）必须留档以与计划原意区分。

**Recommendation**: Approve

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 4/10
- **Ratio**: 0.44 *(Target: <1.5；复审预期 ≤0.75——本轮自身两条增量来源已兑现)*

**Overall Assessment**: 适当的收口治理——行为/注入修复（15 项）+ 三项用户范围裁定留档 + 台账收口 + v5 缺口登记；本轮同时兑现评审的简化要求（velocity_max 移除/脚本合并），Ratio 结构性下降。

## References

- 评审：`.architecture/reviews/feature-semantic-dsl-v2-syntax-review.md`（2026-08-06，14 HIGH + 23 MEDIUM，Pragmatic Ratio 0.86）
- 计划：`docs/superpowers/plans/2026-08-06-semantic-syntax-v4-fixes.md`（27 任务 → 用户裁定 24 执行 + 3 defer）+ `.superpowers/sdd/2026-08-06-semantic-syntax-v4-fixes/progress.md`（逐任务 commit 证据与隔离范围）
- 终验：`.superpowers/sdd/2026-08-06-semantic-syntax-v4-fixes/task-6.1-report.md`（全量门禁跑批 F1/F2/F3 详述）/ `task-6.1b-report.md`（F1 门控 + F2 toggle 元素条件映射修复）
- 关键 commits（ledger 全链 30 SHA 均核验在库）：`4b63c9e1`/`2ea818ab`/`24317777`/`747978a9`（Phase 0）· `4df21d8e`+`08287f53`（1.2）· `0a4f15b5`+`db5e72f7`+`a1e9bee3`（2.1）· `ba0d2474`（2.2）· `c5a41da1`（2.3）· `e34f010a`（2.4）· `bb05cd00`（3.1）· `1e97131e`（3.2）· `ad83aa7e`（3.3）· `38324a71`（4.2）· `be53bc83`（5.1）· `98993481`/`39835f42`（5.2）· `8161c2e0`（5.3）· `22a6898b`（5.4）· `eedcdb05`（5.5）· `bbe3eaad`（5.5b）· `46981059`（5.6）· `b7094d40`+`61f8950d`（5.7）· `531c19a5`（5.8）· `e91d4c61`+`ef6bd0ba`（6.1b）
- 上游：ADR-062（v3 修复轮——本修复轮动因与模式基线）、ADR-056（v2 实现）、ADR-066（Flutter warnings 燃尽——673 基线口径）、ADR-052（跨目标 floor 契约——#87 违约记录点）
- 遗留登记：`.architecture/deferrals.md`（#62/#64/#72/#42 关闭确认 + #84-#87 注册；#85/#86/#87 为 defer-with-trigger，#84 为移除重入注册）
