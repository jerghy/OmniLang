# 修复计划：v5 架构评审 + 深度验证（2026-07-27）

> **来源**: [v5 全局评审](.architecture/reviews/global-review-2026-07-27.md) + [深度验证](.architecture/reviews/global-review-2026-07-27-v5-deep-verification.md)
> **发现数**: 21 项确认（零项反驳，3 项精细化）
> **总预计工作量**: 约 5-7 天（跨 4 个阶段）
> **原则**: 零行为变更优先，可逆性最大化，每次提交后运行测试

---

## 阶段 0：立即执行（关键——安全与零风险去重）

**预计**: 2-3 小时 | **风险**: 极低 | **可逆性**: 全部可通过 git revert 轻松撤销

### P0-1：修复调试窗口沙盒绕过（安全 C1）

- **文件**: `poiesis/dist/debug-window/index.html`（git 中跟踪的过期构建产物）
- **问题**: `dist/` 版本无沙盒 iframe——用户代码在 `__TAURI_INTERNALS__` 上下文中运行
- **修复**:
  1. `git rm --cached poiesis/dist/debug-window/index.html`
  2. 添加 `poiesis/dist/` 到 `poiesis/.gitignore`
  3. 运行 `cd poiesis && npm run build` 重新生成
  4. 提交：`fix(security): remove stale dist build that bypasses P38 sandbox isolation`
- **工作量**: S（10 分钟）
- **影响范围**: 仅构建产物——源代码不变
- **可逆性**: `git revert`
- **验证**: `grep -c "sandbox" poiesis/dist/debug-window/index.html` 应 > 0；`grep -c "__TAURI_INTERNALS__" poiesis/dist/debug-window/index.html` 应为 0

### P0-2：添加 CI 检查以防止沙盒回归

- **文件**: `.github/workflows/ci.yml`
- **问题**: CI 仅检查 `src-tauri/debug-window/` 与 `public/debug-window/`——不检查 `dist/`
- **修复**:
  1. 在现有"Check debug-window HTML sync"步骤后添加：
     ```yaml
     - name: Verify dist debug-window has sandbox
       run: |
         grep -q 'sandbox="allow-scripts"' poiesis/dist/debug-window/index.html || \
           (echo "ERROR: dist debug-window missing sandboxed iframe!" && exit 1)
         grep -qv '__TAURI_INTERNALS__' poiesis/dist/debug-window/index.html || \
           (echo "ERROR: dist debug-window should not access __TAURI_INTERNALS__!" && exit 1)
     ```
  2. 作为 `poiesis` 作业中的构建后步骤插入上述内容（第 105 行之后）
- **工作量**: S（5 分钟）
- **验证**: CI 在下次推送后通过

### P0-3：委托 facade `analyse.rs` → 核心重新导出（C2）

- **文件**: `omnilang/src/analyse.rs`（删除），`omnilang/src/lib.rs`（修改 1 行）
- **问题**: 567 行，diff 输出为空——字节级完全相同
- **修复**:
  1. 将 `omnilang/src/lib.rs` 中的 `pub mod analyse;` 替换为 `pub use omnilang_core::analyse;`
  2. `git rm omnilang/src/analyse.rs`
  3. 提交：`refactor: delegate facade analyse.rs to core (zero-diff dedup, P35 completion)`
- **工作量**: S（5 分钟）
- **影响范围**: 零——代码完全相同
- **可逆性**: `git revert`
- **验证**: `cargo test -p omnilang` 全部通过；`cargo test -p omnilang-core` 全部通过

### P0-4：委托 `codegen/shared/ui_codegen.rs`（C3）

- **文件**: `omnilang/src/codegen/shared/ui_codegen.rs`（删除），`omnilang/src/codegen/shared/mod.rs`（修改 1 行）
- **问题**: 63 行，字节级完全相同——`UiCodegen` trait 在 facade 中重复
- **修复**:
  1. 将 `pub mod ui_codegen;` 替换为 `pub use omnilang_core::codegen::shared::ui_codegen;`
  2. `git rm omnilang/src/codegen/shared/ui_codegen.rs`
