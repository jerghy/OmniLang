# 1. 快速入门

> **目标**：安装 OmniLang，创建你的第一个项目，5 分钟内运行起来。

## 准备工作

- **Python 3.10+** —— 用于 `omnic` CLI 和 Python 编译目标
- **Rust 工具链**（可选）—— 编译 Rust 编译器需要；通过 [rustup](https://rustup.rs/) 安装
- **Node.js**（可选）—— 用于 JavaScript/TypeScript 目标
- **Dart SDK**（可选）—— 用于 Dart/Flutter 目标

## 安装

克隆仓库并以开发模式安装 Python 包：

```bash
git clone https://github.com/omnilang/omnilang.git
cd OmniLang
cargo build -p omnilang
```

编译器是用 Rust 写的，需要编译一次：

```bash
cd OmniLang
cargo build -p omnilang
```

> **注意**：编译器产物为 `omni_compiler.dll`（Windows）/ `libomni_compiler.so`（Linux）/ `libomni_compiler.dylib`（macOS）。Python 包会自动发现它。

## 验证安装

```bash
omnic --help
```

你应该看到：

```
OmniLang compiler — transpile .omni to Python, JS, TS, Rust, Dart, React, Vue, and more

commands:
  build     编译 .omni 源文件到目标语言
  analyse   对 .omni 源文件运行静态分析
  run       编译并运行 OmniLang 文件
  create    创建新的 OmniLang 项目
  serve     启动可视化编辑器服务器
  watch     监听 .omni 文件变化并自动重新编译
```

## 第一个项目：逻辑项目

OmniLang 项目分为两种类型：**logic**（纯计算）和 **view**（带 OmniView UI）。

先创建一个逻辑项目：

```bash
omnic create hello --type logic
```

这会创建：

```
hello/
├── .omnilang.toml         # 项目配置
└── src/
    └── main.omni          # 入口文件
```

打开 `src/main.omni` 看看：

```omni
// hello — OmniLang 逻辑项目
fn main() {
    print("Hello from hello!");
}
```

### 编译并运行

```bash
cd hello

# 编译到 Python
omnic build --target python

# 或者一步到位：编译并运行
omnic run src/main.omni
```

输出：
```
Hello from hello!
```

试试其他编译目标：

```bash
omnic run src/main.omni --target js     # 用 Node.js 运行
omnic run src/main.omni --target dart   # 用 Dart 运行
omnic run src/main.omni --target rust   # 编译后用 Cargo 运行
```

## 第一个项目：视图项目

现在创建一个 UI 项目：

```bash
omnic create my-ui --type view
```

这会生成 `src/main.omni`，包含组件模板：

```omni
// my-ui — OmniLang 视图项目
import omniview;

component App {
    view {
        <div class: app>
            <h1>my-ui</h1>
            <p>欢迎来到你的 OmniView 应用！</p>
        </div>
    }
}
```

`import omniview;` 这一行激活了 UI 层。没有它，文件就会被当作纯逻辑处理。

运行：

```bash
cd my-ui
omnic run src/main.omni
```

这会编译为 **OmniView**（独立的 HTML+JS+CSS）并自动在浏览器中打开。你会看到 "my-ui" 和欢迎信息。

> **原理**：`omnic run` 自动检测到这是视图项目，编译到 `omniview` 目标（独立 HTML），组装 `index.html` 并打开浏览器。对于 React 或 Vue 等框架目标，使用 `omnic build --target react`，然后手动设置构建工具链。

## 项目结构详解

```
my-project/
├── .omnilang.toml         # 项目配置（参见教程 12）
├── src/                   # 源码目录
│   ├── main.omni          # 入口文件
│   ├── lib.omni           # 逻辑模块
│   └── components/        # UI 组件（视图项目）
│       └── Button.omni
└── build/                 # 编译输出（自动生成）
    ├── main.py            # Python 输出
    ├── main.js            # JavaScript 输出
    └── ...                # 取决于编译目标
```

## 核心概念

在深入之前，先建立心智模型：

```
                    ┌──────────────┐
   .omni 源码 ───▶  │  omnic CLI   │ ──▶ Python / JS / Rust / Dart
                    │  (Rust 核心)  │ ──▶ React / Vue / Dioxus / Flutter
                    └──────────────┘ ──▶ OmniView（独立 HTML）
```

1. **写** `.omni` 文件 —— 一种语言同时处理逻辑和 UI
2. **编译** 用 `omnic build` 或 `omnic run` 到任意 10 个目标
3. **运行** 原生执行 —— Python 用 `python`，JS 用 `node`，OmniView 在浏览器中打开

语言本身借鉴了 Rust 和 Dart：
- **OmniLang**（逻辑）≈ Dart —— 函数、结构体、类型、泛型、异步
- **OmniView**（UI）≈ Flutter —— 组件、信号、Store、响应式视图

## 常见问题

| 问题 | 解决方案 |
|------|---------|
| `omnic: command not found` | 在仓库根目录运行 `cargo build -p omnilang` |
| `CompilerUnavailable` | 编译 Rust 编译器：`cargo build -p omnilang` |
| `No .omni files found` | 在项目目录下运行，或指定文件：`omnic build src/main.omni` |
| 视图项目被当作文本打开 | 在文件顶部添加 `import omniview;` |
| 浏览器没有自动打开 | 文件在 `build/debug/index.html`，手动打开即可 |

## 下一步

继续阅读 **[2. 语言基础](02-language-basics.md)**，学习变量、类型、函数和控制流——每个 OmniLang 程序的基石。
