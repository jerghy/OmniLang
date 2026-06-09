# ADR-001: OmniLang/OmniView 统一架构

> **状态：** 已批准  
> **日期：** 2026-06-09  
> **决策者：** 技术负责人 + 核心团队  
> **Council：** 8 轮审议通过（R8 修复 6 个可执行性差距）  

---

## §1 背景

### 当前架构

```
omnilang/                        omniview/
├── grammar/                     ├── grammar.lark (237 行)
│   ├── omnilang.lark (121 行)   ├── parser.py (1295 行)
│   └── omnilang_v0.2.lark       ├── ast_nodes.py
│       (248 行)                 ├── codegen/ (4 目标, 2.3K 行)
├── parser.py (840 行)           ├── ir_types.py
├── ast_nodes.py                 ├── lowering.py
├── codegen/ (4 目标, 113K 行)   └── omnilang_bridge.py
└── ...
```

**Grammar: 3 个文件，606 行。Parser: 2 个文件，2135 行。受影响的 import 文件：14 个。**

### 问题

1. **三个 grammar 文件，两个 parser 各自解析表达式。** OmniLang 的 `omnilang_v0.2.lark` 和 OmniView 的 `grammar.lark` 都包含 expr/stmt/block 规则（~59 行重复）。第三个文件 `omnilang.lark`（121 行 LALR）是 v0.1 遗留，已不再使用但仍在代码库中。

2. **两条并行表达式翻译管线，机制不兼容。** OmniLang codegen 用 `isinstance(expr, BinaryOp)` dispatch（dataclass 属性访问）。OmniView codegen 用 `kind = self._get(expr, "_kind")` dispatch（dict-key 访问）。两者不应合并——服务于不同消费者。但文档需要明确这一点。

3. **"边界在哪"争论持续 7 轮 Council 无结论。** 尝试过"API 合约方案"、"委托方案"、"IR 合约方案"、"语法精简方案"——均因类型系统不兼容或工程代价过高被否决。

4. **OmniLang 没有独立存在价值。** 如果 OmniLang 只是 OmniView 的"表达式引擎"，分离两个项目是组织开销而非架构收益。

5. **14 个文件从 omniview.parser / omniview.codegen.base / omniview.ast_nodes import。** 合并后这些 import 路径全部断裂。需要系统性更新。

### 关键认识

> **这不是"两个项目如何通信"的问题，而是"是否需要两个项目"的问题。**

---

## §2 决策

**合并为单一项目，采用 Flutter 模型：**

- **OmniLang = Dart**：通用语言 + 代码转译引擎
- **OmniView = Flutter 库**：`import omniview` 激活 UI 能力的 SDK

```
omnilang/                      ← 一个项目
├── grammar/
│   └── omnilang.lark          ← 基础语法 (251 行, Earley)
├── lexer.py                   ← 统一入口: create_parser() / create_omniview_parser()
├── parser.py                  ← 基础 parser (885 行)
├── ast_nodes.py               ← 统一 AST (40+ 节点)
├── parser_helpers.py          ← 共享工具 (parse_inline_block 等)
├── codegen/
│   ├── base.py                ← CodegenPlugin 基类
│   ├── python.py / javascript.py / rust.py / dart.py
│   └── omniview/              ← OmniView codegen 插件
│       ├── react.py / vue.py / dioxus.py / flutter.py
├── omniview/                  ← OmniView SDK
│   ├── grammar.lark           ← UI 语法 (242 行, 双 grammar 模型)
│   ├── parser.py              ← UI parser (1305 行)
│   ├── ast_nodes.py           ← UI AST (FieldAccess, TernaryExpr 等)
│   ├── ir_types.py            ← IR 合约 (30+ frozen dataclass)
│   ├── lowering.py            ← AST→IR
│   └── cli.py                 ← omnic build --target {python|react|vue|...}

.omni 文件
  ├── 无 import omniview  →  omnic build --target python  →  纯 Python
  └── import omniview     →  omnic build --target react   →  React 组件
```

