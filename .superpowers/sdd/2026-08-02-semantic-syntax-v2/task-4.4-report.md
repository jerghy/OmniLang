# Task 4.4 Report — design 诊断表实现 + @native 删除 + React runtime 命名隔离

**Status: DONE**

## 1. What was done

### 1.1 Flutter design 诊断：spec §10.3 四行兑现

`omnilang/src/codegen/flutter/design.rs`：

- **ZIndex token → warning + 常量跳过**：`emit_tokens_class` 对 `TokenTypeIR::ZIndex` 发一条 warning（设计文档原文「Flutter z 序由 widget 顺序决定」），不再生成无意义的 `static const int zModal = 100;`（base 与 theme 覆盖一并跳过，每次 token 一条）。`token_static_expr`/`token_const_annotation` 的 ZIndex 分支改为 `None`，死函数 `css_zindex_to_dart` 删除；`token_const_emitted` 因此返回 false，`var(--zModal)` 引用不会产生悬空标识符（引用点走既有的「不可映射值」skip warning）。
  - 消息：`[flutter][design] token 'zModal' of design 'App': Flutter z 序由 widget 顺序决定 — zIndex has no Flutter layout effect; no Dart constant emitted (skipped)`
- **`display: grid` → error**、**`float` → error**、**`cursor` → warning**：新函数 `collect_layout_diagnostics` 按 design 扫描类（含 `&:hover` 等伪状态）、变体 base 与分支，统一四行检查：
  - `display` 值恰为 `grid` → error 「Flutter 不支持 CSS grid」（spec §10.3 原文；`display: flex` 等不报——spec 只列 grid，有测试锁定）
  - `float`（任意值）→ error 「Flutter widget 布局无 float 概念」
  - `cursor`（任意值）→ warning 「cursor 在 Flutter 中无效果」
  - grid/float 走 codegen error 通道：`gen_design_files` 签名扩为 `(Vec<CompiledFile>, Vec<String>, Vec<Diagnostic>)`，错误用 `DiagnosticKind::ECodegen` + `Severity::Error`，锚定 design 块 `source_span`（line/col 回退 0,0）+ `source_file`（复用 flutter/ref.rs 指针捕获错误的模式）；cursor/zIndex 走 T4.3 warnings 字符串通道。`flutter/mod.rs` 三通道并入 `CompileOutput`。
- 14 处测试调用点改 3 元解构；`emits_tokens_class_with_typed_constants` 的 `static const int zModal` 断言翻转为 warning + 不生成断言。

### 1.2 `@native` 死 token 删除

- `crates/omnilang-core/src/lexer/keywords.rs:81` 删除 `"@native" => ...`；`token.rs:72` 删除 `AtNative` 变体；`parser/view/style.rs` 两处（`is_text_token`、`token_text`）同步删除；`lexer/mod.rs` 的 `test_at_native` 替换为 `test_at_native_removed`（`@native` → `Ident("@native")` 回退形状）。
- 全管线验证：`@native("dart") { ... }` 现在产生干净解析错误 `Unexpected token at top level: Ident("@native")`（新测试 `native_escape_hatch_is_rejected` 锁定；brief 里 `theme = "dark"` → error 的要求实测同样已满足——design 块内 `theme = "dark"` 产生 `unexpected content at position 33: 'theme = "dark";...'`，与设计文档「spec 虚构，编译器无此语法」一致；控制器范围未要求自定义迁移提示，未额外实现，本报告记录行为）。

### 1.3 React design runtime 命名隔离（`__design/`）

`omnilang/src/codegen/react/design.rs`：

- 三个 runtime 输出移入 group 目录下的专用子目录：`{dir}__design/design.css`、`{dir}__design/useDesignTokens.ts`、`{dir}__design/useThemeStore.ts`。
- **T4.2 碰撞 guard 删除**（`theme_runtime_ok` 字段 + `theme_runtime_collides_with_store_hook` 函数）：目录隔离使「design runtime basename == 用户 store hook basename」不可达（用户文件不可能落在 `__design/`），按 YAGNI 移除并文档化（DesignGroup doc 注释）；theme runtime 现在无条件发射。
- 导入路径全部更新：
  - 根组件接线（`root_design_wiring`）：目标源改为 `{dir}__design/design.omni` → `import { useThemeStore as useThemeStoreDesign } from './__design/useThemeStore';` 等；
  - runtime 内部 store hook 导入（`emit_use_design_tokens`、`emit_theme_subscription`）：新锚点 `design_runtime_source(dir_prefix)`（合成源路径 `src/{dir}__design/design.omni`，`relative_import_path` 从中推导 `__design/` 目录）→ `'../stores/useThemeStore'`。
