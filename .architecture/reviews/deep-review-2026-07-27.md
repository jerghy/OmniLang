# Deep Architecture Re-Review: Implementation-Focused Technical Analysis

> **评审日期**: 2026-07-27
> **评审类型**: 6 位专家深度代码级技术复审 → 实施路线图
> **基准**: [full-workspace-review-2026-07-27.md](./full-workspace-review-2026-07-27.md)
> **前置条件**: 11 专家全工作区复审 + 用户反馈（FS 权限为设计决策、推迟项目→立即执行、P34 继续搁置）

---

## 核心修正

本次深度复审对先前评审中的多个关键事实错误进行了代码级验证和纠正：

| 先前声称 | 实际验证结果 | 纠正来源 |
|---------|-------------|---------|
| "39 个 Clippy 警告" | **41 个**警告，**19 个** auto-fixable | `cargo clippy --lib` 实测 |
| "OmniViewGenerator 55 个字段" | **45 个**字段，按 10 个关注领域组织 | 逐字段计数 |
| "ts-rs 未被消费" | **CompileResult + Diagnostic 被前端导入**（3 个文件为孤立文件） | `rustCompiler.d.ts:8-9` |
| "TS golden 测试不在 CI" | **已在 CI**（`ci.yml:86-87`，vitest 作业包含 golden.test.ts） | CI 配置验证 |
| "165 处生产 unwrap" | **几乎全部在 `#[cfg(test)]` 中**；生产路径中仅 ~2-3 处 | 逐个审计 |
| "source_span 在 4 个核心结构体上缺失" | **ComponentIR + StoreIR 可填充**（span 随手可用）；ModuleIR + IRProgram **无法填充**（AST 类型无 span 字段） | 逐文件追踪 |

---

## 执行摘要

本次深度复审对 6 个关键领域进行了代码级技术分析。**最重要的发现是 source_map 覆盖 bug**（P4 管道在 `codegen/mod.rs:117` 处丢弃了 OmniView 的逐元素 NID 级源映射），以及**P39 的最小可行优化**（消除未使用计算属性，约 200 行）可以在补齐运动测试覆盖率的同时立即实施。

---

## 深度发现 #1：source_map 覆盖 Bug（P4/P33）

**被 Compiler Architect 在代码遍历中发现**

### 问题

在 `codegen/mod.rs:117`：
```rust
output.source_map = ir.source_map.values().cloned().collect();
```

此行**覆盖**了 OmniView 代码生成器在 `codegen/omniview/view/element.rs:874-888` 中计算的逐元素 source_map（将 `data-nid` 属性映射到源位置），并仅用 IR 级别的逐实体 source_map（5 种实体类型：Component、Store、Function、Motion、Timeline）替换。

### 后果

- **保留**：IR 级 source_map（将 `module:component:Name` 映射到 `SourceSpan`）——通过 `CompileOutput` 暴露
- **丢弃**：逐元素的 NID 级 source_map（将 `data-nid="42"` 映射到 `SourceSpan`）——**不可恢复地丢失**

如果编辑器画布或调试器需要逐元素源映射（点击元素 → 跳转到 OmniLang 源），此 bug 会阻止该功能，尽管代码生成器确实产生了所需的映射数据。

### 修复

```rust
// 不要覆盖——改为追加
output.source_map.extend(ir.source_map.values().cloned());
```
或更好：将两个 source_map 分开（`CompileOutput` 需要第二个字段用于逐元素映射）。

---

## 深度发现 #2：P33 source_span 精确填充计划

**Compiler Architect 的代码级分析**

### 当前状态

| 度量 | 数量 |
|--------|-------|
| 具有 `source_span: Option<Span>` 的结构体 | **37** |
| 在生产降级代码中实际填充 | **1**（仅 ElementIR，位于 `view.rs:599`） |
| 降级中的 `source_span: None` 站点 | **33** |
| 可通过现有 AST span 立即填充 | **6** |
| 由于 AST 类型无 span 字段而无法填充 | **2**（ModuleIR、IRProgram） |
| 没有生产者且没有消费者 | **29** |

