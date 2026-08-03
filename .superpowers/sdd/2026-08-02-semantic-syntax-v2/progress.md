# SDD ledger — plan: docs/superpowers/plans/2026-08-02-semantic-syntax-v2.md

Plan: 8 phases, 36 tasks — OmniLang 语义级 DSL v2（评审 → 设计 → 实现）
Workspace: .superpowers/sdd/2026-08-02-semantic-syntax-v2/

## Task status

- Task 0.1: complete (commits 883b9bf6..6481967f, review clean ✅) — proptest 真断言 + codegen fuzz；flutter feature 运行验证 7/7
- Task 0.1: minor (deferred): compile_for_target helper 命名偏离 brief 示例（合理化改进，采用 generated_code_validation 先例）
- Task 0.2: complete (commits 6481967f..5d136113, review clean ✅) — 缓存命中诊断合并 + 回归测试
- Task 0.2: parked — 未变更文件的解析期错误在缓存命中时仍丢失（CachedFile 只存语义/降级期诊断；缓存格式设计遗留，follow-up 需改缓存格式；审查裁决 acceptable-known-gap）
- Task 0.2: minor (deferred): to_reprocess guard 防重复副作用未测（3 文件 A-imports-B 测试可选）
- Task 0.3: complete (commits 5d136113..de48bcde, review clean ✅) — is_dangerous_url 移位 + feature 矩阵 job
- Task 0.3: plan-gap → 新增 Task 0.3a（vue/dioxus 跨后端耦合解耦，feature-matrix 转绿）；审查裁决 red-job = acceptable-with-documentation，条件 = Phase 0 出口前补任务（已执行）
- Task 0.3a: complete (commits de48bcde..dabdbf79, review clean ✅) — dioxus RustTypeFormatter 复用 + shared/style.rs 提取（vue 解耦），feature-matrix 全绿
- Task 0.3a: minor (deferred): ① shared style 函数在 dioxus-only 构建的 dead-code 警告（既有类别，CI 容忍；cfg-gate 延后）② 共享 fallback 路径遗留 "react/codegen:" eprintln! 前缀（轻微误导诊断）③ vue/template.rs CRLF→LF 行尾归一（review diff 伪影）
- Task 0.4: complete (commits dabdbf79..a53ccbce, review clean ✅) — react-test-app 真实化：React 19 + Zustand 5 + strict tsc + eslint；284 tsc + 109 eslint 全匹配豁免（20 条，各带修复任务引用）；门禁诚实失败已验证
- Task 0.4: important (tracked) ① 豁免计数未强制——已知错误类的新增/收缩不可检测；接受至 T1.3/1.6/3.3 落地（彼时豁免收缩），便宜修复（逐条计数）留档 ② codegen-validate 的 react@18.3.1/zustand@4.5.0 陈旧 pin + 假注释——**随 T1.6 派发时一并升级**（Phase 0 出口前必须注册，已注册）
- Task 0.4: minor (deferred): 报告 21 vs 文件 20 条豁免（计数口径）；eslint 单问题时 "1 problem" 单数行触发 cosmetic 噪音（门禁仍正确失败）
- Task 0.5: complete (commits a53ccbce..f46eba7d, review pending) — 展示项目 Flutter 门禁 + #17 关闭；实测 RED 1125 错误/21 类（MatchBreadthDemo 实测 + 全量枚举）
- Task 0.5: plan-note → un-ignore 条件已修正：Task 1.4 只清 match 专项（快照断言），门禁 #[ignore] 保持至 Phase 5 Flutter 批次（5.5/5.6/5.7）完成（计划文件已修订）
- Task 0.5: deferral 处置 — #17 关闭（证据：6 CRITICAL 已修 + 全量枚举门禁）；#30 安装 match 条目（原 #30 regex_match → #34 重编号，无文档引用冲突）；裸 \ lexer（时代 #31）从未入文件 → Task 0.7 处理；#22 行仍 open 但已修复 → Task 0.7 处理
- Task 0.5: complete (commits a53ccbce..f46eba7d, review clean ✅) — 门禁全量枚举模式 + 诚实 #[ignore]；#17 关闭/#30 安装/#34 重编号
- Task 0.5: minor (deferred): ① #[ignore] 注释与 deferral #30 收尾句仍引用「Task 1.4 移除」→ 随 T1.4 执行时对齐（已计划）② assert 用 written 非 failures.len()（cosmetic）③ #22 行交给 Task 0.7
- Task 0.6: complete (commits 467a8373..d0a92137, review clean ✅) — lint 单跑验证（双管线不相交）、warnings 字段、CLI 三命令打印、json 键
- Task 0.6: important (tracked) — poiesis 编辑器 codegen warnings 不可读（rustCompiler.ts 手维护本地副本无 warnings 字段）→ follow-up 需更新 rustCompiler.ts + problems panel 接线（poiesis 侧，非编译器范围）
- Task 0.6: minor (deferred): ① 报告 12 vs 实际 11 测试 ② 单文件 lint warning 带合成 file:"<input>"（建议 None）③ 失败构建先退出不打印 warnings（Python parity，可接受）④ cached 路径 lint-error 不阻断（与 T0.2 parked 同类，合并 follow-up）
- Task 0.7: complete (commits d0a92137..046b20df, review clean ✅) — #22 关闭（实证 exit 0）、裸 \ 安装 #35、spec Approved、.orig 删除、注释修正
- Task 0.7: info (next governance sync) — deferral 行 #20 陈旧（2026-08-01 状态节已记关闭但表格行未划）
- Task 0.8: complete (commits 046b20df..1595bb46, fix round 1/5: F1/F2/F3 addressed, re-review clean ✅) — control_flow fixture（T5.5/T5.4 契约 #[ignore]）+ WASM DiagnosticJson 统一 + wasm 测试基建修复（20/20 真跑）+ React.Fragment import bug 修复 + F1 warnings 恢复
- Task 0.8: minor (deferred): ① fixture key={item} 非 item.id（结构体项 Flutter 发射缺口，域 5 follow-up）② compile_multi_to_omniview pass-through 保留 Diagnostic serde 形状（部分统一，可接受）③ C FFI 未动（brief 范围外）④ 新 wasm 测试未断言去重（结构性安全）⑤ diagnostics() 信封键名 "errors" 承载 warnings（legacy 命名，正确不更名）

