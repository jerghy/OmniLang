# OmniLang 文档

> 一次编写，到处运行。将 `.omni` 编译为 React、Vue、Flutter、Python、Rust 等。

欢迎来到 OmniLang 文档！无论你是构建 CLI 工具、Web 仪表板还是全栈应用，本指南都会带你从零基础到熟练使用。

## 从哪里开始

**刚接触 OmniLang？** 按顺序阅读 OmniView 语法参考——每章都建立在前一章的基础之上：

### 📖 [→ OmniView 语法参考（13 章）](omniview/README.md)

| # | 章节 | 你将学到 |
|---|------|---------|
| 1 | [快速入门](omniview/01-getting-started.md) | 第一个 OmniView 组件、`omnic run`、`omnic build` |
| 2 | [类型与值](omniview/02-types-and-values.md) | 基本类型、字符串、数组、映射、可选类型 |
| 3 | [表达式与运算符](omniview/03-expressions-and-operators.md) | 算术、比较、逻辑、运算符优先级表 |
| 4 | [函数与控制流](omniview/04-functions-and-control-flow.md) | 函数定义、`if`/`while`/`for`、闭包 |
| 5 | [组件](omniview/05-components.md) | 组件定义、信号、计算属性、生命周期 |
| 6 | [模板](omniview/06-templates.md) | `@if`/`@for`/`@slot`、属性绑定、条件渲染 |
| 7 | [事件与绑定](omniview/07-events-and-bindings.md) | 事件处理、`bind:value`、事件类型参考 |
| 8 | [Store](omniview/08-stores.md) | 全局状态、getter、action、Store 组合 |
| 9 | [样式](omniview/09-styling.md) | 作用域 CSS、内联样式、CSS 变量、响应式 |
| 10 | [设计系统](omniview/10-design-system.md) | Token、Class、Variant、Theme 关键字、`design.omni` |
| 11 | [动画与动效](omniview/11-motion-animation.md) | 过渡、关键帧、滚动驱动动画 |
| 12 | [标准库](omniview/12-standard-library.md) | `std/math`、`std/string`、`std/async`、`std/http` |
| 13 | [项目结构](omniview/13-project-structure.md) | 多文件项目、模块组织、编译目标配置 |

---

## 参考文档

| 文档 | 受众 | 描述 |
|------|------|------|
| [标准库 API](stdlib.md) | 所有用户 | `omni/math`、`omni/string`、`omni/async` API 参考 |
| ~~语法规范~~（待补充） | 语言设计者、工具开发者 | 形式化语法——源自 Rust 编译器实现 |
| ~~v0.1 → v0.2 迁移指南~~（待补充） | 升级用户 | 破坏性变更与迁移步骤 |

## 内部文档

| 文档 | 描述 |
|------|------|
| [架构决策记录](../.architecture/decisions/adrs/) | 10 份 ADR，涵盖编译器管线、编辑器演进、目标优先级等 |
| [架构审查](../.architecture/reviews/) | 全工作区审查、深度审查、计划审查 |
| [设计文档](design/DESIGN_DOCS_INDEX.md) | 设计系统索引（含过时标记和历史追踪） |
| [研究文档](research/) | 框架架构评估、市场分析 |

## 快速链接

- **三条命令安装运行** → [QUICKSTART.md](../QUICKSTART.md)
- **示例项目** → `test/projects/`（logic-cli-tool、logic-counter、mixed-blog、view-ecommerce、view-todo）
- **项目概述** → [README.md](../README.md)
- **架构原则** → [.architecture/principles.md](../.architecture/principles.md)