### 精确的 6 个修复站点

| 文件:行 | IR 结构体 | 可用 AST Span | 代码修改 |
|----------|-----------|------------------|-----------|
| `component.rs:289` | ComponentIR | `comp.span`（ComponentDef L31） | `source_span: Some(comp.span)` |
| `component.rs:634` | StoreIR | `store.span`（StoreDef L303） | `source_span: Some(store.span)` |
| `component.rs:107` | FunctionIR | `f.span`（FunctionDef L376） | `source_span: Some(f.span)` |
| `component.rs:130` | FunctionIR | `a.span`（ActionDef L89） | `source_span: Some(a.span)` |
| `component.rs:620` | FunctionIR | `action.span`（ActionDef） | `source_span: Some(action.span)` |
| `component.rs:260` | HeroIntent | `h.span`（HeroDef L292） | `source_span: Some(h.span)` |

**模块化 IR 和 IRProgram 无法填充**——`parser/ast/mod.rs` 中的 `Module` 和 `Program` 结构体没有 `span` 字段。这是一个 AST 设计限制，而非错误。

### `source_span` 的唯一消费者（3 个生产站点）

1. `semantic/lints/invalid_event_field.rs:69-70` — 诊断线/列
2. `semantic/lints/store_action_event.rs:66-67` — 诊断线/列
3. `codegen/omniview/view/element.rs:877-878` — 源映射入口线/列

所有三个都仅读取 `ElementIR.source_span`。没有其他 IR 结构体的 `source_span` 在任何生产代码中被读取。

### 建议

**填充 6 个可以立即使用的站点**（ComponentIR、StoreIR、3×FunctionIR、HeroIntent）。对于剩余的 29 个面向表达式的结构体，**将字段保留为 `None`**，直到出现需要 span 的具体消费者。不要移除字段——`Option` 的开销可以忽略不计，且缺乏 span 是等待分配工作的诚实信号。

---

## 深度发现 #3：TypeKind 序列化数据丢失

**被 Rust Expert 和 Compiler Architect 验证**

### 代码证据

来自 `ir/types.rs:289-292` 的精确注释：
```rust
Named {
    name: String,
    /// Recursive type arguments (e.g., `Array<String>` → `[String]`).
    /// Not serialized — the custom `Serialize`/`Deserialize` impls preserve
    /// backward-compatible flat-string serialization.
    type_args: Vec<TypeKind>,
},
```

第 336-348 行的自定义序列化器调用 `self.as_str()`，其在 `Named` 变体上仅返回 `name`。降级器在 `component.rs:662-668` 正确地填充了 `type_args`，然后序列化将其剥离。序列化-反序列化往返后：
- `Array<String>` → 序列化 → `"Array"` → 反序列化 → `Array`（无类型参数）
- `Result<i32, String>` → `"Result"` → `Result`（无变体信息）

### 修复

扩展 `Serialize` 以在有 `type_args` 时发出 `{"name":"Array","args":["String"]}`。扩展 `Deserialize` 以读取旧平面格式和新结构化格式。**工作量：M**（2-3 小时，需要在 JSON 边界进行向后兼容性测试）。

---

## 深度发现 #4：P35 Crate 拆分——具体方案

**Rust Expert 的完整依赖分析**

### 依赖 DAG（已验证无循环依赖）

```
lexer/ (第 1 层)
  ↑
parser/ (第 2 层)  ← 仅导入 lexer
  ↑
semantic/ + ir/ (第 3 层)  ← 导入 parser + lexer
  ↑
analyse.rs (第 4 层)  ← 导入上面所有三层；**不导入 codegen**
  ↑
codegen/shared/ + traits/ + writer.rs (第 5 层)  ← 导入 ir + error
  ↑
codegen/omniview/, react/, flutter/, ... (第 6 层)  ← 仅导入第 5 层 + ir
```