- **工作量**: S（3 分钟）
- **验证**: `cargo test -p omnilang` 全部通过

### P0-5：委托 `codegen/event.rs` + `helpers.rs`（C3）

- **文件**: `omnilang/src/codegen/event.rs`（删除），`omnilang/src/codegen/helpers.rs`（删除），`omnilang/src/codegen/mod.rs`（修改 2 行）
- **问题**: 仅 `pub`/`pub(crate)` 可见性差异——通过 crate 边界自然解决
- **修复**:
  1. 将 facade `codegen/mod.rs` 中的 `pub mod event;` 和 `pub(crate) mod helpers;` 替换为 `pub use omnilang_core::codegen::event;` 和 `pub(crate) use omnilang_core::codegen::helpers;`
  2. `git rm` 两个文件
- **工作量**: S（5 分钟）
- **验证**: `cargo test -p omnilang` 全部通过

### P0-6：添加 CI eval 产物持久化 + 更新基线（C4）

- **文件**: `.github/workflows/ci.yml`，`.claude/evals/eval-run-omnilang.sh`
- **问题**: Eval 结果在临时运行器中蒸发；基线来自 2026 年 6 月
- **修复**:
  1. 在 `ci.yml` 中的 `Run OmniLang evals` 之后添加：
     ```yaml
     - name: Upload eval results
       if: always()
       uses: actions/upload-artifact@v4
       with:
         name: eval-results
         path: .claude/evals/omnilang-compiler.log
         if-no-files-found: warn
     ```
  2. 重新测量基线：`cargo test -p omnilang 2>&1 | grep "test result:" | grep -v Doc-tests` 以获取实际测试计数
  3. 相应地更新 `BASELINE_TESTS`、`BASELINE_CLIPPY`、`BASELINE_UNWRAP`、`BASELINE_OVER_800`
  4. 在 eval 作业中添加 `continue-on-error: true`（如最初计划）：
     ```yaml
     evals:
       runs-on: ubuntu-latest
       needs: rust
       continue-on-error: true  # 报告但不阻塞
     ```
- **工作量**: S（15 分钟）
- **验证**: 下一次 CI 运行在"Artifacts"下显示 eval 结果

### P0-7：删除 `CodegenCache` 孤儿文件 + `poiesi/`（C5）

- **文件**: `crates/omnilang-core/src/codegen/shared/cache.rs`（删除），`poiesi/`（删除）
- **问题**: `cache.rs` 存在但不在 `mod.rs` 中声明——甚至未编译。`poiesi/` 只有 1 个未导入的文件。
- **修复**:
  1. `git rm crates/omnilang-core/src/codegen/shared/cache.rs`
  2. `git rm -r poiesi/`
  3. 提交：`chore: remove orphan cache.rs (never compiled) and poiesi/ directory`
- **工作量**: S（2 分钟）
- **可逆性**: `git revert`
- **验证**: `cargo build` 成功；工作区中无残留引用

---

## 阶段 1：短期（安全加固 + 剩余去重）

**预计**: 1-2 天 | **风险**: 低-中 | **依赖**: 阶段 0 已完成

### P1-1：限制 FS 范围——删除 `C:/**` 和 `D:/**`（安全 H1）

- **文件**: `poiesis/src-tauri/capabilities/default.json`
- **问题**: Tauri FS 插件对整个驱动器根目录有读/写权限
- **修复**:
  1. 删除 `{ "path": "D:/**" }` 和 `{ "path": "C:/**" }` 条目
  2. 保留 `$APPDATA`（应用数据）、`$HOME`（用户文件）
  3. 如果项目目录必须在 `$HOME` 之外，添加动态作用域或用户确认流程
- **工作量**: M（包括测试）
- **影响范围**: 任何使用 `@tauri-apps/plugin-fs` 的前端代码——如果项目在 C:/ 或 D:/ 根目录，可能会中断
- **可逆性**: 恢复条目
- **验证**: 从 `C:/Windows` 调用时 `readTextFile("C:/Windows/System32/...")` 被拒绝