- 单元测试与集成测试全部更新为新路径；旧碰撞 guard 测试替换为 `theme_runtime_and_store_hook_coexist_in_same_directory`（design 与 ThemeStore 同目录 → `stores/__design/useThemeStore.ts` 与用户 `stores/useThemeStore.ts` 并存，无覆盖）。
- Flutter 侧同类碰撞核查（控制器要求 check）：**无保证性碰撞**——Flutter 生成文件名是 `{Component}.dart`/`{Store}_store.dart`/`types.dart`/`design.dart`；`design` 是 lexer 关键字，用户组件不可能叫 `design`，store 名带 `_store` 后缀。唯一理论残留：组件命名为 `Design`（大写，非关键字）时 `Design.dart` 与 `design.dart` 仅在大小写不敏感文件系统（Windows/macOS）上冲突——LOW 级、超出本任务范围（React 侧同类问题已由 `__design/` 根治；Flutter 侧移动 design.dart 会造成 ~10 处测试/导入 churn 而收益仅覆盖该边缘命名）。未修复，记录为残留。

### 1.4 测试与门禁

- 新增测试：flutter/design.rs 单元（ZIndex warning+skip、grid error、float error、cursor warning、变体 base/分支、伪状态、display:flex 不报）+ flutter/tests.rs 全管线集成（四诊断经 parse→semantic→IR→codegen 流出，grid/float 使 `success=false`）；react/design.rs 单元（并存测试）+ react/tests.rs（`@native` 拒绝、`__design/` 路径断言）。
- `omnilang/src/codegen/tests_regression/parameterized.rs` React 断言路径更新（`__design/design.css`、`__design/useThemeStore.ts`）。
- **门禁**：
  - `cargo test -p omnilang-core`：846 passed
  - `cargo test -p omnilang --features all-backends`：lib 720 passed + 全部集成套件 passed（含 1 项既有 ignore）
  - `generated_code_validation`：`react_output_compiles_with_tsc`（真实 tsc 编译含 `__design/` 输出）、`flutter_output_compiles_with_dart_analyze`、`match_variant_arms_flutter_compiles` 全绿
  - 重新构建 release binary → 重新生成 learn-omniview react-test-app：`stores/useThemeStore.ts` 与 `__design/useThemeStore.ts` 并存、App.tsx 导入 `./__design/useDesignTokens`、runtime 导入 `../stores/useThemeStore`、main.tsx 导入 `../__design/design.css`；`scripts/verify-react.sh` GATE PASSED（tsc 162 全豁免、eslint 119 全豁免、无 stale exemption——豁免均为错误码级，非路径级，__design 移动未引入新错误类）
  - clippy：我的改动零新增警告（react/design.rs 227/230 的 `is_root_component` 警告为既有代码）

### 1.5 文档同步

- spec §1.2（2026-07-31-semantic-level-dsl-syntax-spec.md）：删除「平台特有层 | `@native("dart") { ... }`」行 + 删除注记（死 token、干净解析错误、1 天可重加）。
- spec §10.3：四行实现状态注记（2026-08-03 Task 4.4，ZIndex 消息/通道、grid/float error 通道、cursor warning 通道、类含伪状态与变体统一扫描）。
- omniview 语法文档：14-grammar-lang.md 两处 token 列表、15-grammar-view.md §3.4 与附录表删除 `@native`。
- `docs/design/visual-editor-design.md` 的 `@native` 块设计记录（编辑器 UI 设计文档，非语言 spec）未改——历史设计记录，超出 spec 同步范围，记录为残留。
- 门禁配套：react-test-app `tsconfig.json` include、`src/main.tsx` design.css 导入、`scripts/verify-react.sh` LINT_SCOPE、`.gitignore`（`__design/` 目录加入已忽略的生成输出）更新。

## 2. Commit

`feat(design): implement spec diagnostics table; remove @native token; isolate design runtime naming` — 18 files, +662/-182（无格式 churn；react/tests.rs 保持 CRLF 基线，diff 61/20）

## 3. Verification summary

- 846 + 720 lib + 全部集成套件 passed；真实 tsc / dart analyze 门禁 3/3 绿（1 项既有 ignore）
- verify-react.sh GATE PASSED（learn-omniview 重新生成后）
- 碰撞场景实测：`stores/useThemeStore.ts`（用户）+ `__design/useThemeStore.ts`（runtime）并存编译
- `@native` → 干净解析错误；`theme = "dark"` → 干净解析错误（既有行为，已记录）
- zIndex/grid/float/cursor 四诊断按 spec §10.3 消息/级别/通道兑现

## 4. Concerns

1. **LOW（已文档化）**：Flutter 侧 `Design`（大写）组件 vs `design.dart` 在大小写不敏感文件系统上的理论冲突未修（范围外；React 侧已根治）。`docs/design/visual-editor-design.md` 的 `@native` 编辑器设计记录未删（历史设计文档，非语言 spec）。
2. **决策记录**：ZIndex warning 消息采用设计文档映射表原文「Flutter z 序由 widget 顺序决定」（控制器括号中的「，zIndex 无效」在文档中无出处，按「design doc 中文原文优先」政策取文档文本）；`display: flex` 不报 error（spec §10.3 仅列 `grid`，测试锁定该边界）。
3. **范围说明**：brief 的 `theme = "dark"` 定制迁移提示与 `@native(...)` 定制错误文案未实现——控制器歧义决议只要求 §10.3 四诊断 + token 删除（自然解析错误即可），且 `theme = "dark"` 已天然报错（与设计文档「spec 虚构」声明一致）。