关键验证：**`analyse.rs` 从未导入 `crate::codegen`**——确认了严格分离。

### 第 1 阶段：最小可行拆分（1-2 天）

1. 创建 `omnilang-core`（第 1-5 层：lexer → parser → semantic → ir → analyse → codegen 基础设施）——~110 个 .rs 文件
2. 创建 `omnilang-codegen-omniview`（~17 个文件）
3. 创建 `omnilang` 外观 crate（重新导出以便向后兼容）
4. 将 `codegen/shared/` 中约 20 个 `pub(crate)` 项提升为 `pub`
5. 在 `codegen/mod.rs` 中更新 `compile()` 调度函数以调用外部 crate

### 爆炸半径

- **更改了 ~50 个文件的 `use` 声明**（`crate::` → `omnilang_core::` 或 `omnilang_codegen_omniview::`）
- **创建了 3 个新的 `Cargo.toml`** 文件
- **可见性变更了 15-20 个项**（`pub(crate)` → `pub`）
- **向后兼容**：`omnilang` 外观保留现有 API
- **CI 优势**：每个代码生成 crate 可以独立构建/测试

---

## 深度发现 #5：P36 代码生成并行化——精确方案

**Performance Specialist 的线程安全审计**

### 线程安全验证

- ✅ `Codegen` 特质：**`Send + Sync` 边界已验证**
- ✅ 整个 `codegen/` 目录中**零个 `RefCell`/`Cell`/`Mutex`**
- ✅ 所有代码生成结构体：仅简单的配置字段（`bool`、`enum`）
- ✅ `OmniViewGenerator`（45 个字段）**是每个调用局部创建的**，从未跨线程共享
- ✅ `.generate()` 是纯的：`&self` + `&IRProgram` + `&FxHashMap` → 拥有的 `CompileOutput`

### 推荐方法：跨后端并行化（选项 A）

由于当前架构仅编译到单个目标，需要一个新的入口点：

```rust
pub fn compile_multi_target(
    sources: FxHashMap<String, String>,
    targets: Vec<Target>,
    ir: &IRProgram,
    ...
) -> FxHashMap<Target, InternalCompileResult> {
    targets.par_iter().map(|&target| {
        let output = compile_with_target(ir, &sources, target, ...);
        (target, output)
    }).collect()
}
```

- **对代码生成后端的零更改**
- **加速**：3 个 P0 后端的代码生成时间节省 1.5x-2x
- **风险**：低——已验证 `Send + Sync`，无共享可变状态
- **限制**：仅对工具链/CI（多后端编译）有益；单后端编辑器路径不受影响

---

## 深度发现 #6：P38b + postMessage 安全——XSS 升级链

**Security Specialist 的逐行审计（`poiesis/src-tauri/debug-window/index.html` 的 363 行）**

### 🔴 关键：5 步 XSS 升级链

```
1. postMessage('*') — 任何窗口可发送消息
   ↓
2. 无 e.origin/e.source 验证 — 仅检查可伪造的 e.data.source 字符串
   ↓
3. iframe 内 eval(comp.js) — 攻击者发送恶意组件 JS
   ↓
4. 污染 __omni_signals 键名 — eval 的代码设置恶意信号键
   ↓
5. innerHTML 注入未净化数据 — signalList.innerHTML = html（L213-L215）
   ↓
   在父窗口上下文中执行 XSS → 完全访问 __TAURI_INTERNALS__
```

### P38b 结论

**`__TAURI_INTERNALS__` 不能完全剥离**——它是调试窗口与主应用通信所需的传输层（仅事件 IPC，无 fs/shell/window）。然而，**P38b 的真正目标已通过 `sandbox="allow-scripts"` 实现**——iframe 内用户编译的代码无法访问任何 Tauri API。防御重点应转向消除上述 XSS 升级链。