### P1-2：向 `open_project` 添加路径验证（安全 H2）

- **文件**: `poiesis/src-tauri/src/commands/project.rs`
- **问题**: 接受任意绝对路径作为项目目录，无验证
- **修复**:
  1. 添加拒绝系统目录的检查列表（`C:/Windows`、`C:/Program Files`、`/etc`、`/usr` 等）
  2. 添加限制：项目目录必须在 `$HOME` 或 `$APPDATA` 内
  3. 使用 `tauri::api::dialog` 添加超出 `$HOME` 路径的用户确认提示
- **工作量**: M（1-2 小时）
- **可逆性**: 放宽检查
- **验证**: `open_project("C:/Windows/System32")` 返回错误

### P1-3：合并 `codegen/tests_regression/` 和 `tests_property.rs`（H2）

- **文件**: `omnilang/src/codegen/tests_regression/mod.rs`、`parameterized.rs`、`javascript.rs`、`tests_property.rs`
- **问题**: 测试文件在两个 crate 中重复——相同逻辑编译两次
- **修复**:
  1. 委托 `tests_property.rs`（完全相同）：`#[cfg(test)] mod tests_property { pub use omnilang_core::codegen::tests_property::*; }`
  2. 对于 `tests_regression/`：在 facade 中创建重新导出垫片，委托到核心的版本
  3. 对于 `parameterized.rs`（98% 相同）：差异仅在于文档注释——将它们合并到核心
- **工作量**: M（2-3 小时）
- **验证**: `cargo test -p omnilang` 和 `cargo test -p omnilang-core` 的测试计数保持不变

### P1-4：从 `ModuleIR` 中删除 Theme 死数据（H5）

- **文件**: `crates/omnilang-core/src/ir/logic.rs`、`crates/omnilang-core/src/ir/lower/mod.rs`、所有构造 `ModuleIR { themes: ... }` 的测试文件
- **问题**: "themes" 数据已解析、已降低、已序列化，但零消费者
- **修复**:
  1. 从 `ModuleIR` 中删除 `themes: Vec<serde_json::Value>` 字段
  2. 从 `ir/lower/mod.rs` 中删除 lowering 代码（第 192-197 行）
  3. 从所有测试结构体初始化器中删除 `themes: vec![]`
  4. 添加 lint 规则：当解析到 `theme` 块时发出 `WMissingTheme` 警告
  5. 在解析器中保留 `theme` 关键字和 `ThemeDef` AST——仅删除 IR 死数据
- **工作量**: S（30 分钟）
- **可逆性**: 恢复字段——但应先实施 YAGNI
- **验证**: `cargo test --workspace` 全部通过；搜索 `\.themes` 仅命中 AST 定义

### P1-5：修复 ADR-009 僵尸 + 代理文档（H6）

- **文件**: `ADR-009`、`.architecture/agent_docs/reference.md`
- **修复**:
  1. 更新 ADR-009 状态为"Rejected"，附上理由："在清理中删除——单后端 WASM 路径是生产瓶颈；多目标并行化在需要之前是 YAGNI"
  2. 添加"Superseded by"指向清理提交
  3. 修复 `reference.md:139`："eval 脚本未接入 CI" → "eval 在 CI 中运行但结果未持久化（见`ci.yml:107`）"
  4. 更新 `reference.md:93` 中 P36 的状态
- **工作量**: S（15 分钟）
- **验证**: 在 ADR-009 中搜索"Accepted"返回 0；向后读取没有矛盾

---

## 阶段 2：中期（性能 + CI + 合并去重）

**预计**: 2-3 天 | **风险**: 中 | **依赖**: 阶段 1 已完成

### P2-1：在 WASM 中实现仅内存增量缓存（H7）

