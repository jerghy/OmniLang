# OmniLang 语义级 DSL v2 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现语义级 DSL v2 全量重设计——10 个语法域（9 重设计 + a11y 新增），每域经设计-验证循环（spike → 门禁 → 浏览器行为验证），最终 learn-omniview 三目标全量编译与文档全对齐。

**Architecture:** 沿用三层架构（semantic → IR → codegen）。语法层变更集中：删除文档虚构与零消费者语法（theme=/@native/SetValue/test-id/paused）、收敛多表面语义（value 绑定/闭包）、新增缺失语义（@themes/@theme 接线、a11y 域、@scroll/@drag 控制维度）。验证网先行（Phase 0 修复全部门禁盲区），每域以「手写对照样例」为地道性验收基线，删除语法全部产生硬诊断 + 迁移提示。

**Tech Stack:** Rust（omnilang + omnilang-core workspace）、React 19 + TS strict + Zustand v5（生成目标）、Flutter + Dart + Riverpod 3.0（生成目标）、CI 门禁（cargo test / tsc --strict / dart analyze / eslint react-hooks / Playwright）。

## Global Constraints

- **设计文档**: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（每域模板：语义定义/BNF v2/手写对照样例/映射表/诊断表/验证清单——任务的实现契约）
- **评审依据**: `.architecture/reviews/feature-omniview-semantic-syntax-compilability.md`
- **四项决策 D1-D4**（已确认）：@themes+@theme 接线；bind:value 唯一双向；capturePointer() 无参；删 typed slots
- **验证协议**: 每域 spike（parser+semantic+≥1 后端+测试）→ 门禁（cargo test / tsc --strict / dart analyze）→ 浏览器行为验证 → BNF spec 同步；每 3 域交叉全测；终验三目标全量 + 9 标签页 + 文档全对齐
- **破坏性政策**: 已删除语法 → 硬诊断 + 迁移提示（validate_ref_ops 先例）；demo 迁移随域；文档同步随域同 commit；每域结束零「open-undispositioned」发现
- **BNF 事实纪律**: 本计划中 BNF 为设计意图；每域 spike 第一步「BNF ↔ parser 事实对照」，以 parser 事实为准同步 BNF spec 文件
- **提交纪律**: 每任务独立 commit，conventional commits（feat/fix/refactor/docs/test/chore），无 attribution
- **门禁基线**: `cargo test --workspace` 全绿（当前 1403 tests）+ clippy 零警告；React 输出 `tsc --noEmit --strict` 零错误；Flutter 输出 `dart analyze` 零错误
- **deferrals**: #17/#22 关闭（Phase 0 附证据）；#28/#30/#33 按任务处置

---

## 文件结构

```
crates/omnilang-core/src/
├── parser/                    # 9 域 BNF v2 变更（top_level/{motion,store,design,component}.rs、view/{element,directives}.rs、expr/{prefix,infix}.rs）
├── semantic/                  # validate_ref_ops（10 ops）、validate_semantic_bindings（bind:value）、validate_effect_deps（新增）、validate_a11y（新增）、validate_theme（新增）、validate_class_names（新增）
├── ir/                        # types.rs（RefOpIR 删 SetValue、ElementIR a11y、ListRenderingIntent）、types_motion.rs（scroll/drag 参数 + #[non_exhaustive]）、design.rs（themes/theme_signal）、lower/{view,expr}.rs（$field 通道、类名校验）
└── codegen/sanitize.rs        # is_dangerous_url 迁入（从 omnilang/codegen/omniview/）
omnilang/src/
├── lib.rs                     # OmniViewResult warnings 字段、compile_to_target_json 收敛
├── codegen/
│   ├── shared/                # 数组方法映射表（filter/map/count）、event map_event 单表、design 主题接线共享逻辑
│   ├── react/                 # component/{function,state,computed,effect}.rs（C1/切片订阅/deps）、design.rs（转义/接线/命名）、view.rs（key/@for/fade）、expr.rs（mutation/array_sort）
│   ├── flutter/               # view.rs（@for 分派/ValueKey/class 解析）、ref.rs（capturePointer 诊断/快照比较）、design.rs（ThemeModeNotifier/接线）、motion.rs、motion_widgets.rs→_omniview_runtime.dart、utils.rs（prop 转换）
│   └── omniview/              # list.rs（keyed 契约沿用）、mod.rs（@theme 信号识别）
└── tests/
    ├── property_tests.rs      # proptest 断言修复 + codegen fuzz
    ├── generated_code_validation.rs  # 展示项目 Flutter 门禁扩展
    ├── tests_regression/      # 每域 fixture + 注入载荷语料 + @if/@for 参数化
    └── js_runtime/            # 消毒器一致性测试
test/projects/learn-omniview/  # demo 迁移（92 文件分批）
.github/workflows/ci.yml       # feature 矩阵 job、eslint 门禁、展示项目门禁
docs/                          # BNF spec 逐域重写 v2、16 篇用户文档同步、设计文档
```

---

# Phase 0: 前置基础设施（验证网先行）

> 顺序优先：任何域修改前必须先闭合评审发现的门禁盲区。

### Task 0.1: proptest 真断言 + codegen fuzz 目标

**Files:**
- Modify: `omnilang/tests/property_tests.rs:73-81`
- Test: `omnilang/tests/property_tests.rs`

**Interfaces:**
- Consumes: `omnilang::compile`（现有）、`codegen::compile_omniview` / `compile_react`（现有入口）
- Produces: 编译永不 panic 的真实保证 + codegen 后端 fuzz 覆盖

- [ ] **Step 1: 写失败测试**——修复空测试并新增 codegen fuzz

```rust
#[test]
fn proptest_compile_never_panics_actually_asserts() {
    // 直接调用 proptest 策略，验证 catch_unwind 结果被断言
    proptest::proptest!(|(s in "\\PC{0,200}")| {
        let result = std::panic::catch_unwind(|| { let _ = omni_test_compile(&s); });
        assert!(result.is_ok(), "compile panicked on input: {:?}", s);
    });
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --test property_tests`
Expected: FAIL（现有 `let _result = ...` 吞掉 panic 不失败——先在测试中故意抛 panic 确认新断言能捕获）

- [ ] **Step 3: 实现**——`property_tests.rs:76-80` 改为：

```rust
fn compile_never_panics(s in "\\PC{0,200}") {
    let result = std::panic::catch_unwind(|| {
        let _ = omni_test_compile(&s);
    });
    assert!(result.is_ok(), "compiler panicked on input: {:?}", s);
}

// 新增：codegen 后端 fuzz（字符串格式化最密集、最易 panic 的部分）
fn codegen_never_panics(s in "\\PC{0,200}") {
    for target in ["omniview", "react", "flutter"] {
        let result = std::panic::catch_unwind(|| {
            let _ = compile_for_target(&s, target);
        });
        assert!(result.is_ok(), "codegen({}) panicked on input: {:?}", target, s);
    }
}
```

- [ ] **Step 4: 运行验证通过**

Run: `cargo test -p omnilang --test property_tests`（`PROPTEST_CASES=100`）
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/property_tests.rs
git commit -m "test: fix vacuous compile_never_panics + add codegen fuzz targets"
```

### Task 0.2: 增量缓存恢复未变更文件诊断

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:388-622`（`analyse_multi_cached`）
- Test: `crates/omnilang-core/src/analyse.rs`（模块测试段）

**Interfaces:**
- Consumes: `CachedFile { ir, diagnostics, ... }`（现有结构，diagnostics 字段已写入未读）
- Produces: 缓存命中文件的诊断合并进输出——`success` 与诊断真实一致

- [ ] **Step 1: 写失败测试**——两文件其一有错误，缓存编译两次，第二次仍报错

```rust
#[test]
fn cached_run_keeps_diagnostics_of_unchanged_files() {
    let a = "view A { @click { } }";          // 有错误（空 handler 或故意非法）
    let b = "view B { }";                      // 干净
    let out1 = analyse_multi_cached(&[a, b], None);
    assert!(out1.diags.iter().any(|d| d.code == SOME_ERROR));

    let out2 = analyse_multi_cached(&[a, b], out1.cache.as_ref());
    assert!(out2.diags.iter().any(|d| d.code == SOME_ERROR), "cache hit dropped diagnostics");
    assert!(!out2.success, "success must not lie while errors exist");
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang-core analyse_multi_cached`
Expected: FAIL（第二次运行 diags 为空）

- [ ] **Step 3: 实现**——缓存命中分支（400-408 行附近）合并 `cached.diagnostics`：

```rust
// 命中分支：extend diags with cached.diagnostics（浅克隆；命中代价换取诊断完整性）
diags.extend(cached.diagnostics.iter().cloned());
```

- [ ] **Step 4: 运行验证通过**