### 9 个具体修复

| 优先级 | 修复 | 行 | 严重性 |
|----------|---------|------|----------|
| **P0** | innerHTML → textContent（信号显示 + 操作列表） | 213-215, 227-229 | **CRITICAL** |
| **P0** | 添加 `e.source` + `e.origin` 验证到消息监听器 | 181, 277 | **CRITICAL** |
| **P1** | 将 postMessage `'*'` 替换为父 origin | 176, 285, 319 | HIGH |
| **P1** | 收紧能力：`core:default` → 仅 `core:event:allow-emit` + `core:event:allow-listen` | `debug-window.json:7` | HIGH |
| **P2** | srcdoc + 字符串拼接 → `URL.createObjectURL(new Blob(...))` | 324-338 | MEDIUM |
| **P2** | 去重 HTML 文件（`public/` 和 `src-tauri/` 字节完全相同） | 两个文件 | MEDIUM |
| **P3** | 重构桥接脚本为可维护代码 | 275-322 | LOW |

---

## 深度发现 #7：P39 IR 优化——最小可行方案

**Compiler Architect 的实现设计**

### 关键发现：`ComputedIntent.deps` 始终为空

在 `component.rs:48`，计算属性依赖被初始化为 `deps: vec![]` 且**从未被填充**。依赖追踪被完全跳过。这是一个比无用信号消除更基础的 bug——代码生成在运行时对计算属性的依赖进行惰性求值，但如果提供了 `deps`，优化会更准确。

### 最小可行优化：消除未使用计算属性

**检测**：遍历组件函数体、效果、视图树和剩余计算属性中的 `ExprIR` 树，以查找 `IdentifierExpr` 引用。从未在任何地方引用的计算属性可以安全移除。

**安全约束**：
- 如果在视图/函数体中被引用 ≥1 次：保留
- 如果仅在其他计算属性中被引用：递归追踪以找到传递闭包
- 如果从未被引用：移除（包括其 `source_map` 条目）

**插入点**：`analyse.rs:310`——位于 Phase 3b（IR 合并）之后，Phase 4（lint）之前

**代码量**：约 200 行（`ir/optimize.rs` + 测试）

**运动测试缺口**：需要额外 7 项测试以达到 95% 分支覆盖率（scroll、drag、InView、driven_by、paused_override、stagger_children、带项目的 timeline）

---

## 死代码清理：5 项确认删除

**Systems Architect 的 grep 审计**

| 项 | 行数 | 文件 | 裁决 |
|-----|------|------|--------|
| `CodegenCache` 结构体 | 135 | `codegen/shared/cache.rs` | **DELETE** — 零调用者 |
| `defaults::build_component_output` | 22 | `codegen/shared/ui_codegen.rs:74` | **DELETE** — 零调用者 |
| `defaults::build_store_output` | 25 | `codegen/shared/ui_codegen.rs:96` | **DELETE** — 零调用者 |
| `lower_expr_to_json`（Expr 变体） | 9 | `ir/lower/expr.rs:543` | **DELETE** — IR 变体仍活跃 |
| `timeline_body_to_config` | 15 | `codegen/omniview/motion_config.rs:117` | **DELETE** — 零调用者 |
| `callback_to_config` | 15 | `codegen/omniview/motion_config.rs:132` | **DELETE** — 零调用者 |
| `keyframe_to_config` | 40 | `codegen/omniview/motion_config.rs:18` | **KEEP** — 仅被测试使用；添加评论 |

---

## CI 缺口：精确修复