- **文件**: `omnilang-wasm/src/lib.rs`、`crates/omnilang-core/src/analyse.rs`
- **问题**: 每次 WASM 调用都将整个 `IncrementalCache` 序列化为 JSON 返回给 JS——每次编辑器击键都是 CPU 密集型的
- **修复**:
  1. 将 `ANALYSIS_CACHE` `thread_local!` 设为主要缓存存储
  2. 移除 `analyse_multi_cached` 和 `resolve_styles_multi_cached` 中的 `serde_json::to_string(&*cache)` 调用
  3. 仅在页面卸载/导航时序列化缓存（用于持久化），而非每次调用
  4. 在 `seed_cache_from_state` 的文档中记录：缓存是尽力而为的，在页面导航时会丢失
- **工作量**: M（3-5 小时）
- **影响范围**: WASM 绑定 + `analyse.rs` 热路径
- **可逆性**: 回退到每次调用序列化
- **验证**: 使用 Chrome DevTools 进行基准测试：编辑器中的热重载应接近瞬时（<50ms 缓存命中）

### P2-2：合并 `codegen/shared/{expr,stmt,types,visitor,html,inline_handler}.rs`（H1）

- **文件**: `omnilang/src/codegen/shared/{expr,stmt,types,visitor,html,inline_handler}.rs` → 删除；功能门控注解移至 `crates/omnilang-core/src/codegen/shared/`
- **问题**: 约 1,500 行带有分化功能门控注解的重复代码
- **修复**:
  1. 将 facade 独有的功能门控 `#[cfg(any(...))]` 注解移至核心副本
  2. 在 facade 的 `codegen/shared/mod.rs` 中替换模块声明为 `pub(crate) use omnilang_core::codegen::shared::*;`
  3. 这 6 个文件的逐个文件 diff 和测试验证
- **工作量**: L（4-6 小时，包括合并和测试）
- **影响范围**: 所有代码生成后端——都必须编译
- **可逆性**: 恢复 facade 副本
- **验证**: `cargo test --workspace --features all-backends` 全部通过；两个 crate 的测试计数相同

### P2-3：向 CI 添加 WASM 测试（H9）

- **文件**: `.github/workflows/ci.yml`
- **问题**: 15 个 WASM 测试不在 CI 中，在 Windows 上崩溃
- **修复**:
  1. 添加仅 Linux 的 `wasm` CI 作业：`wasm-pack test --node omnilang-wasm`
  2. 在 `omnilang-wasm/README.md` 中记录 Windows 限制
  3. 作为单独的作业运行（非阻止性），带有 `continue-on-error: true`
- **工作量**: M（1 小时）
- **验证**: WASM 测试出现在 CI 输出中；已知的 Windows 限制被记录

### P2-4：集中功能门控列表（H3）

- **文件**: `crates/omnilang-core/src/codegen/shared/{expr,stmt,types}.rs`
- **问题**: 3 个文件中出现 16 次相同的 8 元素 `#[cfg(any(...))]` 列表
- **修复**:
  1. 在 `omnilang-core/src/codegen/mod.rs` 中定义单个 `#[macro_export] macro_rules! any_backend_enabled { ... }`
  2. 将所有 16 处替换为 `#[cfg(any_backend_enabled!())]`
- **工作量**: S（20 分钟）
- **可逆性**: 展开宏
- **验证**: `cargo test --features all-backends` 和 `cargo test`（默认功能）均通过

### P2-5：删除 62 个 Rete.js 废弃文档文件（H8）

- **文件**: `docs/rete-docs/`（整个目录，62 个文件，约 5 MB）
- **问题**: Rete.js 在 ADR-002 中被废弃；供应商文档污染搜索结果
- **修复**: `git rm -r docs/rete-docs/`
- **工作量**: S（2 分钟）
- **可逆性**: `git revert`
- **验证**: 工作区中对 "rete-docs" 的搜索仅返回历史提交引用和 `DESIGN_DOCS_INDEX.md` 过时标记

### P2-6：修复损坏的 stdlib 链接 + 记录 theme 关键字（文档 M3、H2）

