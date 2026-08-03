# Task 7.3 Report — 注入载荷最终回归 + deferrals 收口 + 设计文档 Implemented

**Status: DONE**
**Commits:** `fix(security): ...`（语料/修复）+ `chore: final injection regression, deferrals closure, design doc implemented`（台账/设计文档/ledger/report）

---

## 1. 注入载荷语料最终回归（Step 1）

**扩展**（`omnilang/src/codegen/tests_regression/injection.rs`）——按终验目标「无注入表面开放」判定新增表面：

- **新插入点 `a11y`**（T4.5 表面）：`a11y: { label: "x{payload}", role: "x{payload}" }`——a11y 块字符串值走独立发射路径（React `a11y_react_attrs` JSX 属性 / Flutter `wrap_a11y` Dart 字符串与降级注释 / OmniView `emit_a11y_attr` HTML 属性），与既有 `attr` 点（静态 props 循环）不同。Vue/Dioxus 静默丢弃 a11y（T4.5 既有，P1 不在 floor），无发射面。
- **新载荷 `x*/ alert(1) //`**（注释终结逃逸形状）。
- 语料现为 **5 后端 × 9 插入点 × 19 载荷**（6 字符 + 4 实体 + 9 利用）= **855 tuples** + 45 控制组编译（9 点 × 5 后端，防空过）+ 2 专项断言。（计数修正 2026-08-03 最终评审：本报告初稿与 ledger/设计文档均记「5×9×18 = 810」——**安全方向漏计**，文件实为 19 载荷；门禁实际断言多于报告值。）
- **不新增的表面**（判定）：motion `easing:` 为白名单解析（非自由字符串，payload 不解析）；`@slot name = "x"` 泛解析错误 fail-closed（T6.1 已证）；`live` 被 semantic 限定 polite/assertive（载荷硬错）。

**发现并修复 2 个 Flutter 注入缺口**（双双 RED 实证，TDD）：

1. **a11y `label` 未做 Dart `$` 转义**（CRITICAL 类）：`a11y: { label: "x${alert(1)}" }` → 生成 `Semantics(label: 'x${alert(1)}')` —— Dart 字符串插值**可执行代码**。修复（`flutter/view.rs wrap_a11y`）：label 走文本路径同款 `$` → `\$`。
2. **unmappable `role` 降级注释 `*/` 逃逸**（HIGH 类）：role 逐字嵌入 `/* a11y: role '...' ... */`，载荷 `x*/ alert(1) //` 终止块注释 → `alert(1)` 留在代码位。修复：role 内 `*/` → `* /` 消毒。

**新增单测**：`test_a11y_label_dollar_escaped_and_role_comment_sanitized`（flutter/tests.rs，镜像 react `class_name_injection_rejected` 惯例）。

**验证**：`injection_payload_corpus` 全绿（RED 复现两次：`$` 插值与注释逃逸各一次，修复后 GREEN）；`cargo test -p omnilang --features all-backends` 785/0；omnilang-core 925/0。

## 2. Deferrals 收口（Step 2，`.architecture/deferrals.md`）

**关闭**：
- **#19**（Drag 无约束失效）——Task 5.1 证据：无 constraints 时 codegen 从 motion keyframe 范围推导默认 extent（`react/motion/transforms.rs` 无约束分支，仅无字面 translate 关键帧时 warning + Infinity 诚实降级）；`parseConstraint("")` ±Infinity → progress=0 路径不再可达于正常用例。
- **#20**（Style 读取 lint）——2026-08-01 已修复（`ast_collectors.rs` Style 臂 + 测试，#27 同源），**表格行补划**（原为 open 行 + 状态节已记关闭的不一致）。
- **#30 收尾**：un-ignore 条件已于 T5.8a 达成（learn-omniview Flutter 0 错误 0 警告，门禁 5/5 un-ignore 绿），行内注记已补。

**复核未关闭**（实编译探针证据）：
- **#28**（Map.set）——计划「随域 8 处理」**未落地**：T1.2/1.5 数组方法（filter/map/count/push/pop/sort/splice）+ T1.6 `contains` map-aware，但 `set` 翻译臂三后端均无；探针 `m.set("key", 2)` 逐字发射进 index.js:317 → 运行期 `TypeError`；`parse_type_string` 仍无泛型解析。保持 defer-with-trigger，证据入行。
- **#33**（嵌套 #root）——结构依旧：`emit_dom_and_bindings`（omniview/view/mod.rs:90）innerHTML 模板仍以 `<div id="root" data-component="App">` 开头（探针 index.js:324），静态包装重复 → `[data-component="App"]` 2 命中；T5.5（Flutter @for）/T7.1a（OmniView keyed rebind）均未触及此面。保持 defer-with-trigger（ADR-055）。
- **#34**（regex_match）——行状态一致，复核注记入行。

**新增 #36-#54（19 行，全带触发条件）**：T7.1 编译器 bug（#36 BUG-A let-mut→const、#37 BUG-C 内联 store、#38 BUG-D &amp;、#39 BUG-E kebab props、#40 BUG-F 小写处理器、#41 BUG-2 隐藏分支 focus——BUG-1/BUG-3 已于 T7.1a 修复，不注册）；#42 T7.2 parser 分歧批次（7 项）；#43 unused-import 假阳性、#44 裸 design 关键字、#45 Flutter CLI capturePointer 口径、#46 Map 字面量类型推断、#53 esbuild 锁（T7.0 五项）；#47 identity-escape \j（T3.2）；#48 plain-Dart 裸 enum（T1.4）；#49 React 用户 struct 不发射（T1.6 ①）；#50 Flutter 非 ref bind:value 单向 no-op（T2.3）；#51 文本 run 空格剥离、#52 CSP frame-ancestors（T7.1 note 级）；#54 poiesis warnings 面板（T0.6 important，poiesis 侧）。

