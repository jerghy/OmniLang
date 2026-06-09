# OmniLang/OmniView 架构 pivot — 文档计划

> **pivot 方向：** OmniLang = Dart（通用语言 + 代码转译引擎），OmniView = Flutter 库（import 激活 UI）  
> **旧模型（废止）：** OmniLang 和 OmniView 是两个独立语言/项目，通过"委托"边界通信  
> **新模型：** 单一项目，统一语法，`import omniview` 门控 UI codegen  
> **编制日期：** 2026-06-09 | **8 轮 Council 审议通过**  
> **状态：** 文档完成，Phase A0 审计完成 ✅ → 待执行 Phase A

---

## 架构模型（8 轮 Council 结论）

```
omnilang/                      ← 一个项目
├── grammar/
│   └── omnilang.lark          ← 统一语法（含 UI 关键词）
├── parser.py                  ← 唯一 parser
├── ast_nodes.py               ← 统一 AST（含 FieldAccess, TernaryExpr 等 UI 表达式类型）
├── codegen/                   ← codegen 插件系统
│   ├── base.py                ← CodegenPlugin 基类
│   ├── python.py              ← 纯逻辑 → Python
│   ├── javascript.py          ← 纯逻辑 → JavaScript
│   ├── rust.py                ← 纯逻辑 → Rust
│   ├── dart.py                ← 纯逻辑 → Dart
│   └── omniview/              ← OmniView codegen 插件
│       ├── react.py           ← UI → React
│       ├── vue.py             ← UI → Vue
│       ├── dioxus.py          ← UI → Dioxus
│       └── flutter.dart       ← UI → Flutter
├── omniview/                  ← OmniView SDK（UI 语义 + IR + lowering）
│   ├── elements.py
│   ├── directives.py
│   ├── ir_types.py
│   └── lowering.py
└── cli.py                     ← omnic build --target {python|react|vue|...}
```

```
.omni 文件
  │
  ├── 无 import omniview
  │     语法始终包含 UI 关键词（component/view/@if 等始终可解析）
  │     但 codegen 路由到纯逻辑 codegen（python/js/rust/dart）
  │     UI 关键词在无 import 文件中出现 → 警告 + codegen 忽略
  │
  └── import omniview
        codegen 路由到 UI codegen 插件（react/vue/dioxus/flutter）
        UI 关键词正常处理
```

### `import omniview` 语义定义（8 轮 Council 新增）

| 场景 | 行为 |
|------|------|
| 文件含 `import omniview;` | parser 设置 `program.has_omniview = True`。CLI 路由到 `omnilang/codegen/omniview/` 插件 |
| 文件不含 `import omniview;` | `program.has_omniview = False`。CLI 路由到纯逻辑 codegen |
| 无 import 但使用了 UI 关键词 | parser 正常解析（语法统一，关键词始终识别）。codegen 输出警告：`"component 'X' requires import omniview — skipping"`。不输出该组件 |
| 有 import 但 target 无对应 UI 插件 | 错误：`"No UI codegen plugin for target 'X'. Supported: react, vue, dioxus, flutter"` |

---

## 文档清单（2 份）

| # | 文档 | 文件名 | 篇幅 | 优先级 |
|---|------|--------|------|--------|
| 1 | 架构决策记录 (ADR) | `docs/adr/001-omnilang-omniview-unification.md` | ≤5 页 | **P0** |
| 2 | 执行计划 v0.4 | `docs/EXECUTION_PLAN_V0.4.md` | ≤8 页 | **P0** |
| — | Grammar 兼容性审计 | `docs/GRAMMAR_AUDIT.md` | ≤3 页 | ✅ 已完成（Phase A0 产出） |

> **不需要的文档：** 独立的 API 合约（语法即合约）。独立的迁移计划（并入执行计划 §4）。独立的指导思想（并入 ADR §1）。

---

## 文档 1：架构决策记录 (ADR)

**文件：** `docs/adr/001-omnilang-omniview-unification.md`  
**篇幅：** ≤5 页  
**优先级：** P0 阻断  
**受众：** 全团队

### 内容要求

| 章节 | 内容 |
|------|------|
| **§1 背景** | 旧架构问题：两个独立 parser、两个 grammar（485 行，~59 行表达式重复）、`_expr_to_*` 方法按 dict-key dispatch 工作（与 OmniLang codegen 的 isinstance 体系并存但不应合并）、"委托"边界 7 轮 Council 无结论 |
| **§2 决策** | 合并为单一项目。统一语法（omniview 关键词内置——始终可解析）。`import omniview` 门控 codegen 路由。OmniView 变成 OmniLang 的 codegen 插件 + UI SDK |
| **§3 类比 + import 语义** | OmniLang ≈ Dart。OmniView ≈ Flutter 库。`import omniview` 的精确语义见上方表格 |
| **§4 后果** | **保留：** ir_types.py、lowering.py、`_expr_to_*` 方法（dict dispatch 不变）、UI codegen 模板（react/vue/dioxus/flutter）、182 tests、behavior fixtures。**迁移（不删除）：** `FieldAccess`, `TernaryExpr` 从 omniview.ast_nodes → omnilang.ast_nodes。**合并：** 三个 grammar 文件 → 一个。两个 parser → 一个。**新增：** `import omniview` 语法 + codegen 路由 + import 路径更新工具。**删除：** omniview/grammar.lark、omniview/parser.py 中的表达式/语句规则、omniview/omnilang_bridge.py |
| **§5 非目标** | 不改变 IR 类型结构；不改变 `_expr_to_*` 的 dict dispatch 机制；不合并两条表达式翻译管线（OmniLang isinstance 和 OmniView dict-key 保持独立——它们服务于不同消费者）；不删除任何已通过测试 |
| **§6 替代方案** | A) 保持两个项目 + 委托 API（已否决——5 轮 Council 无共识）。B) 完全独立 .omni 语法（已否决——重建表达式语言）。C) 本方案（选择） |
| **§7 验收标准** | ① 统一 grammar 通过所有 182 tests（含 import 路径更新后）② `omnic build --target python` 输出合法 Python ③ `import omniview` 文件 → UI codegen，无 import + UI 关键词 → 警告 ④ 14 个受影响的文件 import 路径已更新 |

