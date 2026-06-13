# OmniDriver Skill TDD 改进报告 V3

## 基于 superpowers:writing-skills 方法论的第三轮创新评估

**日期**: 2026-06-13
**方法**: superpowers:writing-skills（RED-GREEN-REFACTOR）
**测试代理**: 4 baseline + 2 GREEN verification = 6 agents

---

## 1. 执行摘要

对 OmniDriver skill 进行了第三轮 TDD 改进。本轮聚焦于**创新性场景设计**——测试 AI agent 在高级使用场景中的表现：语义化构建、布局架构、数据流水线、画布自诊断。

**核心发现**：V2 skill 已消除大部分基础错误（O.help() 跳过率 0%、无根容器率 0%、validate() 调用率 100%）。但 **6 个新的中高级失败模式** 暴露出来，均与"组合思维"相关——agent 知道每个 API 但不知道如何优雅地组合它们。

**GREEN 阶段** 在 SKILL.md 中新增了 4 个创新章节：📖 Recipe Book、📐 Layout Architecture Patterns、🔄 Progressive Build-Test Loop、🏷️ Naming Convention。GREEN 验证通过率 **95.8%**（23/24）。

---

## 2. RED 阶段：Baseline 测试结果

### 2.1 创新场景设计

| 场景 | 创新点 | 测试目标 |
|------|--------|---------|
| **Product Catalog** | 复杂组件选择 + 搜索过滤 + 购物车 + 产品卡片 | 语义化构建能力 |
| **Dashboard Layout** | Sidebar+Main 布局 + Stats 卡片 + 数据表格 | 布局架构理解 |
| **Live Filter** | Input→Signal→Computed→Display 完整流水线 | 数据流水线编排 |
| **Fix Broken Counter** | 给定 4 个 bug 要求诊断修复 | 自诊断 + 自修复能力 |

### 2.2 Baseline 失败矩阵

| # | 失败模式 | 严重度 | Agent 1 | Agent 2 | Agent 3 | Agent 4 | 命中率 |
|---|---------|--------|---------|---------|---------|---------|--------|
| 1 | **从不检查 snippet()** | 🟡 MEDIUM | ❌ | ❌ | ❌ | ❌ | **100%** |
| 2 | **硬编码显示值** | 🟡 MEDIUM | ✅ | ❌ | ⚠️ | ❌ | **67%** |
| 3 | **一次性构建无增量验证** | 🟡 MEDIUM | ❌ | ❌ | ❌ | ✅ | **75%** |
| 4 | **过度使用自定义 CSS props** | 🟢 LOW | ⚠️ | ❌ | ✅ | N/A | **50%** |
| 5 | **修复时用 connect() 而非 createEventHandler()** | 🟡 MEDIUM | ✅ | ✅ | ✅ | ❌ | **25%** |
| 6 | **假设 OmniLang 语法（JS-like）** | 🟢 LOW | ✅ | N/A | ❌ | ❌ | **67%** |

### 2.3 V2 vs V3 失败模式对比

| V2 失败模式（6 项） | V3 状态 | V3 新失败模式（6 项） |
|-------------------|---------|---------------------|
| 跳过 O.help() (50%) | ✅ 已消除 (0%) | 不检查 snippet() (100%) |
| 无根容器 (50%) | ✅ 已消除 (0%) | 硬编码显示值 (67%) |
| 缺失事件处理 (25%) | ✅ 已消除 (0%) | 无增量验证 (75%) |
| 一对多连线 (25%) | ✅ 已消除 (0%) | CSS props 过度 (50%) |
| 无 validate() (50%) | ✅ 已消除 (0%) | 手动 connect (25%) |
| Handler 源码语法错误 (100%) | ✅ 已消除 (0%) | OmniLang 语法假设 (67%) |

### 2.4 Agent 行为分析