## Phase 0 出口核验（✅ 全部达成）

- ✅ react-test-app strict tsc 全绿（豁免清单 20 条各带任务引用）· ✅ 展示项目 Flutter 门禁（诚实 #[ignore] 至 Phase 5）· ✅ proptest 真断言 + codegen fuzz · ✅ feature 矩阵 job（vue/dioxus 解耦后全绿）· ✅ warnings 三网关可见（CLI/单文件/编辑器路径 + F1）· ✅ #17/#22 关闭 · ✅ 参数化 @if/@for fixture · ✅ WASM 契约统一（单 DiagnosticJson、缓存序列化移除、多文件路由）· ✅ 缓存诊断恢复 · ✅ 治理同步（#35 裸 \、spec Approved、.orig 删除）

- Task 1.1: complete (commits 1595bb46..d0d83c9d, review clean ✅) — 管道闭包唯一 + E_CLOSURE_ARROW 提示（单 choke point 验证：全通用位置覆盖，match/事件内联合法语境未动）+ ParseError.kind 管道 + spec §6.3/§9.3 同步
- Task 1.1: policy (recorded) — v2 诊断消息语言 = 设计文档原文（中文，首个 E_CLOSURE_ARROW）；可逆决策：如用户偏好英文可整体切换并同步设计文档；后续任务（1.2+）同类消息延续此政策
- Task 1.1: minor (deferred): ① 模板 ${expr} mini-parser 不支持闭包、箭头裸错无提示（既有，模板表达式范围外）② 报告计数偏差（46 vs 54 构造点）③ parser/mod.rs 3 处 assert_eq! 格式残留 churn

- Task 1.2: complete (commits d0d83c9d..29069a8d, fix round 1/5: HIGH+LOW-1 addressed, re-review APPROVED ✅) — 数组方法语言化（filter/map/count 三目标映射 + node 运行断言）；HIGH React computed deps 回归修复（ArrayMethod 臂 ×2）
- Task 1.2: minor (deferred): ① 未映射目标（python/rust/dioxus）verbatim 发射——需 follow-up（Dart 族方法表 .len()/.get(i) + Python filter 内置陷阱）② diagnostics 锚定声明 span 非调用点 ③ 08-stores.md 残留 .len() 示例（Dart 无效）④ ClosureExpr 臂缺失 → T3.4 计划内修复

- Task 1.3: complete (commits 29069a8d..8d8cbe3f, review clean ✅) — $field 通道（store→$signal / signal-like→$field / 局部→$ident 三路判别 + 7 签名线程化）+ OmniView $call 参数递归与 @for 样式 per-clone 修复；真实 DataTable 验证
- Task 1.3: minor (deferred): ① 报告称「flex: 1 number fix」未实现（既有 Number 丢弃，无回归）② 新增 fmt 漂移行（phase-end fmt pass）③ 循环项遮蔽同名信号（罕见既有）④ 裸非信号标识符在 style 中仍发 $signal（现可修，follow-up 候选）⑤ 陈旧 $signal_concat 注释 ⑥ React $field+$signal 组合未测（OmniView 覆盖）

- Task 1.4: complete (commits 8d8cbe3f..311fad90, review clean ✅) — 根因双修：merge_file_ir_into 丢弃 module.enums（多文件编译下所有后端看不到 union）+ flutter/types.rs 变体类发射 + match 臂字段绑定 + to_string 映射；#30 关闭；#[ignore] 归因对齐 Phase 5
- Task 1.4: deferral-candidate (tracked) — plain-Dart 后端裸 enum 发射（dart/visitor.rs，field-carrying 变体仍坏，无门禁；formatter 侧已修）→ Phase 7 deferrals 收口时注册新行
- Task 1.4: minor (deferred): ① types.rs 陈旧 _v._variant 注释 ② used/unused 绑定启发式（最坏 = 容忍 lint 警告）③ LetStmtIR.r#type 收集器未扫（Phase 5 typed locals 前瞻）④ types.dart/design.dart 命名碰撞隐患类（既有）

- Task 1.5: complete (commits 311fad90..2e9673ba, review clean ✅) — BlockExpr E_CODEGEN 硬诊断（python/vue）+ debug_assert 提升 log::warn；array_sort 双参闭包真比较器（数值单次求值 + swapped-call 回退）；M1 机制更正注记已写入评审文件
- Task 1.5: minor (deferred): ① 无自动调用计数测试（行为断言修复前也过，仅形状断言钉新发射）② 报告 9 vs 实际 8 测试 ③ 诊断无 file/line（既有 ECodegen 惯例）④ closure_param_count 文本耦合（降级安全）⑤ Python 双参 array_sort 潜在运行时 TypeError（非 P0 既有）⑥ Vue 遗留通道路径