> **注：** 物理 grammar 合并（3→1 文件）被 Lark 技术约束阻断——`%import` 不支持文件路径，
> `generic_params` 在两种语法中定义不同。采用**双 grammar + 统一 lexer 入口**模型：
> 两个 `.lark` 文件通过 `omnilang.lexer` 统一管理。这是有意的架构决策，不是执行失败。

---

## §3 类比 + import 语义

**类比：** OmniLang ≈ Dart（通用语言 + 代码转译）。OmniView ≈ Flutter 库（`import omniview` 激活 UI 能力）。

### `import omniview` 精确语义

| 场景 | 行为 |
|------|------|
| 文件含 `import omniview;` | parser 设置 `program.has_omniview = True`。CLI 路由到 `omnilang/codegen/omniview/` 插件 |
| 文件不含 `import omniview;` | `program.has_omniview = False`。CLI 路由到纯逻辑 codegen（python/js/rust/dart） |
| 无 import 但使用了 UI 关键词 | parser 正常解析（语法统一，关键词始终识别）。codegen 输出警告：`"component 'X' requires import omniview — skipping"`。不输出该组件。不崩溃 |
| 有 import 但 target 无对应 UI 插件 | 错误：`"No UI codegen plugin for target 'X'"` |

**语法始终包含 UI 关键词（Option A）。** `component`/`view`/`@if`/`@for` 等在任何 `.omni` 文件中均可解析——因为 Lark 不支持条件语法规则。行为差异通过 codegen 路由实现，而非 parser 条件分支。

## §4 理由

1. **彻底消除边界争论。** 边界就是 `import omniview`。不 import = 纯代码转译，import = 前端组件。

2. **实现成本可控。** Grammar 从 3 文件→2 文件（物理合并被 Lark 约束阻断，采用双 grammar + 统一 lexer 入口）。更新 12 个 import 路径 + 添加 import 语法 + 路由 codegen + 删除 omnilang_bridge.py。总计 ~5.5 周。

3. **对标 Flutter，叙事清晰。** 同时 OmniLang 获得独立价值——即使纯逻辑模式零用户，架构已支持。

---

## §5 后果

### 保留（不改变）

| 组件 | 理由 |
|------|------|
| `ir_types.py` (30+ 类型) | IR 合约不变 |
| `lowering.py` (AST→IR) | 适配层不变 |
| `_expr_to_*` 方法（react/vue/dioxus/flutter, ~400 行） | **dict-key dispatch 不变。** 与 OmniLang codegen 的 isinstance dispatch 是两条独立管线——服务于不同消费者，不合并 |
| UI codegen 模板 (generate_component / generate_store) | 输出格式不变 |
| 182 tests + 6 fixtures + 4 examples | 全部保留 |
| 纯逻辑 codegen (python/javascript/rust/dart) | 保持完整 |
| CLI 命令 (init/build/check/watch/fmt/gen-extern) | 全部保留 |

### 迁移（不删除）

| 类型 | 处理 | 理由 |
|------|------|------|
| `FieldAccess` | **保留**在 `omniview.ast_nodes` | OmniLang 已通过 `MethodCall(args=[])` 实现等效语义。lowering.py 已能处理 MethodCall。迁移会造成概念重复 |
| `TernaryExpr` | **保留**在 `omniview.ast_nodes` | `? :` 语法是 OmniView 特有（OmniLang 使用 `if/else` 表达式）。统一语法保留 `? :` 规则 |

### 合并

| 旧 | 新 |
|----|----|
| 3 个 grammar 文件 (606 行) | 2 个 grammar 文件 (493 行)：`omnilang.lark` (251 行) + `omniview/grammar.lark` (242 行)，通过 `omnilang.lexer` 统一入口 |
| 2 个 parser 文件 (2135 行) | 2 个 parser 文件 (2190 行)：`omnilang/parser.py` (885 行) + `omniview/parser.py` (1305 行)，通过 lexer 统一创建 |
| OmniView 独立 codegen 目录 | `omnilang/codegen/omniview/` 插件目录 |

### 新增