---

## 文档 2：执行计划 v0.4

**文件：** `docs/EXECUTION_PLAN_V0.4.md`  
**篇幅：** ≤8 页  
**优先级：** P0 阻断  
**受众：** 实现者

### 内容要求

| 章节 | 内容 |
|------|------|
| **§1 基线** | 当前状态：182 tests, 4 UI targets, 6 fixtures, 4 examples。**语法：3 个 grammar 文件** —— omnilang.lark (121 行 LALR) + omnilang_v0.2.lark (248 行 Earley) + omniview grammar.lark (237 行 Earley) = **606 行**。Parser：840 行 + 1295 行 = 2135 行。Codegen：OmniLang 113K 行 + OmniView 2.3K 行。**受影响的 import 文件：14 个**（6 测试文件 + 4 codegen 文件 + lowering + semantic + cli + preprocessor） |
| **§2 目标** | 统一 grammar ≤400 行（含 UI 关键词）。统一 parser ≤1600 行。14 个文件 import 路径更新完成。`omnic build --target python` 输出合法 Python |
| **§3 阶段** | **Phase A0（0.5 天）：grammar 兼容性审计。** 对比表达式类型覆盖、parser mode 冲突、AST 节点对齐。**Phase A（2.5 周）：语法统一。** 合并 3 个 .lark → 1 个。合并 2 个 parser → 1 个。迁移 FieldAccess/TernaryExpr 到 omnilang.ast_nodes。更新 14 个文件的 import 路径。**Phase B（1 周）：import 门控。** 添加 `import omniview` 语法。实现 codegen 路由 + 警告消息。**Phase C（1 周）：验证。** 全测试通过 + ≥10 纯逻辑测试。**Phase D（0.5 周）：文档 + 清理。** 删除 omnilang_bridge.py。标记旧文件 |
| **§4 迁移** | **保留：** ir_types.py、lowering.py、`_expr_to_*` 方法（dict dispatch 不变）、UI codegen 模板、182 tests、fixtures、examples。**迁移：** FieldAccess/TernaryExpr → omnilang.ast_nodes。**合并：** grammar × 3 → 1。parser × 2 → 1。**新增：** import 路径更新脚本（14 个文件）。**删除：** omniview/grammar.lark、omniview/parser.py 表达式规则、omniview/omnilang_bridge.py |
| **§5 风险** | ① 合并后语法歧义 → 保留 Earley 模式。② 14 个文件 ImportError → Phase A 包含路径更新任务。③ `import omniview` 语义模糊 → ADR §3 有精确行为表。④ grammar 合并被低估 → A0 审计先行，Phase A 预算 2.5 周（含缓冲） |
| **§6 回滚** | git revert。Phase A 超过 3.5 周无进展 → 回退 |
| **§7 非目标** | 不合并两条表达式翻译管线（OmniLang isinstance ≠ OmniView dict-key）；不新增前端功能；不改变 codegen 输出 |

---

## 不写的文档

| 不写 | 理由 |
|------|------|
| 独立的 API 合约 | 语法即合约。统一 grammar.lark 是 OmniLang 与 OmniView 之间的唯一接口 |
| 独立的迁移计划 | 合并入执行计划 §4 |
| 独立的指导思想 | 合并入 ADR §1-3 |

---

## 时间线

```
Week 1:     ADR + 执行计划批准
Week 2:     Phase A0（grammar 审计 0.5 天）+ Phase A（语法统一 2.5 周）
Week 4.5:   Phase B（import 门控 1 周）
Week 5.5:   Phase C（验证 1 周）
Week 6:     Phase D（文档 + 清理 0.5 周）
```

**总计：~6 周**（含 1 周文档审批 = ~7 周）

## 验收标准

- [x] ADR 被团队审阅批准
- [x] 统一 grammar ≤400 行 (omnilang.lark ~250行)
- [x] 12 个受影响文件的 import 路径已更新 (审计修正: 14→12)
- [x] FieldAccess、TernaryExpr 保留在 omniview.ast_nodes (审计结论: OmniLang MethodCall 已等效)
- [x] 所有 202 tests 通过 (182 原有 + 20 纯逻辑)
- [x] `omnic build --target python` 对无 import 文件输出合法 Python (纯逻辑模式)
- [x] 无 import + UI 关键词 → codegen 输出警告（不崩溃）
- [x] `import omniview` 文件 → codegen 路由到 UI 插件
- [x] omniview/grammar.lark expr/stmt 规则已标记 DEPRECATED (物理删除延后至 RAW_EXPR 委托)
- [x] omniview/omnilang_bridge.py 已删除
- [x] 新增 20 个纯逻辑模式测试