**Agent 1 (Product Catalog) — 良好表现 (8/10)：**
- 正确使用 Flex 布局产品网格
- 为每个产品创建了独立的 handler（onAddToCart1/2/3），而不是参数化复用
- 没有检查 O.snippet() 或 O.listSnippets()
- 使用 `O.text("", {}, parentId)` 作为间距元素（创造性 workaround，但不 idiomatic）

**Agent 2 (Dashboard Layout) — 最佳表现 (9/10)：**
- 复杂的 sidebar+main 布局架构（Flex row → Flex column × 2）
- 创建了辅助函数 `wireNavItem()` 和 `buildTableRow()` — DRY 原则
- 大量使用自定义 CSS props（backgroundColor, textAlign, letterSpacing, cursor, borderLeft）
- Stats 卡片显示硬编码字符串而非 signal 绑定值
- 没有检查 snippet()

**Agent 3 (Live Filter) — 最佳表现 (9/10)：**
- 完整的 Input→Signal→Computed→Display 数据流水线
- 两个 computed 块分离关注点（filterCount + filterResult）
- 真实的 OmniLang 源码（非注释）
- Computed 源码使用了 JS-like 语法（`for` loop, `.toLowerCase().contains()`），可能不是有效 OmniLang
- 没有检查 snippet()

**Agent 4 (Fix Broken Counter) — 良好表现 (8/10)：**
- 正确诊断所有 4 个 bug
- 使用 O.validate() 进行健康检查
- 正确断开错误连线并重新连接
- **但用了手动 O.connect() 而不是 createEventHandler()** — 这是 V3 的关键发现！
- 假设显示绑定 `{count}` 语法可行

---

## 3. GREEN 阶段：Skill 创新改进

### 3.1 SKILL.md 新增章节

| 章节 | 行数 | 针对的失败模式 |
|------|------|--------------|
| **📖 Recipe Book** | ~70 行 | #1 (snippet awareness), #2 (hardcoded values) |
| **📐 Layout Architecture Patterns** | ~30 行 | #4 (CSS prop overuse) |
| **🔄 Progressive Build-Test Loop** | ~35 行 | #3 (no incremental validation) |
| **🏷️ Naming Convention** | ~25 行 | 新增（语义化命名最佳实践） |

### 3.2 Recipe Book（本轮核心创新）

**创新点**：将 API 参考升级为**完整可运行的代码模板**。4 个 Recipe 覆盖最常见的 UI 模式。

```
Recipe 1: Counter          — 按钮 + signal + handler 的最小完整示例
Recipe 2: Search Filter    — Input→Signal→Computed 数据流水线
Recipe 3: Form Submit      — 多输入 + 提交按钮 + 状态 signal
Recipe 4: Toggle           — 布尔 signal 的开关模式
```

**Snippet 优先原则**：`O.listSnippets()` → Recipe Book → 从零构建

**为什么有效**：Agent 不再是"知道每个积木的形状但不知道怎么搭"，而是可以直接复制粘贴 Recipe 并适配。每个 Recipe 包含完整的布局嵌套、事件处理和数据流连线。

**设计原则**：
- 每个 Recipe <20 行代码（可直接复制粘贴）
- 使用语义化命名（cartCount, handleAddToCart）
- 包含所有 3 层：UI → 事件处理 → 数据流
- 注释标注了 OmniLang 源码模板

### 3.3 Layout Architecture Patterns（布局创新）

**创新点**：不再只是"用 parentId 嵌套"，而是提供 4 种标准布局架构的蓝图。

```
Pattern A: Sidebar + Main    → 后台管理系统
Pattern B: Header + Content  → 单页应用
Pattern C: Card Grid         → 产品展示
Pattern D: Form Layout       → 数据录入
```

**Card vs Flex vs Container 决策指南**：
- `Card` = 视觉分组（边框阴影）
- `Flex` = 方向排列（纯布局）
- `Container` = 纯包裹（无样式无方向）

