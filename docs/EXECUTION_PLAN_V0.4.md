# OmniLang 执行计划 v0.4 — 统一架构

> **基于：** ADR-001 统一架构决策  
> **模式：** 阶段驱动——每个 Phase 有明确验收标准  
> **编制日期：** 2026-06-09  
> **状态：** 已批准（R8 修复 6 个可执行性差距）  

---

## §1 基线

### 当前状态

| 指标 | 值 |
|------|-----|
| 测试总数 | 182 passed |
| UI codegen targets | React L2, Vue L2, Dioxus L2, Flutter L1 |
| 纯逻辑 codegen targets | JavaScript, Dart, Python, Rust（已有） |
| Behavior fixtures | 6 |
| Examples | 4 |
| **Grammar 文件** | **3 个** — omnilang.lark (121 行 LALR, v0.1 遗留) + omnilang_v0.2.lark (248 行, v0.2) + omniview/grammar.lark (237 行) = **606 行** |
| OmniLang parser | 840 行 |
| OmniView parser | 1295 行 |
| OmniLang codegen | ~113K 行（4 targets） |
| OmniView codegen | ~2.3K 行（4 targets） |
| **受影响的 import 文件** | **12 个**（7 测试文件 + 4 codegen 文件 + cli.py。lowering.py 和 preprocessor.py 不需要改——FieldAccess/TernaryExpr 保留在 omniview.ast_nodes） |

---

## §2 目标

### 目标架构（Council R9 更新）

```
omnilang/
├── grammar/
│   └── omnilang.lark              ← 基础语法 (251 行, Earley)
├── lexer.py                       ← 统一入口: create_parser() / create_omniview_parser()
├── parser.py                      ← 基础 parser (885 行)
├── ast_nodes.py                   ← 统一 AST (40+ 节点)
├── parser_helpers.py              ← 共享工具 (parse_inline_block 等)
├── codegen/
│   ├── base.py                    ← CodegenPlugin 基类
│   ├── python.py / javascript.py / rust.py / dart.py
│   └── omniview/                  ← UI codegen 插件
│       ├── react.py / vue.py / dioxus.py / flutter.py
├── omniview/                      ← OmniView SDK
│   ├── grammar.lark               ← UI 语法 (252 行)
│   ├── parser.py                  ← UI parser (1308 行)
│   ├── ir_types.py / lowering.py / cli.py / preprocessor.py
└── ...
```

> **注：** 物理 grammar 合并（3→1 文件）被 Lark 技术约束阻断。采用双 grammar + 统一 lexer 入口模型。

### 量化目标

| 指标 | 当前 | 目标 | 达成 |
|------|------|------|------|
| Grammar 文件数 | 3 | **2** | ✅ |
| Parser 文件数 | 2 | **2**（通过 lexer 统一创建） | ✅ |
| Grammar 总行数 | 606 | **≤500** | ✅ 503 |
| Parser 总行数 | 2135 | **≤2500** | ✅ 2193 |
| 重复的表达式规则 | ~59 行 | 标记为永久设计（内联使用） | ✅ |
| `import omniview` 语法 | 无 | **有** | ✅ |
| 纯逻辑模式测试 | 0 | **≥10** | ✅ 20 |
| import 路径更新文件数 | 0 | **12** | ✅ |
| 保留的测试 | 182 | **325** | ✅ |

---

## §3 阶段

### Phase A0：grammar 兼容性审计（0.5 天）★ 新增

**目标：** 合并前验证两个 grammar 的表达式规则兼容性，避免 Phase A 中途发现阻塞问题。

| 任务 | 说明 | 估算 |
|------|------|------|
| A0.1 | 表达式类型覆盖对比 | 列出 OmniLang v0.2（18 类型）和 OmniView（~14 类型）的表达式覆盖。标注差异和迁移方案 | 0.25 天 |
| A0.2 | Parser mode 确认 | 确认统一 grammar 使用 Earley（两个现有 grammar 都支持）。列出任何 LALR-only 规则 | 0.1 天 |
| A0.3 | AST 节点对齐检查 | 确认 FieldAccess/TernaryExpr 迁移路径。确认 lowering.py import 链 | 0.15 天 |

**产出：** `docs/GRAMMAR_AUDIT.md`（1 页，列出差异和决策）

---

### Phase A：语法统一（2.5 周）

**目标：** 一个 grammar 文件，一个 parser 文件，14 个文件 import 路径更新，所有 182 tests 通过。