- **文件**: `docs/README.md`、`docs/omniview/10-design-system.md`
- **修复**:
  1. `docs/README.md:53`：将 `../stdlib.md` 改为 `stdlib.md`
  2. `docs/omniview/10-design-system.md`：添加"Theme"部分，解释语法、与 `design` 的关系，并注明主题代码生成尚未实现
- **工作量**: S（20 分钟）

---

## 阶段 3：长期（编译器增强 + 类型系统）

**预计**: 2-4 周 | **风险**: 中-高 | **依赖**: 阶段 2 已完成

### P3-1：实现死语句消除优化 pass（编译 C1 部分）

- **文件**: `crates/omnilang-core/src/ir/optimize.rs`
- **问题**: 优化器仅消除死计算属性。无死语句/函数消除，无常量折叠。
- **修复**:
  1. 添加 `eliminate_unreachable_code()` pass：remove statements after unconditional `return`/`break`/`continue`/`throw`
  2. 添加 `eliminate_dead_functions()` pass：对从不被任何 `CallExpr::name` 引用的 `FunctionIR` 实例进行引用计数
  3. 添加 `fold_constants()` pass：walk `ExprIR` for `BinaryOpExpr { op: Add, left: Literal(1), right: Literal(2) }` → `LiteralExpr(3)`
- **工作量**: L（3-5 天）
- **影响范围**: 所有代码生成后端——必须验证输出
- **可逆性**: Feature gate 优化传递
- **验证**: 15+ 个新 `optimize.rs` 测试；golden 快照不变

### P3-2：消除双重 `type_args` 表示（编译 C2）

- **文件**: `crates/omnilang-core/src/ir/types.rs`
- **问题**: `TypeKind::Named { type_args: Vec<TypeKind> }` + `TypeIR { type_args: Vec<TypeIR> }` 都携带泛型参数
- **修复**:
  1. Remove `type_args: Vec<TypeKind>` from `TypeKind::Named`
  2. 更新自定义 Serialize/Deserialize 实现以单独使用 `TypeIR::type_args`
  3. 审计所有 `TypeKind::Named` 上的 match 臂
- **工作量**: M（2-3 天）
- **影响范围**: 所有类型引用；serde 往返兼容性
- **验证**: 所有 snapshot 测试通过；serde 往返测试不变

### P3-3：将 `fn_ref` 回填合并到视图 lowering（编译 C3）

- **文件**: `crates/omnilang-core/src/ir/lower/component.rs`
- **问题**: Two-phase mutation (`backfill_fn_ref_event_fields`) couples construction with post-hoc analysis
- **修复**:
  1. 在 `lower_view_node` 中内联 fn_ref 事件处理程序处理
  2. 删除单独的 `backfill_fn_ref_event_fields` 遍历
  3. 消除重复的视图树遍历
- **工作量**: M（1-2 天）
- **影响范围**: 组件 lowering 路径
- **验证**: Lowering 测试全部通过

### P3-4：将 `ir/types.rs` 拆分为领域子模块（可维护性 M5）

- **文件**: `crates/omnilang-core/src/ir/types.rs`（1,213 行）
- **修复**:
  1. 将事件辅助函数（`event_data_schema` 等）提取到 `ir/events.rs`
  2. 将信号/计算相关类型提取到 `ir/signals.rs`
  3. 将元素/视图树类型提取到 `ir/elements.rs`
  4. 在 `ir/types.rs` 中保留核心 IR 类型（`ExprIR`、`StatementIR`、`ComponentIR`、`IRProgram`）
- **工作量**: M（2-3 小时）
- **可逆性**: 重新导出保持兼容性
- **验证**: 所有导入解析；测试全部通过

### P3-5：将 `parser/expr/mod.rs` 拆分为子模块（Rust H4）