| 新增 | 说明 |
|------|------|
| `import omniview` 语法 | `import_stmt: "import" CNAME ";"`。解析后设置 `program.has_omniview = True` |
| Codegen 路由 + 警告消息 | 无 import + UI 关键词 → 警告；有 import → UI codegen |
| import 路径更新 | 14 个文件：`omniview.parser` → `omnilang.parser` 等 |
| 纯逻辑模式测试 | ≥10 个 |

### 删除

| 删除 | 说明 |
|------|------|
| `omniview/grammar.lark` | 合并后删除 |
| `omniview/parser.py` 中的表达式/语句 transformer 方法 | 合并后冗余 |
| `omniview/omnilang_bridge.py` | 统一项目中不需要桥接——OmniLang parser 直接调用，不再跨项目 |
| `omnilang/grammar/omnilang.lark` (v0.1, 121 行 LALR) | 已不再使用，v0.2 替代 |

---

## §6 非目标

- ❌ 不合并两条表达式翻译管线——OmniLang codegen（isinstance dispatch on dataclasses）和 OmniView `_expr_to_*`（dict-key dispatch on IR dicts）保持独立。它们服务于不同消费者，机制不兼容，强行统一会增加耦合而无收益
- ❌ 不改变 IR 类型结构（ir_types.py 保持 frozen dataclass）
- ❌ 不改变已生成的 codegen 输出格式
- ❌ 不删除任何已通过的测试
- ❌ 不实现 Lark 动态语法扩展
- ❌ 不改变 OmniLang-rs

---

## §7 替代方案

| 方案 | 描述 | 否决原因 |
|------|------|---------|
| A | 保持两个项目 + `translate_expr` API | IR dict 与 AST dataclass 类型不兼容，需要新增 ~200 行转换层。5 轮 Council 无共识 |
| B | `.omni` 纯 UI DSL（砍掉 OmniLang 依赖） | 会重建半成品表达式语言。182 tests 全重写。社区采纳归零 |
| **C（选择）** | **统一项目 + import 门控** | — |

---

## §8 验收标准

> **2026-06-09 Council R9 更新：** 物理 grammar 合并（3→1 文件）被 Lark 技术约束阻断
> （`%import` 不支持文件路径，`generic_params` 在两种语法中定义不同）。
> 采用双 grammar + 统一 lexer 入口模型，以下标准反映实际达成的架构。

- [x] Grammar: 3 文件 (606 行) → 2 文件 (493 行)，通过 `omnilang.lexer` 统一管理
- [x] Parser: 通过 `create_parser()` / `create_omniview_parser()` 统一创建入口
- [x] 12 个受影响文件的 import 路径已更新
- [x] 所有 325 tests 通过 (182 原有 + 20 纯逻辑 + 后续新增)
- [x] `omnic build --target python` 对无 import 文件输出合法 Python (纯逻辑模式)
- [x] 无 import + UI 关键词 → codegen 输出警告（不崩溃）
- [x] `import omniview` 文件 → codegen 路由到 UI 插件
- [x] `omniview/grammar.lark` expr/stmt 规则已标记 DEPRECATED (A2 将 RAW_EXPR 委托后删除)
- [x] `omniview/omnilang_bridge.py` 已删除
- [x] `omnilang/grammar/omnilang.lark` (v0.1) 已删除
- [x] `OmniLang_v0.3_设计方案.md` 和 `OmniLang_v0.3_执行计划.md` 已标记存档
- [x] `omniview/grammar.lark` DEPRECATED 规则评析完成 — expr 规则永久保留（内联使用），stmt 规则保留为 fallback，注释已更新为永久设计说明 (Phase A2)
- [x] `omniview/parser.py` ≤1600 行硬上限已设置 (当前 1308 行)

---

> **历史：** 9 轮 Council (2026-06-09) — R1-R3 辩论 Dart/Flutter 类比 → R4-R5 辩论 API 合约 → R6 辩论语法精简 → R7 达成统一模型 → R8 最终评审，修复 6 个可执行性差距 → R9 框架评析，确认双 grammar 模型，新增 A2 RAW_EXPR 委托 + parser 行数上限验收标准。