| 缺口 | 当前状态 | 修复 |
|------|--------|------|
| Clippy | CI 中无 `cargo clippy` | 添加 `cargo clippy --lib -- -D warnings` |
| Rust 格式化 | CI 中无 `cargo fmt --check` | 添加 `cargo fmt --all -- --check` |
| cargo audit 执行 | 无 `--deny warnings` | 更改为 `cargo audit --deny warnings` |
| ts-rs 绑定新鲜度 | 无检查 | 添加 `cargo test export_bindings --lib && git diff --exit-code poiesis/src/commands/__generated__/` |
| 评估脚本 | 无 CI 集成 | 添加 `bash .claude/evals/eval-run-omnilang.sh` 作业 |
| TS golden 测试 | ✅ 已存在 | 无需操作——已在 CI 中 |

---

## 实施路线图

### 第 1 天：A 组——P33 + source_map bug + 死代码

| # | 行动 | 工作量 |
|---|------|--------|
| 1 | 从 `codegen/mod.rs:117` 修复 source_map 覆盖 bug | S (1 行) |
| 2 | 在 6 个站点填充 `source_span`（ComponentIR、StoreIR、3×FunctionIR、HeroIntent） | S (6 行) |
| 3 | 删除 6 个经确认的死代码项 | S (移除约 200 行) |
| 4 | 添加 `cargo clippy --fix --lib` 以解决 19 个 auto-fixable 警告 | S (自动) |

### 第 1 周：B 组——P38b + TypeKind + CI + postMessage

| # | 行动 | 工作量 |
|---|------|--------|
| 5 | 剥离 `__TAURI_INTERNALS__`，添加后回退到 postMessage | S (1-2 小时) |
| 6 | 将 `postMessage` 来源固定为 `window.location.origin` | S (2 行) |
| 7 | 添加 Clippy + fmt + audit 门控到 CI | S (约 15 行 YAML) |
| 8 | 修复 TypeKind 序列化（结构化 JSON + 向后兼容性） | M (2-3 小时) |
| 9 | 将桥接脚本注入替换为 `URL.createObjectURL(new Blob(...))` | S (20 行) |

### 第 2-3 周：C 组——P35 + P36 + P39 基础

| # | 行动 | 工作量 |
|---|------|--------|
| 10 | **P35 第 1 阶段**：`omnilang-core` + `omnilang-codegen-omniview` + 外观 | M (1-2 天) |
| 11 | **P36**：添加 `compile_multi_target` 入口点 + rayon | M (约 50 行) |
| 12 | **P39**：创建 `ir/optimize.rs` + 消除未使用计算属性（约 200 行） | M (2-3 天) |
| 13 | 修复 41 个 Clippy 警告中剩余的手动警告 | M (2-3 小时) |

### 第 4 周：完成

| # | 行动 | 工作量 |
|---|------|--------|
| 14 | 添加 7 项运动测试以达到 95% 分支覆盖率 | M (1-2 天) |
| 15 | 完整的 P39：无用信号消除 + 简单计算属性内联 | M (3-5 天) |
| 16 | P35 第 2 阶段：提取 React 和 Flutter 后端 | M (2 天) |
| 17 | 调试窗口 HTML 去重 + `agent_docs/` 存根 | S (1 小时) |

---

## 若不解决的风险

| 风险 | 发现 | 严重性 |
|------|--------|----------|
| **逐元素源映射静默丢失** | source_map 覆盖 bug（发现 #1） | 高——工具/调试功能不可用 |
| **类型参数数据损坏** | TypeKind 序列化丢失 `type_args`（发现 #3） | 高——不正确的代码生成 |
| **source_span 虚假信心** | 37 个字段中填充了 1 个（发现 #2） | 中——维护陷阱 |
| **代码生成扩展性** | 无并行化，无 crate 隔离（发现 #4/#5） | 中——随项目大小退化 |
| **死代码累积** | 确认 6 项，约 250 行（发现 #7） | 低——噪音但浪费时间 |
| **调试窗口安全** | `'*'` postMessage + 字符串注入（发现 #6） | 中——纵深防御薄弱 |
| **Clippy 漂移** | CI 中无 Rust lint 门控 | 中——质量信号淹没 |