### 3.4 Progressive Build-Test Loop（方法论创新）

**创新点**：不再是一口气构建全部 UI 再验证，而是每 3-5 个元素验证一次。

```
Phase 1: 根容器 + 标题    → validate()
Phase 2: 按钮 + handler   → validate()
Phase 3: 数据流连线        → validate()
Phase 4: 显示元素          → validate()
```

**为什么有效**：当 validate() 报告问题时，agent 知道是最近 3-5 步引入的，而不是在 50 个元素的画布上大海捞针。

### 3.5 Naming Convention（可维护性创新）

| 类型 | 规则 | 好例子 | 坏例子 |
|------|------|--------|--------|
| signal | camelCase 名词 | searchQuery, cartCount | sig1, data, x |
| action | camelCase 动词 | handleSearch, incrementCounter | handler1, fn |
| computed | 描述性名词 | filteredItems, totalPrice | comp1, temp |
| store | 复数名词 | products, users | store1, list |

### 3.6 更新的检查清单

| 新增检查项 | 针对的失败模式 |
|-----------|--------------|
| O.listSnippets() 是否检查过？ | 失败 #1 |
| signal/action/computed 名称是否语义化？ | 新增 |
| 是否每 3-5 个元素就验证一次？ | 失败 #3 |

| 增强的规则 | 变更 |
|-----------|------|
| Anti-Pattern: 一个 Handler 连多个 Signal | 新增：无论源码中更新了多少 signal，result 最多一条 wire |
| Red Flags | +3 项（snippet 忽略、一次性构建、手动 connect） |
| Common Mistakes | +3 项（snippet 忽略、无增量验证、手动 connect、无意义命名） |

---

## 4. GREEN 验证结果

| 检查项 | RED Agent 1 | RED Agent 2 | RED Agent 3 | RED Agent 4 | GREEN Agent 1 | GREEN Agent 2 |
|--------|------------|------------|------------|------------|--------------|--------------|
| O.help() 第一步 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 根容器 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| O.listSnippets() 检查 | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| createEventHandler() 而非手动 | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| 语义化命名 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Progressive Build-Test Loop | ❌ | ❌ | ❌ | ✅ | ✅ (4 phases) | ✅ (2 phases) |
| Recipe Book 使用 | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| result→set 端口正确 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 单一 result→单一 set | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ (连了2个) |
| 类型注释 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| O.validate() 最终 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| text() 3 参数 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

**GREEN 通过率: 95.8% (23/24)**

---

## 5. REFACTOR 发现与修复

### 5.1 唯一残留问题

**GREEN Agent 2** 连接了 handler.result → todos.set **AND** handler.result → todoCount.set，即使 handler 源码中已经更新了两个 signal。

**Agent 的合理化借口**："Technically valid but poor design."

**修复**：在 Anti-Pattern 部分新增铁律：
> **无论 action 源码中更新了多少个 signal，`result` 端口最多只能有一条出站 wire。**

同时在 Red Flags 中新增映射条目：
> "result 连多个信号是技术可行的" → OmniDriver 会 warn。一个 result 端口=一条 wire。

---

## 6. 指标对比

| 指标 | V1 报告 | V2 报告 | V3 报告 | 变化 |
|------|---------|---------|---------|------|
| Baseline 测试场景 | 3 | 4 | 4 | — |
| 测试 Agent 总数 | 3 | 6 | 6 | — |
| 发现的失败模式 | 6 | 7 | 6 (新) | V2 模式全部消除 |
| Skill 行数 | ~450 | ~680 | ~882 | +202 |
| 新增章节 | 0 | 4 | 4 | Recipe Book + Layout + Build-Test + Naming |
| GREEN 通过率 | 未系统测试 | 100% (14/14) | 95.8% (23/24) | 更严格标准 |
| Snippet 检查率 | N/A | N/A | RED: 0% → GREEN: 100% | **新指标** |
| 增量验证率 | N/A | N/A | RED: 25% → GREEN: 100% | **新指标** |

