# OmniLang 快速入门

一次编写，到处运行。将 `.omni` 编译为 React、Vue、Python 等。

## 三条命令创建你的第一个应用

```bash
# 1. 克隆并安装
git clone <repo-url> && cd OmniLang

# 2. 编译编译器（只需一次）
cargo build -p omnilang

# 3. 创建并运行一个视图项目
omnic create my-app --type view
cd my-app
omnic run src/main.omni
# 浏览器会自动打开，展示你的应用
```

## 刚刚发生了什么？

你创建了一个 OmniView 项目，编译成了独立 HTML，并在浏览器中运行。这个 HTML 文件包含响应式状态管理和完整的交互能力。

## 试试其他目标

```bash
# 编译到 Python
omnic build src/main.omni --target python

# 编译到 JavaScript
omnic build src/main.omni --target js

# 编译到 Rust
omnic build src/main.omni --target rust

# 编译到 React
omnic build src/main.omni --target react
```

## 编写你自己的 .omni

```omni
import omniview;

component Hello {
    signal name: String = "World";

    view {
        <div>
            <h1>你好，{name}！</h1>
        </div>
    }
}
```

保存为 `hello.omni`，然后：

```bash
omnic run hello.omni
```

## 项目类型

| 类型 | 命令 | 适合场景 |
|------|------|---------|
| 逻辑项目 | `omnic create my-cli --type logic` | CLI 工具、库、脚本 |
| 视图项目 | `omnic create my-app --type view` | Web 应用、仪表板 |

## 支持的编译目标

| 目标 | 类型 | 成熟度 |
|------|------|--------|
| `omniview` | UI | L2（生产就绪） |
| `react` | UI | L2（生产就绪） |
| `vue` | UI | L2（生产就绪） |
| `dioxus` | UI | L2（生产就绪） |
| `flutter` | UI | L1（基础） |
| `python` | 纯逻辑 | ✅ 完成 |
| `javascript` | 纯逻辑 | ✅ 完成 |
| `rust` | 纯逻辑 | ✅ 完成 |
| `dart` | 纯逻辑 | ✅ 完成 |

## 示例项目

仓库中的 `test/projects/` 目录包含 5 个完整示例：

- `logic-cli-tool` — CLI 工具
- `logic-counter` — 结构化计数器
- `mixed-blog` — 带 UI 组件的博客
- `view-ecommerce` — 产品列表 + 购物车
- `view-todo` — 待办列表

```bash
cd test/projects/view-todo
omnic run src/main.omni
```

## 下一步

- **[完整教程](docs/README.md)** — 14 章引导式学习路径，从零到生产
- **[语言基础](docs/tutorial/02-language-basics.md)** — 变量、类型、函数、控制流
- **[OmniView 组件](docs/tutorial/07-omniview-components.md)** — 用组件和模板构建 UI
- **[CLI 参考手册](docs/tutorial/11-cli-reference.md)** — 每个 `omnic` 命令详解
- **[实战食谱](docs/cookbook/README.md)** — 即用型常见模式

## 需要帮助？

- 语法错误？OmniLang 会提示常见错误（例如引导将 `or` 改为 `||`）
- 忘记 `import omniview;`？编译器会警告并继续
- 查看[语法规范](docs/omni-grammar-spec.md)了解正式语言参考
- 查看[架构决策记录](docs/adr/001-omnilang-omniview-unification.md)了解架构细节
- 查看[差距报告](docs/REAL_WORLD_GAPS.md)了解已知限制