- Task 1.6: complete (commits 2e9673ba..0c7554e3, review clean ✅) — C1 全覆盖（与 collect_view_read_signals 对齐，无过度收集）+ TS 批次 + contains map-aware + pin 升级 + 豁免 20→17；DataTable 6 fn on-disk 验证
- Task 1.6: deferral-candidate (tracked) — ① React 目标不发射用户 struct → useState<RowItem[]> 不可达（10 处 never[] 诚实豁免）② let-mut→const 需 IR 变更（LetStmtIR 丢 mut 标志，75 构造点）③ **Playwright e2e harness 入口陈旧（main.tsx 未接线）→ CI e2e job 红——Phase 7 浏览器测试前置** ④ test-results.json 提交的是失败 e2e 运行（header-CSS 断言，与本任务无关）
- Task 1.6: minor (deferred): ① JsTypeFormatter 无 Record 臂（latent TS2314）② Option 联合在数组内未加括号（latent 既有）

## Phase 1 出口核验（✅ 全部达成）

- ✅ 闭包唯一性测试 · ✅ 数组方法三目标编译+运行（node 断言）· ✅ style 通道 fixture 全绿 · ✅ match Flutter 专项清零（#30 关闭）· ✅ array_sort node 断言 · ✅ React strict tsc 284→159 + 豁免收缩 · ✅ BNF spec §6/§9 同步

- Task 2.1: complete (commits 0c7554e3..e6bc562c, review clean ✅) — SetValue 删除（10 ops，non_exhaustive 纪律保持）+ 三写形式硬诊断（verbatim 提示）+ 全 workspace 清扫 + fixture 迁移
- Task 2.1: minor (deferred): ① 裸 `ref = x` 形式零测试（随 T2.2 补一行 e2e）② flutter/ref.rs 陈旧 setValue 文档 ③ bind:value 测试近乎空回退 ④ 旧行为 parity 讨论（新报错优于旧静默误改写）

- Task 2.2: complete (commits e6bc562c..e061cefb, review clean ✅) — capturePointer 无参（React 活动指针 tracker 按需门控 + OmniView runtime tracker + Flutter error+throw）+ demo 迁移 + 浏览器实证 + L-1 补测
- Task 2.2: minor (deferred): ① OmniView wrapper null 时静默跳过 vs 注释承诺 loud（跨后端行为分歧）② Flutter 诊断锚定组件 span 非调用点 ③ 报告计数微差

- Task 2.3: complete (commits e061cefb..7984132c, review clean ✅) — bind:value 唯一双向（前提修正：HIGH-1 分叉在被删 value={sig} 上，删除即闭环）+ 两个真实缺口修复（React store 限定 setState、Flutter ref 文本框接线）+ demo 迁移 + 三目标方向 fixture + 浏览器实证
- Task 2.3: deferral-candidate (tracked) — Flutter 非 ref 元素 bind:value 单向 no-op + Checkbox/Switch/Slider onChanged:null（D2「唯一双向」在部分 Flutter 表面未完全兑现）→ 需 controller 合成任务
- Task 2.3: minor (deferred): ① `value={item.id}` 错误提示误导（known-store 检查 → plain prop 回退，语义 pass 细化候选）② 陈旧注释 3 处 + FeatureCard 注释 ③ FormControlsDemo golden 镜像漂移（7330e783 起）④ react/tests.rs CRLF churn

- Task 2.4: complete (commits 7984132c..7fbf9dd1, review clean ✅) — 字段桩全诊断化（warnings 通道线程化 + 降级注释）+ physicalKey.keyId 稳定映射 + map_event 五后端单表（顺带修 onTap/pointer props 死 prop）+ FnRef 4 回归测试 + 三目标 fixture
- Task 2.4: exit-gate item (tracked) — F-1：`e.code` schema String vs Flutter keyId int（运行时失配，文档化；per-target 字段类型化为终修）→ Phase 2 出口清单
- Task 2.4: minor (deferred): ① React onLongPress→onLongpress casing nit ② Flutter fixture 警告走 whitelist-skip 路径（字段级警告仅单测覆盖）

- Task 2.5: complete (commits 7fbf9dd1..a0c9940f, review clean ✅ + follow-up) — BNF §2 同步（§8 实为 Lifecycle，事件注记并入 §2.6）+ §10.2/§10.3/§12 陈旧行关闭

## Phase 2 出口核验（✅ 全部达成）

- ✅ bind:value 三目标双向（方向 fixture + 浏览器实证）· ✅ capturePointer 无参 + Flutter error · ✅ SetValue 诊断 · ✅ 事件字段无静默桩 · ✅ BNF spec §2 同步（含 §10.2/§10.3/§12 follow-up）· ✅ F-1 代码类型项 tracked