- **文件**: `crates/omnilang-core/src/parser/expr/mod.rs`（1,075 行）
- **修复**:
  1. 将前缀解析器提取到 `expr/prefix.rs`
  2. 将中缀绑定幂 + 解析器提取到 `expr/infix.rs`
  3. 将字面量解析提取到 `expr/literal.rs`
  4. 将模板字符串解析提取到 `expr/template.rs`
  5. 每个文件应 < 300 行
- **工作量**: M（2-3 小时）
- **可逆性**: 纯重构，零行为变更
- **验证**: 解析器测试全部通过

### P3-6：将 `omnic` 默认功能切换为仅 P0（实用主义 H1）

- **文件**: `omnic/Cargo.toml`
- **问题**: 每个 CLI 构建都编译全部 9 个后端
- **修复**:
  1. 将 `features = ["all-backends"]` 改为 `features = ["codegen-omniview", "codegen-react", "codegen-flutter"]`
  2. 添加单独的 CI 作业：`cargo test --features all-backends` 用于非 P0 回归检查
- **工作量**: S（5 分钟）
- **验证**: `cargo build -p omnic` 仅编译 3 个后端

---

## 阶段 4：积压工作（完善）

**预计**: 持续进行 | **风险**: 低 | **依赖**: 无

### P4-1：编写 BNF 语法规范文档

- **文件**: `docs/grammar.md`（新建）
- **工作量**: L（2-3 天从 Rust 解析器源代码提取）
- **价值**: 双解析器同步的权威参考，AI 代理分析准确性，第三方工具构建

### P4-2：在语义分析中实现类型检查 pass

- **文件**: `crates/omnilang-core/src/semantic/validate_types.rs`
- **工作量**: L（3-5 天用于推理引擎）
- **价值**: 在编译时而非运行时捕获类型错误，减少每个后端的类型处理

### P4-3：将 `build_component_map` 适配器迁移为借用引用

- **文件**: `omnilang/src/codegen/shared/mod.rs`，所有 4+ 个后端调用者
- **工作量**: M（2-4 小时用于生命周期注解）
- **价值**: 消除每次调用时所有 ComponentIR 值的克隆

### P4-4：提取 Zustand 副作用以订阅 `subscribe`

- **文件**: `poiesis/src/store/slices/element-crud.ts`
- **工作量**: S（30 分钟）
- **价值**: 防止状态提交和自定义事件之间的竞态条件

### P4-5：消除剩余的 `as any` 类型转换

- **文件**: `PropsSection.tsx`、`rustCompiler.ts`、`graph-parser.ts`、`useWasmCompile.ts`
- **工作量**: M（跨 5-6 个模块分散）
- **价值**: 使用 WASM API 变更将在编译时捕获，而非运行时

---

## 任务依赖图

```
阶段 0（所有独立，可并行）
├── P0-1 ──→ P0-2（CI 检查依赖于正确的构建产物）
├── P0-3 ──→ P1-3（委托 analyse 在合并测试回归之前）
├── P0-4 ──→ P2-2（委托 ui_codegen 在合并共享之前）
├── P0-5 ──→ P2-2（委托 event/helpers 在合并共享之前）
├── P0-6（独立）
└── P0-7（独立）

阶段 1（所有依赖于阶段 0 完成）
├── P1-1（独立）
├── P1-2（独立）
├── P1-3 ──→ P2-2
├── P1-4（独立）
└── P1-5（独立）

阶段 2
├── P2-1（独立）
├── P2-2 ← P0-4, P0-5, P1-3
├── P2-3（独立）
├── P2-4 ← P2-2
├── P2-5（独立）
└── P2-6（独立）

阶段 3
├── P3-1（独立）
├── P3-2（独立）
├── P3-3（独立）
├── P3-4 ← P2-2
├── P3-5（独立）
└── P3-6（独立）

阶段 4（全部独立，可在任何时间执行）
```

---

## 质量门控（每个阶段后）

