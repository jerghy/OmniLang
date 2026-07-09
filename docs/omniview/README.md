# OmniView 语法参考文档

OmniView 是 OmniLang 语言的 UI 框架层，用于构建响应式 Web 应用。本文档涵盖 OmniView 的完整语法参考。

## 文档结构

| 文件 | 内容 |
|------|------|
| [01-项目结构与模块](01-project-structure.md) | 文件组织、`import`/`export`、模块系统 |
| [02-组件](02-components.md) | `component` 定义、signals、computed、functions、lifecycle |
| [03-模板语法](03-templates.md) | `view` 模板、HTML 元素、`@if`/`@for`/`@slot` 指令 |
| [04-响应式系统](04-reactivity.md) | `signal`、`computed`、`getter`、数据绑定 |
| [05-Store 状态管理](05-stores.md) | `store` 定义、全局/内联 store、`@store` 引用 |
| [06-事件系统](06-events.md) | 事件处理器、`emit`、事件声明、事件类型 |
| [07-样式系统](07-styling.md) | 组件 `style` 块、CSS 属性、设计类引用 |
| [08-设计系统](08-design-system.md) | `design` 定义、设计令牌、CSS 类、`variant` 组件变体 |
| [09-运动与动画](09-motion.md) | `@motion`、`@timeline`、动画触发器、缓动函数 |
| [10-类型系统](10-type-system.md) | 基础类型、`struct`、tagged union、可选类型、类型别名 |
| [11-表达式系统](11-expressions.md) | 运算符、三元表达式、块表达式、空值合并、可选链、match |
| [12-逻辑与异步](12-logic-and-async.md) | `fn`、控制流、`async`/`await`、标准库 |

## 快速入门

一个最小的 OmniView 组件：

```omni
import omniview;

component Counter {
    signal count: i32 = 0;

    fn increment() {
        count = count + 1;
    }

    view {
        <div>
            <p>Count: {count}</p>
            <button @onPress={increment}>+1</button>
        </div>
    }
}
```

## 编译器命令

```bash
# 运行开发服务器
omnic run

# 构建到 OmniView 目标（HTML/CSS/JS）
omnic build --target omniview

# 构建到 React 目标
omnic build --target react
```

## 核心概念

### 第一公民（First-Class Citizens）

OmniView 将以下概念提升为语言第一公民：

| 概念 | 关键字 | 说明 |
|------|--------|------|
| 组件 | `component` | UI 的基本构建块，封装状态、视图和样式 |
| Store | `store` | 全局/局部状态管理容器 |
| 信号 | `signal` | 响应式数据单元，变化时自动更新 UI |
| 设计系统 | `design` | 设计令牌 + CSS 类 + 组件变体的统一定义 |
| 动画 | `@motion` / `@timeline` | 声明式动画和时间线定义 |
| 类型 | `struct` / `type` | 结构体、tagged union、类型别名 |

### 文件类型

OmniLang 文件使用 `.omni` 扩展名：

- **UI 文件**：以 `import omniview;` 开头，可包含组件、store、设计系统
- **纯逻辑文件**：不写 `import omniview;`，只包含 `fn`、`struct`、`type` 等
- **入口文件**：通常是 `main.omni`，作为编译入口

### 多文件项目结构

```
src/
├── main.omni           # 入口 + App 根组件
├── design.omni         # 全局设计系统
├── lib.omni            # 工具函数
├── lib/                # 纯逻辑模块
│   ├── types.omni      # 类型定义
│   ├── validation.omni # 验证逻辑
│   └── api.omni        # API 调用
├── stores/             # 状态管理
│   └── TodoStore.omni
├── shared/             # 共享模块
│   ├── theme.omni      # 主题令牌
│   └── constants.omni  # 常量定义
├── components/         # UI 组件
│   ├── Header.omni
│   ├── Counter.omni
│   └── ...
└── pages/              # 页面级组件
    ├── Dashboard.omni
    └── SettingsPage.omni
```
