# OmniView 语法参考文档

OmniView 是 OmniLang 语言的 UI 框架层，用于构建响应式 Web 应用。本文档从零开始，涵盖语言基础和框架特性。

## 文档导航

### 第一部分：入门

| 文件 | 内容 |
|------|------|
| [01-快速入门](01-getting-started.md) | 第一个组件、项目结构、编译运行 |

### 第二部分：语言基础

| 文件 | 内容 |
|------|------|
| [02-类型与值](02-types-and-values.md) | 基础类型、struct、tagged union、可选类型、类型别名 |
| [03-表达式与运算符](03-expressions-and-operators.md) | 运算符优先级、闭包/lambda、match、可选链、展开 |
| [04-函数与控制流](04-functions-and-control-flow.md) | `fn`、`async`/`await`、循环、`let`/`let mut`、`try`/`catch` |

### 第三部分：UI 框架

| 文件 | 内容 |
|------|------|
| [05-组件](05-components.md) | `component`、`prop`、`signal`、`computed`、生命周期、`ref` |
| [06-模板语法](06-templates.md) | `view` 块、HTML 元素、插值、属性绑定、`@if`/`@for`/`@slot` |
| [07-事件与绑定](07-events-and-bindings.md) | 事件处理器、`emit`、`bind:value`/`bind:checked`、事件数据 |
| [08-Store 状态管理](08-stores.md) | `store` 定义、全局/内联 store、`@store` 引用、持久化 |
| [09-样式系统](09-styling.md) | `style` 块、CSS 属性、CSS 变量、内联样式、作用域 |
| [10-设计系统](10-design-system.md) | `design` 块、设计令牌、CSS 类、`variant` 组件变体 |

### 第四部分：高级特性

| 文件 | 内容 |
|------|------|
| [11-运动与动画](11-motion-animation.md) | `@motion`、`@timeline`、触发器、缓动、`states:`、Hero |
| [12-标准库参考](12-standard-library.md) | `std/*` 全部 7 个模块 + 内置方法 |
| [13-项目结构与模块](13-project-structure.md) | 文件组织、`import`/`export`、`pub` 可见性、模块系统 |

### 第五部分：语法规范（面向工具开发者）

| 文件 | 内容 |
|------|------|
| [14-语法规范-核心语言](14-grammar-lang.md) | BNF 产生式：词法、类型、表达式、语句、顶层声明、模块系统 |
| [15-语法规范-UI框架](15-grammar-view.md) | BNF 产生式：组件、Store、视图模板、指令、样式、设计系统、动画 |

## 核心概念

OmniView 将以下概念提升为**语言第一公民**（Language-First Citizens）：

| 概念 | 关键字 | 编译行为 |
|------|--------|----------|
| 组件 | `component` | 编译为 JS 工厂函数 + 内联 HTML |
| 信号 | `signal` | 编译为 `__omni_signal()` getter/setter |
| Store | `store` | 编译为全局 JS 变量（`{StoreName}_{field}` 命名） |
| 设计系统 | `design` | 编译为 `:root { --token }` + scoped CSS |
| 动画 | `@motion` / `@timeline` | 编译为 WAAPI `el.animate()` 调用 |
| 类型 | `struct` / `type` | 编译时擦除，仅用于类型检查 |

## 编译器命令

```bash
omnic run                              # 开发服务器（自动编译 + 热更新）
omnic build --target omniview          # 构建到 OmniView（HTML/CSS/JS）
omnic build --target react             # 构建到 React（TSX + CSS Modules）
```

## 文件类型

- **UI 文件**：以 `import omniview;` 开头，可包含 component、store、design
- **纯逻辑文件**：不写 `import omniview;`，只包含 `fn`、`struct`、`type`、`pub` 导出
- **入口文件**：通常是 `src/main.omni`，作为编译入口

## 推荐项目结构

```
src/
├── main.omni              # 入口 + App 根组件
├── design.omni            # 全局设计系统（LearnDesign）
├── lib.omni               # 入口级工具函数
├── lib/                   # 纯逻辑库模块
│   ├── types.omni         # pub struct + pub type 定义
│   └── api.omni           # API 调用
├── stores/                # 全局状态管理
│   ├── TodoStore.omni
│   └── ThemeStore.omni
├── components/            # 可复用 UI 组件
│   ├── Header.omni
│   └── Counter.omni
└── pages/                 # 页面级组件
    ├── Dashboard.omni
    └── SettingsPage.omni
```
