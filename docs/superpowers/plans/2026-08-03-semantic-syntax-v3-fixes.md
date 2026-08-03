# OmniLang 语义级 DSL v3 修复实现计划（13 专家 v2 对齐评审 + 设计层评审全量处置）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 13 专家 v2 对齐评审（`.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`）与设计层评审（8 个设计问题）发现的全部问题——注入三连、React/Flutter 行为缺口、静默降级残留、门禁度量升级、文档漂移、结构性债、设计层制度——使语义 DSL 从「编译级地道」达到「行为级地道」。

**Architecture:** 沿用三层架构（semantic → IR → codegen）。按「验证网先行 → 安全 → 行为 → 静默清零 → IR wave → 文档 → 设计层制度」顺序推进：任何修改前先闭合门禁盲区（#60 入 CI、all-backends、缓存 parse 洞、门禁度量升级），安全注入三连与语料扩展闭环优先，随后双目标行为修复（React 廉价路径先行、Flutter 运行时测试基建先行），静默降级逐个硬诊断化，结构性修复合批为一次 IR wave，文档治理收口，最后把设计层原则固化为制度（词汇表审查/语法生命周期/floor 机器化）。

**Tech Stack:** Rust（omnilang + omnilang-core workspace）、React 19 + TS strict + Zustand v5（生成目标）、Flutter + Dart + Riverpod 3.0（生成目标）、CI 门禁（cargo test / tsc --strict / eslint react-hooks / dart analyze / Playwright / docs-gate）。

## Global Constraints

- **评审依据**: `.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`（13 专家全部发现 + 优先级）+ 设计层评审 8 问题（本会话）
- **设计契约**: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（已 Implemented；本次修改后同步状态注记）
- **用户确认决策**:
  - **D-A**: inline store 在 React 目标先硬诊断 E_CODEGEN + 迁移提示；完整实现与 #36/#49 合并为 IR wave（Phase 5）
  - **D-B**: 未知事件名 → warning + 「最近的合法事件」建议（validate_a11y 先例），非 error
  - **D-C**: 单一总计划，8 阶段顺序执行，每阶段结束零「open-undispositioned」发现
  - **D-D**: `aria-*` 原生属性 → 硬诊断 + 迁移到 a11y 块；`alt=` 明确为普通 DOM 属性（保留透传 + Flutter semanticLabel 映射），文档化边界
  - **D-E**: filter/map/count 语言方法 vs stdlib `array_filter`/`array_map` → prove-or-retire（迁移 4 处 demo 调用到语言方法，门禁验证后删 stdlib 重复；语料抗拒则删语言方法）
  - **D-F**: `@default-theme` ↔ `@theme` 信号 initial 双声明 → 一致性验证（mismatch = error）
  - **D-G**: Flutter 多主题（>2）塌缩 → 编译 warning（诚实降级，不实现自定义 ThemeMode）
  - **D-H**: motion-in-@for → 编译诊断（双目标），直到逐项实现（长期项）
  - **D-I**: BUG-A 两步走——立即 `const`→`let` 廉价路径 + un-fixme；IR `LetStmtIR.mut` 字段在 Phase 5 完整落地
- **破坏性政策**: 已删除语法 → 硬诊断 + 迁移提示（validate_ref_ops 先例）；demo 迁移随任务；文档同步随任务同 commit
- **验证协议**: 每任务 TDD（写失败测试 → 验证失败 → 实现 → 验证通过 → commit）；每阶段结束跑全量门禁（cargo test / verify-react / dart analyze / docs-gate / Playwright）
- **提交纪律**: 每任务独立 commit，conventional commits（feat/fix/refactor/docs/test/chore），无 attribution
- **deferrals**: #36/#37/#49/#50/#55/#60 在本计划内关闭（附证据）；#41/#42/#43/#47/#51/#54/#59 按任务处置；新增 deferral 候选（multi-theme 塌缩、injection 插入点、struct 字面量 TypeError）随任务注册
- **豁免纪律**: 每条豁免删除 = 根因修复落地 + 回归测试；豁免清单计数同步（当前 166 tsc + 121 eslint，文件 18 条）

---

## 文件结构

```
crates/omnilang-core/src/
├── analyse.rs                  # 缓存 parse 诊断洞（0.3）
├── semantic/
│   ├── validate_event_names.rs # 新增：事件名白名单 + 最近建议（4.2）
│   ├── validate_a11y.rs        # role 词汇 ARIA 化（4.3）、aria-* 硬诊断（4.4）
│   ├── validate_ref_ops.rs     # setSelection 事实同步（4.13）
│   ├── validate_theme.rs       # @default-theme 一致性（4.7）
│   ├── validate_types.rs       # 具体类型方法名 warning（4.10）
│   ├── validate_array_methods.rs  # ArrayMethod floor 判定（4.8）
│   ├── lints/ast_collectors.rs # DesignClass/Variant 计入 used（4.12）
│   └── lints/unused_import.rs  # 假阳性修复（4.12）
├── ir/                         # types.rs（LetStmtIR.mut 5.1、BindingRef 5.x）、lower/view.rs（in_view 填充 3.4）、lower/stmt.rs（mut 线程化 5.1）
└── codegen/
    ├── json.rs                 # 对象键转义（1.6）
    ├── sanitize.rs             # CSS 结构字符中和（1.4）、is_dangerous_url 既有
    └── shared/expr.rs          # block-expr Rust/Dioxus（4.8）、struct 字面量 typed 发射（3.1）
omnilang/src/
├── ffi.rs                      # C-FFI 信封统一（1.6）
├── lib.rs                      # OmniViewResult warnings 形状（0.5 相关）
├── codegen/
│   ├── react/
│   │   ├── mod.rs              # const→let（2.1）、inline store 硬诊断（2.2）、尾表达式 return（2.3）、ES5 var→let（2.11）
│   │   ├── motion/keyframes.rs # transform 字符串转义（1.2）、相等守卫（2.10）、_M 提升（2.6）
│   │   ├── view.rs             # 实体转义分离（2.4）、事件/样式映射（2.5）、useCallback/memo（2.8）
│   │   ├── component/state.rs  # 字面量信号 hoist（2.9）、store_slices.rs（2.7、2.10）
│   │   ├── design.rs           # useDesignTokens 切片（2.7）、format_class_ref 既有
│   │   └── component/function.rs # 尾表达式（2.3）、TS2722 ?.()（5.6）
│   ├── flutter/
│   │   ├── view.rs             # bind:value 接线（3.3）、textarea/disabled（3.5）、ref.watch 提升（3.7）、[scroll] hoist（3.8）、IndexedStack（3.9）
│   │   ├── design.rs           # ThemeMode 四态 + multi-theme 警告（3.6）
│   │   ├── motion.rs           # DefaultTextStyle.merge（3.9）
│   │   ├── ref.rs / runtime    # in-view/focus（3.4）、Image URL（3.10）
│   │   └── main.rs 壳          # Scaffold/SafeArea（3.9）
│   ├── omniview/utils.rs       # escape_html 属性上下文（1.3）
│   ├── vue/template.rs         # escape_html 同步（1.3）
│   └── shared/                 # 事件名 canonical 表导出（4.2）、覆盖地图辅助
├── tests/
│   ├── generated_code_validation.rs  # Flutter warnings 断言（0.5）、widget test 入口（3.2）
│   ├── tests_regression/injection.rs # 语料扩展（1.1）、覆盖地图（1.5）
│   └── property_tests.rs       # fuzz 全后端（5.9）
omnilang-wasm/src/lib.rs        # 无变更（契约已统一）
scripts/
├── verify-react.sh             # 豁免计数上限（0.4）
├── verify-react.exemptions     # 台账重连（0.6）、各任务豁免删除
└── verify-docs.py              # docs-gate 语义化（6.6）
.github/workflows/ci.yml        # #60 入 CI（0.1）、all-backends（0.2）
test/projects/learn-omniview/   # demo 迁移（随任务）
docs/                           # 16 篇用户文档（Phase 6）、v1 spec Superseded（6.2）、cheatsheet（6.8）
docs/superpowers/specs/         # 设计文档 BNF 修正（4.13/6.7）、词汇表审查模板（7.1）
.architecture/deferrals.md      # 台账随任务更新
```

---

# Phase 0: 验证网补齐（门禁先行）

> 顺序优先：任何修改前必须先闭合评审发现的门禁盲区（strategy HIGH-2 + systems HIGH-2/3 + rust HIGH）。本阶段全部为 CI/台账/门禁改动，零编译器行为变更。

### Task 0.1: #60 OmniView 浏览器套件接入 CI

**Files:**
- Modify: `.github/workflows/ci.yml`（e2e job 或新增 omniview-e2e job）
- Test: `test/projects/learn-omniview/build/` 再生成产物 + `playwright.omniview.config.ts`

**Interfaces:**
- Consumes: `omnic build --target omniview -o <dir>`（现有 CLI）、`npx playwright test --config=playwright.omniview.config.ts`（现有 44 测试）
- Produces: CI 中「先再生成后测试」的 OmniView 运行时门禁——关闭 #60

- [ ] **Step 1: 写失败测试（CI 配置本身即测试）**

新增独立 job（与 e2e 并行）：
```yaml
  omniview-e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build omnic (release)
        run: cargo build --release -p omnic
      - name: Regenerate OmniView output
        run: ./target/release/omnic build test/projects/learn-omniview/src --target omniview -o test/projects/learn-omniview/build
      - name: Install Playwright browsers
        run: npx playwright install chromium --with-deps
      - name: Run OmniView suite
        run: npx playwright test --config=playwright.omniview.config.ts
        working-directory: test/projects/learn-omniview/build/react-test-app
```

- [ ] **Step 2: 本地验证当前失败（手动运行 = 无 CI 守护）**

Run: `npx playwright test --config=playwright.omniview.config.ts`（working-directory: `test/projects/learn-omniview/build/react-test-app`）
Expected: 44 passed 0 failed（套件本身绿；「失败」= CI 中不存在该 job——在本地用 `act` 或直接确认 ci.yml 无此 job 来验证前置缺失）

- [ ] **Step 3: 实现**——按 Step 1 YAML 修改 `ci.yml`（新增 `omniview-e2e` job，`needs: []` 与 e2e 并行）