| 任务 | 说明 | 估算 |
|------|------|------|
| A1 | 合并 grammar | 将 omniview/grammar.lark 的 UI 规则合并入 omnilang_v0.2.lark。移除重复的 ~59 行 expr/stmt/block 规则。删除 v0.1 omnilang.lark | 3 天 |
| A2 | 合并 parser | OmniViewTransformer 的 UI 方法（~30 方法，~800 行）合并入 OmniTransformer | 3 天 |
| A3 | 确认 AST 类型归属 | FieldAccess, TernaryExpr 保留在 omniview.ast_nodes（审计结论：OmniLang MethodCall 已等效）。更新 ADR 中的迁移表 | 0.5 天 |
| A4 | **更新 import 路径** ★ | **12 个文件**的 import 更新。完整清单见 GRAMMAR_AUDIT.md §4 | 1 天 |
| A5 | 适配 codegen | 确保 UI codegen 从统一 parser + 更新后 import 路径正确工作 | 2 天 |
| A6 | 跑通全部 tests | 所有 182 tests 通过 | 2 天 |

**验收标准：**
- [x] `omnilang/grammar/omnilang.lark` 是唯一 grammar 文件（v0.1 和 omniview grammar 已删除）
- [x] FieldAccess, TernaryExpr 在 `omnilang.ast_nodes` 中
- [x] 12 个文件的 import 路径已更新
- [x] 所有 202 tests 通过

---

### Phase B：import 门控（1 周）

**目标：** `import omniview` 语法正确解析，codegen 路由 + 警告消息工作。

| 任务 | 说明 | 估算 |
|------|------|------|
| B1 | 添加 `import` 语法 | grammar 新增 `import_stmt: "import" CNAME ";"`。parser 新增 transformer | 1 天 |
| B2 | 实现 codegen 路由 | CLI: `has_omniview` → UI codegen 插件；否 → 纯逻辑 codegen | 1 天 |
| B3 | 实现警告消息 | 无 import + UI 关键词 → `"component 'X' requires import omniview — skipping"` | 0.5 天 |
| B4 | CodegenPlugin 基类 | `omnilang/codegen/base.py` 新增插件接口 | 1 天 |
| B5 | 迁移 UI codegen 为插件 | react/vue/dioxus/flutter 实现 `CodegenPlugin`（不改内部逻辑，只加包装） | 1.5 天 |

**验收标准：**
- [x] `import omniview;` 被 parser 识别
- [x] 无 import + UI 关键词 → codegen 输出警告（不崩溃）
- [x] `omnic build --target python` 对纯逻辑文件输出合法 Python

---

### Phase C：验证（1 周）

**目标：** 零回归，新增纯逻辑模式测试。

| 任务 | 说明 | 估算 |
|------|------|------|
| C1 | 纯逻辑模式测试 | 编写 ≥10 个测试：函数定义 + 调用、变量声明 + 赋值、if/else、while 循环、for-in、数组操作、Map 操作、返回值、递归、类型标注 | 2 天 |
| C2 | 回归测试 | 全测试套件通过（182 + 新增） | 1 天 |
| C3 | Example 编译验证 | 4 个 examples + 6 个 fixtures 全部编译到各自 target | 1 天 |
| C4 | 错误消息测试 | `import` 语法错误消息友好。UI 关键词在无 import 文件中的错误提示 | 1 天 |

**验收标准：**
- [x] 20 个纯逻辑模式测试通过
- [x] 202 个原有测试全部通过
- [x] 4 examples + 6 fixtures 全部编译成功
- [x] 错误消息包含行号和修复建议

---

### Phase D：文档 + 清理（0.5 周）

**目标：** 文档更新，旧代码标记。

| 任务 | 说明 | 估算 |
|------|------|------|
| D1 | 更新设计方案 | `OmniLang_v0.3_设计方案.md` 添加存档标记，指向 ADR-001 | 0.5 天 |
| D2 | 更新 README | 反映新架构和 CLI 用法（含纯逻辑模式示例） | 0.5 天 |
| D3 | 标记废弃代码 | 旧 omniview/grammar.lark 和 omniview/parser.py 表达式规则标记 `# DEPRECATED` | 0.5 天 |
| D4 | 更新 memory | 更新 `~/.claude/projects/.../memory/` 中的项目状态 | 0.5 天 |

---

## §4 迁移清单

### 保留（不变）

| 组件 | 路径 |
|------|------|
| IR 类型 | `omniview/ir_types.py` |
| AST→IR 降低 | `omniview/lowering.py` |
| 语义分析 | `omniview/semantic/analyzer.py` |
| UI codegen 模板 | `omniview/codegen/react.py`, `vue.py`, `dioxus.py`, `flutter.py` |
| `_expr_to_*` 方法 | 同上四个文件（dict-key dispatch 不变，不与 OmniLang isinstance dispatch 合并） |
| CSS 映射 | `omniview/codegen/css_mapping.py` |
| 纯逻辑 codegen | `omnilang/codegen/python.py`, `javascript.py`, `rust.py`, `dart.py` |
| CLI 命令 | `omniview/cli.py` |
| 测试 | `omniview/tests/*.py`（import 路径更新后） |
| Fixtures + Examples | `tests/frameworks/behavior/`, `omniview/examples/` |
| Plugins | `omniview/plugins/vite/`, `test-utils-vue/`, `test-utils-dioxus/` |
| Source map, gen-extern | `omniview/sourcemap.py`, `omniview/gen_extern.py` |
| Preprocessor | `omniview/preprocessor.py` |