```bash
# 阶段 0 门控
cargo test --workspace          # 必须全部通过
cargo clippy --workspace -- -D warnings  # 零警告
cargo build --workspace          # 必须编译成功
grep -q "sandbox" poiesis/dist/debug-window/index.html  # 沙盒存在

# 阶段 1 门控（所有上述 +）
cargo test --features all-backends  # 所有后端必须通过
grep -c "__TAURI_INTERNALS__" poiesis/dist/debug-window/index.html  # 必须为 0
grep -rn "\.themes" crates/omnilang-core/src/codegen omnilang/src/codegen  # 必须为空

# 阶段 2 门控（所有上述 +）
wasm-pack test --node omnilang-wasm  # WASM 测试必须通过（Linux）
grep -rn "rete-docs" docs/  # 无残留引用

# 阶段 3 门控（所有上述 +）
cargo test --workspace          # 无回归
```

---

## 决策记录需求

| ADR | 操作 | 理由 |
|-----|--------|-----------|
| ADR-009 | 更新：Accepted → Rejected | P36 已删除；记录 YAGNI 理由 |
| ADR-011（新建）| 创建 | "P35 Crate Boundary Completion"——记录委托 vs 合并策略 |
| ADR-012（新建）| 创建 | "Theme Data Removal"——记录为什么 themes 被从 IR 中移除 |
| ADR-013（新建）| 创建 | "Cache Architecture"——仅内存 WASM 缓存设计 |

---

## 进度跟踪

| 阶段 | 任务 | 状态 | 提交 | 备注 |
|-------|------|--------|--------|-------|
| 0 | P0-1 沙盒修复 | ⬜ 待处理 | — | — |
| 0 | P0-2 CI 沙盒检查 | ⬜ 待处理 | — | — |
| 0 | P0-3 analyse 委托 | ⬜ 待处理 | — | — |
| 0 | P0-4 ui_codegen 委托 | ⬜ 待处理 | — | — |
| 0 | P0-5 event/helpers 委托 | ⬜ 待处理 | — | — |
| 0 | P0-6 CI eval 持久化 | ⬜ 待处理 | — | — |
| 0 | P0-7 删除死代码 | ⬜ 待处理 | — | — |
| 1 | P1-1 FS 范围 | ⬜ 待处理 | — | — |
| 1 | P1-2 open_project 验证 | ⬜ 待处理 | — | — |
| 1 | P1-3 测试委托 | ⬜ 待处理 | — | — |
| 1 | P1-4 Theme 死数据 | ⬜ 待处理 | — | — |
| 1 | P1-5 ADR 僵尸 | ⬜ 待处理 | — | — |
| 2 | P2-1 WASM 缓存 | ⬜ 待处理 | — | — |
| 2 | P2-2 共享合并 | ⬜ 待处理 | — | — |
| 2 | P2-3 WASM CI | ⬜ 待处理 | — | — |
| 2 | P2-4 cfg 宏 | ⬜ 待处理 | — | — |
| 2 | P2-5 Rete.js 删除 | ⬜ 待处理 | — | — |
| 2 | P2-6 文档修复 | ⬜ 待处理 | — | — |
| 3 | P3-1 死代码消除 | ⬜ 待处理 | — | — |
| 3 | P3-2 type_args 修复 | ⬜ 待处理 | — | — |
| 3 | P3-3 fn_ref 回填 | ⬜ 待处理 | — | — |
| 3 | P3-4 types.rs 拆分 | ⬜ 待处理 | — | — |
| 3 | P3-5 expr 拆分 | ⬜ 待处理 | — | — |
| 3 | P3-6 omnic 功能 | ⬜ 待处理 | — | — |
| 4 | P4-1 BNF 语法 | ⬜ 待处理 | — | — |
| 4 | P4-2 类型检查 | ⬜ 待处理 | — | — |
| 4 | P4-3 借用迁移 | ⬜ 待处理 | — | — |
| 4 | P4-4 副作用提取 | ⬜ 待处理 | — | — |
| 4 | P4-5 any 类型转换 | ⬜ 待处理 | — | — |

**图例**: ⬜ 待处理 | 🟡 进行中 | ✅ 已完成 | ❌ 已阻止