- [ ] **Step 4: 验证**——`ci.yml` 语法检查（`npx yaml-lint` 或 GitHub Actions schema 校验）；本地重跑套件确认 44 绿

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: wire OmniView 44-test browser suite into CI (closes #60)"
```

### Task 0.2: CI 测试面补齐——all-backends + Flutter 模块测试 + 注入语料 Flutter 行

**Files:**
- Modify: `.github/workflows/ci.yml`（codegen-validate job、test-all-backends job）

**Interfaces:**
- Consumes: `cargo test -p omnilang --features codegen-flutter`（现有）、`cargo test -p omnilang --features all-backends`（现有）
- Produces: CI 中全后端组合测试 + Flutter 模块测试 + 注入语料 Flutter 行的守护

- [ ] **Step 1: 写失败测试（CI 配置）**

codegen-validate job 改为（当前只跑 integration binary）：
```yaml
      - name: Test omnilang with codegen-flutter
        run: cargo test -p omnilang --features codegen-flutter
```
test-all-backends job：删除 `if: false` 行，保留原命令。

- [ ] **Step 2: 验证前置缺失**——确认当前 `ci.yml:285` 只跑 `--test generated_code_validation`、`ci.yml:162` 为 `if: false`

- [ ] **Step 3: 实现**——按 Step 1 修改两处

- [ ] **Step 4: 验证**——本地 `cargo test -p omnilang --features codegen-flutter`（lib + integration）与 `cargo test -p omnilang --features all-backends` 全绿

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: enable all-backends tests + flutter module tests + injection corpus flutter rows"
```

### Task 0.3: 缓存 parse 诊断洞——编辑器绿屏显示坏代码

**Files:**
- Modify: `crates/omnilang-core/src/analyse.rs:457-459`（parse 诊断收集）、`:531`（缓存写入）、`:606-610`（缓存回放）
- Test: `crates/omnilang-core/src/analyse.rs`（模块测试段）

**Interfaces:**
- Consumes: `CachedFile { ir, diagnostics, ... }`（现有）
- Produces: 缓存命中时 parse 诊断同样恢复——`success` 与诊断真实一致（rust HIGH）

- [ ] **Step 1: 写失败测试**——parse 错误文件缓存命中两次，第二次仍报错

```rust
#[test]
fn cached_run_keeps_parse_diagnostics() {
    let bad = "view A { @click { } }";  // 触发 parse 错误（E0001 类）
    let clean = "view B { }";
    let out1 = analyse_multi_cached(&[bad, clean], None);
    assert!(!out1.success, "run 1 must fail on parse error");
    let out2 = analyse_multi_cached(&[bad, clean], out1.cache.as_ref());
    assert!(!out2.success, "cache hit must not flip success on parse error");
    assert!(out2.diags.iter().any(|d| d.code == E0001), "parse diag dropped on cache hit");
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang-core cached_run_keeps_parse_diagnostics`
Expected: FAIL（run 2 报 success=true、diags 空——parse 诊断从不缓存）

- [ ] **Step 3: 实现**——`:531` 缓存写入处把 parse 阶段诊断（`:457-459` 收集的）并入 `CachedFile.diagnostics`：

```rust
// 缓存写入：parse_diags 与 local_diags 合并后再存
let cached = CachedFile { ir, diagnostics: parse_diags.into_iter().chain(local_diags).collect(), .. };
```

- [ ] **Step 4: 运行验证通过**

Run: `cargo test -p omnilang-core analyse`（全部缓存相关测试）
Expected: PASS（含既有 lowering 错误回归测试）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/analyse.rs
git commit -m "fix: cache parse-phase diagnostics — success must not lie on cache hits"
```

### Task 0.4: React 门禁硬化——路径 glob 收窄 + 实例计数上限 + fixture strict:true

**Files:**
- Modify: `scripts/verify-react.sh:77-85`（豁免匹配）、`scripts/verify-react.exemptions`（新增第 4 列计数上限）、`omnilang/tests/generated_code_validation.rs:217`（REACT_TSCONFIG）

**Interfaces:**
- Consumes: `verify-react.sh` 现有 used/unused 双向校验
- Produces: 门禁从「类别盲」变「类别有界」+ 新文件默认无豁免（systems HIGH-1）

- [ ] **Step 1: 写失败测试**——脚本行为测试：exemptions 中 `count<actual` 时脚本退出非零

在 `verify-react.sh` 内新增计数校验逻辑前，先写一个 fixture 文件验证当前行为（当前超计数静默通过）：
```bash
# scripts/test_exemption_count.sh（临时验证脚本）
EXEMPTIONS=$(mktemp)
printf 'components/DataTable.tsx\tTS2588\tlet mut→const\t2\n' > "$EXEMPTIONS"
# 造出 3 个 TS2588 的场景 → 当前脚本应放行（失败点）
```

- [ ] **Step 2: 验证当前失败**——Run: `bash scripts/test_exemption_count.sh`；Expected: 当前脚本 exit 0（超计数放行 = 缺陷）

- [ ] **Step 3: 实现**——三处：
1. `verify-react.sh`：解析 exemptions 第 4 列（计数上限），`grep -c` 实际匹配数 > 上限即失败
2. exemptions 每行补第 4 列（从当前头部计数快照填写，如 `TS2588 7`、`TS2722 23`、`exhaustive-deps 93`）
3. 路径 glob 从 `*` 收窄为具体文件（如 `components/DataTable.tsx`、`components/ErrorHandlingDemo.tsx`——每条豁免按实际文件枚举）
4. `generated_code_validation.rs:217`：`REACT_TSCONFIG` 改 `"strict": true`（fixture 矩阵与展示项目同标准），同步修正受影响的 fixture 期望

- [ ] **Step 4: 验证**——`bash scripts/verify-react.sh` 全绿（每条豁免计数=实际、路径精确匹配）；`cargo test -p omnilang --test generated_code_validation` 全绿

- [ ] **Step 5: Commit**

```bash
git add scripts/verify-react.sh scripts/verify-react.exemptions omnilang/tests/generated_code_validation.rs
git commit -m "ci: harden react gate — path-scoped exemptions, instance count caps, strict fixture tsconfig"
```

### Task 0.5: Flutter warnings 断言基线——1,004 条编译警告纳入门禁

**Files:**
- Modify: `omnilang/tests/generated_code_validation.rs`（`learn_omniview_flutter_output_passes_dart_analyze` 或新增 companion 测试）
- Test: 同上

**Interfaces:**
- Consumes: `omnic build -t flutter` 的 warnings 输出（`OmniViewResult.warnings` 已存在）
- Produces: 展示项目 Flutter warnings 数断言（当前 1,004：881 ECodegen + 123 lint）——每阶段任务必须单调下降（flutter LOW）

- [ ] **Step 1: 写失败测试**——当前 warnings 未被断言（门禁绿 ≠ warnings 零）

```rust
#[test]
fn learn_omniview_flutter_warnings_are_tracked() {
    // 复用 flutter gate 的编译 harness，收集 warnings
    let warnings = build_learn_omniview_flutter_with_warnings();
    // 基线断言：warnings 数必须 ≤ 当前基线（1004），且随修复单调下降
    assert!(warnings.len() <= 1004, "warnings regressed: {} > 1004", warnings.len());
    // 记录分布：ECodegen 与 unused-import 分开计数
    let ecodegen = warnings.iter().filter(|w| w.code == "ECodegen").count();
    let unused = warnings.iter().filter(|w| w.code == "unused-import").count();
    eprintln!("warnings baseline: {} total ({} ECodegen, {} unused-import)", warnings.len(), ecodegen, unused);
}
```

- [ ] **Step 2: 验证当前失败**——Run: `cargo test -p omnilang --features codegen-flutter --test generated_code_validation learn_omniview_flutter_warnings_are_tracked`
Expected: FAIL（`warnings` 字段当前未暴露到测试 harness——需先接上编译入口的 warnings 通道）

- [ ] **Step 3: 实现**——harness 调 `compile_multi_to_flutter`（或 CLI `omnic build` 解析 stderr warnings），统计总数与分类

- [ ] **Step 4: 验证**——PASS（基线 1004 记录入测试注释；后续任务（3.3/3.5/3.6/4.12）将逐步压低此数，每降一次更新断言上限）

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/generated_code_validation.rs
git commit -m "test: track flutter showcase warnings baseline (1004) in gate"
```

### Task 0.6: 豁免台账重连——13/18 条根因注册 + 计数漂移修正

**Files:**
- Modify: `scripts/verify-react.exemptions`（task 列 → deferral 编号）、`.architecture/deferrals.md`（补未登记根因）、`docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md:48`（计数同步）
- Test: 无代码——台账一致性审计（strategy MEDIUM-1）

**Interfaces:**
- Consumes: 现有豁免清单 18 条 + deferrals #36-#60
- Produces: 每条豁免可追溯到 deferral 编号或任务；计数三处一致（文件/ADR/设计文档）

- [ ] **Step 1: 写失败测试（审计脚本）**——断言豁免清单每条 task 列有 deferral 编号或显式「无根因任务」标记

```bash
# scripts/check_exemption_ledger.sh
awk -F'\t' '$4 == "" || $4 == "-" { print "untracked exemption: " $0 }' scripts/verify-react.exemptions
# 期望输出为空；当前 13 行无 deferral 映射 → 非空（失败点）
```

- [ ] **Step 2: 验证当前失败**——Run: `bash scripts/check_exemption_ledger.sh`；Expected: 输出 13+ 行无映射

- [ ] **Step 3: 实现**——逐条核对：
1. 已有映射的保留（TS2588→#36、TS2304 CartStore→#37、TS2304/TS2339/TS2322 结构体→#49）
2. 未映射的补入 deferrals（带触发条件）：TS2722（事件处理器 `?.()` 发射）、TS2554（回调 arity）、TS18048（ref 访问）、TS2345/TS2367/TS2353（样式/类型转换）、TS7006/7005/7034（implicit-any——随 #49）、exhaustive-deps 93x（随 2.6 cleanup 依赖收集 + 5.x）、no-var（随 2.11）
3. 修正三处计数：设计文档 20→18 条、ADR-056 21→18 条、文件头「166 tsc + 118 eslint」→ 实际运行值（0.4 完成后的新基线）

- [ ] **Step 4: 验证**——审计脚本输出为空；`verify-react.sh` 仍全绿

- [ ] **Step 5: Commit**

```bash
git add scripts/verify-react.exemptions scripts/check_exemption_ledger.sh .architecture/deferrals.md docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md .architecture/decisions/adrs/ADR-056-semantic-dsl-v2-implementation.md
git commit -m "chore: reconnect exemption ledger to deferrals — 13 untracked root causes registered"
```

---

# Phase 1: 安全注入修复（CRITICAL）

> 顺序优先：语料扩展先行（Step 1 先写失败测试），三个注入点修复后语料变绿。安全专家端到端验证的三处 exploit 全部在本阶段闭环。

### Task 1.1: 注入语料扩展——motion/design 插入点 + unquoted handler 模式 + .html EXEC_PATTERNS

**Files:**
- Modify: `omnilang/src/codegen/tests_regression/injection.rs:470-507`（插入点枚举）、`:77-87`（HTML_PATTERNS）、`:354-362`（.html 扫描）
- Test: 同上

**Interfaces:**
- Consumes: 现有 855 元组矩阵（19 载荷 × 9 插入点 × 5 后端 + 45 控制编译）
- Produces: 11 个插入点 × 19 载荷 + unquoted handler 形状 + </style> 重开形状（security HIGH-1/2/3 + LOW-4）

- [ ] **Step 1: 写失败测试**——新增三个插入点与两个新模式：

```rust
// 插入点 10：motion transform 字符串（React 目标）
// 载荷族：`x", [alert(1)] : "`、`x',alert(1),'`（安全专家验证过的 exploit 形状）
// 插入点 11：design token/class 值（CSS 声明值）
// 载荷族：`x; } body { background: url(https://evil.example/leak) }`、`x</style><script>alert(1)</script>`
// HTML_PATTERNS 新增：`onmouseover=`、`onerror=`、`onload=`、`onclick=`（无引号形式）、`</style><script`
// .html 文件扫描：抽出 <script> 区域，对剥离后的区域跑 EXEC_PATTERNS（alert(/document.cookie）
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features all-backends --test tests_regression injection`
Expected: FAIL（motion/design 插入点产生 EXEC_PATTERNS 命中——当前发射未转义；unquoted handler 命中 HTML_PATTERNS 新形状）

- [ ] **Step 3: 实现**——只改语料结构与扫描逻辑（不修发射——修复在 1.2/1.3/1.4）：

```rust
// 插入点表追加（按现有 INSERTION_POINTS 结构）：
//   ("motion-transform", |view| format!("@motion slide {{ from {{ translateX: {q} }} }}", q = payload)),
//   ("design-token",     |view| format!("token t1: color = {q}", q = payload)),
// .html 扫描追加：
//   for script in extract_script_regions(html) { assert_no_executable_patterns(&script); }
```

- [ ] **Step 4: 运行验证（预期仍失败，但失败形态已确认）**——记录每个插入点的具体命中（这是 1.2-1.4 的验收基线）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/tests_regression/injection.rs
git commit -m "test: extend injection corpus — motion/design insertion points + unquoted handler shapes"
```

### Task 1.2: React motion transform 字符串转义（端到端 exploit #1）

**Files:**
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs:173-177`（`format_transform_part` 字面量分支）
- Test: `omnilang/src/codegen/tests_regression/injection.rs`（motion 插入点）

**Interfaces:**
- Consumes: `escape_literal_str`（`crates/omnilang-core/src/codegen/json.rs` 现有）
- Produces: motion 字符串值在 JS 字符串字面量位置被正确转义——`[alert(1)]` 计算键形状不再可执行（security HIGH-1）

- [ ] **Step 1: 写失败测试**（1.1 已写，此处补断言形状）

```rust
#[test]
fn motion_transform_string_is_escaped() {
    let src = r#"@motion slide { from { translateX: "10px\", [alert(1)] : \"" } }"#;
    let out = compile_react(src);
    assert!(!out.contains("alert("), "motion string leaked into JS code position: {}", out);
    assert!(out.contains(r"translateX(10px\\\""), "expected escaped literal, got: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features all-backends --test tests_regression injection::motion`
Expected: FAIL（`alert(` 出现在产物中）

- [ ] **Step 3: 实现**

```rust
// keyframes.rs format_transform_part 字面量分支：
ExprIR::LiteralExpr(LiteralValue::Str(s)) => {
    format!("{}({})", prop, escape_literal_str(s, '"'))
}
```

- [ ] **Step 4: 运行验证通过**——motion 插入点 19 载荷全绿；`cargo test -p omnilang --features all-backends` 注入语料全量绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/motion/keyframes.rs omnilang/src/codegen/tests_regression/injection.rs
git commit -m "fix(security): escape motion transform string values — closes verified alert(1) vector"
```

### Task 1.3: unquoted inline-handler 属性逃逸（端到端 exploit #2）

**Files:**
- Modify: `omnilang/src/codegen/omniview/utils.rs:177-183`（`escape_html` 属性上下文）、`omnilang/src/codegen/vue/template.rs:30`（同步）
- Test: `omnilang/src/codegen/tests_regression/injection.rs`（HTML_PATTERNS 新形状）

**Interfaces:**
- Consumes: 1.1 的 HTML_PATTERNS 扩展
- Produces: `title="x onmouseover=alert(1)"` 形状不再出现在任何 HTML 产物（security HIGH-2）

- [ ] **Step 1: 写失败测试**（1.1 已覆盖，补 OmniView 专项）

```rust
#[test]
fn omni_html_attribute_context_blocks_unquoted_handlers() {
    let src = r#"<div title="x onmouseover=alert(1)"/>"#;
    let out = compile_omniview(src);
    assert!(!out.contains("onmouseover="), "unquoted handler leaked: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 escape_html 保留空格与 `=`）

- [ ] **Step 3: 实现**——`escape_html` 增加属性上下文转义：空格 → `&#32;`、`=` → `&#61;`（或对属性值走单独的 `escape_attr`，转义 `& < > " ' = 空格`）。Vue 同步同逻辑。

- [ ] **Step 4: 运行验证通过**——unquoted handler 形状全后端消失；语料 .html 扫描绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/utils.rs omnilang/src/codegen/vue/template.rs omnilang/src/codegen/tests_regression/injection.rs
git commit -m "fix(security): escape attribute context in HTML emission — blocks unquoted handler breakout"
```

### Task 1.4: design token CSS 结构字符中和（端到端 exploit #3）

**Files:**
- Modify: `crates/omnilang-core/src/codegen/sanitize.rs:63-102`（`sanitize_css_value`）
- Test: `omnilang/src/codegen/tests_regression/injection.rs`（design 插入点）+ 消毒器单测

**Interfaces:**
- Consumes: 1.1 的 design 插入点
- Produces: `}`/`</style>`/`;` 结构字符在 CSS 声明值中被中和——`:has()` 外泄与 `</style>` 重开不可构造（security MEDIUM-3）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn css_value_rejects_structural_characters() {
    assert!(sanitize_css_value("x; } body { background: url(https://evil.example/leak) }").contains('}') == false);
    assert!(sanitize_css_value("x</style><script>alert(1)</script>").contains("</style>") == false);
    // 合法值必须保留：rgba(0,0,0,0.5) 中的括号与逗号、16px、url() 校验逻辑（既有）
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（`}` 与 `</style>` 原样通过）

- [ ] **Step 3: 实现**——`sanitize_css_value` 追加结构字符处理：拒绝 `}`、`</style>`（→ 空或警告占位）；`;` 仅在合法声明尾部（简单化：拒绝 `;` 后的任何内容，或按 TokenTypeIR 分型白名单——先做结构字符拒绝，类型白名单为长期项）

- [ ] **Step 4: 运行验证通过**——design 插入点 19 载荷全绿；既有 CSS 消毒器回归（`\a` 解码、hex 归一化）全绿

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/sanitize.rs omnilang/src/codegen/tests_regression/injection.rs
git commit -m "fix(security): neutralize CSS structural chars in design token values — closes stylesheet injection"
```

### Task 1.5: 覆盖地图测试——枚举全部用户字符串发射面

**Files:**
- Create: `omnilang/src/codegen/tests_regression/emission_surface.rs`
- Test: 同上

**Interfaces:**
- Consumes: 各后端所有 `write!`/`format!` 发射点（人工枚举 + 守卫注释）
- Produces: 发射面 ↔ 语料覆盖映射表——每个用户字符串发射点被断言「语料覆盖 ∨ 校验 ∨ 转义」（security 长期项 #8）

- [ ] **Step 1: 写失败测试**

```rust
// 以表驱动声明「发射面清单」：每个条目 = (后端, 发射点位置, 覆盖机制)
// 覆盖机制 ∈ {Corpus(插入点名), Charset, Escaped, Sanitized}
#[test]
fn every_string_emission_surface_is_covered() {
    let surfaces = emission_surfaces(); // 表驱动：含 motion-transform / design-token / class-name / text / attr / style / event-name / url / a11y-label ...
    for s in surfaces {
        match s.cover {
            Corpus(point) => assert!(INSERTION_POINTS.contains(point), "orphan surface: {}", s),
            Escaped | Sanitized | Charset => assert!(s.defense_has_test(), "untested defense: {}", s),
        }
    }
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（motion/design 无插入点映射——1.1 后已补；其余面列出清单）

- [ ] **Step 3: 实现**——表驱动清单（含每个发射点的防御机制与对应测试名）；为缺失防御的面补测试（逐面核对，本任务只登记不修——修复在各专项任务）

- [ ] **Step 4: 运行验证通过**——清单与语料/防御一一对应

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/tests_regression/emission_surface.rs
git commit -m "test: emission-surface coverage map — every user-string emission point is corpus-covered or defended"
```

### Task 1.6: 边界硬化——JSON 对象键转义 + C-FFI 信封统一

**Files:**
- Modify: `crates/omnilang-core/src/codegen/json.rs:125`（`format_json_value` Bracket 模式）、`omnilang/src/ffi.rs:49,64,74,87,109,125,140,143,174`（7+ 处裸字符串信封）
- Test: `crates/omnilang-core/src/codegen/json.rs` 单测、`omnilang/src/ffi.rs` 边界形状测试

**Interfaces:**
- Consumes: `escape_literal_str`、`DiagnosticJson::synthetic`（`crates/omnilang-core/src/error.rs` 现有）
- Produces: 对象键转义 + 引号（rust MEDIUM latent）；C-FFI 边界错误统一为 9 字段 DiagnosticJson 对象（rust MEDIUM + systems MEDIUM-2）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn json_object_keys_are_escaped_and_quoted() {
    let s = format_json_value(&BracketValue { keys: vec!["a\"b".into()], values: vec![json!(1)] });
    assert!(!s.contains("a\"b"), "raw key leaked: {}", s);
    assert!(s.contains(r"\"a\\\"b\""), "expected escaped quoted key: {}", s);
}

#[test]
fn c_ffi_boundary_errors_are_structured() {
    // 触发 ffi.rs 早期返回（如空输入），解析 errors 数组
    let out = omni_compile_multi_to_target(&[], "react");
    let errors: Vec<serde_json::Value> = serde_json::from_str(&out.errors_json).unwrap();
    for e in &errors {
        assert!(e.get("code").is_some() && e.get("line").is_some(), "raw string envelope: {}", e);
    }
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（键原样；errors 为字符串数组）

- [ ] **Step 3: 实现**——json.rs 键经 `escape_literal_str` + 引号；ffi.rs 7+ 处 `"errors": [format!("...")]` 改为 `DiagnosticJson::synthetic(code, msg, span)` 数组

- [ ] **Step 4: 运行验证通过**——两测试 PASS；`cargo test -p omnilang-core ffi` 既有边界形状测试仍绿（`crates/omnilang-core/src/ffi.rs:200-240`）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/json.rs omnilang/src/ffi.rs
git commit -m "fix: escape JSON object keys; unify C-FFI error envelopes to DiagnosticJson"
```

---

# Phase 2: React 行为修复

> 顺序优先：BUG-A 廉价路径先行（用户可见崩溃 + 解锁 19 条 fixme 测试），随后静默行为 bug，最后性能/产物清理批次。每任务结束 `bash scripts/verify-react.sh` 必须保持全绿（豁免只减不增）。

### Task 2.1: BUG-A 廉价路径——const→let + un-fixme 19 条 Playwright 测试

**Files:**
- Modify: `omnilang/src/codegen/react/mod.rs:393-397`（`LetStmtIR` 无条件 `const` 发射）
- Modify: `test/projects/learn-omniview/build/react-test-app/tests/omniview-react.spec.ts`（un-fixme）
- Modify: `scripts/verify-react.exemptions`（删 TS2588 条目——stale 机制强制）
- Test: 被 un-fixme 的 19 条 Playwright 测试即为回归测试（strategy HIGH-1 + ts/react C1）

**Interfaces:**
- Consumes: 现有 `verify-react.sh` stale 双向强制
- Produces: UI/Language 标签页运行时不再崩溃；Playwright 36/36 全跑（D-I 第一步；IR 完整修复在 5.1）

- [ ] **Step 1: 写失败测试**——un-fixme 19 条测试（`spec.ts:79,93,157,266` 等，头部注释 BUG-A 相关条目），保留 fixme 注释中根因描述但移除 `test.fixme` 包装

- [ ] **Step 2: 运行验证失败**

Run: `npx playwright test --config=playwright.config.ts`（working-directory: `test/projects/learn-omniview/build/react-test-app`）
Expected: FAIL（UI/Language 标签页 `Assignment to constant variable`——DataTable.tsx:60-69 等）

- [ ] **Step 3: 实现**——`react/mod.rs:393-397` 无条件 `const` 改为 `let`：

```rust
// emit_let_stmt（或等价函数）：
writeln!(out, "let {name} = {rhs};")?;  // 原为 const——eslint 未启用 prefer-const，门禁不破
```

- [ ] **Step 4: 运行验证通过**——Playwright 36/36 0 failure；`verify-react.sh` 全绿（TS2588 条目已 stale → 门禁强制要求删除，删除后通过）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/mod.rs test/projects/learn-omniview/build/react-test-app/tests/omniview-react.spec.ts scripts/verify-react.exemptions
git commit -m "fix(react): emit let instead of const for LetStmtIR — closes BUG-A app unmount; un-fixme 19 tests"
```

### Task 2.2: inline store 在 React 目标硬诊断 E_CODEGEN（D-A 第一步）

**Files:**
- Modify: `omnilang/src/codegen/react/mod.rs`（检测 `StoreIR` 内联形态 → 诊断）
- Modify: `scripts/verify-react.exemptions`（删 TS2304 CartStore 条目）
- Modify: `test/projects/learn-omniview/src/components/InlineStoreDemo.omni`（迁移到顶层 store 或加诊断展示）
- Test: `omnilang/src/codegen/react/tests.rs`（诊断回归）

**Interfaces:**
- Consumes: IR `StoreIR`（现有，含 inline 标记或可从来源判定）
- Produces: inline store 在 React 目标产生 E_CODEGEN + 迁移提示（「顶层 store 或等待 IR wave」）——BUG-C 从 ReferenceError 变诚实诊断（ts/react H2 + pragmatic H2 + perf H1）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn inline_store_react_hard_diagnosed() {
    let src = r#"view Inline { store Cart { items = []; } }"#;
    let (_, diags) = compile_react_with_diags(src);
    let d = diags.iter().find(|d| d.code == "E_CODEGEN").expect("inline store must diagnose on react");
    assert!(d.message.contains("inline store"), "expected migration hint, got: {}", d.message);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前零诊断，生成冻结代码 + 未定义引用）

- [ ] **Step 3: 实现**——React codegen 入口检测 inline store 并发射 E_CODEGEN（消息含迁移提示）；demo 侧 `InlineStoreDemo.omni` 改为顶层 store 或保留 + 期望诊断快照

- [ ] **Step 4: 运行验证通过**——新测试 PASS；`verify-react.sh` 全绿（删 TS2304 条目）；Playwright InlineStoreDemo 测试不再 fixme（组件不再崩溃——若已迁移）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/mod.rs omnilang/src/codegen/react/tests.rs scripts/verify-react.exemptions test/projects/learn-omniview/src/components/InlineStoreDemo.omni
git commit -m "fix(react): hard-diagnose inline stores with migration hint (BUG-C interim)"
```

### Task 2.3: 尾表达式 return——match/block 尾值静默丢失

**Files:**
- Modify: `omnilang/src/codegen/react/component/function.rs`（fn 尾发射）
- Modify: `omnilang/src/codegen/react/mod.rs:381-384`（`ExprStmtIR` 发射——尾值保留）
- Test: `omnilang/src/codegen/react/tests.rs` + Playwright DataFetching 成功路径（ts/react H1）

**Interfaces:**
- Consumes: `ExprStmtIR`（现有）
- Produces: fn 尾表达式（IIFE/block/match）发射 `return <expr>;`——`error_message` 不再静默 undefined

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn fn_tail_match_emits_return() {
    let src = r#"
        fn score(item) { match item { 1 => "a", _ => "b" } }
        view V { @click { log(score(1)) } }
    "#;
    let out = compile_react(src);
    assert!(out.contains("return "), "tail match must emit return: {}", out);
}

#[test]
fn fn_tail_block_emits_return() {
    let src = r#"fn f(x) { let y = x + 1; y } view V { }"#;
    let out = compile_react(src);
    assert!(out.contains("return y;") || out.contains("return y"), "tail block value lost: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（DataFetching.tsx:6-11 形状：`(() => {...})();` 无 return）

- [ ] **Step 3: 实现**——fn 体发射器：若最后一条 StatementIR 是 ExprStmtIR 且其表达式为 IIFE/block/match 形态，发射 `return <expr>;`（其余语句形态不变）；`ExprStmtIR` 普通位置仍发射 `{expr};`（副作用保留）

- [ ] **Step 4: 运行验证通过**——两测试 PASS；`verify-react.sh` 绿（若 TS2367 类豁免受影响则同步收窄）；Playwright DataFetching 成功路径可见（api.omni fetch_demo OK 分支可达）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/mod.rs omnilang/src/codegen/react/component/function.rs omnilang/src/codegen/react/tests.rs
git commit -m "fix(react): emit return for tail match/block expressions — closes silent value drop"
```

### Task 2.4: 实体转义路径分离——JS 字符串字面量不转义

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs:1007`（字符串字面量发射）
- Test: `omnilang/src/codegen/react/tests.rs` + Playwright（SignalPatternsDemo 标题不再显示 `&amp;amp;`）（ts/react M1）

**Interfaces:**
- Consumes: 现有转义函数（`escape_html` 类）
- Produces: JSX 字符串字面量（`{'...'}`）不经 HTML 实体转义；实体转义仅保留在 JSXText/HTML 路径

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn jsx_string_literal_not_entity_escaped() {
    let src = r#"view V { <div>{"Signal & Store"}</div> }"#;
    let out = compile_react(src);
    assert!(!out.contains("&amp;"), "string literal must not be entity-escaped: {}", out);
    assert!(out.contains("Signal & Store"), "literal text changed: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 `{'Signal &amp;amp; Store'}`）

- [ ] **Step 3: 实现**——view.rs:1007 字符串字面量分支不再调用 HTML 实体转义（JS 字符串只做 JS 引号/反斜杠转义）；确认 JSXText 路径仍转义（回归既有安全测试）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；既有实体转义安全回归全绿；Playwright 界面文本正确

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/tests.rs
git commit -m "fix(react): stop entity-escaping JS string literals — visible &amp;amp; text resolved"
```

### Task 2.5: 事件/样式属性映射——camelCase + 合成事件名

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（style 属性键 camelCase 化）、`omnilang/src/codegen/shared/event.rs` 或 react 侧事件名映射（`onAnimationEnd`/`onTransitionEnd`）
- Test: `omnilang/src/codegen/react/tests.rs` + Playwright 控制台错误检查（ts/react M3）

**Interfaces:**
- Consumes: 既有 style 发射路径、事件映射表
- Produces: `font-size` → `fontSize`；`@onAnimationEnd` → `onAnimationEnd`（React 合成事件集）——控制台错误与静默死事件消除

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn style_keys_are_camel_cased() {
    let out = compile_react(r#"view V { <div style={{ "font-size": "16px" }}/> }"#);
    assert!(!out.contains("font-size"), "kebab style key leaked: {}", out);
    assert!(out.contains("fontSize"), "expected camelCase: {}", out);
}

#[test]
fn animation_events_use_synthetic_names() {
    let out = compile_react(r#"view V { <div @onAnimationEnd={f}/> }"#);
    assert!(!out.contains("onAnimationend"), "must emit React synthetic name: {}", out);
    assert!(out.contains("onAnimationEnd"), "expected onAnimationEnd: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（kebab 原样、onAnimationend 原样）

- [ ] **Step 3: 实现**——style 键发射时 kebab→camelCase（已知 CSS 属性映射表或通用 kebab 转换）；事件名映射表补 `AnimationEnd→onAnimationEnd`、`TransitionEnd→onTransitionEnd` 等合成事件集

- [ ] **Step 4: 运行验证通过**——两测试 PASS；Playwright Design 标签页控制台错误检查（原 fixme 测试 18）可 un-fixme

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/shared/event.rs omnilang/src/codegen/react/tests.rs
git commit -m "fix(react): camelCase style keys; map events to React synthetic names"
```

### Task 2.6: cleanup 依赖收集 + _M 表提升

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_effect_deps.rs`（cleanup 闭包读取纳入 body 读取）
- Modify: `omnilang/src/codegen/react/motion/mod.rs:204-219`（`_M` 表模块级提升）
- Test: `omnilang/src/codegen/react/tests.rs` + exhaustive-deps 豁免缩减（ts/react M4 + perf L1）

**Interfaces:**
- Consumes: 既有 effect deps 收集（含闭包下探）、motion 定义（编译期常量）
- Produces: unmount cleanup 中读取的信号进入 deps——`Timer` final count 正确；`_M` 每渲染重分配消除

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn effect_cleanup_reads_are_deps() {
    let src = r#"
        view Timer { signal count = 0;
            @effect { @onUnmount { log(count) } }
        }
    "#;
    let (out, diags) = compile_react_with_diags(src);
    assert!(diags.iter().all(|d| d.code != "E_EFFECT_DEPS_MISSING"), "cleanup read must be accepted as dep");
    assert!(out.contains("count"), "cleanup must reference count");
}

#[test]
fn motion_table_hoisted_to_module_scope() {
    let out = compile_react(r#"view V { <div @motion="fadeIn" @hover={m}/> } @motion fadeIn { from { opacity: 0 } }"#);
    // 断言 _M 在组件函数体外
    let fn_start = out.find("function V").unwrap();
    let table = out.find("const _M").unwrap();
    assert!(table < fn_start, "_M must be module-scoped, not per-render");
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（cleanup 读取未纳入 deps；_M 在函数体内）

- [ ] **Step 3: 实现**——validate_effect_deps 收集器下探 cleanup 闭包（`@onUnmount` 体）；`_M` 表提升到模块级（motion 定义是编译期常量；含 store 引用的 motion 值走 2.10 的 slice 通道后即可提升）

- [ ] **Step 4: 运行验证通过**——两测试 PASS；exhaustive-deps 豁免从 93x 降（真实类清除后重计数，剩余 `_M` 类随提升清零）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_effect_deps.rs omnilang/src/codegen/react/motion/mod.rs omnilang/src/codegen/react/tests.rs scripts/verify-react.exemptions
git commit -m "fix(react): effect cleanup deps + module-scoped motion table — closes stale cleanup and per-render alloc"
```

### Task 2.7: 切片订阅补全——useDesignTokens 根订阅 + 死 hooks

**Files:**
- Modify: `omnilang/src/codegen/react/design.rs:1007-1011`（`useDesignTokens` 发射）与 `__design/useDesignTokens.ts` 形状
- Modify: `omnilang/src/codegen/react/component/store_slices.rs`（hooks 生成按需化或删除）
- Test: `omnilang/src/codegen/react/tests.rs` + 生成产物断言（ts/react M2 + perf M2 同源）

**Interfaces:**
- Consumes: 既有 per-field selector hooks 生成（`useLiveAccent` 等）
- Produces: `useDesignTokens` 以切片订阅；死 hooks 不再生成（或显式接线）——App 根整 store 订阅消除

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn design_tokens_hook_uses_slices() {
    let out = compile_react_with_design(r#"design { token accent: color = "#f00"; }"#);
    // useDesignTokens 必须用 per-field selectors
    assert!(out.contains("useDynamicTokenStore((s) => s."), "whole-store subscription in design hook: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 `const DynamicTokenStore = useDynamicTokenStore();` 整对象）

- [ ] **Step 3: 实现**——design.rs 发射切片 selector；store_slices.rs 生成的 per-field hooks 改为「按需生成」（有消费者才发射）或删除后统一内联 selector

- [ ] **Step 4: 运行验证通过**——新测试 PASS；`verify-react.sh` 绿；生成产物中死 hooks 消失

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/design.rs omnilang/src/codegen/react/component/store_slices.rs omnilang/src/codegen/react/tests.rs
git commit -m "perf(react): slice-subscribe design tokens hook; emit per-field hooks on demand"
```

### Task 2.8: useCallback/React.memo——prop 函数身份稳定

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`（组件 `fn` 作为 prop 时 useCallback 包装）
- Modify: `omnilang/src/codegen/react/component/mod.rs`（子组件 React.memo 包装——无 slots/children 时）
- Test: `omnilang/src/codegen/react/tests.rs` + 生成产物断言（perf M4）

**Interfaces:**
- Consumes: 既有 effect deps 依赖分析（fn body 读取集合）
- Produces: prop 传递的 fn 用 `useCallback(fn, [deps])`；无 children 的组件实例用 `React.memo`——列表级联重渲染消除

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn prop_fn_wrapped_in_use_callback() {
    let out = compile_react(r#"
        fn handleAdd(x) { TodoStore.add(x) }
        view TodoList { <AddTodo onAdd={handleAdd}/> }
    "#);
    assert!(out.contains("useCallback"), "prop fn must be memoized: {}", out);
    assert!(out.contains("React.memo(AddTodo)") || out.contains("memo(AddTodo)"), "child must be memoized: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（裸箭头 + 无 memo）

- [ ] **Step 3: 实现**——检测「组件 fn 被作为子组件 prop 引用」→ 发射 `useCallback(fn, deps)`（deps 复用 fn body 读取分析）；组件实例化（无 children prop）包 `React.memo`

- [ ] **Step 4: 运行验证通过**——新测试 PASS；Playwright 全量仍 0 failure；`verify-react.sh` 绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/view.rs omnilang/src/codegen/react/component/mod.rs omnilang/src/codegen/react/tests.rs
git commit -m "perf(react): useCallback for prop fns + React.memo for slot-free children"
```

### Task 2.9: never-mutated 字面量信号 hoist 到模块级

**Files:**
- Modify: `omnilang/src/codegen/react/component/state.rs:19,42,68-70`（非变异信号判定扩展）
- Test: `omnilang/src/codegen/react/tests.rs`（perf M3）

**Interfaces:**
- Consumes: 既有 `mutated_signals` 分析（useState 决策用）
- Produces: 字面量初始化的非变异信号提升为模块级 const——FilterBar `demoFrameworks` 不再每渲染重分配、memo 恢复生效

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn literal_never_mutated_signal_hoisted() {
    let out = compile_react(r#"view V { signal items = ["a", "b"]; <div>{items.length}</div> }"#);
    let fn_start = out.find("function V").unwrap();
    let decl = out.find("const items =").unwrap();
    assert!(decl < fn_start, "literal signal must be module-scoped: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（`const items = [...]` 在组件体内）

- [ ] **Step 3: 实现**——`mutated_signals` 分析扩展：信号「从未变异 + 字面量初始化 + 非 store/非 props 依赖」→ 模块级 const（引用处同步——函数内引用不变，作用域提升自然生效）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；FilterBar memo 实际生效（demo 行为验证：输入过滤不再全列表重算——Playwright 可加断言）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/component/state.rs omnilang/src/codegen/react/tests.rs
git commit -m "perf(react): hoist never-mutated literal signals to module scope"
```

### Task 2.10: motion store 引用走 slice 通道 + 滚动相等守卫 + duration 提升

**Files:**
- Modify: `omnilang/src/codegen/react/component/store_slices.rs:856-942`（`motion_whole_store_stores` 回退消除）
- Modify: `omnilang/src/codegen/react/motion/transforms.rs:194-207,413-417`（相等守卫 + duration 缓存）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md:710`（声称同步为已实现——本任务实现后同步）
- Test: `omnilang/src/codegen/react/tests.rs`（perf M1/M5）

**Interfaces:**
- Consumes: `store_field_renames`（现有）、motion 值格式化路径
- Produces: motion 值中的 `Store.field` 解析为 slice 局部（整 store 订阅回退消除）；滚动/拖拽 handler 进度相等守卫 + duration 单次求值

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn motion_store_value_uses_slice() {
    let out = compile_react(r#"
        store S { signal pos = 0; }
        view V { <div @scroll={m} @motion="m" style={{ transform: "translateX({S.pos}px)" }}/> }
    "#);
    assert!(!out.contains("const S = useS()"), "motion store ref must not force whole-store: {}", out);
    assert!(out.contains("(s) => s.pos"), "expected slice selector: {}", out);
}

#[test]
fn scroll_progress_has_equality_guard() {
    let out = compile_react(r#"view V { <div @scroll={m} @motion="m"/> } @motion m { from { opacity: 0 } }"#);
    assert!(out.contains("scrubCache") || out.contains("lastProgress"), "missing equality guard: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（motion 值强制整 store；无守卫）

- [ ] **Step 3: 实现**——motion 值表达式经 `store_field_renames` 解析为 slice；transforms.rs 加 `if (p === last) return;` 守卫 + duration 提升到 handler 外

- [ ] **Step 4: 运行验证通过**——两测试 PASS；`verify-react.sh` 绿；设计文档 710 行同步（声称与实现一致）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/component/store_slices.rs omnilang/src/codegen/react/motion/transforms.rs omnilang/src/codegen/react/tests.rs docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md
git commit -m "perf(react): slice motion store refs; scroll/drag equality guard + hoisted duration"
```

### Task 2.11: 生成物清理批次——未用标识符 + ES5 var + 模块级副作用

**Files:**
- Modify: `scripts/verify-react.exemptions`（no-unused-vars 17 条、no-var 10 条处置）
- Modify: `.eslintrc` 或等价配置（`argsIgnorePattern: "^_"`）
- Modify: `omnilang/src/codegen/react/`（未消费 motion 机制不发散、`__design/useDesignTokens.ts:11-88` var→let 生成、ElementRefDemo pointerdown 移入 effect）
- Test: `omnilang/src/codegen/react/tests.rs` + `verify-react.sh`（ts/react L1/L2/L3）

**Interfaces:**
- Consumes: motion 消费分析（哪些 motion/解析辅助实际使用）
- Produces: 17 条 no-unused-vars + 10 条 no-var 豁免清零；模块级副作用消除

- [ ] **Step 1: 写失败测试**——`verify-react.sh` 跑完断言豁免条目数下降（审计脚本）：

```bash
# scripts/check_exemption_count.sh（临时）——断言 no-unused-vars 与 no-var 条目不存在
grep -c "no-unused-vars" scripts/verify-react.exemptions   # 期望 0
grep -c "no-var" scripts/verify-react.exemptions           # 期望 0
```

- [ ] **Step 2: 验证当前失败**——Run: 两 grep 均 > 0

- [ ] **Step 3: 实现**——① eslint 配置加 `argsIgnorePattern: "^_"`（清 `_i` 4 条）；② 发射器跳过未消费的 motion 机制（velocity 计算、playAll、parseScrollTrigger——按 motion 引用分析裁剪）；③ `useDesignTokens.ts` 生成时 var→let（React 构建时生成 let 拷贝，共享源保持 ES5）；④ ElementRefDemo pointerdown 移入 effect

- [ ] **Step 4: 验证**——两 grep 为 0；`verify-react.sh` 全绿；Playwright 全量 0 failure

- [ ] **Step 5: Commit**

```bash
git add scripts/verify-react.exemptions omnilang/src/codegen/react/ scripts/.eslintrc* 2>/dev/null || git add scripts/verify-react.exemptions omnilang/src/codegen/react/
git commit -m "refactor(react): prune unused motion machinery, let-copy for design runtime, scoped args ignore"
```

---

# Phase 3: Flutter 行为修复

> 顺序优先：CRITICAL struct 字面量 + widget test 基建先行（唯一能看见行为的门禁），随后交互缺口批次。每任务结束 Flutter 门禁（dart analyze 0/0）+ 0.5 的 warnings 基线必须保持或下降。

### Task 3.1: CRITICAL——store action 结构体字面量 → typed 构造调用

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/expr.rs:454-465`（`format_struct_field` JSON 键路径）+ `crates/omnilang-core/src/ir/lower/expr.rs:294-297`（struct 字面量 lowering）
- Test: `omnilang/src/codegen/flutter/tests.rs` + widget test（TodoStore.add() 执行）

**Interfaces:**
- Consumes: typed 上下文 struct 发射路径（`format_struct_field_named`，shared/expr.rs:920-935 既有）
- Produces: `{ id: nextId, text, done: false }` → `Todo(id: ..., text: ..., done: false)`——`copyWith` cast 不再失败（flutter CRITICAL）

- [ ] **Step 1: 写失败测试**——先字符串断言，再 widget test（3.2 基建依赖此测试先落地）：

```rust
#[test]
fn store_action_struct_literal_emits_constructor() {
    let src = r#"
        struct Todo { id: int, text: String, done: bool }
        store TodoStore {
            signal todos: [Todo] = [];
            fn add(nextId, text) { todos = [ { id: nextId, text, done: false }, ...todos ] }
        }
    "#;
    let out = compile_flutter(src);
    assert!(!out.contains(r"'id':"), "JSON-quoted keys in Dart: {}", out);
    assert!(out.contains("Todo(id:"), "expected typed constructor: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**

Run: `cargo test -p omnilang --features codegen-flutter flutter::tests::store_action_struct_literal_emits_constructor`
Expected: FAIL（当前 `{ 'id': state.nextId, ... }` Dart Map）

- [ ] **Step 3: 实现**——struct 字面量在「类型已知上下文」（store action 赋值给 typed 信号/参数）走 `format_struct_field_named` → `Todo(...)` 构造调用；unknown 类型上下文保持 Map 字面量（Dart 合法）

- [ ] **Step 4: 运行验证通过**——字符串测试 PASS；widget test（3.2 后补）：`tester.tap(addBtn)` 后 TodoStore 状态含新项、无异常

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/expr.rs crates/omnilang-core/src/ir/lower/expr.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): emit typed struct constructors in store actions — closes TodoStore.add TypeError"
```

### Task 3.2: widget test 基建——生成代码运行时测试 harness

**Files:**
- Create: `omnilang/tests/flutter_widget_tests/`（harness + 首批冒烟测试）
- Modify: `omnilang/Cargo.toml`（test 入口或文档化运行方式）
- Test: 同上（flutter 长期项 #2 + 0.5 配套）

**Interfaces:**
- Consumes: 生成 Dart 项目（learn-omniview Flutter 输出）
- Produces: `tester.pumpWidget` 冒烟测试框架——@if/@for/motion/ref/[scroll]+store 读的执行级验证（首个生成代码运行时测试）

- [ ] **Step 1: 写失败测试**——harness 首测：挂载生成 app 主组件

```dart
// test/smoke_test.dart（learn-omniview Flutter 输出项目内）
testWidgets('app mounts without exception', (tester) async {
  await tester.pumpWidget(const ProviderScope(child: LearnOmniviewApp()));
  expect(tester.takeException(), isNull);
});
```

- [ ] **Step 2: 运行验证当前状态**——Run: `flutter test`（learn-omniview Flutter 输出目录）
Expected: FAIL（当前 app 挂载即抛——TodoStore 初始化或首页依赖链）

- [ ] **Step 3: 实现**——① harness 目录 + ProviderScope 包装 + 入口组件名对齐（生成的 App 类名）；② 首批冒烟：app 挂载、@if 切换、@for 渲染、bind:value 输入回写、TodoStore.add()

- [ ] **Step 4: 运行验证通过**——冒烟测试全绿（3.1 修复后）；记录到 README 或 CI（本任务先本地，CI 接线并入 0.2 后续或独立 job——标记 `flutter-widget-tests` job 候选）

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/flutter_widget_tests/ test/projects/learn-omniview/build/flutter-test-app/test/
git commit -m "test(flutter): widget-test harness — first runtime verification of generated code"
```

### Task 3.3: bind:value 非 ref 接线——onChanged 回写

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs:1436-1461`（`controller_covers` 判定扩展）
- Modify: `omnilang/src/codegen/flutter/utils.rs:294-310`（widget 白名单）
- Test: `omnilang/src/codegen/flutter/tests.rs` + widget test（输入回写）（flutter HIGH + compiler HIGH + 0.5 warnings 下降）

**Interfaces:**
- Consumes: 既有 `bind:value` IR（`SemanticBinding`）
- Produces: 无 @ref 的 `bind:value` 发射 `onChanged: (v) => setState(() => signal = v)`（或 controller）——14 处死绑定恢复；「onChanged: null 永久禁用控件」消失

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn bind_value_without_ref_emits_on_changed() {
    let out = compile_flutter(r#"view V { <input bind:value={q}/> }"#);
    assert!(!out.contains("onChanged: null"), "disabled control: {}", out);
    assert!(out.contains("onChanged:"), "missing write-back: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 warning + drop → onChanged: null）

- [ ] **Step 3: 实现**——`bind:value` 在无 controller 合成路径时发射 onChanged 回写（`(v) => setState(() => signal = v)`；信号语义从 IR 已知）；有 @ref 时保持 controller 路径

- [ ] **Step 4: 运行验证通过**——新测试 PASS；0.5 warnings 基线下降（14 条 binding warning 消失）；widget test：输入 textfield → 信号更新断言

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/utils.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): wire bind:value two-way on non-ref inputs via onChanged"
```

### Task 3.4: @in-view/@focus 触发器接线 + 实例化断言

**Files:**
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:310-312`（`in_view_once/repeat/threshold` 从 `TriggerKind::InView` 填充）
- Modify: `omnilang/src/codegen/flutter/view.rs:1651-1682`（`has_inview` 判定）+ `TriggerKind::Focus` 处理（FocusNode listener）
- Test: `omnilang/src/codegen/flutter/tests.rs:7802`（fixture 补**实例化**断言）+ widget test（flutter HIGH）

**Interfaces:**
- Consumes: `TriggerKind::InView/Focus`（IR 既有）、`OmniInViewWidget`（runtime 既有，从未实例化）
- Produces: `@in-view` 发射 `OmniInViewWidget(` 实例化；`@focus` 接线或显式 warning——动画不再静默不播

- [ ] **Step 1: 写失败测试**——fixture 断言从「helper 类存在」升级为「实例化」：

```rust
#[test]
fn in_view_trigger_instantiates_wrapper() {
    let src = r#"view V { <div @in-view={m} @motion="m"/> } @motion m { from { opacity: 0 } }"#;
    let out = compile_flutter(src);
    assert!(out.contains("OmniInViewWidget("), "wrapper never instantiated: {}", out);
    assert!(out.contains("inViewOnce") || out.contains("inViewThreshold"), "trigger config missing: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前只有 import，无实例化；in_view_* 恒 None）

- [ ] **Step 3: 实现**——lowering 从 `TriggerKind::InView` 填充 `in_view_once/repeat/threshold`；flutter 发射器 `has_inview` 读触发器集；`@focus` 生成 FocusNode listener（`focusNode.addListener(() { if (hasFocus) controller.forward(); })`）——若 Focus 接线复杂，本任务至少发射 warning（诚实降级，D-H 同款）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；widget test：滚动进入视口 → opacity 动画启动断言（若可测）；0.5 warnings 基线不增

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/lower/view.rs omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): wire @in-view/@focus triggers — wrapper instantiation + FocusNode listener"
```

### Task 3.5: textarea/disabled 映射

**Files:**
- Modify: `omnilang/src/codegen/flutter/utils.rs:18-49`（tag 映射表）、`omnilang/src/codegen/flutter/view.rs`（boolean 属性处理）
- Test: `omnilang/src/codegen/flutter/tests.rs` + 0.5 warnings 下降（flutter HIGH）

**Interfaces:**
- Consumes: 既有 tag map、boolean-attr 发射路径
- Produces: `<textarea>` → `TextField(maxLines: 4)`（多行意图）；`disabled` → `enabled: false`（表单控件）/ 其他元素 warning——不可见空 Container 与静默可编辑消失

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn textarea_maps_to_multiline_text_field() {
    let out = compile_flutter(r#"view V { <textarea/> }"#);
    assert!(!out.contains("child: Container()"), "textarea rendered as empty container: {}", out);
    assert!(out.contains("maxLines:"), "multiline intent lost: {}", out);
}

#[test]
fn disabled_emits_enabled_false() {
    let out = compile_flutter(r#"view V { <input disabled/> }"#);
    assert!(out.contains("enabled: false"), "disabled silently dropped: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（Container() 空壳；disabled 无痕消失）

- [ ] **Step 3: 实现**——tag map 补 `textarea => TextField(maxLines: 4)`（绑定随 3.3 通道走）；boolean `disabled` → `enabled: false`（TextField/Checkbox/Switch 类），其余元素 warning

- [ ] **Step 4: 运行验证通过**——两测试 PASS；FormPage widget test：message 字段可见可输入、disabled 字段不可编辑

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/utils.rs omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): map textarea to multiline TextField; disabled to enabled:false"
```

### Task 3.6: 主题选择器 + themeMode 四态 + multi-theme 塌缩警告

**Files:**
- Modify: `omnilang/src/codegen/flutter/utils.rs:307-317`（widget_event_whitelist：`Container` 事件映射）
- Modify: `omnilang/src/codegen/flutter/design.rs:1289-1476`（`themeModeProvider` 四态 + `emit_theme_switch` 警告）
- Test: `omnilang/src/codegen/flutter/tests.rs` + widget test（点主题卡片 → ThemeMode 变化）（flutter HIGH + compiler MEDIUM）

**Interfaces:**
- Consumes: `DesignIR.themes`（列表）、`themeModeProvider` 既有
- Produces: `@onPress` 在通用元素 → InkWell/GestureDetector 包装；`themeModeProvider` 支持全部主题（自定义映射 light/dark 之外为主题→ThemeData 索引）；>2 主题编译 warning「Flutter 仅支持 light/dark 两态；其余主题不可达」（D-G）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn generic_element_on_press_gets_tappable_wrapper() {
    let out = compile_flutter(r#"view V { <div @onPress={f}>x</div> }"#);
    assert!(out.contains("InkWell(") || out.contains("GestureDetector("), "no tappable wrapper: {}", out);
}

#[test]
fn multi_theme_flutter_warns() {
    let (out, diags) = compile_flutter_with_diags(r#"design { @themes { light, dark, sepia } }"#);
    assert!(diags.iter().any(|d| d.message.contains("不可达") || d.message.contains("light/dark")),
        "missing multi-theme warning: {:?}", diags);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（`onPress` dropped；sepia 静默死代码）

- [ ] **Step 3: 实现**——通用元素事件 → InkWell 包装（无 onTap 子冲突时）；`themeModeProvider` 以主题名 → ThemeMode/自定义索引映射（>2 主题：ThemeData 列表 + 索引选择——若实现成本高则按 D-G 只警告并映射到 light/dark）；`emit_theme_switch` 对 >2 主题发 warning

- [ ] **Step 4: 运行验证通过**——两测试 PASS；widget test：点 sepia 卡片（若实现）或点 dark 卡片断言 themeModeProvider 变化；ThemeSwitcher 不再死代码

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/utils.rs omnilang/src/codegen/flutter/design.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): tappable theme cards; multi-theme warning; themeMode provider wiring"
```

### Task 3.7: ref.watch 提升出闭包 + [scroll]+store 读 fixture

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（`@for` itemBuilder/.where 谓词内 watch 提升）
- Modify: `omnilang/src/codegen/flutter/mod.rs:793-802`（computed 发射位置）
- Test: `omnilang/src/codegen/flutter/tests.rs` + widget test（flutter MEDIUM）

**Interfaces:**
- Consumes: Riverpod `ref.watch` 发射路径
- Produces: store 读取提升为 build 局部变量；`.where` 谓词/itemBuilder 内不再有 watch——Riverpod 3.0 作用域合规

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn store_read_hoisted_out_of_filter_predicate() {
    let src = r#"
        store S { signal items = []; signal filter = ""; }
        view V { <for item in S.items.where(|x| x.contains(S.filter))>{item}</for> }
    "#;
    let out = compile_flutter(src);
    let where_pos = out.find(".where(").unwrap();
    let watch_pos = out.find("ref.watch").unwrap();
    assert!(watch_pos < where_pos, "watch must be hoisted above predicate: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（watch 在 .where 内）

- [ ] **Step 3: 实现**——迭代表达式/谓词中的 store 读取先求值为 build 局部（`final __filter = ref.watch(...);`）；[scroll] itemBuilder 内读取同提升（itemBuilder 数据经 item 值传递）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；widget test：[scroll] 列表 + store 读冒烟（无 Riverpod scope 异常）；0.5 warnings 不增

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/mod.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): hoist ref.watch out of closures — Riverpod scope compliance"
```

### Task 3.8: [scroll] @for O(N²) 修复——iterable 单次求值

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs:2016-2020,2033-2036`（`format_list_rendering`）
- Test: `omnilang/src/codegen/flutter/tests.rs`（perf H2）

**Interfaces:**
- Consumes: 既有 [scroll] 分派
- Produces: `itemCount: __omniIter.length` + `var item = __omniIter[index]`（前置 `final __omniIter = {iter};`）——N+1 次求值 → 1 次；@empty 三求值同修

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn scroll_for_iterable_evaluated_once() {
    let out = compile_flutter(r#"view V { <for [scroll] item in S.items.filter(|x| x > 0)>{item}</for> }"#);
    // iterable 表达式只能出现一次求值点（hoist 变量）
    let evals = out.matches("items.filter").count();
    assert_eq!(evals, 1, "iterable evaluated {} times — O(N²): {}", evals, out);
    assert!(out.contains("final __omniIter"), "missing hoist: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（`items.filter` 出现 2+ 次）

- [ ] **Step 3: 实现**——[scroll] 分支前置 `final __omniIter = <iter>;`，itemCount/itemBuilder 引用变量；@empty Builder 同用

- [ ] **Step 4: 运行验证通过**——新测试 PASS；既有 [scroll] fixture 断言更新（形状变化）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "perf(flutter): single-eval iterable in scroll @for — O(N²) to O(N)"
```

### Task 3.9: DefaultTextStyle.merge + IndexedStack 标签页 + Scaffold/SafeArea 壳

**Files:**
- Modify: `omnilang/src/codegen/flutter/motion.rs:740-765`（`DefaultTextStyle.merge`）
- Modify: `omnilang/src/codegen/flutter/view.rs:1889-1974`（顶层 @if 互斥组 → IndexedStack）
- Modify: `omnilang/src/codegen/flutter/design.rs` 或 main 壳（Scaffold/SafeArea）
- Test: `omnilang/src/codegen/flutter/tests.rs`（flutter MEDIUM ×3）

**Interfaces:**
- Consumes: 既有 @if 发射、main.dart 生成
- Produces: motion color/fontSize 动画不再整体替换继承样式；标签页切换保留页面状态；根壳有 Scaffold/SafeArea

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn motion_text_style_merges_inherited() {
    let out = compile_flutter(r#"view V { <div @motion="m" @enter={m}/> } @motion m { from { color: red } }"#);
    assert!(out.contains("DefaultTextStyle.merge"), "must merge, not replace: {}", out);
}

#[test]
fn page_switcher_uses_indexed_stack() {
    let out = compile_flutter(r#"view App { @if page == "a" { <PageA/> } @else { <PageB/> } }"#);
    assert!(out.contains("IndexedStack("), "tab switch unmounts state: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（`DefaultTextStyle(` 替换；三元互斥）

- [ ] **Step 3: 实现**——motion 文本样式包装改 `DefaultTextStyle.merge(style: ...)`（color+fontSize 同 wrapper）；顶层互斥 @if 组（组件级页面切换形态）发射 IndexedStack + `index` 表达式；main.dart 根包 Scaffold(body: SafeArea(...))

- [ ] **Step 4: 运行验证通过**——两测试 PASS；widget test：切标签页后表单输入保留断言

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/motion.rs omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/design.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): merge motion text style; IndexedStack page switcher; Scaffold/SafeArea shell"
```

### Task 3.10: Image URL/资源路径 + fontFamily 栈

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`（`Image.network` 相对路径判定）
- Modify: `omnilang/src/codegen/flutter/design.rs:35`（fontFamily 多值栈）
- Test: `omnilang/src/codegen/flutter/tests.rs`（flutter LOW）

**Interfaces:**
- Consumes: 既有 Image.network 发射
- Produces: 相对/本地路径 → 正确资源处理（AssetImage 或警告）；fontFamily 栈 → 首字体 + 警告（若多字体栈不可表达）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn relative_image_path_warns_or_assets() {
    let (out, diags) = compile_flutter_with_diags(r#"view V { <img src="/dot.png"/> }"#);
    assert!(diags.iter().any(|d| d.code == "W" || d.code == "E"), "relative path must warn: {:?}", diags);
}

#[test]
fn font_family_stack_uses_first_font_with_warning() {
    let (out, diags) = compile_flutter_with_diags(r#"design { token fontFamily: font = "system-ui, sans-serif"; }"#);
    assert!(diags.iter().any(|d| d.message.contains("font")), "font stack must warn: {:?}", diags);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（相对路径静默发射 NetworkImage；fontFamily 栈原样）

- [ ] **Step 3: 实现**——Image 路径判定（绝对 URL → NetworkImage；相对 → AssetImage + 警告）；fontFamily 逗号栈 → 取首值 + 警告

- [ ] **Step 4: 运行验证通过**——两测试 PASS；0.5 warnings 基线更新（新增警告入基线——只允许本任务新增的诚实警告，净效果为行为改善）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/view.rs omnilang/src/codegen/flutter/design.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix(flutter): relative image paths and font stacks warn honestly"
```

---

# Phase 4: 静默降级清零（语义/诊断层）

> 目标：恢复「绝无 parse-and-ignore」声明的可验证性。每个任务 = 一个静默路径 → 硬诊断/warning + 迁移提示 + 回归测试。顺序：parser 遗留 → 新语义验证 → 死表面删除 → 收口决策。

### Task 4.1: `states:`/`on:` 硬诊断化（哨兵机制清除）

**Files:**
- Modify: `crates/omnilang-core/src/parser/view/element.rs:196-231`（两分支 → 解析错误 + 迁移提示）
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:585-601`（删哨兵消费）、`omnilang/src/codegen/react/motion/mod.rs:275`（删跳过）
- Test: `crates/omnilang-core/src/parser/view/element.rs` 测试段（domain HIGH-1）

**Interfaces:**
- Consumes: 既有错误通道（`E_*` 诊断码模式）
- Produces: `states:` → 「states: 已删除（ADR-053），用 @motion + 触发器替代」；`on:` → 「on: 已删除，用 @onXxx={...}」——两者均解析错误 + 回归测试；`"__states_removed"` 哨兵删除

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn legacy_states_syntax_hard_errors() {
    let src = r#"view V { states: { hover: { opacity: 0.5 } } }"#;
    let diags = parse_and_diag(src);
    let d = diags.iter().find(|d| d.message.contains("states")).expect("must diagnose");
    assert!(d.message.contains("迁移") || d.message.contains("替代"), "missing migration hint: {}", d.message);
}

#[test]
fn legacy_on_block_hard_errors() {
    let src = r#"view V { on: { x => y } }"#;
    let diags = parse_and_diag(src);
    assert!(diags.iter().any(|d| d.message.contains("on:") || d.message.contains("@onXxx")), "must diagnose on:");
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前零诊断 + 哨兵静默丢弃；on: 误报「箭头仅用于事件内联」）

- [ ] **Step 3: 实现**——parser 两分支发射解析错误（verbatim 迁移提示）；删除哨兵字符串与全部消费点；回归测试断言错误存在

- [ ] **Step 4: 运行验证通过**——两测试 PASS；`cargo test -p omnilang-core` 全绿；既有相关 fixture（若有使用 states:/on: 的）更新

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/view/element.rs crates/omnilang-core/src/ir/lower/view.rs omnilang/src/codegen/react/motion/mod.rs
git commit -m "fix: hard-diagnose legacy states:/on: with migration hints — removes sentinel machinery"
```

### Task 4.2: validate_event_names——未知事件名 warning + 最近合法建议（D-B）

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_event_names.rs`
- Modify: `crates/omnilang-core/src/semantic/mod.rs`（pass 注册）、`omnilang/src/codegen/shared/event.rs:22-90`（canonical 表导出为白名单）
- Test: 新 pass 测试段 + 三目标 fixture（ai HIGH）

**Interfaces:**
- Consumes: canonical 事件表（shared/event.rs 既有）、`ElementIR.events`
- Produces: 未知事件名 → warning + 最近合法建议（编辑距离或前缀匹配）；白名单导出供 docs-gate 复用（6.6）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn unknown_event_warns_with_suggestion() {
    let src = r#"view V { <div @onTotallyBogus={f}/> }"#;
    let (_, diags) = compile_with_diags(src, Target::React);
    let d = diags.iter().find(|d| d.severity == Severity::Warning).expect("must warn");
    assert!(d.message.contains("onTotallyBogus"), "must name the event: {}", d.message);
    assert!(d.message.contains("@onPress") || d.message.contains("@onClick") || d.message.contains("@onTap"),
        "missing nearest suggestion: {}", d.message);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 "No issues found"）

- [ ] **Step 3: 实现**——白名单 = canonical 表键集 + 文档化别名（press/click/tap 族）；未知键 → warning（消息含原名 + 最近建议）；建议算法：共享前缀最长或最小编辑距离（≤2 的别名如 onPress/onClick/onTap 族内置映射）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；全量语料编译 warnings 无新增（现有 @onXxx 全部在表内——domain 专家已验证语料规范）；0.5 warnings 基线不增

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_event_names.rs crates/omnilang-core/src/semantic/mod.rs omnilang/src/codegen/shared/event.rs
git commit -m "feat(semantic): event-name validation — unknown names warn with nearest suggestion"
```

### Task 4.3: a11y role 词汇 ARIA 化——textbox/img 内部映射

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_a11y.rs:29-30`（子集 → ARIA 规范名）
- Modify: `omnilang/src/codegen/flutter/view.rs:811-822`（`textField`→`textbox` 映射、`image`→`img`）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md` §11.4 + `test/projects/learn-omniview/src/components/A11yDemo.omni:41`
- Test: validate_a11y 测试段 + 三目标 fixture（domain HIGH-2）

**Interfaces:**
- Consumes: 既有 role 子集校验
- Produces: 语义词汇 = WAI-ARIA 规范名（`button/textbox/link/checkbox/radio/img`）；Flutter 内部映射回旗标；web 输出合法 ARIA——`role="textField"` 消失、`role="searchbox"` 可接受

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn a11y_role_uses_aria_vocabulary() {
    let src = r#"view V { <div a11y: { role: "textbox" }/> }"#;
    let out_react = compile_react(src);
    assert!(out_react.contains("role=\"textbox\""), "invalid ARIA passthrough: {}", out_react);
    let out_flutter = compile_flutter(src);
    assert!(out_flutter.contains("textField: true"), "must map to flutter flag: {}", out_flutter);
}

#[test]
fn legacy_flag_role_warns() {
    let (_, diags) = compile_with_diags(r#"view V { <div a11y: { role: "textField" }/> }"#, Target::React);
    assert!(diags.iter().any(|d| d.message.contains("textbox")), "must suggest canonical name: {:?}", diags);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（textbox 被拒 + textField 通过；web 输出非法 role）

- [ ] **Step 3: 实现**——子集换 ARIA 名；内部映射表 `textbox→textField:true`、`img→image:true`；遗留旗标名 warning + 建议规范名；A11yDemo 与 spec §11.4 同步

- [ ] **Step 4: 运行验证通过**——两测试 PASS；三目标 fixture 更新；docs-gate 232 块重跑（A11yDemo 相关代码块）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_a11y.rs omnilang/src/codegen/flutter/view.rs docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md test/projects/learn-omniview/src/components/A11yDemo.omni
git commit -m "fix(a11y): ARIA-canonical role vocabulary with flutter-flag internal mapping"
```

### Task 4.4: aria-* 原生属性硬诊断 + alt= 边界裁决（D-D）

**Files:**
- Modify: `crates/omnilang-core/src/semantic/`（新校验或 validate_a11y 扩展：`aria-*` 属性检测）
- Modify: `docs/omniview/06-templates.md:232-240`（aria-* 教学删除、alt= 边界文档化）
- Modify: `test/projects/learn-omniview/src/components/EventsDemo.omni:37`（迁移）
- Test: validate 测试段（domain MEDIUM-1）

**Interfaces:**
- Consumes: `ElementIR` 属性集
- Produces: `aria-*` → 硬诊断 + 「用 a11y: { label/role/... } 块」迁移提示；`alt=` 保留为普通 DOM 属性（Flutter semanticLabel 映射既有，文档化）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn raw_aria_attributes_hard_diagnosed() {
    let (_, diags) = compile_with_diags(r#"view V { <div aria-label="x"/> }"#, Target::All);
    let d = diags.iter().find(|d| d.message.contains("aria")).expect("must diagnose aria-*");
    assert!(d.message.contains("a11y"), "missing migration hint: {}", d.message);
}

#[test]
fn alt_remains_plain_dom_prop() {
    let out_react = compile_react(r#"view V { <img alt="logo"/> }"#);
    assert!(out_react.contains("alt=\"logo\""), "alt must stay a DOM prop: {}", out_react);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（aria-label 静默透传；alt 无断言）

- [ ] **Step 3: 实现**——语义 pass 检测 `aria-*` 属性前缀 → 硬诊断 + 迁移提示；alt= 保持现状 + 文档边界声明（06-templates 改教学：aria-* 已删除、alt= 是 DOM 属性非 a11y 表面）；EventsDemo 迁移到 a11y 块

- [ ] **Step 4: 运行验证通过**——两测试 PASS；全量语料无 aria-* 残留（grep 验证）；docs-gate 绿

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/ docs/omniview/06-templates.md test/projects/learn-omniview/src/components/EventsDemo.omni
git commit -m "fix(a11y): hard-diagnose raw aria-* with migration hint; document alt= DOM boundary"
```

### Task 4.5: @fallback/@error 死 token 删除 + TestId 死变体删除

**Files:**
- Modify: `crates/omnilang-core/src/lexer/keywords.rs:81-82`（删两 token）
- Modify: `crates/omnilang-core/src/parser/ast/view.rs:197`（`ElementProp::TestId` 删除）、`crates/omnilang-core/src/ir/lower/view.rs:553-562`（data-testid 臂删除）、`crates/omnilang-core/src/semantic/resolve.rs:1001`（忽略臂删除）
- Modify: `docs/omniview/14-grammar-lang.md:61-62`（关键字列表同步）
- Test: lexer/parser 测试段（domain MEDIUM-3 + pragmatic L1）

**Interfaces:**
- Consumes: 现有 token/枚举结构
- Produces: `@error`/`@fallback` 自然解析错误（同 @native 处置）；`ElementProp::TestId` 变体 + 两个臂删除——「半接线属性」模式清零

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn at_error_token_removed() {
    let diags = parse_and_diag(r#"view V { <div>@error x</div> }"#);
    assert!(diags.iter().any(|d| d.severity == Severity::Error), "@error must not render as text");
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前渲染为字面文本零诊断）

- [ ] **Step 3: 实现**——删两 token（lexes 为 Ident → 自然错误）；删 TestId 变体与三处臂；grammar 文档关键字列表同步

- [ ] **Step 4: 运行验证通过**——新测试 PASS；`cargo test -p omnilang-core` 全绿；docs-gate 绿

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/lexer/keywords.rs crates/omnilang-core/src/parser/ast/view.rs crates/omnilang-core/src/ir/lower/view.rs crates/omnilang-core/src/semantic/resolve.rs docs/omniview/14-grammar-lang.md
git commit -m "fix: remove dead @fallback/@error tokens and TestId variant — zero silent text/attributes"
```

### Task 4.6: filter/map/count prove-or-retire（D-E）

**Files:**
- Modify: `test/projects/learn-omniview/src/`（4 处 stdlib 调用迁移：TodoList.omni:30、DataTable.omni:41、FilterBar.omni:23、array_ext.omni:14）
- Modify: `crates/omnilang-core/src/semantic/validate_array_methods.rs` / stdlib 表（按验证结果）
- Test: 门禁重跑 + 语料 grep（pragmatic M1）

**Interfaces:**
- Consumes: 语言方法 `filter/map/count`（v2 域 8）、stdlib `array_filter/array_map`
- Produces: 一种意图一个名字——prove 路径：4 处迁移到语言方法 + 三目标门禁验证 → 通过则 stdlib 重复降级（警告或删除）；语料抗拒则删语言方法

- [ ] **Step 1: 写失败测试（语料迁移即测试）**——4 处调用迁移为 `items.filter(|x| ...)` 语言方法

- [ ] **Step 2: 运行验证当前状态**——Run: 三目标全量门禁（`omnic build -t react/flutter/omniview` + verify-react + dart analyze）
Expected: 记录迁移前基线（stdlib 路径当前全绿）

- [ ] **Step 3: 实现**——迁移 4 处；若语言方法在任一目标行为不等价（如 Flutter map 语义差异），回退该处并记录 → 判定 retire：删语言方法（semantic/validate_array_methods.rs + IR ArrayMethod + 三后端映射）并保留 stdlib；若等价：stdlib 表 `array_filter/array_map` 标记 deprecated warning

- [ ] **Step 4: 运行验证通过**——三目标门禁全绿（按所选路径）；语料 grep 确认单名状态

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/src/ crates/omnilang-core/src/semantic/validate_array_methods.rs crates/omnilang-core/src/codegen/shared/expr.rs
git commit -m "refactor: array methods prove-or-retire — one name per intent (see commit body for verdict)"
```

### Task 4.7: @default-theme ↔ @theme initial 一致性验证 + E_PAUSED block 形式

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_theme.rs:162-196`（新增一致性检查）
- Modify: `crates/omnilang-core/src/parser/top_level/motion.rs:384`（E_PAUSED_REMOVED 扩展 block 形式）
- Test: 两 pass 测试段（domain MEDIUM-2 + LOW-4）

**Interfaces:**
- Consumes: `DesignIR.default_theme`、`theme_signal` initial
- Produces: 双声明 mismatch → error（D-F）；`@motion fadeIn { paused: true }` → verbatim 迁移提示

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn default_theme_mismatch_errors() {
    let src = r#"
        design { @themes { light, dark } @default-theme light; }
        store S { signal mode: String @theme = "dark"; }
    "#;
    let (_, diags) = compile_with_diags(src, Target::All);
    assert!(diags.iter().any(|d| d.severity == Severity::Error), "mismatch must error");
}

#[test]
fn paused_block_form_has_migration_hint() {
    let diags = parse_and_diag(r#"@motion fadeIn { paused: true }"#);
    assert!(diags.iter().any(|d| d.message.contains("paused")), "must name paused");
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（mismatch 无诊断；paused 报通用 expected-from/to）

- [ ] **Step 3: 实现**——validate_theme：两者都存在时比较 initial == default_theme（mismatch → error，消息含两边值与位置）；motion block 形式走 E_PAUSED_REMOVED

- [ ] **Step 4: 运行验证通过**——两测试 PASS；demo 双声明一致（design.omni:25 light == ThemeStore.omni:19 "light"）——门禁不破

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_theme.rs crates/omnilang-core/src/parser/top_level/motion.rs
git commit -m "fix: validate @default-theme/@theme-initial consistency; extend paused migration hint"
```

### Task 4.8: block-expr Rust/Dioxus + ArrayMethod floor（#55）

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/expr.rs:1063-1064`（Rust-family `format_block_expr`）
- Modify: `crates/omnilang-core/src/codegen/shared/expr.rs` 或各后端（`ArrayMethod` 未映射后端 → E_CODEGEN）
- Test: `crates/omnilang-core/src/codegen/shared/expr.rs` 测试段 + 各后端 fixture（rust MEDIUM + compiler MEDIUM）

**Interfaces:**
- Consumes: Rust 块表达式语法（目标端）、BlockExpr E_CODEGEN 先例（Python/Vue 既有）
- Produces: Rust/Dioxus 语句块表达式发射 `{ stmts; tail }`（或诊断）；ArrayMethod 在 python/rust/dioxus → E_CODEGEN「此目标不支持语言数组方法」+ 建议 stdlib

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn rust_block_expr_emits_or_diagnoses() {
    let (out, diags) = compile_rust_with_diags(r#"view V { fn f(x) { let y = x; y } }"#);
    if out.is_empty() && diags.is_empty() {
        panic!("must either emit block expr or diagnose");
    }
    assert!(!diags.is_empty() || out.contains("let y = x;"), "silent drop: {:?}", diags);
}

#[test]
fn array_method_python_hard_diagnosed() {
    let (_, diags) = compile_python_with_diags(r#"view V { <for item in items.filter(|x| x > 0)>{item}</for> }"#);
    assert!(diags.iter().any(|d| d.code == "E_CODEGEN"), "verbatim emission must be gated");
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（Rust 静默丢语句；python verbatim filter()）

- [ ] **Step 3: 实现**——Rust-family 发射 `{ stmts; tail }` 块表达式（Rust 支持）；ArrayMethod 未映射后端发射 E_CODEGEN（消息建议 stdlib `array_filter` 等）

- [ ] **Step 4: 运行验证通过**——两测试 PASS；`cargo test -p omnilang-core` 全绿

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/codegen/shared/expr.rs
git commit -m "fix: emit rust block expressions; floor-gate ArrayMethod on unmapped backends (#55)"
```

### Task 4.9: motion-in-@for 编译诊断（D-H）

**Files:**
- Modify: `crates/omnilang-core/src/semantic/`（新校验：@for 子元素带 @motion → 诊断）
- Test: semantic 测试段（perf M2 + compiler 上下文）

**Interfaces:**
- Consumes: `ListRenderingIntent` + motion 触发器 IR
- Produces: @for 内 motion → warning「motion 在 @for 子元素尚未支持逐项实例化；当前退化为单一共享实例（React）或全部项同时播放（Flutter）」——静默分叉变为诚实降级（D-H；逐项实现为长期项）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn motion_in_for_warns() {
    let src = r#"view V { <for item in items key={item.id}><div @motion="m" @enter={m}>{item}</div></for> }"#;
    let (_, diags) = compile_with_diags(src, Target::All);
    assert!(diags.iter().any(|d| d.message.contains("motion") && d.message.contains("@for")),
        "must warn on motion-in-for: {:?}", diags);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前零诊断）

- [ ] **Step 3: 实现**——语义 pass 检测 @for body 内元素携带 motion 触发器 → warning（含 D-H 语义说明与建议）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；语料无此形态（grep 确认）——warnings 基线不增

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/
git commit -m "fix: diagnose motion-in-@for — honest degradation until per-item allocation"
```

### Task 4.10: 具体类型方法名验证——warning + stdlib 建议

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_types.rs:56`（`MethodCall => SemanticType::Any` 分支扩展）
- Test: validate_types 测试段（ai LOW）

**Interfaces:**
- Consumes: 既有类型推断（String/i32/struct 具体类型）
- Produces: 具体接收者类型上的未知方法 → warning + stdlib 建议（`toUpper` → `to_uppercase`）；Any 接收者保持现状（不级联）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn unknown_method_on_concrete_type_warns() {
    let (_, diags) = compile_with_diags(r#"view V { fn f(s: String) { s.toUpper() } }"#, Target::All);
    let d = diags.iter().find(|d| d.message.contains("toUpper")).expect("must warn");
    assert!(d.message.contains("to_uppercase"), "missing stdlib suggestion: {}", d.message);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前 "No issues found"）

- [ ] **Step 3: 实现**——String/i32/float/bool/数组等具体类型的已知 stdlib 方法表；未知 → warning（不 error，避免过度约束）；Any 接收者跳过

- [ ] **Step 4: 运行验证通过**——新测试 PASS；语料编译 warnings 无新增（grep 语料方法名——语料全用正确 stdlib）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/validate_types.rs
git commit -m "feat(semantic): warn on unknown methods of concrete types with stdlib suggestions"
```

### Task 4.11: 诊断位置精确化 + suggestion 字段填充

**Files:**
- Modify: `crates/omnilang-core/src/semantic/validate_semantic_bindings.rs:183-198`、`validate_ref_ops.rs:119-141`（`ElementIR.source_span` 线程化）
- Modify: `crates/omnilang-core/src/semantic/lints/*.rs`（`line:0,col:0` → AST span）
- Modify: 迁移提示消息 → `suggestion` 字段（validate_effect_deps.rs:137 先例）
- Test: 各 pass 测试段位置断言（ai MEDIUM）

**Interfaces:**
- Consumes: `ElementIR.source_span`（IR 既有，未线程化）、`Diagnostic.suggestion`
- Produces: 语义/ lint 诊断位置精确到元素/表达式；迁移提示进入结构化 suggestion 字段——JSON 契约承诺兑现

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn semantic_binding_diag_has_element_position() {
    let src = "view V {\n  <div bind:value={x}/>\n}";
    let (_, diags) = compile_with_diags(src, Target::All);
    let d = diags.iter().find(|d| d.code == "E_VALUE_BINDING_REMOVED").unwrap();
    assert!(d.line >= 2, "must point at element line, got {}", d.line);
    assert!(d.suggestion.is_some(), "suggestion must be populated: {:?}", d);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（当前锚定组件起始行 1:1、suggestion: null）

- [ ] **Step 3: 实现**——两 pass 的 `make_diag` 用匹配处的 `source_span`；lints 从 AST 节点取 span；verbatim 迁移提示文本同时写入 `suggestion`（message 保持人类可读——docs-gate 消息匹配逻辑同步检查 6.6）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；既有断言仅 message 的测试更新为断言位置；docs-gate 仍绿（verify-docs.py 的 message 前缀匹配不受影响——确认后通过）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/
git commit -m "fix(diagnostics): thread element spans into semantic/lint diags; populate suggestion field"
```

### Task 4.12: unused-import 假阳性清零——DesignClass/Variant 计入 used

**Files:**
- Modify: `crates/omnilang-core/src/semantic/lints/ast_collectors.rs:283-286`（`ClassValue::DesignClass/Variant/TestId` 排除反转）
- Test: `crates/omnilang-core/src/semantic/lints/` 测试段 + 0.5 warnings 基线骤降（pragmatic M2 + compiler MEDIUM + ai MEDIUM）

**Interfaces:**
- Consumes: 既有 used-name 收集
- Produces: `class: LearnDesign.X` 计为使用——123 条假阳性（46× LearnDesign）清零；warnings 通道只剩真实信号

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn design_class_ref_counts_as_use() {
    let src = r#"import { LearnDesign } from "../design"; view V { <div class: LearnDesign.card/> }"#;
    let (_, diags) = compile_with_diags(src, Target::All);
    assert!(!diags.iter().any(|d| d.message.contains("never used")), "false positive: {:?}", diags);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（import 警告未用）

- [ ] **Step 3: 实现**——`ClassValue::DesignClass`/`Variant` 引用计入 used-name 集合（TestId 已随 4.5 删除）；同步检查 @store 指令与元素标签是否计为使用（compiler 指出的其余假阳性面）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；learn-omniview 全量编译 warnings 123→0（或接近）；0.5 基线更新

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/lints/ast_collectors.rs omnilang/tests/generated_code_validation.rs
git commit -m "fix(lint): count DesignClass/Variant refs as uses — 123 false positives cleared"
```

### Task 4.13: spec/设计文档 BNF 事实同步（setSelection/bind:checked/measure/@default-theme）

**Files:**
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md:94-96`（域 1 BNF：`setSelection`、`measure()` + 字段访问）
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（`bind:checked` 补 BNF、`@default-theme` 补 域 3）
- Test: docs-gate 重跑（domain LOW-2 + compiler LOW + doc MEDIUM-8）

**Interfaces:**
- Consumes: parser 事实（`validate_ref_ops.rs:70,284-330`）
- Produces: 设计文档/规范 BNF 与 parser 事实一致——`setSelectionRange`/`measureWidth` 拼写修正、`bind:checked` 入 BNF、`@default-theme` 入设计文档

- [ ] **Step 1: 写失败测试（文档审计脚本）**——grep 两份文档中的已证伪拼写：

```bash
grep -n "setSelectionRange\|measureWidth\|measure 'width'" docs/superpowers/specs/*.md  # 期望 0
grep -n "bind:checked" docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md  # 期望 ≥1
grep -n "@default-theme" docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md  # 期望 ≥1
```

- [ ] **Step 2: 验证当前失败**——三 grep 均不符合期望

- [ ] **Step 3: 实现**——按 parser 事实改写两文档相关 BNF 行（设计文档域 1：`measure` 单形式 + 结果字段说明；spec：补 `bind:checked`、`@default-theme` 产生式）

- [ ] **Step 4: 验证**——三 grep 符合期望；docs-gate 232 块重跑全绿

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/
git commit -m "docs: sync spec/design BNF with parser facts — setSelection, measure(), bind:checked, @default-theme"
```

---

# Phase 5: IR wave + 结构性重构

> 合批原则（strategy 建议）：一次 fixture churn、一次 golden 重同步、一次 match 臂审查。5.1-5.3 共享 `ir/types.rs` 变更窗口，顺序执行不可穿插其他 IR 变更；5.4-5.9 为独立结构项。

### Task 5.1: LetStmtIR.mut 完整落地（#36 关闭，D-I 第二步）

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs:539-543`（`LetStmtIR` 加 `is_mut: bool`）
- Modify: `crates/omnilang-core/src/ir/lower/stmt.rs:38`（parser `mutable` 标志线程化）
- Modify: `omnilang/src/codegen/react/mod.rs:391`（按 `is_mut` 发射 `let`/`const`）、`omnilang/src/codegen/flutter/stmt.rs:30`（`var`/`final`）、`omnilang/src/codegen/omniview/stmt.rs`（对齐）
- Modify: `scripts/verify-react.exemptions`（TS2588 条目删除——2.1 已删，确认零残留）
- Test: 三目标 fixture + ~64 处构造点更新（含测试 fixture 字面量 ~21 处）

**Interfaces:**
- Consumes: parser `Stmt::VarDecl.mutable`（parser/mod.rs:804 既有）
- Produces: `let mut x` → React `let`、Flutter `var`；不可变 → React `const`、Flutter `final`——一 IR 缺口两目标方向相反的错码消失（compiler CRITICAL 完整版）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn mut_flag_threads_through_all_targets() {
    let src = r#"view V { fn f() { let mut x = 0; x = x + 1; log(x) } }"#;
    let r = compile_react(src);
    assert!(r.contains("let x"), "react must emit let for mut: {}", r);
    let f = compile_flutter(src);
    assert!(f.contains("var x"), "flutter must emit var for mut: {}", f);
    let imm = compile_flutter(r#"view V { fn f() { let y = 1; log(y) } }"#);
    assert!(imm.contains("final y"), "flutter must emit final for immutable: {}", imm);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（react 恒 const、flutter 恒 var）

- [ ] **Step 3: 实现**——① `LetStmtIR` 加字段；② lowering 线程化；③ 三后端发射器按标志分支；④ 全量构造点更新（`cargo build` 报错驱动，~64 处；测试 fixture 字面量 ~21 处机械补 `is_mut: false`）；⑤ 删 TS2588 豁免（若 2.1 未删净）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；`cargo test --workspace` 全绿；verify-react 绿；golden 重同步

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/ crates/omnilang-core/src/parser/ omnilang/src/codegen/ scripts/verify-react.exemptions
git commit -m "feat(ir): LetStmtIR mut flag — const/final/let/var per target (closes #36)"
```

### Task 5.2: inline store 入 IR + React 发射（#37 关闭）

**Files:**
- Modify: `crates/omnilang-core/src/ir/`（store body 承载：inline store → 模块级 store 形态）
- Modify: `omnilang/src/codegen/react/component/store_slices.rs:963-969`（「no slice selectors possible」分支实现）
- Modify: `omnilang/src/codegen/react/component/state.rs:68-70`（信号 → useState 通道）
- Modify: `test/projects/learn-omniview/src/components/InlineStoreDemo.omni`（撤销 2.2 的硬诊断展示——恢复可用）
- Test: `omnilang/src/codegen/react/tests.rs` + Playwright InlineStoreDemo（ts/react H2 完整版）

**Interfaces:**
- Consumes: IR store 统一形态（顶层/内联同一表示，v2 域 5 设计承诺）
- Produces: inline store → 模块级 Zustand store 发射 + 切片订阅——`CartStore.addItem` 可执行、状态可更新（D-A 第二步；仿 Flutter 空 provider 哨兵方案）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn inline_store_emits_working_store() {
    let out = compile_react(r#"view V { store Cart { signal items = []; fn add(x) { items = [...items, x] } } }"#);
    assert!(out.contains("createStore") || out.contains("useCart"), "must emit real store: {}", out);
    assert!(!out.contains("Cart.addItem") || out.contains("useCart"), "bare-name calls must resolve: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（2.2 硬诊断后此测试当前为编译错误）

- [ ] **Step 3: 实现**——lowering 把 inline store 提升为模块级 StoreIR（body 承载）；React 发射完整 store + 切片订阅；2.2 的硬诊断移除（E_CODEGEN 分支删除）；demo 恢复

- [ ] **Step 4: 运行验证通过**——新测试 PASS；Playwright InlineStoreDemo 0 runtime error；`verify-react.sh` 绿（TS2304 相关条目已删）

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/ omnilang/src/codegen/react/ test/projects/learn-omniview/src/components/InlineStoreDemo.omni
git commit -m "feat(react): inline stores lowered to module stores with slice subscriptions (closes #37)"
```

### Task 5.3: 用户 struct 类型发射（#49 最小集）

**Files:**
- Modify: `omnilang/src/codegen/react/`（`struct Todo { ... }` → TS interface/type + 值构造路径）
- Modify: `omnilang/src/codegen/flutter/`（既有类发射核对——若已发射则只补 React）
- Modify: `scripts/verify-react.exemptions`（TS2322 ×10、TS2304 ×3、TS7006 部分、TS2339 部分删除）
- Test: `omnilang/src/codegen/react/tests.rs` + `never[]` 类型修复（ts/react M8 + strategy LOW-1）

**Interfaces:**
- Consumes: IR 类型信息（struct 定义）
- Produces: `Todo`/`ApiResult`/`Rating` 等用户类型 → TS 类型声明 + 构造调用——TS2322/TS2304/TS7006/TS2339 类豁免缩减约 30+ 条（compiler 长期项 #8）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn user_struct_emits_ts_type() {
    let out = compile_react(r#"struct Todo { id: int, text: String, done: bool } view V { }"#);
    assert!(out.contains("type Todo =") || out.contains("interface Todo"), "missing TS type: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（无类型发射——TS2339/TS2322 类豁免存在）

- [ ] **Step 3: 实现**——struct → TS type（字段类型映射：int→number、String→string、[T]→T[]、enum→union）；组件状态/参数/返回类型引用该类型；DataTable `never[]` 修复（类型化状态初始化）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；`verify-react.sh` 全绿且豁免条目下降 ~30；golden 重同步

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/ scripts/verify-react.exemptions
git commit -m "feat(react): emit user struct types — exemption burn-down ~30 entries (closes #49)"
```

### Task 5.4: store-action 参数遮蔽移植 React/Flutter + 回归测试

**Files:**
- Modify: `omnilang/src/codegen/react/mod.rs:256-264` + `expr.rs:913-995`（`param_names` 排除）
- Modify: `omnilang/src/codegen/flutter/expr.rs:752-758`（`StoreActionExprFormatter` 作用域感知）
- Test: `omnilang/src/codegen/react/tests.rs` + `omnilang/src/codegen/flutter/tests.rs`（maintainability HIGH-1）

**Interfaces:**
- Consumes: OmniView `fn_locals` 模式（omniview/store.rs:688-694 既有）
- Produces: action 参数名与 store 信号同名时参数保持裸名——`validate_all(items)` 迭代实参而非 store（三后端一致）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn store_action_param_shadows_signal_react() {
    let src = r#"
        store S { signal items = [1]; fn validate_all(items) { log(items.length) } }
    "#;
    let out = compile_react(src);
    assert!(!out.contains("validate_all(state.items)"), "param rewritten to store: {}", out);
    assert!(out.contains("validate_all(items)"), "param must stay bare: {}", out);
}
// 同款 flutter 测试
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（React/Flutter 均发射 `state.items`）

- [ ] **Step 3: 实现**——action 参数名集合传入重命名 formatter（React `format_expr_ir_tsx_renamed`、Flutter `StoreActionExprFormatter`），命中的 IdentifierExpr 保持裸名

- [ ] **Step 4: 运行验证通过**——两测试 PASS；既有 store fixture 全绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/mod.rs omnilang/src/codegen/react/expr.rs omnilang/src/codegen/flutter/expr.rs omnilang/src/codegen/react/tests.rs omnilang/src/codegen/flutter/tests.rs
git commit -m "fix: port param shadowing to react/flutter store actions — silent wrong-code class closed"
```

### Task 5.5: walker 收敛 + pass 编排移出 lower()

**Files:**
- Modify: `crates/omnilang-core/src/semantic/`（6 个 pass 迁移到 `ir/walk.rs` 回调过滤）
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs:317-383`（pass 循环移出）→ `crates/omnilang-core/src/analyse.rs:359-362`（统一管线）
- Modify: `crates/omnilang-core/src/semantic/mod.rs:1`（header 准确化）
- Test: 全量 `cargo test -p omnilang-core`（maintainability MEDIUM ×2）

**Interfaces:**
- Consumes: `ir/walk.rs`（`walk_expr/walk_stmt/walk_view` 既有，覆盖全部 v2 变体）
- Produces: 单一遍历族（~600 行重复删除）；pass 管线单点声明——新 pass 有规范归宿

- [ ] **Step 1: 写失败测试（行为等价回归）**——为每个迁移的 pass 补「行为不变」断言：同一输入迁移前后诊断集合相等（迁移过程中先写对照测试，迁移完成删除对照保留原测试）

- [ ] **Step 2: 运行验证当前基线**——Run: `cargo test -p omnilang-core`；记录全绿基线

- [ ] **Step 3: 实现**——① validate_ref_ops/validate_array_methods/validate_store_access/validate_component/validate_a11y/validate_semantic_bindings 的遍历改 walk_* + 回调过滤；② react/flutter `collect_*`/`*_identifiers` 族同迁（若时间允许——可拆子 commit）；③ lower() 内 7 pass 移到 analyse.rs 统一管线；④ semantic/mod.rs header 重写

- [ ] **Step 4: 运行验证通过**——全量测试与基线一致；`cargo clippy` 无新增

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/semantic/ crates/omnilang-core/src/ir/
git commit -m "refactor: converge ~9 IR traversals onto shared walker; move pass orchestration out of lower()"
```

### Task 5.6: TS2722 事件处理器 `?.()` 发射

**Files:**
- Modify: `omnilang/src/codegen/react/component/function.rs` 或事件发射点（`onX={f}` → `onX={(e) => f?.()}` 或 deps 分析后直接调用）
- Modify: `scripts/verify-react.exemptions`（TS2722 ×23 删除）
- Test: `omnilang/src/codegen/react/tests.rs` + Playwright 事件交互（strategy 记账 + ts/react）

**Interfaces:**
- Consumes: 事件处理器引用分析（可为 undefined——ref 回调未接线场景）
- Produces: `?.()` 安全调用发射——23 条 TS2722 清零（纯类型错误清除，零行为风险）

- [ ] **Step 1: 写失败测试**

```rust
#[test]
fn event_handler_call_sites_use_optional_call() {
    let out = compile_react(r#"view V { signal f = null; <button @onClick={f}/> }"#);
    assert!(out.contains("?.()"), "must guard possibly-undefined handler: {}", out);
}
```

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（裸 `f()` 发射——TS2722 豁免存在）

- [ ] **Step 3: 实现**——事件处理器调用点发射 `f?.()`（仅当类型分析表明可能 undefined；恒非空时保持 `f()` 避免噪音）

- [ ] **Step 4: 运行验证通过**——新测试 PASS；`verify-react.sh` 绿（TS2722 条目删除）

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/react/ scripts/verify-react.exemptions
git commit -m "fix(react): optional-call event handlers — TS2722 class cleared"
```

### Task 5.7: 超大文件拆分——flutter/design.rs + resolve.rs 补测

**Files:**
- Modify: `omnilang/src/codegen/flutter/design.rs`（3548 行 → 按域拆：tokens/classes-variants/theme-wiring）
- Modify: `crates/omnilang-core/src/semantic/resolve.rs`（1090 行零测试 → 拆 + 补类型推断测试）
- Test: 各拆分模块测试（maintainability MEDIUM）

**Interfaces:**
- Consumes: 既有内部 section 结构（design.rs 已有分节横幅）
- Produces: 文件 <800 行/模块；resolve.rs 类型推断有针对性测试（`infer_token_expr_type` 等）

- [ ] **Step 1: 写失败测试（resolve.rs 补测先行）**——`infer_token_expr_type` 的已知分支逐项测试（String/i32/数组/模板/方法调用返回）

- [ ] **Step 2: 运行验证**——新测试覆盖 resolve.rs 关键路径；记录零测试基线

- [ ] **Step 3: 实现**——design.rs 按域拆 3 文件（pub 接口保持——re-export 兼容既有调用）；resolve.rs 拆分 + 测试并入

- [ ] **Step 4: 运行验证通过**——全量 `cargo test --workspace` 绿；flutter 门禁绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/flutter/ crates/omnilang-core/src/semantic/resolve.rs
git commit -m "refactor: split flutter/design.rs by domain; add resolve.rs type-inference tests"
```

### Task 5.8: 工具链硬化——clippy -D warnings + escape_literal_str 单遍 + FxHasher

**Files:**
- Modify: `crates/omnilang-core/src/codegen/json.rs:9-16`（单遍 escape）
- Modify: `crates/omnilang-core/src/analyse.rs:58-62`（`DefaultHasher` → `FxHasher` 或等价）
- Modify: CI 或 clippy 配置（-D warnings）
- Test: 既有测试 + clippy 全量（rust LOW）

**Interfaces:**
- Consumes: 现有实现
- Produces: clippy 零警告强制；escape 单遍（6 次替换 → 1 次）；hash 语义不变（存储型 hash 安全）

- [ ] **Step 1: 写失败测试**——`cargo clippy --workspace --all-targets` 当前 ~75 警告 → 目标 0

- [ ] **Step 2: 运行验证当前状态**——记录警告分类（24 ts-rs 属性、16 collapsible if-let、11 冗余闭包）

- [ ] **Step 3: 实现**——escape 单遍 `String::with_capacity`；hash 换 `FxHasher`；clippy 警告逐类修（ts-rs 属性用 `#[allow]` 带理由或配置项——按类别裁决）；CI 加 `-D warnings`

- [ ] **Step 4: 运行验证通过**——clippy 0 警告；全量测试绿

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ .github/workflows/ci.yml
git commit -m "chore: clippy -D warnings; single-pass literal escape; FxHasher"
```

### Task 5.9: fuzz 全后端——codegen_never_panics 覆盖 10 目标

**Files:**
- Modify: `omnilang/tests/property_tests.rs:110-115`（目标列表扩展）
- Test: 同上（rust 长期项）

**Interfaces:**
- Consumes: `compile_for_target`（现有，全目标可用）
- Produces: Dioxus/Vue/Python/JS/Rust/Dart 与 P0 三目标同等 panic-fuzz

- [ ] **Step 1: 写失败测试**——目标列表补 `["dioxus", "vue", "python", "js", "rust", "dart"]`（all-backends feature 下）

- [ ] **Step 2: 运行验证失败**——Expected: FAIL（非 P0 后端 panic 或 feature 门控不足）

- [ ] **Step 3: 实现**——按 all-backends feature 门控目标列表；修复 fuzz 暴露的 panic（如有）

- [ ] **Step 4: 运行验证通过**——`PROPTEST_CASES=50 cargo test -p omnilang --features all-backends --test property_tests` 全绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/property_tests.rs
git commit -m "test: extend codegen fuzz to all 10 backends"
```

---

# Phase 6: 文档与治理

> 目标：docs-gate 从「只验 parse」升级为「验语义」；双权威收敛；v2 恢复功能被文档埋没的问题修复。顺序：最高用户伤害（@scroll/@drag 误述）→ 权威结构 → 门禁升级 → 速查表。

### Task 6.1: @scroll/@drag 恢复表述修复 + 子参数示例（doc HIGH-2）

**Files:**
- Modify: `docs/omniview/11-motion-animation.md:125,167`、`docs/omniview/06-templates.md:207`、`docs/omniview/15-grammar-view.md:1091-1095,1009-1014`
- Test: docs-gate 重跑 + grep 审计脚本

**Interfaces:**
- Consumes: parser 事实（`parse_trigger_sub_params` 既有：`@scroll start:/end:/scrub:`、`@drag-* constraints:/elastic:/threshold:/velocity_max:`）
- Produces: 三处「已移除/待重新实现」→「v2 恢复（Task 5.1）」；11-motion 触发器章节补子参数示例与语义表；whitelist 17→19

- [ ] **Step 1: 写失败测试（审计脚本）**

```bash
grep -n "待重新实现\|已移除.*ADR-053\|pending reimplementation" docs/omniview/*.md  # 期望 0（scroll/drag 相关）
grep -n "17-property\|17 属性" docs/omniview/15-grammar-view.md  # 期望 0
```

- [ ] **Step 2: 验证当前失败**——两 grep 均有命中

- [ ] **Step 3: 实现**——修正三处误述 + 补子参数示例（素材取 v1 spec §3.4 控制维度表——准确）+ whitelist 19 + @scroll/@drag 语义表

- [ ] **Step 4: 验证**——两 grep 干净；docs-gate 232 块全绿

- [ ] **Step 5: Commit**

```bash
git add docs/omniview/
git commit -m "docs: restore @scroll/@drag control-dimension teaching — v2 Task 5.1 recovery"
```

### Task 6.2: 双权威收敛——v1 spec Superseded（doc HIGH-3）

**Files:**
- Modify: `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`（标题/状态行 + §3.6/§10.3/§12/§13 修正或标注）
- Modify: `docs/omniview/11-motion-animation.md:110`（引用指针更新）
- Test: docs-gate 重跑 + 权威引用 grep

**Interfaces:**
- Consumes: v2 设计文档（活跃权威）
- Produces: v1 spec 明确「Superseded（2026-08-03 起由 v2 设计文档接管）」；已证伪事实（steps() error、内联动画体未实现、§12 权威关系）修正或删除；11-motion 引用指向 v2 设计文档

- [ ] **Step 1: 写失败测试（审计脚本）**

```bash
grep -n "steps()" docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md | head -5  # 期望 0 或标注
grep -n "2026-08-02-semantic-syntax-v2-design" docs/omniview/11-motion-animation.md  # 期望 ≥1
```

- [ ] **Step 2: 验证当前失败**——steps() error 断言存在；11-motion 未引用 v2 设计文档

- [ ] **Step 3: 实现**——v1 spec 头部改「Superseded」+ 日期；§3.6（steps → warning + Curves.linear 回退）、§13（内联动画体已实现——与同文件 §3.7 一致化）、§12（权威关系声明删除）修正；11-motion:110 指向 v2 设计文档

- [ ] **Step 4: 验证**——审计脚本干净；docs-gate 全绿

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md docs/omniview/11-motion-animation.md
git commit -m "docs: mark v1 spec Superseded — single authoritative BNF in v2 design doc"
```

### Task 6.3: 15-grammar 事实修复批次（doc HIGH-1 + LOW-10 组）

**Files:**
- Modify: `docs/omniview/15-grammar-view.md`（:1161 @themes 分号、:121 store_inline fn 成员、:401-402 type_constraint、:415 RefBinding value、:37/273 void 元素表、:1009 whitelist）
- Modify: `docs/omniview/14-grammar-lang.md:587`（timeline_def）、`docs/omniview/08-stores.md:137`（@for key 示例）
- Test: docs-gate + grep 审计

**Interfaces:**
- Consumes: parser 事实（design.rs:181-194 @themes 花括号、store.rs:30、view/mod.rs:581 param）
- Produces: grammar 文档与 parser 逐项一致——@themes 花括号形式、fn-in-store 错误、D2/D4 残留删除、void 元素表统一

- [ ] **Step 1: 写失败测试（审计脚本）**

```bash
grep -n "@themes light, dark;" docs/omniview/15-grammar-view.md  # 期望 0
grep -n "type_constraint" docs/omniview/15-grammar-view.md  # 期望 0
grep -n "timeline_def" docs/omniview/14-grammar-lang.md  # 期望 0
```

- [ ] **Step 2: 验证当前失败**——均有命中

- [ ] **Step 3: 实现**——逐处修正（@themes 花括号 + 注记、store_inline 去 fn 成员 + 错误说明、type_constraint/RefBinding value 删除、void 元素表以 parser 为准（含 param）、whitelist 19、timeline_def 删除、08-stores 示例加 `key={item.id}`）

- [ ] **Step 4: 验证**——审计脚本干净；docs-gate 全绿

- [ ] **Step 5: Commit**

```bash
git add docs/omniview/
git commit -m "docs: grammar-view parser-fact batch — @themes form, dead tokens, void elements, for-key example"
```

### Task 6.4: 跨目标行为表修正——fade/wait Flutter 列 + steps 统一（doc MEDIUM-4/5）

**Files:**
- Modify: `docs/omniview/06-templates.md:261`（fade = AnimatedSwitcher、wait = AnimatedSwitcher + FadeTransition）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md` 域 6 映射表 wait 行
- Modify: steps() 三处统一（11-motion:104、v1 spec §3.6/§10.3——随 6.2、设计文档域 2）
- Test: docs-gate + codegen 事实对照（`flutter/view.rs:1959-1975`、`flutter/motion.rs:576-592`）

**Interfaces:**
- Consumes: codegen 实现事实
- Produces: 唯一一张跨目标行为表与实现一致——两权威源不再互相颠倒

- [ ] **Step 1: 写失败测试（审计脚本）**

```bash
grep -n "无过渡效果" docs/omniview/06-templates.md  # 期望 0
grep -n "error" docs/omniview/11-motion-animation.md | grep -i steps  # 期望 0
```

- [ ] **Step 2: 验证当前失败**——均有命中

- [ ] **Step 3: 实现**——按 flutter/view.rs 事实重写 fade/wait 表；设计文档域 6 wait 行改「AnimatedSwitcher + 显式 FadeTransition」；steps 三处统一「warning + Curves.linear 回退」

- [ ] **Step 4: 验证**——审计干净；docs-gate 全绿

- [ ] **Step 5: Commit**

```bash
git add docs/omniview/06-templates.md docs/omniview/11-motion-animation.md docs/superpowers/specs/
git commit -m "docs: fix cross-target behavior table — fade/wait flutter reality, steps unified as warning"
```

### Task 6.5: 陈旧教学修正——nominal + mode 魔法名 + 索引（doc MEDIUM-6 + domain MEDIUM-4 + LOW-1）

**Files:**
- Modify: `docs/omniview/02-types-and-values.md:118-125`（nominal 降级）
- Modify: `docs/omniview/10-design-system.md:103-116`（mode 魔法名 → @theme 信号驱动）
- Modify: `docs/design/DESIGN_DOCS_INDEX.md`（重新生成）
- Test: docs-gate + grep

**Interfaces:**
- Consumes: parser/语义事实（`is_nominal` 仅解析不强制；`omniview/mod.rs:388-415` @theme 接线）
- Produces: 02 与 14 一致（「语法已解析，类型强制尚未实现」）；10-design 内部自洽（@theme 唯一接线机制）

- [ ] **Step 1: 写失败测试（审计脚本）**

```bash
grep -n "不能互相赋值" docs/omniview/02-types-and-values.md  # 期望 0
grep -n "自动同步到" docs/omniview/10-design-system.md  # 期望 0
```

- [ ] **Step 2: 验证当前失败**——均有命中

- [ ] **Step 3: 实现**——三处修正 + DESIGN_DOCS_INDEX 对实际文件重新生成

- [ ] **Step 4: 验证**——审计干净；docs-gate 全绿

- [ ] **Step 5: Commit**

```bash
git add docs/
git commit -m "docs: nominal caveat, @theme-only wiring, regenerate design docs index"
```

### Task 6.6: docs-gate 语义化——删除语法族诊断白名单 + notes 计数（doc MEDIUM-7）

**Files:**
- Modify: `scripts/verify-docs.py:70-94,217-237`（`is_parse_error` 扩展 + fragment 语义诊断白名单 + notes 打印）
- Modify: `.github/workflows/ci.yml:312-327`（CI 输出 notes 计数）
- Test: docs-gate 重跑（doc MEDIUM-7 + ai MEDIUM 同源）

**Interfaces:**
- Consumes: 语义诊断码（E0006 VALUE_BINDING_REMOVED、E_PAUSED、setValue、capturePointer 参数、@slot 类型约束、fn-in-store、@themes 分号、E_CLOSURE_ARROW——4.1/4.2/4.4 后完整）
- Produces: fragment 中重现删除语法族 → 门禁失败；CI 打印 notes 计数；平台映射表行（「React: / Flutter:」开头）纳入抽取

- [ ] **Step 1: 写失败测试**——verify-docs.py 新增「删除语法族语义诊断失败清单」：

```python
# scripts/verify-docs.py 追加
DELETED_SYNTAX_CODES = {"E0006", "E_PAUSED_REMOVED", "E_SLOT_REMOVED", "E_CLOSURE_ARROW", ...}
# fragment 诊断中命中 DELETED_SYNTAX_CODES → FAIL（当前仅 PARSE_ERROR_PREFIXES 失败）
```

- [ ] **Step 2: 验证当前失败**——构造含 `value={sig}` 的 fragment → 当前记为 note 通过（ai 专家实测过）

- [ ] **Step 3: 实现**——白名单失败化 + CI 打印 notes 计数（非 verbose 也可见）+ 映射表行抽取 + 现有 147 notes 审计（当前内容干净——白名单化后应零新增失败）

- [ ] **Step 4: 验证**——docs-gate 232 块全绿（含白名单）；注入 `value={sig}` 测试块确认失败

- [ ] **Step 5: Commit**

```bash
git add scripts/verify-docs.py .github/workflows/ci.yml
git commit -m "ci: docs-gate semantic layer — deleted-syntax families fail; notes count surfaced"
```

### Task 6.7: 设计文档 §7 承诺处置 + 状态注记（compiler LOW + domain LOW）

**Files:**
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（§7 #28/#33 状态、状态行注记、域 1 BNF 已完成于 4.13）
- Test: docs-gate + deferrals 台账同步

**Interfaces:**
- Consumes: deferrals #28/#33 实际状态（Map.set verbatim、嵌套 #root 双包装）
- Produces: 设计文档 §7 承诺如实标注（未落地 → 移至 deferral 或声明移出范围）；状态行不再过度宣称

- [ ] **Step 1: 写失败测试（审计脚本）**

```bash
grep -n "Map.set" docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md  # 核对 §7 表述
grep -n "Implemented" docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md | head -3
```

- [ ] **Step 2: 验证当前状态**——§7 两项承诺标完成而 deferrals 复核未关闭

- [ ] **Step 3: 实现**——§7 标注实际状态（#28/#33 移出本设计范围或引用 deferral）；状态行加注「本计划 Phase 6 起 v3 修复时代」；deferrals 台账同步

- [ ] **Step 4: 验证**——审计输出与台账一致；docs-gate 绿

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md .architecture/deferrals.md
git commit -m "docs: honest §7 commitments status — #28/#33 disposition; v3 era annotation"
```

### Task 6.8: 00-cheatsheet——每域一例速查表（ai MEDIUM）

**Files:**
- Create: `docs/omniview/00-cheatsheet.md`
- Modify: `docs/omniview/README.md`（导航补条目）
- Test: docs-gate（cheatsheet 全部代码块为完整可编译块）

**Interfaces:**
- Consumes: 10 域设计文档 + 事件 canonical 表（4.2 导出）+ 错误码表
- Produces: 10 域 × 1 最小可编译示例 + 事件名表 + 删除语法表（含硬诊断消息）+ 错误码速查——LLM few-shot 与人工速查的最小上下文

- [ ] **Step 1: 写失败测试（内容即测试）**——撰写 cheatsheet：每域一例（完整 `view { }` 包裹）、事件表（canonical + 别名 + 已删除）、删除语法表（verbatim 诊断消息）、错误码表（E_* 前缀 + 含义 + 修复）

- [ ] **Step 2: 验证**——docs-gate 对 cheatsheet 全部块 parse-check 通过；每块 `omnic build` 可编译（grep 断言 10 域全覆盖）

- [ ] **Step 3: 实现**——README 导航补条目；LLM 可读性自查（每例 5-15 行）

- [ ] **Step 4: 验证通过**——docs-gate 全绿含新文档

- [ ] **Step 5: Commit**

```bash
git add docs/omniview/00-cheatsheet.md docs/omniview/README.md
git commit -m "docs: add 00-cheatsheet — one compilable example per domain + event/error tables"
```

---

# Phase 7: 设计层制度（P1-P8 固化）

> 目标：把「事后的可检验原则」变成「事前的机制保证」——词汇表审查、语义集分离、floor 机器化、语法生命周期、行为基准。本阶段产出以制度文档 + 机器断言为主，非语法变更。

### Task 7.1: 词汇表审查入设计模板 + a11y 词汇表文档化（设计 P1）

**Files:**
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（每域模板加「词汇来源」字段）
- Create: `docs/superpowers/specs/vocabulary-registry.md`（词汇登记册：语义词汇 ↔ 来源域 ↔ 目标映射）
- Test: 模板审计（新域设计检查清单）

**Interfaces:**
- Consumes: 4.2 事件白名单、4.3 a11y 词汇表、19 属性白名单、RefOp 10 ops
- Produces: 每域设计时强制回答「词汇的语义域是什么」（WAI-ARIA/Flutter 旗标/CSS/DOM）——P1 复发机制关闭

- [ ] **Step 1: 写失败测试（模板检查清单）**——词汇登记册条目必须含：语义词汇 / 来源域 / 目标映射 / 排除词汇（平台旗标名）

- [ ] **Step 2: 实现**——设计文档每域模板加「词汇来源」字段；登记册建档（5 个封闭词汇域：事件/a11y role/motion 属性/RefOp/design token 类型）；「禁止借词」规则（Flutter 旗标/React 合成事件名不得直接成为语义词汇——先查登记册）

- [ ] **Step 3: 验证**——登记册与 4.2/4.3 白名单一致（交叉断言测试或文档审计脚本）

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/
git commit -m "docs: vocabulary-registry — per-domain lexical-source audit in design template"
```

### Task 7.2: 语义集 vs 已实现集分离（设计 P3）

**Files:**
- Create: `docs/superpowers/specs/semantic-universe.md`（语义全集：前端功能全集，与实现无关）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（白名单/floor 标注「语义集定义」+ 实现状态单独列）
- Test: 审计脚本（语义集条目 → 实现状态矩阵）

**Interfaces:**
- Consumes: 19 属性白名单、ArrayMethod、RefOp、事件表
- Produces: 白名单按「前端功能有哪些」定义；「当前支持哪些」单独跟踪——语法表面不再随实现进度漂移（P3 机制）

- [ ] **Step 1: 写失败测试（审计脚本）**——语义全集每条目必须映射到实现状态（支持/部分/未支持 + 诊断行为）

- [ ] **Step 2: 实现**——语义全集建档（可动画属性全集按 CSS 动画语义域、数组操作全集、事件全集）；设计文档白名单节加「语义集 vs 实现状态」双列表；新增语法必须先在语义全集登记

- [ ] **Step 3: 验证**——审计脚本干净；docs-gate 绿

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/
git commit -m "docs: semantic-universe — split semantic set from implementation status"
```

### Task 7.3: floor 机器化第一版——floor 表 → fixture 矩阵映射（设计 P5）

**Files:**
- Create: `omnilang/tests/floor_matrix.rs`（floor 条目 → 参数化 fixture + 断言）
- Modify: `docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md`（floor 表每行标注矩阵测试名）
- Test: 同上

**Interfaces:**
- Consumes: 参数化 fixture 基建（既有 control_flow fixture 先例）、行为断言（widget test/Playwright——3.2 基建）
- Produces: 24-item floor 每项可机器验证——「floor 内行为对等」从声明变断言（P5 机制；第一版覆盖 P0 三目标 + 旗舰 demo 行为）

- [ ] **Step 1: 写失败测试**——floor 表解析 + 覆盖检查：每行必须映射到 ≥1 个 fixture 测试（编译级）或行为测试（行为级，标注 future）

```rust
#[test]
fn every_floor_item_has_machine_verification() {
    let floor = floor_table(); // 从设计文档或硬编码清单解析
    for item in floor {
        assert!(fixture_for(item).is_some(), "floor item {} unverified", item.name);
    }
}
```

- [ ] **Step 2: 验证当前失败**——多数 floor 项无对应 fixture 映射（仅 @if/@for 有）

- [ ] **Step 3: 实现**——floor 24 项逐项映射（编译级先行：参数化 fixture 已覆盖的标注；缺失的按类补 fixture——复用 4.x 阶段的 fixture 成果）；行为级标注为「需 widget test/Playwright」并列入 3.2 基建的扩展队列

- [ ] **Step 4: 验证通过**——覆盖检查全绿；`cargo test -p omnilang --test floor_matrix` 绿

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/floor_matrix.rs docs/superpowers/specs/2026-08-02-semantic-syntax-v2-design.md
git commit -m "test: floor-matrix — 24-item floor machine-verifiable (compile tier; behavior tier tracked)"
```

### Task 7.4: 语法生命周期制度（设计 P6）

**Files:**
- Create: `docs/superpowers/specs/syntax-lifecycle.md`（新语法提案 → spike → 语料试点 → 收口评审 → BNF 批准）
- Modify: `.architecture/deferrals.md`（模板引用）
- Test: 审计脚本（BNF 新增语法必须可追溯提案记录）

**Interfaces:**
- Consumes: v2 设计-验证循环先例（每域 spike → 门禁 → 浏览器验证）
- Produces: 新语法进 BNF 前必须：demo 原型 + 预期使用基准 + 收口评审记录——filter/map/count 0 消费者类不再可能（P6 机制）

- [ ] **Step 1: 写失败测试（审计脚本）**——BNF 变更记录与提案记录交叉：`docs/superpowers/specs/*lifecycle*` 中每个已批准语法有「提案 → 试点 → 批准」三段记录

- [ ] **Step 2: 实现**——生命周期文档（四阶段模板 + 批准标准：≥1 demo 使用 + 三目标门禁 + 使用基准预估）；deferrals 引用；v2 现有语法按「已批准」补登记（豁免——一次性补档）

- [ ] **Step 3: 验证**——审计脚本干净；文档评审通过

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/syntax-lifecycle.md .architecture/deferrals.md
git commit -m "docs: syntax lifecycle — proposal→spike→corpus→approval before BNF"
```

### Task 7.5: LLM eval 语料 + 生成代码源注释 + 再生成契约（设计 P7 部分 + ai MEDIUM）

**Files:**
- Create: `omnilang/tests/llm_error_corpus.rs`（~30 fixtures：错误事件名/错误方法名/删除语法/缺 view{}/错误闭包形式）
- Modify: `omnilang/src/codegen/react/component/mod.rs:30`（`// [omni] file:line` 头注释）、flutter 同
- Create: `docs/omniview/regeneration-contract.md`
- Test: 语料断言（每 fixture ≥1 诊断 + 正确位置 + 可行动提示）

**Interfaces:**
- Consumes: 4.2/4.10/4.11 的诊断能力（位置精确 + suggestion 填充）
- Produces: LLM 错误模式回归语料（CI 断言）；生成代码可追踪源位置；再生成覆盖契约文档（改 .omni 不改生成代码）

- [ ] **Step 1: 写失败测试**——~30 fixtures 每例断言「诊断存在 + 位置正确 + 提示可行动」：

```rust
#[test]
fn llm_error_corpus_diagnoses() {
    for case in LLM_ERROR_CASES {
        let (_, diags) = compile_with_diags(case.src, Target::React);
        assert!(!diags.is_empty(), "case {} must diagnose", case.name);
        assert!(diags.iter().any(|d| d.line >= case.expected_line), "case {} wrong position", case.name);
    }
}
```

- [ ] **Step 2: 验证当前失败**——错误事件名/方法名类零诊断（4.2/4.10 已修——本任务验证闭环）；位置断言部分失败（4.11 后应已修——确认）

- [ ] **Step 3: 实现**——语料建档（6 类 × 5 例）；React/Flutter 生成头注释 `// [omni] <file>:<line>`（IR source_map 既有数据）；再生成契约文档（编辑 .omni 而非生成代码 + 覆盖语义声明）

- [ ] **Step 4: 验证通过**——语料全绿；生成产物含源注释；文档入 docs 导航

- [ ] **Step 5: Commit**

```bash
git add omnilang/tests/llm_error_corpus.rs omnilang/src/codegen/react/ omnilang/src/codegen/flutter/ docs/omniview/regeneration-contract.md
git commit -m "test: LLM error corpus + generated-code source markers + regeneration contract"
```

### Task 7.6: 行为对等基准——P0 行为断言扩展（设计 P5/P7 部分）

**Files:**
- Modify: `test/projects/learn-omniview/build/react-test-app/tests/`（Playwright 行为断言扩展：主题切换后 data-theme 实际变化、bind:value 回写、@for keyed 重排）
- Modify: `omnilang/tests/flutter_widget_tests/`（3.2 基建扩展：同场景 Flutter 侧）
- Test: 同上

**Interfaces:**
- Consumes: 2.x/3.x 行为修复成果、3.2 widget test 基建
- Produces: 「同一 .omni 源 → 三目标行为一致」的最小行为对等断言集——floor 对等的运行时证据（P5 行为层；P7 产品假说数据起点）

- [ ] **Step 1: 写失败测试**——5 个行为场景 × 2 目标（React Playwright + Flutter widget test）：① 主题切换（点卡片 → data-theme/ThemeMode 变化）② bind:value 输入回写 ③ @for 排序重排 key 稳定 ④ @if 切换保留状态（3.9 后）⑤ motion @enter 播放

- [ ] **Step 2: 验证当前状态**——逐个运行记录当前行为（2.x/3.x 后应已基本可用；失败项即剩余缺口）

- [ ] **Step 3: 实现**——补断言补缺口（失败项在本任务内修复或登记为 blocker——每项四选一处置）；断言集入 CI（React 套件扩展 + flutter widget job）

- [ ] **Step 4: 验证通过**——5 场景 × 2 目标全绿；CI 接入

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/build/react-test-app/tests/ omnilang/tests/flutter_widget_tests/ .github/workflows/ci.yml
git commit -m "test: behavioral parity baseline — 5 scenarios × react/flutter in CI"
```

---

# 终验（Phase 8 验收）

### Task 8.1: 全量终验 + 评审闭环

**Files:**
- Modify: `.architecture/reviews/feature-omniview-semantic-syntax-compilability-v2.md`（处置状态标注）或新建 v3 对齐评审
- Modify: `.architecture/deferrals.md`（#36/#37/#49/#50/#55/#60 关闭 + 新增候选注册）

**验收标准（对照评审发现逐项）**：

- [ ] **注入面**：motion/design/unquoted-handler 三 payload 族全后端回归绿；覆盖地图测试全绿；`[alert(1)]` 类载荷被拒绝（1.1-1.6）
- [ ] **React 行为**：Playwright 全量 0 runtime error（19 fixme 全部 un-fixme 且绿）；豁免 287 → 目标 <100（2.x + 5.x 燃尽）
- [ ] **Flutter 行为**：widget test 冒烟全绿（TodoStore.add() 可执行）；bind:value 表单/主题选择器/@in-view 可用；warnings 1004 → 目标 <300（3.x + 4.12）
- [ ] **静默清零**：「parse-and-ignore 零」可验证——states:/on:/aria-*/未知事件名/block-expr/@fallback/@error/多主题/motion-in-@for 全部产生诊断（4.x 全量）
- [ ] **文档**：docs-gate 含语义白名单全绿；双权威收敛；@scroll/@drag 教学恢复（6.x 全量）
- [ ] **结构**：param 遮蔽三后端一致；walker 收敛；lower() 纯 lowering；clippy -D warnings 0（5.4-5.8）
- [ ] **设计层**：词汇登记册/语义全集/生命周期文档落地；floor 矩阵 ≥24 项机器验证（7.x 全量）
- [ ] **治理**：豁免台账 100% 可追溯；deferrals #36-#60 处置完成（关闭/触发条件更新）；ADR 决策记录（未知事件名级别 D-B、array methods 收口结果 D-E、inline store IR 形态）→ 新建 ADR-057 或更新 ADR-056 注记

- [ ] **Commit**: 终验结果记录 + 评审文件状态更新 + 会话记忆

```bash
git add .architecture/reviews/ .architecture/deferrals.md docs/superpowers/specs/
git commit -m "docs(review): v3 fix-round completion — acceptance matrix vs v2 review findings"
```

---

## 阶段依赖与顺序

```
Phase 0（门禁）──► Phase 1（安全）──► Phase 2（React）──► Phase 3（Flutter）
                                                          │
Phase 4（静默清零）◄──────────────────────────────────────┘（可与 3.x 并行）
          │
          ▼
Phase 5（IR wave：5.1→5.2→5.3 严格顺序；5.4-5.9 可并行）
          │
          ▼
Phase 6（文档）──► Phase 7（设计层制度）──► Phase 8（终验）
```

- **关键路径**: 0 → 1 → 2/3 → 5.1-5.3 → 终验
- **可并行**: 4.x 与 2.x/3.x（不同模块）；6.x 与 5.x 尾部；7.x 全程可后置
- **硬前置**: 1.1 先于 1.2-1.4（语料先行）；3.1/3.2 先于 3.3-3.10（行为可测）；5.1 先于 5.2/5.3（同一 ir/types.rs 窗口）；4.2/4.10/4.11 先于 7.5（诊断能力依赖）

## 计划自审记录

- **覆盖对照**（13 专家发现 → 任务）：注入三连→1.2/1.3/1.4；Flutter TypeError→3.1；BUG-A/BUG-C→2.1/2.2+5.1/5.2；states:/on:→4.1；bind:value→3.3；in-view/focus→3.4；textarea/disabled→3.5；主题选择器+multi-theme→3.6；未知事件名→4.2；门禁度量→0.4/0.5；#60→0.1；all-backends→0.2；缓存 parse 洞→0.3；尾表达式→2.3；a11y role→4.3；aria-*→4.4；文档漂移→6.1-6.5+4.13；该删未删→4.5/4.6/4.7；结构性债→5.4-5.9；性能批次→2.6-2.10+3.8；AI 工作流→7.5+6.8；诊断位置→4.11；unused-import→4.12；C-FFI/JSON 键→1.6；block-expr/ArrayMethod→4.8；motion-in-@for→4.9；方法名→4.10
- **设计层 8 问题 → 任务**：P1 词汇纯度→4.2/4.3/4.4+7.1；P2 单名原则→4.6+7.4；P3 语义集分离→7.2；P4 逻辑语义建模→5.1/5.4/5.5（BindingRef 完整版标注为 5.5 后长期项，触发条件：下次 IR 变更时评估）；P5 floor 机器化→7.3+7.6；P6 语法生命周期→7.4；P7 产品假说→7.5/7.6；P8 语义浓度分级验证→7.3（floor 分级标注）
- **LOW 项顺带处置标注**（随主任务触碰时一并处理）：systems LOW-2 stale 注释（generated_code_validation.rs:360-363,672-675）→ 随 0.5；systems LOW-3 resolve_styles 成功语义 → 随 0.5（warnings 通道统一时裁决）；systems LOW-4 覆盖率 70%→80% → 随 0.2；compiler LOW a11y alt 非 image/hidden 可聚焦 → 随 4.4；domain LOW-3 a11y label 动态化 → defer（触发条件：真实 app 出现动态 label 需求）；pragmatic L3 scroll-x 零 demo → 保留（对称镜像，7.4 生命周期试点时补 demo 行）；maintainability LOW OmniView 双语句发射器 → 随 5.5（pass 编排统一时裁决）；maintainability LOW 测试残留目录 → 随 5.7；systems HIGH-3 evals continue-on-error → 随 0.2；systems MEDIUM-1 warnings 网关两形状 → 随 0.5（长期）；ts/react risks fixme 陈旧假设 → 随 2.1（un-fixme 时更新断言）
- **占位符扫描**：无 TBD/TODO 占位；所有代码步骤含具体实现或精确文件行号
- **跨任务接口一致性**：`escape_literal_str`（1.2/1.6/5.8 共用）、`E_CODEGEN`（2.2/4.8 同码）、`source_span` 线程化（4.11 产出 → 7.5 消费）、`warnings 基线`（0.5 建立 → 3.3/3.5/4.12 更新）、`ElementIR.source_span`（IR 既有）、`Diagnostic.suggestion`（4.11 填充 → 7.5 断言）、exemptions 格式（0.4 加计数列 → 0.6 重连 → 2.1/2.2/5.1/5.3/5.6 删条目）、`verify-docs.py` 消息匹配（4.11 同步 suggestion 时 6.6 检查）