Run: `cargo test -p omnilang-core analyse_multi_cached`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/analyse.rs
git commit -m "fix: cached analysis restores diagnostics of unchanged files"
```

### Task 0.3: is_dangerous_url 移入 shared + feature 矩阵 CI job

**Files:**
- Modify: `crates/omnilang-core/src/codegen/sanitize.rs`（迁入 `is_dangerous_url`）、`omnilang/src/codegen/omniview/view/element.rs:40-51`（改 `pub use`）、`omnilang/src/codegen/react/view.rs:16`、`omnilang/src/codegen/vue/template.rs:32`、`omnilang/src/codegen/dioxus/view.rs:34`（改引用路径）
- Modify: `.github/workflows/ci.yml`（feature 矩阵 job；启用或废弃 `test-all-backends` `if: false`）
- Test: `crates/omnilang-core/src/codegen/sanitize.rs` 测试段

**Interfaces:**
- Consumes: `is_dangerous_url(&str) -> bool`（现有行为：剥离 CR/LF/TAB 后拦截 javascript:/data:/vbscript:）
- Produces: `crates/omnilang-core::codegen::sanitize::is_dangerous_url`——三后端经 `omnilang::codegen::shared::sanitize` 或直接引用

- [ ] **Step 1: 写失败测试**

```bash
cargo check -p omnilang --no-default-features --features codegen-react
```
Expected: FAIL（E0432 unresolved import crate::codegen::omniview）

- [ ] **Step 2: 实现**——移动函数到 `sanitize.rs`（保留原测试），OmniView 侧 `pub use` 兼容：

```rust
// crates/omnilang-core/src/codegen/sanitize.rs
pub fn is_dangerous_url(raw: &str) -> bool { /* 原实现原样迁移 */ }
// omnilang/src/codegen/omniview/view/element.rs
pub use crate::codegen::shared::sanitize::is_dangerous_url; // 或直接改引用
```

- [ ] **Step 3: 验证**——三个后端各自独立编译通过

```bash
cargo check -p omnilang --no-default-features --features codegen-react
cargo check -p omnilang --no-default-features --features codegen-vue
cargo check -p omnilang --no-default-features --features codegen-dioxus
```
Expected: PASS ×3 + `cargo test -p omnilang-core sanitize` PASS

- [ ] **Step 4: 更新 CI**——新增 per-feature check 矩阵 job（`ci.yml` 仿 codegen-validate job）：

```yaml
feature-matrix:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      feature: [codegen-react, codegen-vue, codegen-dioxus, codegen-flutter, codegen-omniview]
  steps:
    - run: cargo check -p omnilang --no-default-features --features ${{ matrix.feature }}
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/sanitize.rs omnilang/src/codegen/ .github/workflows/ci.yml
git commit -m "fix(codegen): move is_dangerous_url to shared sanitize; add per-backend feature matrix CI"
```

### Task 0.3a: 解耦 vue/dioxus 跨后端导入（feature-matrix 门禁转绿）

> 来源：T0.3 审查发现（Important-1）——计划缺口补丁。feature-matrix job 在 vue/dioxus 上 RED，因预存在耦合：`omnilang/src/codegen/vue/template.rs:127-128` 导入 `crate::codegen::react::view`（format_style_*）、`omnilang/src/codegen/dioxus/mod.rs:65,222,292` 导入 `super::rust::rs_type`。均为「非 P0 后端借共享格式化逻辑」的同构问题。

**Files:**
- Modify: `omnilang/src/codegen/vue/template.rs:127-128`（react::view 依赖 → 提取/复用 `codegen/shared/` 等价实现或本地内联小函数）
- Modify: `omnilang/src/codegen/dioxus/mod.rs:65,222,292`（rust::rs_type 依赖 → 同法处理）

**Interfaces:**
- Consumes: 无新接口；仅消除对 sibling 后端的 `crate::codegen::<other>` 导入
- Produces: `cargo check -p omnilang --no-default-features --features codegen-vue` 与 `--features codegen-dioxus` 全绿；feature-matrix job 全绿

- [ ] **Step 1: 写失败测试**——`cargo check -p omnilang --no-default-features --features codegen-vue` / `--features codegen-dioxus`：确认当前 E0432（T0.3 后残余 2/3 错误）
- [ ] **Step 2: 实现**——消除两处 sibling 后端导入：优先复用 `crates/omnilang-core/src/codegen/shared/` 或 `omnilang/src/codegen/shared/` 中的等价逻辑；无等价共享则在该后端模块内提取最小本地实现（复制不得超过 20 行，超出则上报 BLOCKED 由控制器定夺共享提取）
- [ ] **Step 3: 验证**——两后端独立 check 全绿；`cargo check --workspace` 默认组合全绿；`cargo test -p omnilang --lib` 全绿
- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/vue/ omnilang/src/codegen/dioxus/
git commit -m "fix(codegen): decouple vue/dioxus from sibling backend imports — feature matrix green"
```

### Task 0.4: react-test-app 真实化（React 19 + Zustand 5 + strict tsc + eslint）

**Files:**
- Modify: `test/projects/learn-omniview/build/react-test-app/package.json`（react ^19、zustand ^5、新增 typescript/eslint 依赖）、`tsconfig.json`（`strict: true` + `*.module.css` ambient declaration）
- Create: `test/projects/learn-omniview/build/react-test-app/eslint.config.js`、`env.d.ts`、`scripts/verify-react.sh`
- Modify: `.github/workflows/ci.yml`（调用 verify 脚本）

**Interfaces:**
- Produces: `verify-react.sh`——`tsc --noEmit` + `eslint`（react-hooks 规则）对全部生成文件，非零退出即失败

- [ ] **Step 1: 建门禁脚本**

```bash
# scripts/verify-react.sh
cd test/projects/learn-omniview/build/react-test-app
npx tsc --noEmit -p tsconfig.json
npx eslint src --max-warnings 0
```

- [ ] **Step 2: tsconfig 升级**

```jsonc
// tsconfig.json
{ "compilerOptions": { "strict": true, "jsx": "react-jsx", "moduleResolution": "bundler",
  "target": "ES2022", "module": "ESNext", "skipLibCheck": true } }
// env.d.ts
declare module "*.module.css" { const classes: Record<string, string>; export default classes; }
```

- [ ] **Step 3: 修复门禁暴露的错误**——按 ts/react 评审：C1 函数发射、C2 样式通道、typed useState、fn 参数类型等（本任务只修到「门禁绿」，根因修复见 Task 1.6/1.3；先临时禁掉未修根因的严格项亦可，但须记录）。升级依赖后：

```bash
cd test/projects/learn-omniview/build/react-test-app
npm install react@^19 zustand@^5 typescript eslint eslint-plugin-react-hooks
bash scripts/verify-react.sh
```
Expected: 失败清单 = 真实缺口（作为后续任务输入），本任务目标：门禁本身真实可用（能失败、能通过）