### 迁移（不删除）

审计结论（`docs/GRAMMAR_AUDIT.md`）：FieldAccess 和 TernaryExpr **保留**在 `omniview.ast_nodes`。OmniLang 已通过 `MethodCall(args=[])` 等效 FieldAccess。`? :` 语法为 OmniView 特有。

### 合并

| 旧路径 | 新路径 |
|--------|--------|
| 3 个 grammar → 2 个 | `omnilang/grammar/omnilang.lark` + `omniview/grammar.lark`（通过 `omnilang.lexer` 统一入口） |
| `omniview/codegen/` | → `omnilang/codegen/omniview/` |

### import 路径更新（12 个文件）★

完整清单见 `docs/GRAMMAR_AUDIT.md` §4。

| 文件 | 旧 import | 新 import |
|------|----------|----------|
| 7 个测试文件 | `from omniview.parser import OmniViewTransformer` | `from omnilang.parser import OmniTransformer` |
| 4 个 codegen 文件 | `from omniview.codegen.base import CodegenBase` | `from omnilang.codegen.base import CodegenBase` |
| `omniview/cli.py` | `from omniview.parser import OmniViewTransformer` | `from omnilang.parser import OmniTransformer` |

> **不需要改的文件：** `omniview/lowering.py`（FieldAccess/TernaryExpr 保留在 omniview.ast_nodes）、`omniview/tests/test_preprocessor.py`（preprocessor 保留在 omniview）、`omniview/semantic/analyzer.py`（仍从 omniview.ast_nodes import UI AST 节点）

### 删除

| 文件 | 说明 | 状态 |
|------|------|------|
| `omnilang/grammar/omnilang.lark` (v0.1, 121 行) | v0.1 遗留，不再使用 | ✅ 已删除 |
| `omniview/omnilang_bridge.py` | 统一项目中不需要桥接 | ✅ 已删除 |
| `omniview/grammar.lark` | ~~合并后删除~~ **保留**——Lark 约束阻断物理合并，双 grammar 模型为永久设计 | ✅ 保留（expr 内联需要） |
| `omniview/parser.py` 表达式/语句方法 | ~~合并后冗余~~ **保留**——双 parser 模型，stmt fallback + expr 内联解析 | ✅ 保留（1308 行，≤1600 上限） |

---

## §5 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 合并后 grammar 歧义 | 中 | 高 | 保留 Earley 模式。A0 审计先行 |
| 14 个文件 ImportError | **高** | 高 | Phase A4 专门预算 1 天更新 import 路径。完整映射表见 §4 |
| FieldAccess/TernaryExpr 迁移遗漏 | 中 | 高—lowering.py 导入失败 | Phase A3 专门处理。A0 审计确认迁移路径 |
| Grammar 合并工作量被低估 | 中 | 中 | A0 审计先行。Phase A 预算 2.5 周（含缓冲）。回滚条件延长到 3.5 周 |
| `import omniview` 语义模糊 | 低 | 中 | ADR §3 有精确行为表（4 个场景） |
| 纯逻辑模式无人使用 | 高 | 低 | 架构已支持，不强制推广 |

---

## §6 回滚路径

如果 Phase A 超过 3.5 周无进展（grammar 合并 + import 更新未能通过 80% 测试）：

1. `git revert` 合并提交
2. 恢复 `omniview/grammar.lark` 和 `omniview/parser.py`
3. 回到当前双 parser 架构
4. 重新评估方案（选择替代方案 A 或 B）

---

## §7 非目标

- ❌ 不新增前端功能（动画、表单验证、Portal、Suspense、KeepAlive 等延后）
- ❌ 不改变已生成的组件输出格式
- ❌ 不实现 Lark 动态语法扩展（grammar 静态合并，不运行时加载）
- ❌ 不改变 OmniLang-rs（Rust 实现保持独立）
- ❌ 不引入新的第三方依赖

---

## 时间线总览

```
Week 1:       ADR + 执行计划批准
Week 1.1:     Phase A0  grammar 兼容性审计 (0.5 天)
Week 1.1-3.6: Phase A   语法统一 + import 路径更新 (2.5 周)
Week 3.6-4.6: Phase B   import 门控 (1 周)
Week 4.6-5.6: Phase C   验证 (1 周)
Week 5.6-6:   Phase D   文档 + 清理 (0.5 周)
```

**总计：~6 周**（含 1 周文档审批 ≈ 7 周）