- Task 3.1: complete (commits a0c9940f..cc5538a0, fix rounds 2/5: HIGH-1 Vue 实体绕过 + CRITICAL { 回退注入, re-reviews clean ✅) — 类名双保险 + 载荷语料 5×8×17 + 3 个同类缺口（Dioxus 裸文本/Dart $ 插值/Vue 事件属性）+ Vue &-优先转义 + { 特例全后端移除（含 Flutter ${{}}）
- Task 3.1: minor (deferred): ① Dioxus Interpolation 非字面分支发射 "{formatted}" 裸（parser 可达，编译失败 fail-closed，follow-up）② React/vue style_classes 死块清理任务（react/view.rs:481-497）③ parser 插值回退宽松接受（未来可加诊断）

- Task 3.2: complete (commits cc5538a0..fcb9c9a5, review clean ✅) — 单一来源 shared/sanitize.rs::sanitize_css_helpers_js（双调用点）+ 行为等价逐行验证（含 \a 洞修复）+ 22 用例双拷贝断言 + RED 实证
- Task 3.2: deferral-candidate (tracked) — CSS identity-escape（\j）残留于 Rust 参考实现自身 → 注册 deferral（扩展 normalize_css_escapes 单文件变更）
- Task 3.2: minor (deferred): ① React sanitizeCss null 行为变化（String(value ?? '') → String(value)，不可利用、与 OmniView 一致、未报告）② test_sanitizers.js 手工镜像发射体（既有模式，4 emitter 单测 + 钉快照缓解）

- Task 3.3: complete (commits fcb9c9a5..32096432, fix rounds 2/5: MEDIUM-1 array_push renames + LOW-1/3 修复 + LOW-2/4 文档化 + NEW-1 OptionalChain panic, re-reviews clean ✅) — 切片订阅全量（FilterBar 5 slices/0 whole-store）+ 全部读点改写 + 5 类 fallback + bind:value 写回不变
- Task 3.3: deferral-candidate (tracked) — 内联 store action 未发射（IR 不携带内联 store body，需独立任务）
- Task 3.3: minor (deferred): ① 豁免 TS2339/TS2304 重新归因（真根：React 用户类型发射/内联 store/事件类型）② LOW-2 prop-defaults 文档化（签名作用域，改会造成 TDZ 更糟）③ LOW-4 lib-fn 模块作用域排除锁定 ④ e2e 红为 HEAD 既有（A/B 实证）

- Task 3.4: complete (commits 32096432..be281f14, fix round 1/5: F1 computed props deps, re-review ADDRESSED ✅) — ClosureExpr 臂（FilterBar [searchText, demoFrameworks]）+ validate_effect_deps pass（E_EFFECT_DEPS_MISSING 字段级匹配）+ demo 迁移 + computed deps 补 props
- Task 3.4: minor (deferred): ① 内联 store 读取不验证（false-negative only）② F2 no-arg-effect React 文档分歧（既有）③ F3 walker 部分覆盖（IndexExpr 等，与 signal deps 同级既有限制）
- Task 3.5: complete (commits be281f14..418ddc70, review clean ✅) — store BNF 统一（顶层/内联 signal|getter|action，fn 两层报「store 内 fn 成员不允许；用 getter 或 action」+ BNF spec §6.2 同步 + $counter 文档删除）+ React mutation 三修（M2 自读函数式更新 setCount((c)=>c+1)/useStore.setState((s)=>({...}))、M4 action 连续 set() 单对象合并（重复目标防丢写）、M3 空事件对象零 Object.assign + 无参箭头）+ demo 重建 verify-react GATE PASSED（162 tsc / 119 eslint 全豁免）
- Task 3.5: minor (deferred): ① Vue 同款空 Object.assign 噪音（template.rs:99）未修（controller 范围限定 React）② array_sort 回归测试形状随 M2 更新（functional-updater 形式，行为更优）③ 错误恢复的 stray-brace 噪音（store fn 错误后组件/ store 丢弃，既有 recovery 语义）④ eslint 117→119 +2 组成未验证（豁免类内通过，出口检查项）——**2026-08-03 closed-by-gate**（见下方出口核验行）

## Phase 3 出口核验（✅ 全部达成，1 项检查项）

- ✅ 注入载荷语料回归（5×8×17，CRITICAL 群闭环）· ✅ 消毒器单一来源一致性（22 用例双拷贝）· ✅ React 切片订阅 + effect deps 验证（FilterBar [searchText, demoFrameworks] + node eval 实证）· ✅ store BNF 统一 · ✅ BNF spec §6 同步（§4 → Phase 4 域 3 同步时处理）
- ✅ 出口检查项（**closed-by-gate**，2026-08-03 最终评审处置）：eslint 117→119 +2 组成——增长均落在豁免类内，门禁 per-class 匹配验证组成（T7.3：121 全豁免、零失效条目）；类内增长未逐条计数与 T0.4 ①「豁免计数未强制」同源已跟踪项。记录处置使本项不再悬挂。

- Task 4.1: complete (commits 418ddc70..316986dd, fix round 1/5: F1-F4, re-review PASS ✅) — @themes/@theme 语法 + IR + 跨文件语义 pass + 成员检查/String? 拒绝/重复声明错误/span 锚定 + spec §4 同步 + theme= 虚构删除
- Task 4.1: minor (deferred): ① RUNTIME_EXPR_SENTINEL 硬编码重复（drift 风险）② DesignDef.span 缺 serde(default)（当前惰性）

- Task 4.2: complete (commits 316986dd..5164ae6c, review clean ✅) — 三目标接线（React 用户 store 切片订阅 + data-theme effect；Flutter ThemeModeNotifier + main.dart shell；OmniView IR 驱动消除 mode 魔法名）+ 浏览器实证（React/OmniView 实测 data-theme 翻转 + CSS 覆盖生效）+ harness 入口修复（Vite shell，T1.6 阻断解除）
- Task 4.2: minor (deferred): ① Flutter shell home: App() 无参——根组件有必填 props 时 main.dart 不编译（latent，demo 无影响；随 T5.7 或终验处置）② no-@theme Flutter 丢 StateProvider 可变性（watch 保留，无消费方破坏）③ 陈旧 test-results 产物 ④ OmniView 实证无提交 artifact（report-only）

- Task 4.3: complete (commits 5164ae6c..e41cca06, review clean ✅ — 审查者独立重建+实跑复现全部声明) — classes/variants 真实装饰（0 桩 0 TODO）+ 编译期变体合并（ThemeExtension 否决理由成立，设计文档已对齐 d51f1844）+ warnings 通道（learn-omniview 554 诊断实证）
- Task 4.3: minor (deferred): ① layout-only 类双警告 + 未引用类也警告（全局 dedup follow-up）② warning 前缀不一致 [flutter] vs [flutter][design] ③ view_uses_design_classes 不扫 style_classes（真实源不可达）

- Task 4.4: complete (commits d51f1844..68f93878, review clean ✅) — §10.3 四行诊断实现（ZIndex/cursor warning + grid/float error，flex 正确不报）+ @native 删除（干净解析错误）+ __design/ 命名隔离（守卫移除 YAGNI 正确）+ 碰撞场景实证
- Task 4.4: minor (deferred): ① parser 测试残留 "@native" 拼写列表（语义陈旧）② Flutter Design vs design.dart 大小写不敏感文件系统边缘（文档化）③ react-test-app lint 脚本旧路径（clean regen 后坏）④ 根目录陈旧 untracked runtime 文件（gitignored 无害）

- Task 4.5: complete (commits f0e68fb8..6be54ec5, fix round 1/5: MEDIUM-1 label/alt 冲突, re-review ADDRESSED ✅) — a11y 域（label/role/hidden/live/alt 三目标）+ 顺带修复 React 重复 aria-label 与 img Image.network 非法 Dart（HIGH-4 案例提前捕获）+ demo 8 属性迁移 + label/alt 冲突诚实降级
- Task 4.5: minor (deferred): ① 组件级诊断 span 风格（house-wide）② 不可映射 role 双警告（spec §11.6 文档化有意）③ 裸 aria-* 与 a11y 块共存无去重诊断 ④ Vue/Dioxus 静默丢弃 a11y（P1 不在 floor）⑤ warning 在 @for 内重复（既有角色警告同行为）

## Phase 4 出口核验（✅ 全部达成）

- ✅ 主题切换三目标浏览器实证（data-theme 翻转 + CSS 覆盖生效）· ✅ classes/variants Flutter 视觉生效（0 桩 0 TODO）· ✅ design 诊断表全实现（§10.3 四行 + @native 删除）· ✅ a11y 域（shape 断言 + 真实 dart analyze；runtime widget-test 受 Phase 5 门禁阻塞，审查接受替代）· ✅ BNF spec §4/§11 同步
- 交叉验证注记：逐域浏览器实证已覆盖（T2.2 指针捕获/T2.3 值绑定/T4.2 主题）；全量 9 标签页浏览器 + 文档审计归 Phase 7 终验（计划约定）

- Task 5.1: complete (commits 6be54ec5..484fb235, fix round 1/5: HIGH Flutter scroll no-op + MEDIUM×2, re-review PASS ✅) — @scroll/@drag 维度恢复（v2 BNF + non_exhaustive enums + 三后端消费 + node 实证）；HIGH 修复（elementTop 混入文档坐标）+ velocity_max 三目标一致（decay 诚实文档化未实现）+ start==end 硬错误
- Task 5.1: minor (deferred): ① dart-analyze 门禁 warning-count 怪癖（行首 "warning - " 漏计——假「failed unexpectedly」方向，安全；已记录 follow-up）② parse_expr_no_assign 排除三元（doc 注释）③ BNF spec 参数全必填 vs 实现接受子集 ④ OmniView scrubDelay: true 原始发射 vs 显式 1.0

- Task 5.2: complete (commits 484fb235..bfbe8e08, fix round 1/5: MEDIUM-1 clip 视觉 + LOW-2 demo 同步, re-review ADDRESSED ✅) — 白名单 19 + BorderRadiusTween/TextStyle 映射 + React Npx 单位 + 顺带 latent color→DefaultTextStyle 类型修复 + ADR-053 D3 理由修正
- Task 5.2: minor (deferred): ① React width/height 裸数字 keyframe（既有，follow-up）② backgroundColor+borderRadius 组合嵌套 Container(color:) 仍方形绘制（无 fixture 触及）③ 跨目标裁剪 parity 差异（Flutter 裁剪子树 vs CSS 仅自身背景——文档化）

- Task 5.3: complete (commits bfbe8e08..315d221b, review clean ✅) — paused 双形式硬诊断（E_PAUSED_REMOVED verbatim）+ PlaybackProp::Paused 死变体删除 + 文档清剿（spec 实际行 :258/15-grammar-view/11-motion 教学点/demo）
- Task 5.3: note (Phase 7) — golden KeyframeAnimationDemo 预存在 playAll 漂移（非本任务，需 resync）

- Task 5.4: complete (commits 315d221b..99900bb7, review clean ✅) — fade 经 AnimatedSwitcher（KeyedSubtree + 相异 ValueKey 触发过渡，IR fallback 形状精确验证）；wait 字节不变；T0.8 契约测试瘦身至 T5.5-only。补充细节（终验时与文件尾部重复段合并，2026-08-03 最终评审清理）：`mode:"fade"` Flutter 真实现——AnimatedSwitcher 默认过渡 300ms（DSL 无时长参数，v2 BNF transition_mode 为裸字符串，与 wait 发射一致）+ 分支/fallback 稳定 ValueKey（omni_if_0 / omni_if_fb；AnimatedSwitcher 需要子 key 变化才触发过渡——Widget.canUpdate 对同 runtimeType 无 key 子组件视为同一组件原地更新）；`"wait"` 发射字节不变（降级注释仅 React/OmniView 有，Flutter 路径未动）；契约测试拆分为 fade 断言进活跃 assert_control_flow_flutter，`flutter_control_flow_contract` 缩为 T5.5-only（key: ValueKey(item)）保持 #[ignore]；验证 538 lib tests + 全集成套件绿；dart analyze 门禁 15 fixtures 零 error 零 warning（control_flow.omni 含新形状）；widget-test 基建不存在（无 Flutter test 工程），shape 断言 + 真实 dart analyze 替代（T4.5 a11y 同先例）；设计文档同步：域 2 映射表 fade 行「已实现」+ 验证清单勾选
- Task 5.4: minor (deferred): ① flutter_control_flow_contract 注释精度（仅 key 断言失败非全部）② ConditionalIntent.transition_mode 字段注释缺 "fade"（既有漂移）③ widget-test 验证本身（切换可见性 → 淡入过程存在）待补基建（设计文档已注记；shape 断言 + 真实 dart analyze 替代，T4.5 同先例）④ @elif 链 key 形状（omni_if_1...）由同一循环路径覆盖，无专门断言（YAGNI）

- Task 5.5: complete (commits 99900bb7..3a9e7d99, fix round 1/5: MEDIUM map 字面量分类, re-review ADDRESSED ✅) — @for 契约（Flutter Column/ValueKey 分派 + React 索引 key/单次求值 + for-key lint + 契约测试 un-ignore 全绿 + node 行为实证含 key 碰撞证明）
- Task 5.5: minor (deferred): ① 数组字面量首元素启发式（既有对称行为）② typed const 仅值推断（空类型数组假阴性）③ React [scroll] no-op 为 spec 文档化行为 ④ 嵌套滚动列表警告为设计文档行（非 spec，deferral 候选）

- Task 5.6: complete (commits 3a9e7d99..a6378b05, review clean ✅) — _omniview_runtime.dart 共享（3/79 按需门控）+ 类型化配置（字符串解析清零）+ currentFrameTimeStamp scrub + 绑定快照四类全覆（焦点稳定）
- Task 5.6: minor (deferred): ① scrub 措辞不精确（评论/报告级，零代码影响）② legacy scroll 桥接无运行时执行测试（手动验证等价）

- Task 5.7: complete (commits a6378b05..1f5fd577, review clean ✅) — prop 转换层（fontWeight→FontWeight/img 位置参数/icon 150 名表）/==与 hashCode 与哨兵 copyWith/安全 cast/按元素 controller（共享 bug 消灭实证）/保守 const（20 白名单 SDK 验证）；展示项目 1089→333、警告 10000→0；Curves elastic 名修复（门禁归因正当，−300）
- Task 5.7: minor (deferred): ① plain-prop 路径 fontWeight="bolder"/color="red" 仍静默发非法 Dart（无 warn 通道，展示项目不触及）② em/rem/% 静默转裸数字（类型正确语义错误，无诊断）③ 双 emitter 序数奇偶无配对单测（建议回归）④ 0-signal store 未用 _unset（analyzer info）

## Phase 5 出口核验（✅ 达成，1 项待续）

- ✅ @scroll/@drag 维度（node 实证 + 契约测试）· ✅ fade 三目标对等（AnimatedSwitcher）· ✅ @for 契约（node 实证含 key 碰撞证明）· ✅ Flutter 常见输入 dart analyze 全绿（转换层）· ✅ 语义绑定无焦点偷窃（快照比较）· ✅ BNF spec §3/§7 同步（T5.1/T5.4/T5.5 内）
- ⏳ **learn-omniview Flutter 全量 0 错误 → un-ignore**：333 残余（stdlib to_string/len、事件参数接线、store 属性访问形状、measure-position 对等）→ **T5.8 收尾任务**

- Task 5.8: complete (commits 1f5fd577..1a57b52e, review clean ✅) — 333→2（四已知类 + 8 新发现类；lib-fn 跨文件内联 ~130 错误为最大发现）
- Task 5.8a: complete (commits 1a57b52e..196328ce, fix round 1/5: Div 收窄 + String .get, re-review APPROVED ✅) — **learn-omniview Flutter 0 错误 0 警告，门禁 un-ignore 5/5 绿**；事实修正（int+dynamic=num 根因 → format_closure_body 收窄 + select 类型化参数）；type_annotate.rs 静态重注解 pass（发现 IdentifierExpr.type_ann 从未填充）；10 类警告清零
- Task 5.8a: minor (deferred): ① u64 警告未扫 prop_defaults/getter/action 体 ② type_annotate 未接入 computed 发射 ③ 闭包参数遮蔽外层符号（病态场景）④ String? 接收者 .get 仍 `(s)[i]!`（既有）

## Phase 5 出口核验（✅ 全部达成）

- ✅ @scroll/@drag 维度（node 实证 + 契约测试）· ✅ fade 三目标对等 · ✅ @for 契约（node 实证）· ✅ Flutter 常见输入 dart analyze 全绿 · ✅ 语义绑定无焦点偷窃 · ✅ **learn-omniview Flutter 全量 0 错误 0 警告 → 门禁 un-ignore 绿** · ✅ BNF spec §3/§7 同步

- Task 7.0 (added per user direction 2026-08-03): learn-omniview 展示项目重构（v2 语法纯净/结构/10 域覆盖/golden resync/产物清理）——计划文件已更新 + commit 3d02f45b；Phase 7 首任务
- Task 6.1: complete (commits 196328ce..83e1ad35, review clean ✅) — typed slots + 遗留形式三删除（verbatim 诊断文本逐字验证）+ IR/三后端清理 + demo 迁移 + slots_three_forms fixture + spec §5/grammar 同步
- Task 6.1: deferral-candidate (tracked) — 裸 `design` 关键字在元素文本中破坏文本 run（既有 parser 怪癖，demo workaround，独立修复任务）
- Task 6.1: minor (deferred): ① react/mod.rs fmt reflow 残留（无 fmt 门禁）② react/view.rs 陈旧 slot 注释 + 防御路径 ③ 设计文档 prose 删除列表与诊断表文本不一致（L326 vs L354，诊断表为准）④ BNF `('=' string)?` 死片段（@slot name = "x" 泛解析错误 fail-closed）

## Phase 6 出口核验（✅ 全部达成）

- ✅ slot 三形态对等 fixture（default+named+fallback 含裸实例）· ✅ 三删除形式 × 三 P0 后端错误测试（hint 文本断言）· ✅ demo 迁移零残留（grep 验证）· ✅ BNF spec §5 同步

- Task 7.0: complete (commits 83e1ad35..2adff28b, review clean ✅ — 审查者亲验 diff -rq 零差异/计数实测/严格 demo-only) — 展示项目 v2 重构：69 文件纯净审计 + 新 A11yDemo + MotionTriggers 重写 + ForDirectiveKeyDemo 强化 + golden 全同步 + 产物清理（−95k）+ 三目标全绿
- Task 7.0: compiler-followups (registered) — ① unused-import lint 系统性假阳性（元素标签/DesignClass/@store 未计为使用 → 123 条虚假警告，需编译器修复）② Map 字面量返回类型不推断（Map<String, <unknown>>）③ design 关键字文本怪癖（T6.1 已记）④ Flutter CLI capturePointer 硬中止 vs 门禁过滤不一致 ⑤ esbuild 锁 build/debug
- Task 7.0: minor (deferred): ① gitignored 媒体资产（dot.png 等——fresh clone 404，pre-existing，已并入 harness 目录未提交）② EventsDemo 45 标签计数口径 ③ ElementRefDemo:128 历史对比注释

- Task 7.1: complete (commits 2adff28b..1f5a5d86, review clean ✅ 附条件) — 三目标门禁全绿 + 浏览器套件（OmniView 41+4 fixme / React 16+19 fixme / theme-wiring，0 failures）+ 7 个编译器 bug 经浏览器全测暴露（非回归，既有缺陷被 T7.0 demo 暴露）
- Task 7.1: bugs-registered (priority) — **BUG-1 P0 OmniView keyFn 硬编码 item（list.rs:164,275，2 行修复，整 DataTable 标签页死）→ T7.1a**；BUG-3 P0 OmniView keyed 重排永不重渲染（rebind fn 发射未调用，list.rs:458-460）→ T7.1a；BUG-A P0 React let-mut→const TS2588（应用卸载——T1.6 既有 deferral-candidate ② 同根）；BUG-C React 内联 store CartStore 未发射（TS2304 豁免类）；BUG-D &amp; 字面渲染；BUG-E kebab-case style props；BUG-F 小写事件处理器；BUG-2 OmniView 隐藏分支 focus 绑定丢失；另有 parser 文本 run 空格剥离 + CSP frame-ancestors 注记
- Task 7.1: accept-deviation (recorded) — 「0 runtime error」「列表重排保状态」在 fixme 标记下达成（诚实报告，修复后 fixme 自移除）

- Task 7.1a: complete (commits 1f5a5d86..133f1e70, review clean ✅) — BUG-1 item_var 参数化 + BUG-3 rebind 接线（appendChild 移动 + __omni_forItemVar + typeof 守卫）；浏览器实证（44 passed/0 failed，fixme 自移除，KNOWN_ERRORS 移除）
- Task 7.1a: minor (deferred): ① 嵌套 @for for_rebind_fn 向量不恢复（安全 no-op，既有）② 多根 fragment 模板复用克隆仍不可移动（既有，无 demo 使用）

- Task 7.2: complete (commits 133f1e70..8623bd7d, fix round 1/5: MEDIUM 10-design 陈旧句 + 3 LOW, re-review ADDRESSED ✅) — 232 块全量审计 0 失败 + ~40 教学错误修正 + docs-gate CI job + 7 parser 分歧记录；10-design 英文句漏网根因（中文关键词搜索）已修
- Task 7.2: parser-divergences (recorded for follow-up) — union `;` 不一致、@themes/@default-theme `;` 不对称、style-entry `;` 要求、class-value 字符串优先规则、fn-type-param E0010、import 诊断误导、view-text 宽容
- Task 7.2: minor (deferred): ① is_parse_error 单元断言未提交（ad-hoc 验证）② --check 死标志（文档化有意）

- Task 7.3: complete (commits 4ecdee96 + docs commit, review clean ✅) — **注入载荷最终回归 + deferrals 收口 + 设计文档 Implemented**：① 语料扩展——新 `a11y` 插入点（T4.5 表面：a11y 块字符串 → React JSX 属性/Flutter Dart 字符串与降级注释/OmniView HTML 属性）+ `x*/ alert(1) //` 注释终结载荷 → 5 后端 × 9 插入点 × 19 载荷（6 字符 + 4 实体 + 9 利用 = **855 元组**）+ 45 控制编译（9 点 × 5 后端），全绿。（计数修正 2026-08-03 最终评审：原记「5×9×18 = 810」为**安全方向漏计**——实为 19 载荷，门禁断言多于报告值；ledger/报告/设计文档三处同步修正）发现并修复 **2 个 Flutter 注入缺口**（双双 RED 实证）：a11y `label` 未做 Dart `$` 转义（`Semantics(label: 'x${alert(1)}')` 字面插值可执行）；unmappable `role` 降级注释逐字嵌入（`*/` 终止块注释留代码）。修复：label 走 `$`→`\$`（镜像文本路径）+ role 注释 `*/`→`* /` 消毒 + 2 个单测（`test_a11y_label_dollar_escaped_and_role_comment_sanitized`）。② deferrals 收口（.architecture/deferrals.md）：**#19 关闭**（T5.1 默认 extent 推导证据）、**#20 关闭**（2026-08-01 修复补划）、#30 un-ignore 条件达成注记（T5.8a）、#28/#33 实编译探针复核（`m.set("key", 2)` 逐字发射 → TypeError；innerHTML 仍重包 #root，`[data-component="App"]` 2 命中——均保持 open 附证据）、#34 复核一致；**新增 #36-#54 共 19 行**（T7.1 BUG-A/C/D/E/F/2 + T7.2 parser 分歧批次 + T7.0 五项 + T6.1 design 关键字 + T3.2 identity-escape + T1.4/1.6/2.3/3.3 deferral-candidates + T7.1 note 级 + T0.6 poiesis warnings，全带触发条件）。③ 设计文档：状态 Design→**Implemented** + §4 验证协议逐项 ✅（§4.2 终验 + §4.4 八项，附任务证据）+ §5 十域验证清单全部勾选。④ 全量套件终跑：omnilang-core 925 + omnilang all-backends 785/14 suites + generated_code_validation flutter 5/5（learn-omniview dart analyze 0 错误 0 警告）+ verify-react GATE PASSED（166/121 全豁免）+ docs gate 232 块 0 失败 + learn-omniview omniview 构建 0 错误
- Task 7.3: minor (deferred): ① 语料 a11y 点未覆盖 `live`（semantic 限定 polite/assertive，载荷硬错 fail-closed，无独立发射面）② Vue/Dioxus 静默丢弃 a11y（T4.5 既有，P1 不在 floor）③ 注释逃逸消毒仅覆盖 Flutter role 降级注释（其他后端无同类 verbatim 注释嵌入面）④ 终验浏览器套件未重跑（T7.1 全绿后无编译器行为变更触及浏览器面——注入修复仅影响 a11y 载荷路径）

## Phase 7 出口核验（✅ 全部达成）

- ✅ learn-omniview 三目标全量绿（React verify-react GATE PASSED 166/121 全豁免零失效 / Flutter dart analyze 0 错误 0 警告 un-ignore / OmniView 构建 0 错误）· ✅ 9 标签页浏览器全测 0 runtime error（T7.1 全量 + T7.1a 修复后 44 passed/0 failed，fixme 自移除）· ✅ 16 篇文档与编译器事实一致（T7.2 232 块审计 0 失败 + docs-gate CI + T7.3 复核）· ✅ 设计文档 §4 验证协议逐项 ✅（Design→Implemented）· ✅ 注入载荷语料最终回归 5×9×19 = 855 全绿（原「5×9×18 = 810」为安全方向漏计，2026-08-03 最终评审修正；含 a11y 新表面 2 缺口闭环）· ✅ deferrals 收口（#19/#20 关闭、#28/#33 证据更新、#36-#54 注册）

---

## Final-review fix wave（2026-08-03，最终整支评审 MINOR-FIXES 处置——纯文档/台账修正，零代码变更）

- **计划 T7.1 出口改述**：`docs/superpowers/plans/2026-08-02-semantic-syntax-v2.md` T7.1 Step 1 补偏差注记——字面「0 错误 + 豁免清单清空」未达成，最终状态为 166 tsc + 121 eslint 全豁免（清单未清空、零失效）+ React 浏览器 16 passed + 19/35 fixme（BUG-A/BUG-C）；与设计文档 §4.2 诚实出口对齐。
- **语料计数修正**：19 载荷（6 字符 + 4 实体 + 9 利用）× 9 插入点 × 5 后端 = 855 元组 + 45 控制编译（9 点 × 5 后端）；原「18/810」为安全方向漏计，ledger/T7.3 报告/设计文档三处同步修正（早前 5×8×17 为 T3.1 时代历史快照，保持）。
- **五个实质性 minor 入行**：T1.2 ① / T5.7 ① / T2.2 ① / T3.1 ① / T1.3 ④ 已升格注册为 deferral **#55-#59**（见 `.architecture/deferrals.md`，全带触发条件）；T7.3 报告 §5-4「实质性已全部入行」声明同步注记修正。
- **T3.5 出口检查项关闭**：eslint +2 组成——closed-by-gate（门禁 per-class 匹配验证组成，T7.3 121 全豁免零失效；类内增长未计数与 T0.4 ① 同源已跟踪）。
- **follow-up 注册**：#60 OmniView 44-test 浏览器套件入 CI（e2e job 仅跑 React config）；#47 行补 identity-escape（`\j`）语料扩展注记。
- **清理**：文件尾部重复的 T5.4 段已并入主线条目后删除（最终评审 blemish）。