- [ ] **Step 4: 最小修复使门禁可运行**——`*.module.css` ambient 声明、tsc 脚本注册、eslint 配置（react-hooks exhaustive-deps 开）。对尚未根因修复的错误（C1/C2 等），在 `verify-react.sh` 中列出已知豁免清单（带 issue 引用），根因修复后移除。

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/build/react-test-app/ .github/workflows/ci.yml
git commit -m "ci: real react gate — React 19 + Zustand 5 + strict tsc + eslint react-hooks"
```

### Task 0.5: 展示项目 Flutter 门禁

**Files:**
- Modify: `omnilang/tests/generated_code_validation.rs:46-101`（新增 learn-omniview 编译 + 全输出 dart analyze）
- Modify: `.architecture/deferrals.md`（#17 关闭附证据）

**Interfaces:**
- Consumes: 现有 per-file 枚举模式（`flutter_analyze_targets`）
- Produces: codegen-validate job 覆盖 learn-omniview 全量 Dart 输出

- [ ] **Step 1: 写测试**——learn-omniview main.omni → Flutter → 全输出文件 dart analyze

```rust
#[test]
fn learn_omniview_flutter_output_passes_dart_analyze() {
    // 沿用现有：build target flutter → collect all .dart files → run dart analyze
    // 无 dart 工具链时 SKIP（沿用现有优雅 SKIP 模式）
}
```

- [ ] **Step 2: 运行验证失败**——当前 learn-omniview Flutter 输出含不可编译 Dart（match #30：`Rating` 类未发射、`level` 未声明）

Run: `cargo test -p omnilang --test generated_code_validation learn_omniview_flutter`
Expected: FAIL（dart analyze 报错，与评审一致）

- [ ] **Step 3: 暂标记 skip + 记录**——本任务只立门禁；根因（match 变体臂）在 Task 1.4 修复后移除 skip。deferral #17 关闭（6 个原始 CRITICAL 已修 + 门禁全量枚举证据），#30 更新为「learn-omniview Flutter 门禁已立，待 match 修复」。

- [ ] **Step 4: Commit**

```bash
git add omnilang/tests/generated_code_validation.rs .architecture/deferrals.md
git commit -m "ci: showcase project Flutter gate — learn-omniview dart analyze; close deferral #17"
```

### Task 0.6: warnings 网关收口

**Files:**
- Modify: `omnilang/src/lib.rs:143-152`（`OmniViewResult` 增 `warnings: Vec<String>` 并转发 `gen_output.warnings`）、`omnilang/src/lib.rs:239-257`（`compile_to_target_json` 补 warnings 键或删除）、`omnic/src/commands/{build,run,watch}.rs`（打印 warnings）、`crates/omnilang-core/src/lib.rs:84-121`（`run_pipeline` 末尾运行 lint，从 `analyse_multi` 提升）

**Interfaces:**
- Consumes: `LintRule` 注册表（现有）、`gen_output.warnings`
- Produces: 三条网关（CLI/WASM 单文件/编辑器）warnings 可见

- [ ] **Step 1: 写测试**——单文件路径（`compile_omniview`）保留警告

```rust
#[test]
fn compile_omniview_surfaces_warnings() {
    let src = r#"view X { <div class: "{'x'}" > }"#; // 触发已知 warning 的输入
    let out = compile_omniview(src);
    assert!(!out.warnings.is_empty());
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 warnings 被丢弃）

- [ ] **Step 3: 实现**——`OmniViewResult` 加字段 + lint 提升（`run_pipeline` 末尾调用 `run_lints`；`analyse_multi` 改为共享，避免双管线漂移）+ CLI 三命令打印 `data["warnings"]`

- [ ] **Step 4: 运行验证通过** + `cargo test -p omnilang` 全绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/lib.rs omnic/src/commands/ crates/omnilang-core/src/lib.rs
git commit -m "fix: warnings surfaced through CLI and single-file pipelines; lint runs in shared run_pipeline"
```

### Task 0.7: 治理同步

**Files:**
- Modify: `.architecture/deferrals.md`（#17/#22 关闭附证据；#18/#19/#21/#23/#28/#33 补触发条件）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md:2`（状态 Draft → Approved + 最后同步 commit）
- Delete: `crates/omnilang-core/src/lib.rs.orig`（0 字节死文件）
- Modify: `crates/omnilang-core/src/codegen/mod.rs:34-39`（陈旧注释「backends 在 separate crates」→ 实际结构）

- [ ] **Step 1: 逐项核对事实**（#17 已修证据在 `generated_code_validation.rs:31-37,358-362`；#22 实测 `cargo check --features all-backends --tests` 通过）→ 更新台账

- [ ] **Step 2: 删除 lib.rs.orig + 修注释 + spec 状态**

- [ ] **Step 3: Commit**

```bash
git add .architecture/deferrals.md docs/superpowers/specs/ crates/omnilang-core/src/
git commit -m "chore: governance sync — close #17/#22, spec Approved, remove dead .orig"
```

### Task 0.8: 参数化 fixture 补 @if/@for + WASM FFI 契约统一

**Files:**
- Create: `omnilang/tests/fixtures/parameterized/control_flow.omni`（@if ternary/transition + @for key_expr + empty_body + [scroll]）
- Modify: `omnilang/tests/tests_regression/parameterized.rs`（注册 fixture）
- Modify: `omnilang-wasm/src/lib.rs`（M1：单一 DiagnosticJson 序列化器；M4：`compile_multi_to_omniview` 停止每调用序列化缓存；M5：`diagnostics`/`analyse_dependencies` 走 `analyse_multi_cached`）
- Modify: `crates/omnilang-core/src/ffi.rs`（统一 `diagnostic_to_json_value` 到单一 Serialize struct）

**Interfaces:**
- Produces: `control_flow.omni` fixture 三后端断言；WASM 单一 `DiagnosticJson`（含 end_line/end_col，统一小写 severity）

- [ ] **Step 1: 写 fixture + 测试**

```omni
// control_flow.omni —— 三后端参数化断言：@if/@for 形状
view ListPage {
  @for item in items key={item.id} {
    <div>{item.name}</div>
  }
  @if show mode:"fade" { <div>shown</div> } @else { <div>hidden</div> }
}
```

- [ ] **Step 2: 注册进 parameterized.rs**——每后端断言：React 有 `key={`、Flutter 有 `ValueKey`（预期行为；当前 Flutter 失败 → 记录为 Task 5.5 输入）

- [ ] **Step 3: WASM 契约统一**——`#[derive(Serialize)] struct DiagnosticJson` 单一声明，`ffi.rs`/`lib.rs` 全部信封改 typed struct；`compile_multi_to_omniview` 返回 `cache_state: null`；wasm 单元测试断言形状相等

- [ ] **Step 4: 运行验证**——`cargo test -p omnilang --features all-backends`（含 #22 修复回归）+ wasm 测试

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/ omnilang-wasm/src/ crates/omnilang-core/src/ffi.rs
git commit -m "test: control_flow parameterized fixture; fix(wasm): unify DiagnosticJson contract, stop per-call cache serialization"
```

**Phase 0 退出门**: react-test-app strict tsc 全绿；codegen-validate 含展示项目 Flutter；proptest 真断言；feature 矩阵 job；warnings 三网关可见；deferral #17/#22 关闭。

---

# Phase 1: 域 8 Expressions（地基）

### Task 1.1: 闭包语义统一

**Files:**
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（§6.3/§9.3 箭头闭包示例 → 管道 `|x|`）
- Modify: `crates/omnilang-core/src/parser/expr/prefix.rs`（确认 `|x|` 唯一通用闭包；通用表达式位置 `=>` → 报错带提示）
- Modify: `crates/omnilang-core/src/error.rs`（新诊断码）
- Test: `crates/omnilang-core/src/parser/expr/prefix.rs` 测试段

**Interfaces:**
- Consumes: 现有 `ClosureExpr`（管道形式）
- Produces: 通用表达式位置箭头 → `E_CLOSURE_ARROW` 错误「通用闭包用管道 |x|；箭头仅用于事件内联」

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn fat_arrow_rejected_in_general_expressions() {
    let err = parse_expr("items.filter(item => item.done)").unwrap_err();
    assert!(err.message.contains("|x|"), "must hint pipe closure: {}", err.message);
}
#[test]
fn pipe_closure_parses() {
    assert!(parse_expr("items.filter(|item| item.done)").is_ok());
}
```

- [ ] **Step 2: 运行验证失败**（当前 `=>` 在通用位置报裸 Unexpected token 或 parse 错误）

- [ ] **Step 3: 实现**——prefix parser：通用表达式遇 `=>`（非 match 臂、非事件内联上下文）报 `E_CLOSURE_ARROW` + 迁移提示

- [ ] **Step 4: spec 文档同步**——§6.3/§9.3 全部示例改管道（`items.filter(|item| ...)`）；`count` 用法同步 Task 1.2

- [ ] **Step 5: 运行验证** + Commit

```bash
git add crates/omnilang-core/src/parser/expr/prefix.rs crates/omnilang-core/src/error.rs docs/superpowers/specs/
git commit -m "feat(parser): pipe closure is the only general closure; arrow hints in general expressions"
```

### Task 1.2: 数组方法语言化（filter/map/count）

**Files:**
- Modify: `crates/omnilang-core/src/semantic/`（数组方法解析——`expr.filter(|x| ...)` 识别为语言方法）、`crates/omnilang-core/src/ir/types.rs`（或 lowering：数组方法 → `ExprIR::ArrayMethod`）
- Modify: `omnilang/src/codegen/shared/expr.rs`（映射表：filter → JS `filter`/Dart `where`；map → `map`；count → JS `filter(pred).length`/Dart `where(pred).length`）
- Modify: `omnilang/src/codegen/{react,flutter,omniview}/expr.rs`（消费映射）
- Modify: `docs/omniview/08-stores.md:17`（`.filter()` 从「侥幸可用」转正式语义）与 `docs/omniview/12-standard-library.md`（方法表补充）
- Test: 参数化 fixture `array_methods.omni`（三后端）

**Interfaces:**
- Consumes: 管道闭包（Task 1.1）
- Produces: `ExprIR::ArrayMethod { method: Filter|Map|Count, subject, predicate }`

- [ ] **Step 1: 写失败测试**（三后端生成形状）

```rust
// react: items.filter(|i| i.done) → items.filter((i) => i.done)
// flutter: → items.where((i) => i.done).toList()
// count: react → items.filter((i) => i.done).length；flutter → items.where((i) => i.done).length
```

- [ ] **Step 2: 运行验证失败**（当前：未知方法回退为原样发射——Flutter 生成 `items.filter(...)` 编译失败）

- [ ] **Step 3: 实现**——语义识别 + IR 节点 + 三后端映射；`array_filter` 等 stdlib 函数保留不动

- [ ] **Step 4: 运行验证**——三后端 fixture 编译（tsc + dart analyze）+ node 执行测试（filter/count 结果正确）

- [ ] **Step 5: 文档同步 + Commit**

```bash
git add crates/omnilang-core/src/ omnilang/src/codegen/shared/ omnilang/src/codegen/react/ omnilang/src/codegen/flutter/ docs/omniview/
git commit -m "feat: array methods filter/map/count as language methods with cross-target mapping"
```

### Task 1.3: 样式表达式通道 $field（C2 根因）

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/expr.rs:557-565`（`lower_expr_to_json_from_ir`：仅真信号/store 引用用 `$signal` 下划线形式；`obj.field` → `$field: ["obj","field"]`）
- Modify: `omnilang/src/codegen/react/view.rs:1129-1133`（`format_style_value_expr` 处理 `$field`）
- Modify: `omnilang/src/codegen/omniview/`（同通道消费处）
- Test: `omnilang/tests/tests_regression/`——`@for` + `style: { color: fn(item.field) }` fixture

**Interfaces:**
- Consumes: 现有 JSON 标记通道（`$signal`/`$concat`/`$call`）
- Produces: `$field: [obj, field]` 形式——React 发射 `obj.field`（真实表达式），OmniView 同

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn style_expr_field_access_not_mangled() {
    // @for 项字段：style: { color: getScoreColor(row.score) }
    // react 输出必须含 "getScoreColor(row.score)"，不得含 "row_score"
    let react = compile_react(src);
    assert!(react.contains("row.score"));
    assert!(!react.contains("row_score"));
}
```

- [ ] **Step 2: 运行验证失败**（当前生成 `getScoreColor(row_score)`——评审 C2 实测）

- [ ] **Step 3: 实现**——lowering 判定：`IdentifierExpr.field` 的 obj 解析为信号/store 引用 → `$signal`；否则 `$field`；两个 codegen 消费端补 `$field` 分支

- [ ] **Step 4: 运行验证**——fixture 三目标 + node 执行（style 值正确）+ OmniView 回归（评审指出 OmniView 同样坏）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/expr.rs omnilang/src/codegen/react/view.rs omnilang/src/codegen/omniview/ omnilang/tests/
git commit -m "fix: style expression channel — \$field form for field access, \$signal only for true signals"
```

### Task 1.4: match Flutter 变体臂 + 本地类型发射（deferral #30）

**Files:**
- Modify: `omnilang/src/codegen/flutter/expr.rs`（match 臂：`case "Star": return ...` 变体臂——`Rating` 类发射缺失、`level` 未声明）
- Modify: `omnilang/src/codegen/flutter/mod.rs`（match 用到的本地类型声明收集）
- Modify: `omnilang/tests/generated_code_validation.rs`（移除 Task 0.5 的 skip）
- Modify: `.architecture/deferrals.md`（#30 关闭）

**Interfaces:**
- Consumes: `MatchIR`（现有）+ 类型信息
- Produces: 含 match 变体臂的组件 Flutter 输出通过 dart analyze

- [ ] **Step 1: 写失败测试**——`MatchBreadthDemo.omni` 的 Flutter 输出编译：

```rust
#[test]
fn match_variant_arms_flutter_compiles() {
    // 生成输出含 switch/if-case 变体臂 + 完整类型发射；dart analyze 0 错误
}
```

- [ ] **Step 2: 运行验证失败**（评审实测：`switch (r._variant) { case "Star": ... }` 且 `Rating` 类未发射）

- [ ] **Step 3: 实现**——match 表达式（含字面量臂与变体臂）Flutter 发射修复；match 引用的本地类型在文件级收集发射

- [ ] **Step 4: 运行验证**——MatchBreadthDemo 专项错误清零（`Undefined class 'Rating'`/`Undefined name 'level'` 消失，快照断言）；**保持 Task 0.5 门禁 `#[ignore]`**——T0.5 实测门禁 RED 于 1125 错误/21 类（Curves easing 312、motion controller 重复 225、stdlib 方法发射 202、String→Color props 84 等），un-ignore 条件 = Phase 5 Flutter 批次（5.5/5.6/5.7）完成后全量 dart analyze 0 错误

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/ omnilang/tests/ .architecture/deferrals.md
git commit -m "fix(flutter): match variant arms emit complete types — close deferral #30"
```

### Task 1.5: BlockExpr 非 P0 硬诊断 + array_sort comparator（React 实测 bug）

**Files:**
- Modify: `omnilang/src/codegen/python/expr.rs:54-56`、`omnilang/src/codegen/vue/expr.rs:40-42`（语句体 BlockExpr → `E_CODEGEN` 诊断而非丢弃）
- Modify: `crates/omnilang-core/src/codegen/shared/expr.rs:144-147`（`debug_assert` → 正式诊断路径）
- Modify: `crates/omnilang-core/src/stdlib/array.rs:263`（`array_sort` comparator：双参数闭包直接作比较器——JS 目标发射 `[...arr].sort((a,b) => ...)`；单参数闭包保留 key-extractor 语义）
- Test: 两个回归测试（BlockExpr 诊断 + array_sort 实际排序）

**Interfaces:**
- Consumes: `SortIR`（现有）、`BlockExpr`（现有）
- Produces: `array_sort` 双参数闭包 → 真实比较器（修复「Toggle Sort 不排序」）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn array_sort_two_arg_closure_sorts() {
    // items.sort(|a,b| a.name > b.name) → 生成 JS comparator；node 执行断言排序结果
}
#[test]
fn block_expr_statements_diagnosed_on_python_vue() {
    // { let x = 1; x } → python/vue 目标 E_CODEGEN 错误，非静默丢弃
}
```

- [ ] **Step 2: 运行验证失败**（评审 M1 实测：comparator 恒 0 不排序；python/vue 静默丢语句）

- [ ] **Step 3: 实现**——array_sort 发射修正 + BlockExpr 诊断

- [ ] **Step 4: 运行验证**——node 执行排序断言 + python/vue fixture 断言错误

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/stdlib/array.rs crates/omnilang-core/src/codegen/shared/expr.rs omnilang/src/codegen/python/ omnilang/src/codegen/vue/
git commit -m "fix: array_sort comparator semantics; block-expr statements hard-diagnosed on python/vue"
```

### Task 1.6: React 生成质量批次 A（C1 函数发射 + TS 类型）

**Files:**
- Modify: `omnilang/src/codegen/react/component/function.rs:200-249`（`collect_called_fns_from_view` 补 `Interpolation`/`el.props`/`inline_style`/`bindings` 遍历）
- Modify: `omnilang/src/codegen/react/component/function.rs:41-49`（fn 参数从 `FunctionIR` 类型注释）
- Modify: `omnilang/src/codegen/react/component/state.rs:40-43`（`useState<T>([])` 类型化初始化）

**Interfaces:**
- Consumes: `collect_view_read_signals`（state.rs:482-509 已有覆盖模式——对齐它）
- Produces: 组件内被插值/内联样式引用的函数全部发射；`const greet = (name: string) => ...`；`useState<RowItem[]>([])`

- [ ] **Step 1: 写失败测试**——DataTable 类组件 fixture：

```rust
#[test]
fn interpolation_called_fns_are_emitted() {
    // src 含 {getScoreColor(row.score)} 插值调用 → react 输出声明该函数
    let react = compile_react(src);
    assert!(react.contains("const getScoreColor"));
    assert!(react.contains("getScoreColor(row.score)"));
}
```

- [ ] **Step 2: 运行验证失败**（评审 C1 实测：6 个未声明函数 TS2304）

- [ ] **Step 3: 实现**——遍历补全（对齐 `collect_view_read_signals` 覆盖）+ 参数类型 + typed useState

- [ ] **Step 4: 运行验证**——learn-omniview React 输出 `tsc --noEmit --strict` 错误数显著下降（对照 Task 0.4 基线 60+）；逐步移除 verify-react.sh 豁免清单

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/ omnilang/tests/ test/projects/learn-omniview/build/react-test-app/
git commit -m "fix(react): emit functions called from interpolations/props/styles; typed fn params and useState"
```

**Phase 1 退出门**: 闭包唯一性测试；数组方法三目标编译+运行；style 通道 fixture 全绿；match Flutter 进门禁；array_sort node 断言；learn-omniview React strict tsc 基线大幅下降。BNF spec §6/§9 与 parser 事实一致。

---

# Phase 2: 域 1 Ref + 域 7 Events

### Task 2.1: RefOpIR 删 SetValue（11 → 10 ops）

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`（删 `RefOpIR::SetValue`）、`crates/omnilang-core/src/semantic/validate_ref_ops.rs:442,486-563`（删 rewrite + DirectAssign 特例）、`omnilang/src/codegen/{flutter/ref.rs:982,omniview/expr.rs:417,react/component/mod.rs:680}`（删 arm）
- Modify: `crates/omnilang-core/src/error.rs`（`ref.value = x`/`ref.setValue(x)` → 诊断「用 bind:value」）

**Interfaces:**
- Consumes: 现有 RefOpIR
- Produces: 10 ops 集合 + 写路径硬诊断

- [ ] **Step 1: 写失败测试**——`ref.value = x` → error + 迁移提示；`bind:value` 不受影响

- [ ] **Step 2: 运行验证失败**（当前 SetValue 存在且无诊断）

- [ ] **Step 3: 实现**——枚举删 arm + rewrite 路径删除 + 新诊断；三后端删消费

- [ ] **Step 4: 运行验证**——`cargo test -p omnilang-core`（ref 域 20+ 测试）+ 三后端编译

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/types.rs crates/omnilang-core/src/semantic/validate_ref_ops.rs omnilang/src/codegen/
git commit -m "refactor(ref): remove RefOpIR::SetValue — write ops diagnosed to bind:value"
```

### Task 2.2: capturePointer() 无参（D3）

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_ref_ops.rs`（参数校验：有参 → error）、`omnilang/src/codegen/react/`（活动指针实现）、`omnilang/src/codegen/omniview/expr.rs`（runtime 活动 pointerId）、`omnilang/src/codegen/flutter/ref.rs:983-990`（error 诊断 + `UnsupportedError` throw 而非 TODO 注释）

**Interfaces:**
- Consumes: RefOpIR CapturePointer/ReleasePointer
- Produces: 无参语义 + Flutter 硬诊断（spec §10.3 兑现）

- [ ] **Step 1: 写失败测试**——`capturePointer(1)` → error「capturePointer 无参数」；Flutter `capturePointer()` → error + 生成代码含 `UnsupportedError`（非注释）

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——语义参数校验 + 三后端（React：`setPointerCapture` 活动指针；Flutter：诊断 + throw）

- [ ] **Step 4: 运行验证**——ElementRefDemo 迁移（去 `capturePointer(1)` 编造值）+ 浏览器指针捕获行为

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/ omnilang/src/codegen/react/ omnilang/src/codegen/flutter/ omnilang/src/codegen/omniview/ test/projects/learn-omniview/
git commit -m "feat(ref): capturePointer() parameterless; flutter hard error per spec"
```

### Task 2.3: 值绑定收敛（D2）

**Files:**
- Modify: `crates/omnilang-core/src/parser/view/element.rs`（删 `value={sig}` 语义绑定形式；`bind:value` 保留）、`crates/omnilang-core/src/semantic/validate_semantic_bindings.rs`（改 bind:value 验证）、`omnilang/src/codegen/react/component/state.rs:970-982`（bind:value → `value`+`onChange` 双向）、`omnilang/src/codegen/flutter/ref.rs:564-602`（保留双向）、删除 auto-bind 行为（FormPage `value={""}` 反模式消失）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` §2.2/§2.4 + `docs/omniview/06-templates.md`（统一 bind:value 教学）

**Interfaces:**
- Consumes: v1 `value={sig}` 语法（删除）
- Produces: 唯一双向绑定 `bind:value`；`value={expr}` = 普通单向 prop

- [ ] **Step 1: 写失败测试**——方向一致性 fixture：

```rust
#[test]
fn bind_value_is_bidirectional_on_all_targets() {
    // react 输出必须含 onChange 写回（评审实测当前无 → 修复）；flutter 含 onChanged 回写
    let react = compile_react(src);
    assert!(react.contains("onChange"));
}
#[test]
fn value_sig_binding_form_rejected() {
    // 旧语义绑定形式 → error「用 bind:value 或普通 value prop」
}
```

- [ ] **Step 2: 运行验证失败**（React 当前单向——评审 HIGH-1 实测）

- [ ] **Step 3: 实现**——parser/semantic 收敛 + React 补 onChange + auto-bind 删除（无隐式绑定：`value` 永远显式）+ demo 迁移（FormPage 删 `value={""}` 反模式）

- [ ] **Step 4: 运行验证**——浏览器：输入回写信号、信号更新回写控件（三目标）；spec/06-templates 同步

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ omnilang/src/codegen/react/ omnilang/src/codegen/flutter/ docs/ test/projects/learn-omniview/
git commit -m "feat(value): bind:value sole two-way binding; remove value={sig} form and implicit auto-bind"
```

### Task 2.4: Events 字段桩诊断化 + map_event 单表 + FnRef 回归

**Files:**
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs:424-581`（字段提取桩 → warnings 通道 + 稳定映射）、`omnilang/src/codegen/shared/event.rs`（`map_event` 单表；react/utils.rs:222 与 omniview/utils.rs:42-44 合并）、`crates/omnilang-core/src/ir/lower/view.rs:882`（FnRef 扫描回归测试补齐）

**Interfaces:**
- Consumes: `EventHandlerIR.used_fields`
- Produces: 未映射字段 → warning + 降级注释；`code` 用稳定 `keyCode`/`keyLabel` 映射

- [ ] **Step 1: 写失败测试**——事件字段 fixture（键盘/剪贴板/拖放）：

```rust
#[test]
fn unmapped_event_fields_warn_not_silent() {
    // ClipboardEvent.text 在 flutter → warning + 降级注释，非静默空串
}
#[test]
fn key_code_uses_stable_mapping() {
    // KeyEvent.code → keyCode/keyLabel，绝不发射 debugName
}
#[test]
fn fnref_stop_propagation_scan_regression() { /* 评审：零测试，补齐 */ }
```

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——warnings 通道线程化进 field_extract + 单表合并 + 稳定 key 映射 + 回归测试

- [ ] **Step 4: 运行验证**——三目标事件 fixture + `cargo test -p omnilang-core` 全绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/ crates/omnilang-core/src/ir/lower/view.rs omnilang/tests/
git commit -m "fix(events): field-stub diagnostics, unified map_event table, keyCode stable mapping"
```

**Phase 2 退出门**: bind:value 三目标双向（浏览器实证）；capturePointer 无参 + Flutter error；SetValue 诊断；事件字段无静默桩。BNF spec §2/§8 同步。

---

# Phase 3: 域 9 Styling + 域 5 Stores

### Task 3.1: 类名字符集校验 + 转义（注入双保险）

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:448-496`（StringLiteral/StyleRef 类名 → `[a-zA-Z0-9_-]` 校验，非法 → error）
- Modify: `omnilang/src/codegen/react/design.rs:151-157`（`format_class_ref` → `escape_literal_str(name, '\'')`）
- Modify: `omnilang/src/codegen/dioxus/view.rs:133-137`（死代码清理或转义复用）
- Test: 注入载荷语料（`class: "x',alert(1),'"` → 编译错误 + 生成代码不含 alert( 模式）

**Interfaces:**
- Consumes: `expect_ident` 字符集策略（`[a-zA-Z0-9_-]`，parser/design.rs:225-234 既有）
- Produces: 类名注入面按构造封闭（评审 CRITICAL 修复）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn class_name_injection_rejected() {
    let src = r#"<div class: "x',alert(document.cookie),'">"#;
    let err = compile_react(src).unwrap_err();
    assert!(err.message.contains("类名仅允许字母数字下划线连字符"));
}
#[test]
fn react_class_name_escaped() {
    // 合法类名路径：生成 styles['name']；任何引号/反斜杠被转义
}
```

- [ ] **Step 2: 运行验证失败**（评审 CRITICAL 实测：当前产出合法 TSX 载荷）

- [ ] **Step 3: 实现**——lowering 校验 + `format_class_ref` 转义 + dioxus 清理

- [ ] **Step 4: 运行验证**——载荷语料回归（`'`/`"`/`\`/反引号/`${`/`</` 在 class/style/text/attr/event 每插入点 per-backend 安全快照）+ tsc 通道

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/view.rs omnilang/src/codegen/react/design.rs omnilang/src/codegen/dioxus/ omnilang/tests/
git commit -m "fix(security): class-name charset validation + react escaping — closes injection vector"
```

### Task 3.2: 样式值消毒单一事实来源

**Files:**
- Modify: `omnilang/src/codegen/omniview/runtime.rs`（生成 JS helper 字符串——含 hex-escape 归一化 + CR/LF/TAB 预剥离 + 全关键词 + url() 上下文）
- Modify: `omnilang/src/codegen/react/design.rs:370-393`（`emit_sanitize_css` 改为共用生成字符串）、`omnilang/src/codegen/omniview/view/style.rs:52`（同源）
- Test: `omnilang/tests/js_runtime/test_sanitizers.js` 扩展为两份输出行为一致性测试（含 `\6a\61\76\61\73\63\72\69\70\74\3a` 绕过载荷）

**Interfaces:**
- Consumes: `sanitize_css_value`（Rust 编译期，既有）
- Produces: React/OmniView 运行时共用同一生成 helper

- [ ] **Step 1: 写失败测试**——hex-escape 载荷穿透 React helper（评审 MEDIUM 实测缺口）

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——runtime.rs 生成统一 helper + React 改用 + 一致性测试（三份输出同断言集）

- [ ] **Step 4: 运行验证**——`test_sanitizers.js` 全过 + React 运行时测试

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/runtime.rs omnilang/src/codegen/react/design.rs omnilang/tests/js_runtime/
git commit -m "fix(security): single-source CSS sanitizer helper shared by React and OmniView runtimes"
```

### Task 3.3: React 切片订阅（评审 HIGH-1/H2）

**Files:**
- Modify: `omnilang/src/codegen/react/component/mod.rs:394-403`（整 store hook → 每字段切片 selector 调用）、`component/computed.rs:21-27`（useMemo deps → 切片值）、`component/effect.rs:27-37`（deps 从 body 读取分析——与 Task 3.4 联动）

**Interfaces:**
- Consumes: `used_store_signals` + `store_signal_map`（既有分析）
- Produces: `const items = useTodoStore((s) => s.items);` + `useMemo(..., [items])`

- [ ] **Step 1: 写失败测试**——store 引用组件快照：

```rust
#[test]
fn react_store_reads_use_slice_selectors() {
    let react = compile_react(store_component_src);
    assert!(react.contains("useTodoStore((s) => s.items)"));
    assert!(!react.contains("const TodoStore = useTodoStore()"));
    // useMemo deps 含 [items] 而非 [TodoStore]
}
```

- [ ] **Step 2: 运行验证失败**（当前整 store 订阅——build/FilterBar.tsx:9 实测）

- [ ] **Step 3: 实现**——切片发射 + computed/effect deps 切片化 + 删除整 store 常量

- [ ] **Step 4: 运行验证**——strict tsc 全绿 + 浏览器行为不变回归（FilterBar 筛选仍工作——修复 perf HIGH-3 后验证）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/ omnilang/tests/
git commit -m "perf(react): slice-selector store subscription; memo deps on slices"
```

### Task 3.4: effect deps 语义验证

**Files:**
- Modify: `crates/omnilang-core/src/semantic/`（新增 `validate_effect_deps`：声明 deps 覆盖 body 读取的 store 字段，漏 → 编译错误）
- Modify: `omnilang/src/codegen/react/component/effect.rs:27-37`（deps 已正确时保留；computed deps 补 props——M6）
- Test: fixture `effect_deps.omni`（漏字段 → error；全覆盖 → 通过）

**Interfaces:**
- Consumes: effect body 标识符分析（`getter_expr_contains_ident` 闭包修复——perf HIGH-3 一行）
- Produces: `E_EFFECT_DEPS_MISSING` 错误 + body 闭包内依赖识别

- [ ] **Step 1: 修闭包依赖盲区**——`getter_expr_contains_ident` 补 `ClosureExpr` 臂（utils.rs:113-153，一行级）：

```rust
ExprIR::ClosureExpr { body, .. } => getter_expr_contains_ident(body, name),
```

- [ ] **Step 2: 写失败测试**——`@effect(notifications)` + body 读 `unreadCount` → error「依赖漏掉：NotificationStore.unreadCount」

- [ ] **Step 3: 实现**——语义 pass + React computed deps 含 props（M6）

- [ ] **Step 4: 运行验证**——demo 全量编译（修复 NotificationsPanel 类静默）+ FilterBar 输入筛选实时更新（浏览器——perf HIGH-3 实证）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/ omnilang/src/codegen/react/ omnilang/tests/
git commit -m "feat(semantic): effect deps must cover body reads; closure-aware dependency analysis"
```

### Task 3.5: store BNF 统一 + React mutation 质量批次

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/store.rs:19-32`（顶层 store 接受 signal|getter|action；删 function_def 差异）、`docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` §6.2（BNF 同步）
- Modify: `omnilang/src/codegen/react/component/state.rs:40-43`（`setX(x+n)` → 函数式更新）、`react/view.rs:385-411`（删空 `Object.assign(e, {})`，66 处）、`react/mod.rs:812-940`（`set()` 合并单次调用）
- Modify: `docs/omniview/15-grammar-view.md:556-561`（`$counter` 存取删除）

**Interfaces:**
- Produces: 统一 store BNF + React mutation 地道化

- [ ] **Step 1: 写失败测试**——`setCount(count + 1)` → `setCount((c) => c + 1)`；action 多 set() → 单对象；store fn → error

- [ ] **Step 2: 运行验证失败**（当前 `setCount(count + 1)`——评审 M2 实测）

- [ ] **Step 3: 实现**——parser 统一 + React mutation 三修

- [ ] **Step 4: 运行验证**——strict tsc 全绿 + eslint exhaustive-deps 通过 + demo 行为回归

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/top_level/store.rs omnilang/src/codegen/react/ docs/
git commit -m "feat(store): uniform store BNF; refactor(react): functional updaters, coalesced set(), no empty Object.assign"
```

**Phase 3 退出门**: 注入载荷语料回归全绿；消毒器一致性；React 切片订阅 + effect deps 验证（浏览器实证）；store BNF 统一。BNF spec §4/§6 同步。

---

# Phase 4: 域 3 Design+主题 + 域 10 a11y

### Task 4.1: @themes + @theme 信号（D1 语法层）

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs`（`@themes { dark, light }` 声明，design 块内）、`crates/omnilang-core/src/parser/top_level/store.rs`（signal 加 `@theme` 标记）、`crates/omnilang-core/src/ir/design.rs`（ThemesIR + theme_signal 引用）、`crates/omnilang-core/src/semantic/`（validate_theme：@theme 信号值必须 ∈ @themes 枚举）
- Test: `themes.omni` fixture（解析 + 语义验证）

**Interfaces:**
- Consumes: `DesignIR`（现有）
- Produces: `DesignIR { themes: Vec<String>, theme_signal: Option<Path> }`

- [ ] **Step 1: 写失败测试**——`@themes { dark, light }` + `signal mode: String @theme` 解析通过；`signal x: Int @theme` → error；无 @themes 时 @theme → error

- [ ] **Step 2: 运行验证失败**（语法不存在）

- [ ] **Step 3: 实现**——parser/IR/语义验证

- [ ] **Step 4: 运行验证**——fixture 全过 + 既有 design 测试回归

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/
git commit -m "feat(design): @themes declaration + @theme signal marker with semantic validation"
```

### Task 4.2: 主题接线三目标（D1 实现）

**Files:**
- Modify: `omnilang/src/codegen/react/design.rs:395-461`（生成订阅用户 @theme store → `data-theme` 写入——当前孤儿 useThemeStore 无人接线）、`omnilang/src/codegen/react/component/mod.rs:233`（副作用 import 修正）、`omnilang/src/codegen/flutter/design.rs:814-850`（`ThemeModeNotifier` + `MaterialApp(themeMode:)` 自动接线，替代注释）、`omnilang/src/codegen/omniview/mod.rs:384-405`（按 @theme 信号识别，删 `mode` 魔法名）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` §4.5/§4.6（`theme = "dark"` 删除，文档化实际机制）

**Interfaces:**
- Consumes: `DesignIR.theme_signal`（Task 4.1）
- Produces: 三目标主题切换行为接通

- [ ] **Step 1: 写失败测试**——React 输出含订阅 @theme store 的 useEffect 写 `data-theme`；Flutter 输出含 `MaterialApp(themeMode: ref.watch(...))`

- [ ] **Step 2: 运行验证失败**（评审 HIGH-2 实测：当前无 data-theme 写入）

- [ ] **Step 3: 实现**——三后端接线 + demo（ThemeSwitcher 用 @theme 信号 + action 切换）+ spec 删除虚构语法

- [ ] **Step 4: 浏览器实证**——点主题卡片 → `data-theme` 属性变化 + `[data-theme="dark"]` CSS 覆盖生效（React + OmniView）；Flutter widget-test（themeMode 切换）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/ omnilang/src/codegen/flutter/ omnilang/src/codegen/omniview/ docs/ test/projects/learn-omniview/
git commit -m "feat(theme): @theme wiring across targets — data-theme subscription, ThemeModeNotifier, signal recognition"
```

### Task 4.3: classes/variants Flutter 实现 + design warnings 通道

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs:806-843`（`style_classes` 解析 → `AppDesign.<class>()` 装饰；`class_map_bindings` 合并真实样式；删除空 `BoxDecoration()` TODO 桩）、`omnilang/src/codegen/flutter/design.rs:216-258`（`gen_design_files` 加 warnings 返回通道；跳过/降级发诊断）、`omnilang/src/codegen/flutter/mod.rs:42-66`（warnings channel 扩展）

**Interfaces:**
- Consumes: `style_classes`/`class_map_bindings`（IR 既有，Flutter 从不读）
- Produces: class:/variant: 在 Flutter 有视觉效果；不可映射 design 值 → warning

- [ ] **Step 1: 写失败测试**——`class:Design.Card` → `decoration: AppDesign.card()`（快照断言非空桩）；`token zIndexX: ZIndex = 10` → warning

- [ ] **Step 2: 运行验证失败**（当前空 BoxDecoration + 静默跳过——评审 HIGH 实测）

- [ ] **Step 3: 实现**——style_classes 解析 + warnings 通道

- [ ] **Step 4: 运行验证**——dart analyze 全绿 + 浏览器/快照视觉效果对照

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/
git commit -m "feat(flutter): resolve design classes/variants to AppDesign decorations; design warnings channel"
```

### Task 4.4: design 诊断表实现 + 命名冲突

**Files:**
- Modify: `omnilang/src/codegen/flutter/design.rs:196-253`（ZIndex warning、display:grid error、float error、cursor warning——spec §10.3 兑现）
- Modify: `crates/omnilang-core/src/lexer/keywords.rs:81` + `token.rs:72`（删 `AtNative` token）、`crates/omnilang-core/src/error.rs`（`theme = "dark"`/`@native` 诊断）
- Modify: `omnilang/src/codegen/react/design.rs`（runtime 模块移入 `__design/` 目录，杜绝 `useThemeStore` basename 冲突）

**Interfaces:**
- Produces: design 域全部诊断按 spec §10.3 实现；`@native` 删除；React runtime 命名隔离

- [ ] **Step 1: 写失败测试**——`theme = "dark"` → error「主题切换 = 设置 @theme 信号」；`@native(...)` → error「平台特有层已删除」；zIndex → warning

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——诊断 + token 删除 + 命名隔离（生成文件路径变更 + import 修正）

- [ ] **Step 4: 运行验证**——tsc/dart analyze 全绿 + 载荷回归

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/lexer/ omnilang/src/codegen/ omnilang/tests/
git commit -m "feat(design): implement spec diagnostics table; remove @native token; isolate design runtime naming"
```

### Task 4.5: a11y 域（新增）

**Files:**
- Modify: `crates/omnilang-core/src/parser/view/element.rs`（`a11y: { label: string, role: string, hidden: expr, live: "polite"|"assertive", alt: expr }` 块）、`crates/omnilang-core/src/ir/types.rs`（ElementIR 增 a11y 字段）、`crates/omnilang-core/src/semantic/`（validate_a11y：role 白名单）
- Modify: `omnilang/src/codegen/react/view.rs`（→ aria-label/role/aria-hidden/aria-live/alt）、`omnilang/src/codegen/flutter/utils.rs:79-103` + `view.rs:478-482`（→ `Semantics(label:)`/`ExcludeSemantics`/`liveRegion`/`Image.alt`；不可映射 role → warning）、`omnilang/src/codegen/omniview/`（aria 透传）
- Modify: `test/projects/learn-omniview/src/components/FormPage.omni:82-142`（6 处 a11y 属性迁移到 a11y 块）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（§11 a11y 域 + floor 表 + 诊断表）

**Interfaces:**
- Produces: `ElementIR.a11y: Option<A11yIR>`——`A11yIR { label, role, hidden, live, alt }`

- [ ] **Step 1: 写失败测试**——`<button a11y: { label: "Close", hidden: !visible }>` 三目标生成形状（React `aria-label`/`aria-hidden`；Flutter `Semantics(label:)`/`ExcludeSemantics`）

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——parser/IR/语义 + 三后端映射 + demo 迁移 + spec 文档

- [ ] **Step 4: 验证**——Flutter 语义树断言（Semantics 节点 label/flag 存在——widget-test 或 flutter_test）；不可映射 role → warning + 注释

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ omnilang/src/codegen/ test/projects/learn-omniview/ docs/
git commit -m "feat(a11y): semantic accessibility domain — label/role/hidden/live/alt across three targets"
```

**Phase 4 退出门**: 主题切换三目标浏览器实证；classes/variants Flutter 视觉生效；design 诊断表全实现；a11y 语义树断言。BNF spec §4/§5/§11 同步。

---

# Phase 5: 域 2 动画 + 域 6 控制流

### Task 5.1: @scroll/@drag 控制维度恢复

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/motion.rs:198-230`（`@scroll start/end/scrub`、`@drag-* constraints/elastic/threshold/velocity_max` 子参数解析）、`crates/omnilang-core/src/ir/types_motion.rs`（触发器参数 IR + **补 `#[non_exhaustive]`**）、`crates/omnilang-core/src/semantic/validate_motion.rs`
- Modify: `omnilang/src/codegen/react/motion/transforms.rs:291-300`（extent 推导升级为显式参数消费）、`omnilang/src/codegen/flutter/motion_widgets.rs`（参数类型化——Task 5.6 共享化联动）、`omnilang/src/codegen/omniview/`
- Modify: `docs/superpowers/plans/2026-07-31-semantic-level-dsl-design.md` §3.6/§3.7（未实现标注 → 已实现）

**Interfaces:**
- Produces: `TriggerKind::Scroll { start, end, scrub }` / `Drag { constraints, elastic, threshold, velocity_max }`（Option 字段）

- [ ] **Step 1: 写失败测试**——`@scroll start: 0 end: 1000 scrub: true` 解析 + React 生成滚动区间映射；`@drag-horizontal constraints: [-100, 100] elastic: 0.5` 解析 + 约束解算

- [ ] **Step 2: 运行验证失败**（当前子参数解析失败——design doc 教授的语法不解析，评审 MEDIUM-4）

- [ ] **Step 3: 实现**——parser + IR（含 non_exhaustive）+ 语义 + 三后端消费

- [ ] **Step 4: 浏览器实证**——滚动区间 → 进度 → 动画；拖拽范围限制 + 弹性回弹（React/OmniView；Flutter widget-test）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ omnilang/src/codegen/ docs/superpowers/plans/
git commit -m "feat(motion): restore @scroll start/end/scrub and @drag constraints/elastic dimensions"
```

### Task 5.2: 白名单语义集 + borderRadius/fontSize

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/motion.rs:10-27`（白名单 + borderRadius/fontSize）、`omnilang/src/codegen/flutter/motion.rs`（`BorderRadiusTween`/`TextStyle.lerp` 映射）、`omnilang/src/codegen/react/motion/`、`omnilang/src/codegen/omniview/`
- Modify: `.architecture/decisions/adrs/ADR-053-semantic-level-dsl-syntax-formalization-and-implementation.md:58-73`（理由文本：白名单 = 语义集 v1，非任意 CSS 排除的论证修正）

**Interfaces:**
- Produces: 19 项白名单；borderRadius/fontSize 三目标动画

- [ ] **Step 1: 写失败测试**——`@hover { borderRadius: 16px }` 三目标生成（Flutter `BorderRadiusTween`）+ 快照

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——白名单 + Flutter 映射 + 文档理由修正

- [ ] **Step 4: 运行验证**——三目标动画运行（浏览器）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/top_level/motion.rs omnilang/src/codegen/ .architecture/decisions/adrs/
git commit -m "feat(motion): borderRadius/fontSize join semantic whitelist"
```

### Task 5.3: paused 删除 → 硬诊断

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/motion.rs:370`（paused 接受分支 → 硬诊断 error「paused 已删除，用 @if 条件渲染替代」）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md:209`、`docs/omniview/15-grammar-view.md:1037-1039`（「accepted and ignored」→ 已删除）

**Interfaces:**
- Produces: `E_PAUSED_REMOVED` 诊断

- [ ] **Step 1: 写失败测试**——`@paused` / `paused: true` → error + 迁移提示

- [ ] **Step 2: 运行验证失败**（当前静默接受）

- [ ] **Step 3: 实现**——诊断 + 文档同步

- [ ] **Step 4: 运行验证**——`grep paused` 仅余文档/历史

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/top_level/motion.rs docs/
git commit -m "feat(motion): paused hard-diagnosed with conditional-render migration hint"
```

### Task 5.4: @if fade Flutter 真实现

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs:1073-1081`（`"fade"` → `AnimatedSwitcher`；`"wait"` 降级注释保留）

**Interfaces:**
- Produces: `mode:"fade"` Flutter 视觉淡入（当前静默无效果）

- [ ] **Step 1: 写失败测试**——`@if show mode:"fade"` → Flutter 输出含 `AnimatedSwitcher`（快照断言）

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——fade 分支包 AnimatedSwitcher（默认过渡）；补 widget-test（切换可见性 → 淡入过程存在）

- [ ] **Step 4: 运行验证**——dart analyze + widget-test

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/tests/
git commit -m "feat(flutter): @if mode:fade via AnimatedSwitcher"
```

### Task 5.5: @for 三目标契约（评审 CRITICAL 群）

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:785-792`（对象项 key 修复：有 key_expr 用 expr；无 → `key={_i}` + 语义警告注释）、`crates/omnilang-core/src/semantic/`（对象项无 key → warning「建议提供 key=」）、`omnilang/src/codegen/flutter/view.rs:1087-1119`（`scroll_hint` 分派：false → `Column(children:)` + `KeyedSubtree`/`key: ValueKey(expr)`；true → `ListView.builder`）、`omnilang/src/codegen/react/view.rs:793-853`（iterable 单次求值）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` §7.4（映射表实现对齐）

**Interfaces:**
- Consumes: `ListRenderingIntent { key_expr, empty_body, scroll_hint }`（IR 既有）
- Produces: 三目标 keyed 契约统一 + 非滚动默认文档流

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn flutter_for_non_scroll_emits_column() {
    let flutter = compile_flutter(for_src_no_scroll);
    assert!(flutter.contains("Column(children:"));
    assert!(!flutter.contains("ListView.builder"));
}
#[test]
fn flutter_for_key_expr_emits_value_key() {
    assert!(flutter.contains("ValueKey"));
}
#[test]
fn react_for_object_items_no_key_warns() {
    // 对象项无 key → 语义 warning + 生成 key={_i}
    assert!(diags.iter().any(|d| d.code == W_FOR_KEY));
}
```

- [ ] **Step 2: 运行验证失败**（评审 CRITICAL 实测：当前无条件 ListView.builder + 无 ValueKey；React key=[object Object]）

- [ ] **Step 3: 实现**——Flutter 分派 + ValueKey；React key 策略 + 语义警告；iterable 单次求值（渲染作用域 const 或 IIFE）

- [ ] **Step 4: 运行验证**——control_flow fixture（Task 0.8）全绿 + 浏览器：keyed 列表重排/过滤保状态（三目标）+ 嵌套 @for 无崩溃（Flutter widget-test）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/ omnilang/src/codegen/flutter/ crates/omnilang-core/src/semantic/ omnilang/tests/
git commit -m "fix(for): cross-target keyed contract — flutter Column/ValueKey dispatch, react index-key fallback, object-item warning"
```

### Task 5.6: Flutter helper 共享化 + 类型化配置 + 语义绑定快照

**Files:**
- Modify: `omnilang/src/codegen/flutter/motion_widgets.rs:33-416`（`_Omni*` helper 提取为共享生成文件 `_omniview_runtime.dart`，每文件 import 而非重复发射）、`motion_widgets.rs:214-225,330-352`（start/constraints 配置编译期类型化）、`:251`（scrub 用 Ticker 时间替代 60fps 硬编码）、`:324`（删 `_startPos` 死字段）
- Modify: `omnilang/src/codegen/flutter/mod.rs:543-554` + `ref.rs:616-682`（语义绑定快照比较——`_prevX` 模式已有，绑定值变化才应用；消灭焦点偷窃/滚动重放）

**Interfaces:**
- Produces: 单份共享 runtime 文件；类型化配置；dep 语义与 React 对等

- [ ] **Step 1: 写失败测试**——两组件同用 @in-view → 生成文件各 import `_omniview_runtime.dart` 而非各自内联 90 行；语义绑定：无关 setState 后 focus 不重夺（widget-test：焦点在输入框 B，setState 更新 A，焦点保持 B）

- [ ] **Step 2: 运行验证失败**（评审 MEDIUM 实测：每文件重复 + 焦点被偷）

- [ ] **Step 3: 实现**——helper 提取 + 配置类型化 + 快照比较

- [ ] **Step 4: 运行验证**——dart analyze + widget-test（焦点保持 + 滚动不重播）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/
git commit -m "refactor(flutter): shared _omniview_runtime.dart; typed trigger configs; dep-scoped semantic bindings"
```

### Task 5.7: Flutter 质量批次（prop 转换 + ==/hashCode + measure cast + const）

**Files:**
- Modify: `omnilang/src/codegen/flutter/utils.rs:79-111,240-250`（per-widget 值转换层：`fontWeight:"bold"` → `FontWeight.w700` 等；`img` → `Image.network(src)` 位置参数；`icon` → `Icon(Icons.name)` 已知名表 + 不可映射诊断）、`omnilang/src/codegen/flutter/view.rs:478-497`（消费转换）、`omnilang/src/codegen/flutter/mod.rs:656-731`（store state `==`/`hashCode` + nullable 哨兵 copyWith）、`flutter/ref.rs:960-966`（`is RenderBox` 安全检查）、`flutter/mod.rs:297-311,352-412`（motion controller 按元素实例化 + 使用图剪枝）、`flutter/view.rs:64-175`（字面子树 const 注入）
- Modify: `omnilang/src/codegen/flutter/tests.rs:4289-4341`（修正断言 `fontWeight: 'bold'` 为错误形状的快照）

**Interfaces:**
- Produces: 常见输入不再破坏 dart analyze；Flutter 输出可 const、controller 不跨元素共享

- [ ] **Step 1: 写失败测试**——`fontWeight="bold"` → `FontWeight.w700`（快照）；`<img src="x">` → `Image.network('x')`；`<icon name="home"/>` → `Icon(Icons.home)`；store 相等性（`==` 生成）

- [ ] **Step 2: 运行验证失败**（评审 HIGH-4 实测：三输入破坏 dart analyze）

- [ ] **Step 3: 实现**——转换层 + 类型化 + 相等性 + cast 修复 + controller 按元素 + const 保守注入（全部字面参数子树）

- [ ] **Step 4: 运行验证**——dart analyze 全绿 + 快照更新 + widget-test 回归

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/
git commit -m "fix(flutter): prop type conversion, store equality, safe casts, per-element controllers, const injection"
```

**Phase 5 退出门**: @scroll/@drag 维度浏览器实证；fade 三目标对等；@for 契约 fixture + 浏览器；Flutter 常见输入 dart analyze 全绿；语义绑定无焦点偷窃；**learn-omniview Flutter 全量 dart analyze 0 错误 → 移除 Task 0.5 门禁 `#[ignore]`**（T0.5 实测基线 1125 错误/21 类，逐批清零）。BNF spec §3/§7 同步。

---

# Phase 6: 域 4 Slots

### Task 6.1: typed slots 删除 + 遗留形式清理（D4）

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/component.rs`（`@slot name: Type` → error「v2 不支持 slot 数据参数/类型约束」；删 data_param 解析）、`crates/omnilang-core/src/ir/types.rs`（`ViewTree::Slot` 删 type_constraint/data_param）、`omnilang/src/codegen/react/component/imports.rs:190-200`（删类型 prop 生成）、`omnilang/src/codegen/flutter/mod.rs:222-228`（删降级路径）、`crates/omnilang-core/src/parser/`（遗留 `@slot(name)` 指令 + `slot="name"` 消费 → 硬诊断「用 body 级声明 + 命名 props」）
- Modify: `test/projects/learn-omniview/src/components/AdvancedTypesDemo.omni:164`（迁移：删声明侧类型）+ `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md` §5

**Interfaces:**
- Produces: Slot 纯净三形态（default + named + fallback）；类型约束 → 硬诊断

- [ ] **Step 1: 写失败测试**——`@slot header: String` → error + 提示；`slot="name"` → error + 迁移提示；三目标 named/fallback fixture 行为对等

- [ ] **Step 2: 运行验证失败**

- [ ] **Step 3: 实现**——parser/IR/codegen 删除 + 诊断 + demo 迁移 + spec 同步

- [ ] **Step 4: 运行验证**——三目标 slot fixture 全绿 + 全量测试

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ omnilang/src/codegen/ test/projects/learn-omniview/ docs/
git commit -m "feat(slots): remove typed slots and legacy forms — slot = parent content only"
```

**Phase 6 退出门**: slot 三目标对等 fixture；三种已删除形式全部硬诊断；demo 迁移完成。

---

# Phase 7: 终验

### Task 7.1: 三目标全量编译 + 浏览器全测

**Files:**
- Test: learn-omniview 全量（92 文件）

- [ ] **Step 1: React 全量**——`omnic build -t react` → `tsc --noEmit --strict` 0 错误 + eslint 0 告警（verify-react.sh 豁免清单清空）
- [ ] **Step 2: Flutter 全量**——`omnic build -t flutter` → `dart analyze` 0 错误 0 警告
- [ ] **Step 3: OmniView 全量**——`omnic build -t omniview` → 浏览器 9 标签页全测（渲染 + 动画 + 交互 + 主题切换 + 列表重排保状态）
- [ ] **Step 4: 修复终验发现的任何回归**（回归即回到对应任务修复，验证后重跑）
- [ ] **Step 5: Commit**

```bash
git commit -m "test: full three-target validation of learn-omniview"
```

### Task 7.2: 文档全量对齐审计

**Files:**
- Test: `docs/omniview/` 16 篇 + BNF spec v2 + 设计文档

- [ ] **Step 1: 逐篇审计**——每篇 fenced 代码块对 parser 验收（仿 deferrals #24/#25 模式；可编译块 → `omnic build` 验证）
- [ ] **Step 2: 修复残留分歧**（含 README/01 CLI 命令节、10-design-system 陈旧句等历史项）
- [ ] **Step 3: 建立文档编译 CI 门禁**——抽取 docs 可编译代码块 + learn-omniview 一并编译，解析错误即失败
- [ ] **Step 4: Commit**

```bash
git commit -m "docs: full docs-parser alignment audit + doc-compile CI gate"
```

### Task 7.3: 注入载荷最终回归 + deferrals 收口

**Files:**
- Test: 载荷语料 per-backend 安全快照（Task 3.1 扩展全后端）
- Modify: `.architecture/deferrals.md`（#28 Map.set、#33 嵌套 #root 处置收口；本计划未覆盖项补触发条件）

- [ ] **Step 1: 载荷语料全后端回归**（含 Flutter/Dart 字符串、OmniView HTML/JS）
- [ ] **Step 2: deferrals 收口**——#28/#33 处置（实现或 defer-with-trigger）；#19（触发器子属性——Task 5.1 已实现）关闭
- [ ] **Step 3: 更新设计文档状态**——「Design → Implemented」+ 验收对照（设计文档 §4 验证协议逐项打勾）
- [ ] **Step 4: Commit**

```bash
git commit -m "chore: final injection regression, deferrals closure, design doc status"
```

**Phase 7 退出门（终验）**: learn-omniview 三目标全量绿；9 标签页浏览器全测 0 runtime error；16 篇文档与编译器事实一致；设计文档 §4 验证协议逐项 ✅。

---

## 执行顺序总览

| 阶段 | 任务 | 依赖 |
|------|------|------|
| 0 | 0.1-0.8（基础设施 8 项） | 无 |
| 1 | 1.1-1.6（域 8 Expressions） | 0 |
| 2 | 2.1-2.4（域 1 Ref + 域 7 Events） | 1 |
| 3 | 3.1-3.5（域 9 Styling + 域 5 Stores） | 1 |
| 4 | 4.1-4.5（域 3 Design+主题 + 域 10 a11y） | 2, 3 |
| 5 | 5.1-5.7（域 2 动画 + 域 6 控制流） | 1, 2 |
| 6 | 6.1（域 4 Slots） | 2 |
| 7 | 7.1-7.3（终验） | 全部 |

每任务独立 commit；每阶段结束跑全量测试 + 该域门禁；每 3 域交叉全量浏览器测试 + 文档审计。

## Self-Review 记录

- **Spec 覆盖**: 设计文档 §5 十域全部映射到任务（域 8→Phase 1、域 1/7→Phase 2、域 9/5→Phase 3、域 3/10→Phase 4、域 2/6→Phase 5、域 4→Phase 6）；§4 验证协议 → Phase 0 八项 + 各任务验证步骤；§7 影响面（parser/semantic/IR/codegen/docs/demo/deferrals/CI）全部落入任务 Files；伴随性修复清单 → 1.5/1.6/3.3/3.4/3.5/5.6/5.7
- **决策覆盖**: D1→4.1/4.2/4.4、D2→2.3、D3→2.2、D4→6.1
- **评审 CRITICAL 覆盖**: React 注入→3.1、React C1/C2→1.6/1.3、Flutter @for→5.5、主题→4.1/4.2、proptest→0.1、缓存诊断→0.2