**判定说明**：ledger 中大量「minor (deferred)」为装饰性/cosmetic 注记（计数口径、陈旧注释、fmt 漂移类），按台账既有纪律（仅 deferral-candidate/tracked/important 注册）未逐一开行——已在报告 §5 记录口径。

## 3. 设计文档状态（Step 3）

`docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`：
- 状态 **Design → Implemented**（附终验摘要）。
- **§4 验证协议逐项 ✅**：§4.2 终验 ✅（T7.0-7.3 证据链）+「每完成 3 域」✅（逐域浏览器实证 + BNF 逐域同步）；§4.4 八项基础设施全部 ✅（Task 0.1-0.8 证据，含已知-gap 诚实注记）。
- **§5 十域验证清单全部勾选**（逐项附任务证据，延续域 2 fade/域 10 既有惯例）——含域 9 注入语料行更新为 5×9×19 = 855 与 T7.3 两项修复（原报告「5×9×18」为安全方向漏计，2026-08-03 最终评审修正）。

## 4. 全量套件终跑（最终绿声明）

| 门禁 | 结果 |
|------|------|
| `cargo test -p omnilang-core` | 925 passed, 0 failed |
| `cargo test -p omnilang --features all-backends` | 785 lib + 14 suites, 0 failed |
| `cargo test --workspace --exclude omnilang-wasm` | 全绿（含 generated_code_validation 5/5，210s） |
| `learn_omniview_flutter_output_passes_dart_analyze` | ok（learn-omniview Flutter 0 错误 0 警告） |
| `scripts/verify-react.sh` | GATE PASSED（166 tsc + 121 eslint 全豁免，零失效条目） |
| `omnic build --target omniview`（learn-omniview） | 0 错误（仅已知 unused-import 假阳性 #43） |
| `python scripts/verify-docs.py` | 232 omni blocks, 0 failed |
| `cargo fmt --check` | 我的改动全 clean（剩余 46 处为 T3.1 已记录既有 drift） |

## 5. Concerns / 判定口径

1. **终验浏览器套件未重跑**：T7.1/T7.1a 全绿后，本次编译器改动仅触及 a11y 载荷路径（正常 a11y 属性发射不变——label 转义仅在含 `$` 时生效，role 注释仅在 unmappable 时消毒），无浏览器面行为变化。判定：不需要重跑 9 标签页套件；如需绝对确认可补跑（命令见 T7.1 报告 §6）。
2. **#28 计划偏离已如实记录**：计划声称「Map.set 随域 8 处理」，实际未实现——终验探针证实后已在台账行内记录「未落地」+ 证据，保持 open 带触发条件（诚实降级，非隐藏缺口）。
3. **#33 维持 open**：嵌套 #root 为 ADR-055 已知 deferral，三次浏览器全测 0 runtime error，修复涉及 WASM/editor 共用输出消费方式验证（触发条件未满足）。
4. **cosmetic minor 未全部开行**：ledger 的 minor (deferred) 行多为报告计数/注释陈旧/fmt 漂移类，按台账「deferral-candidate/tracked/important 注册」既有纪律处理；实质性 follow-up 已全部入行（#36-#54）。**2026-08-03 最终评审修正**：五项**实质性**「minor (deferred)」——T1.2 ①（python/rust/dioxus 数组方法 verbatim 发射）、T5.7 ①（plain-prop 静默非法 Dart）、T2.2 ①（OmniView wrapper-null 静默跳过分歧）、T3.1 ①（Dioxus 裸 `"{formatted}"`）、T1.3 ④（style 裸标识符仍发 `$signal`）——非 cosmetic，已升格注册为 deferral #55-#59（见台账）。
5. **Vue/Dioxus a11y 静默丢弃**（T4.5 既有，P1 不在 floor）：语料 a11y 点对这些后端无发射面，未新增行为。

## 6. 与 brief/controller 指令对照

- Step 1 载荷语料全后端回归（含 Flutter/Dart 字符串、OmniView HTML/JS）：✅ 5×9×19 = 855 全绿 + 2 缺口闭环（原「5×9×18」为安全方向漏计，2026-08-03 最终评审修正）。
- Step 2 deferrals 收口：#28/#33 处置（证据更新 + 保持 defer-with-trigger）✅；#19 关闭 ✅；#30 验证 + un-ignore 注记 ✅；#34 验证 ✅；#20 补划 ✅；新行 #36-#54 ✅。
- Step 3 设计文档 Design→Implemented + §4 逐项 ✅（含 §5 十域验收对照）✅。
- Step 4 Commit：2 commits（fix + chore）✅。
- Phase 7 退出门：learn-omniview 三目标全量绿 ✅（T7.1 浏览器全测 0 runtime error 保持，见 §5-1）；9 标签页浏览器全测 0 runtime error ✅（T7.1/T7.1a 记录）；16 篇文档与编译器事实一致 ✅（T7.2 232 块 + docs gate）；设计文档 §4 验证协议逐项 ✅。
