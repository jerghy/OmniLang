# OmniLang 文档

> 一次编写，到处运行。将 `.omni` 编译为 React、Vue、Flutter、Python、Rust 等。

欢迎来到 OmniLang 文档！无论你是构建 CLI 工具、Web 仪表板还是全栈应用，本指南都会带你从零基础到熟练使用。

## 从哪里开始

**刚接触 OmniLang？** 按顺序阅读教程——每章都建立在前一章的基础之上：

### 轨道一：快速入门

| # | 文档 | 你将学到 |
|---|------|---------|
| 1 | [快速入门](tutorial/01-getting-started.md) | 安装 omnic、创建项目、构建和运行第一个应用 |
| 2 | [语言基础](tutorial/02-language-basics.md) | 变量、类型、函数、`if`/`while`/`for`、数组、映射 |
| 3 | [结构体与数据类型](tutorial/03-structs-and-data.md) | 结构体、枚举、泛型、带标签联合体、null 处理 |

### 轨道二：高级语言特性

| # | 文档 | 你将学到 |
|---|------|---------|
| 4 | [模块与导入](tutorial/04-modules-and-imports.md) | 导入系统、可见性、路径别名、标准库 |
| 5 | [所有权与闭包](tutorial/05-ownership-and-closures.md) | 移动语义、借用规则、闭包、async/await |
| 6 | [模式匹配](tutorial/06-pattern-matching.md) | `match` 表达式、解构、穷尽性检查 |

### 轨道三：OmniView 框架（UI）

| # | 文档 | 你将学到 |
|---|------|---------|
| 7 | [OmniView 组件](tutorial/07-omniview-components.md) | 组件、属性、视图模板、`@if`/`@for`/`@slot` |
| 8 | [响应式与状态](tutorial/08-reactivity-and-state.md) | 信号、计算属性、副作用、`bind:value`、生命周期 |
| 9 | [Store 与 Action](tutorial/09-stores-and-actions.md) | 全局状态、getter、action、多 Store 组合 |
| 10 | [样式与主题](tutorial/10-styling-and-themes.md) | 作用域 CSS、设计令牌、内联样式、响应式布局 |

### 轨道四：工具链与工作流

| # | 文档 | 你将学到 |
|---|------|---------|
| 11 | [CLI 参考手册](tutorial/11-cli-reference.md) | 每个 `omnic` 命令详解：build、run、analyse、create、serve、watch |
| 12 | [配置指南](tutorial/12-configuration.md) | `.omnilang.toml` 完整配置、目标选项、分析规则 |
| 13 | [编译目标与输出](tutorial/13-targets-and-output.md) | 10 个编译目标、输出结构、运行编译结果 |
| 14 | [VS Code 与编辑器](tutorial/14-vscode-and-editor.md) | VS Code 扩展、LSP 功能、可视化编辑器 |

### 轨道五：实战食谱

| 文档 | 你将学到 |
|------|---------|
| [食谱索引](cookbook/README.md) | 按类别组织的食谱目录 |
| [逻辑食谱](cookbook/logic-recipes.md) | CLI 工具、数据处理、算法 |
| [UI 食谱](cookbook/ui-recipes.md) | 待办应用、表单、仪表板、电商 |

---

## 参考文档

| 文档 | 受众 | 描述 |
|------|------|------|
| [语法规范](omni-grammar-spec.md) | 语言设计者、工具开发者 | 形式化语法 v2.0——源自 Rust 编译器实现 |
| [标准库](stdlib.md) | 所有用户 | `omni/math`、`omni/string`、`omni/async` API 参考 |
| [v0.1 → v0.2 迁移指南](migration-v0.1-v0.2.md) | 升级用户 | 破坏性变更与迁移步骤 |

## 内部文档

| 文档 | 描述 |
|------|------|
| [ADR-001](adr/001-omnilang-omniview-unification.md) | 架构决策：OmniLang=Dart、OmniView=Flutter 模型 |
| [E2E 验证报告](E2E_VALIDATION_REPORT.md) | 真实世界测试：5 个应用、23 个差距、10 个已修复 |
| [真实世界差距](REAL_WORLD_GAPS.md) | 已知限制（剩余 12 个非关键问题） |

## 快速链接

- **三条命令安装运行** → [QUICKSTART.md](../QUICKSTART.md)
- **示例项目** → `test/projects/`（logic-cli-tool、logic-counter、mixed-blog、view-ecommerce、view-todo）
- **项目概述** → [README.md](../README.md)