---

## 7. 创新性尝试总结

### 7.1 Recipe Book（📖 模板库模式）

**问题**：Agent 知道 API 但每次从零构建，效率低、易出错。

**创新**：提供 4 个完整 Recipe，每个 <20 行，覆盖 UI + 事件 + 数据流。Agent 可以复制粘贴并适配。

**验证**：RED 阶段 0% agent 检查 snippet/recipe → GREEN 阶段 100% 先检查。

### 7.2 Progressive Build-Test Loop（🔄 渐进式构建）

**问题**：Agent 一次性构建 30+ 元素，validate() 报错时找不到根因。

**创新**：每 3-5 个元素验证一次，问题出现时知道是最近几步引入的。

**验证**：RED 阶段 75% agent 一次性构建 → GREEN 阶段 100% 使用渐进式。

### 7.3 Layout Architecture Patterns（📐 布局蓝图）

**问题**：Agent 过度使用自定义 CSS props 弥补布局理解的不足。

**创新**：提供 4 种标准布局架构的蓝图（Sidebar+Main、Header+Content、Card Grid、Form Layout），附带 Card/Flex/Container 决策指南。

### 7.4 Naming Convention（🏷️ 语义化命名）

**问题**：虽然不是本轮 RED 的失败模式，但预防性添加可以避免未来的可维护性问题。

**创新**：signal/action/computed/store/变量的命名规则表，附带好/坏对比示例。

### 7.5 createEventHandler First Rule（🔌 强化）

**问题**：Agent 在修复画布时倾向使用手动 `O.connect()` + UUID，而不是推荐的 `createEventHandler()`。

**创新**：在 Common Mistakes 和 Red Flags 中强调"始终优先用 createEventHandler()"。

---

## 8. 本轮 Skill 变更总结

### SKILL.md 新增

| 新增 | 说明 | 行数 | 针对 |
|------|------|------|------|
| 📖 Recipe Book | 4 个完整可运行模板 | ~70 行 | snippet 忽略、硬编码值 |
| 📐 Layout Architecture Patterns | 4 种标准布局蓝图 | ~30 行 | CSS props 过度 |
| 🔄 Progressive Build-Test Loop | 渐进式构建方法论 | ~35 行 | 无增量验证 |
| 🏷️ Naming Convention | 语义化命名规则 | ~25 行 | 预防性 |

### SKILL.md 增强

| 增强 | 说明 |
|------|------|
| Anti-Pattern 铁律 | result 端口最多一条出站 wire（无论源码更新多少 signal） |
| Red Flags | +4 项（snippet、一次性构建、手动 connect、result 多连） |
| Common Mistakes | +3 项（snippet、增量验证、手动 connect、命名） |
| Self-Check | +3 项（snippet 检查、命名规范、增量验证） |

---

## 9. 建议

### 9.1 代码改进（新发现）

| 优先级 | 功能 | 说明 |
|--------|------|------|
| P2 | `O.listSnippets()` 返回结果中包含 Recipe 引用 | Agent 更容易发现 Recipe Book |
| P2 | `O.validate()` 报告 fan-out wire 数量 | 在当前实现中 console.warn 已有 fan-out 检测，validate() 可加入 |

### 9.2 Skill 维护

1. **每季度运行 baseline** — 新模型可能有新的组合模式失败
2. **扩充 Recipe Book** — 收集真实使用中的常见模式，加入新 Recipe
3. **监控 snippet 使用率** — 如果 agent 从不使用 snippet，说明发现机制不够好
4. **A/B 测试 Recipe 格式** — inline code vs reference link，哪个成功率更高？

---

*报告由 superpowers:writing-skills TDD 方法论生成。RED 阶段 4 agent × 4 场景。GREEN 阶段 2 agent × 2 场景验证。REFACTOR 关闭 1 个残留 loophole。*
