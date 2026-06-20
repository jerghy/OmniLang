# 11. CLI 参考手册

> **目标**：掌握每个 `omnic` 命令，含实战示例。

`omnic` CLI 是你与 OmniLang 编译器交互的主要接口。共 6 个子命令：

| 命令 | 用途 |
|------|------|
| `build` | 将 `.omni` 编译为目标语言 |
| `run` | 编译并执行（UI 用浏览器打开，逻辑用原生运行时） |
| `analyse` | 静态分析 —— 捕获错误、警告、可移植性问题 |
| `create` | 创建新项目 |
| `serve` | 启动可视化编辑器服务器 |
| `watch` | 监听文件变化并自动重新编译 |

---

## 安装

```bash
# 从 OmniLang 仓库安装
git clone https://github.com/omnilang/omnilang.git
cd OmniLang
pip install -e .

# 编译 Rust 编译器（只需一次）
cd omni-compiler && cargo build && cd ..
```

验证：

```bash
omnic --help
```

---

## `omnic build` — 编译

将 `.omni` 源码编译到 10 个目标中的任意一个。

### 语法

```
omnic build [files...] [--target TARGET] [--output DIR]
```

### 示例

```bash
# 自动发现 src/ 下所有 .omni 文件，编译到自动检测的目标
omnic build

# 将单个文件编译到 Python
omnic build app.omni --target python

# 将多个文件编译到 JavaScript
omnic build app.omni lib.omni --target js

# 将一个目录编译到 React
omnic build src/ --target react

# 指定输出目录
omnic build --target vue --output ./dist
```

### 目标自动检测

省略 `--target` 时：
- 如果源码包含 `import omniview` 或 `component` 开头 → 默认为 `react`
- 否则 → 默认为 `python`

在 `.omnilang.toml` 中覆盖：

```toml
[project]
ui_target = "vue"    # 设置默认 UI 目标
```

### 有效的编译目标

| 目标 | 类型 | 输出 |
|------|------|------|
| `python` | 逻辑 | `.py` 文件 |
| `js` | 逻辑 | `.js` 文件（ES 模块） |
| `ts` | 逻辑 | `.ts` 文件（TypeScript） |
| `rust` | 逻辑 | `.rs` 文件 |
| `dart` | 逻辑 | `.dart` 文件 |
| `react` | UI | `.tsx` 文件 |
| `vue` | UI | `.vue` 单文件组件 |
| `dioxus` | UI | `.rs` 文件（Dioxus RSX） |
| `flutter` | UI | `.dart` 文件（Flutter 组件） |
| `omniview` | UI | `.js` + `.html` + `.css`（独立文件） |

### 输出

编译后的文件写入输出目录（默认：`./build`）。每个源文件产生一个或多个输出文件：

```bash
$ omnic build src/main.omni --target python
  -> build/main.py
成功编译 1 个文件到 python。
```

---

## `omnic run` — 编译并执行

编译并立即运行结果。对于 UI 目标，在浏览器中打开。

### 语法

```
omnic run [file.omni] [--target TARGET]
```

### 示例

```bash
# 自动发现 src/main.omni，用自动检测的目标运行
omnic run

# 运行指定文件
omnic run src/main.omni

# 指定目标运行
omnic run src/main.omni --target python
omnic run src/main.omni --target js

# 视图项目自动编译到 omniview 并在浏览器中打开
omnic run src/main.omni
```

### 执行方式

| 目标 | 执行器 |
|------|--------|
| `python` | `python <file>` |
| `js` | `node <file>` |
| `ts` | `npx ts-node <file>` |
| `dart` | `dart run <file>` |
| `rust` | 创建 Cargo 项目 → `cargo run` |
| `omniview` | 组装 `index.html` → 在浏览器中打开 |
| `react`/`vue`/`dioxus`/`flutter` | 自动重定向到 `omniview` 以直接执行 |

> **OmniView 输出**：将 body 片段 + JS + CSS 组合成独立的 `index.html`，位于 `build/debug/index.html`。自动在默认浏览器中打开。

---

## `omnic analyse` — 静态分析

运行编译器的诊断引擎，不生成代码。捕获错误、警告、可移植性问题和代码风格问题。

### 语法

```
omnic analyse [files...] [--format terminal|json]
```

### 示例

```bash
# 分析项目中的所有文件
omnic analyse

# 分析指定文件
omnic analyse src/main.omni

# 输出 JSON 格式（用于工具链/CI）
omnic analyse --format json
```

### 诊断分类

| 级别 | 描述 | 示例 |
|------|------|------|
| `error` | 将导致运行时失败 | 类型不匹配、未定义的引用 |
| `warning` | 可能是 bug | 未使用的变量、死代码 |
| `portability` | 跨目标行为差异 | 整数除法、NaN 比较 |
| `style` | 代码质量问题 | 非惯用命名、复杂表达式 |
| `security` | 潜在安全漏洞 | SQL 注入、硬编码凭据 |

### 在 .omnilang.toml 中配置

```toml
[analyze]
level = "port"     # "error" | "warning" | "port" | "style"

[analyze.rules]
P0101 = "allow"    # 忽略 cross_lang_div_behavior
P0110 = "deny"     # 将 potential_divzero 升级为错误
```

### 退出代码

| 代码 | 含义 |
|------|------|
| `0` | 没有发现问题，或仅有警告 |
| `1` | 发现错误 |
| `2` | 编译器不可用（找不到 DLL） |

---

## `omnic create` — 创建项目

创建具有正确结构的新 OmniLang 项目。

### 语法

```
omnic create <name> --type logic|view
```

### 示例

```bash
# 创建逻辑项目（纯计算）
omnic create my-cli-tool --type logic

# 创建视图项目（带 OmniView UI）
omnic create my-dashboard --type view
```

### 创建的文件

**逻辑项目：**
```
my-cli-tool/
├── .omnilang.toml
└── src/
    └── main.omni        # fn main() { print("Hello from my-cli-tool!"); }
```

**视图项目：**
```
my-dashboard/
├── .omnilang.toml
└── src/
    └── main.omni        # import omniview; component App { view { ... } }
```

### 创建之后

```bash
cd my-dashboard
omnic run src/main.omni
```

---

## `omnic serve` — 可视化编辑器

启动 OmniLang 可视化编辑器 —— 基于浏览器的 IDE，包含节点/画布界面。

### 语法

```
omnic serve --dir DIR [--port PORT] [--vite-port VITE_PORT] [--dev] [--no-frontend]
```

### 示例

```bash
# 为某个项目启动编辑器
omnic serve --dir ./my-dashboard

# 自定义端口
omnic serve --dir ./my-dashboard --port 3000 --vite-port 3001

# 强制 Vite 开发模式（开发用）
omnic serve --dir ./my-dashboard --dev
```

### 编辑器功能

- 可视化画布，支持节点式编辑
- 实时编译预览
- 文件树浏览器
- 基于 WebSocket 的实时重新加载
- `.omni.json` 导入/导出画布状态
- 组件、Store 和数据流连接

详见[教程 14：VS Code 与编辑器](14-vscode-and-editor.md)。

---

## `omnic watch` — 自动重新编译

监听文件变化并自动重新编译。

### 语法

```
omnic watch <file.omni> [--target TARGET] [--output DIR]
```

### 示例

```bash
# 监听并每次保存后自动重新编译到 Python
omnic watch src/main.omni --target python

# 监听视图组件并重新编译到 React
omnic watch src/App.omni --target react --output ./dist
```

### 行为

- 每 0.5 秒轮询文件修改时间
- 变化时重新编译，打印带时间戳的结果
- 按 `Ctrl+C` 停止（显示总编译次数）

```
正在监听 src/main.omni（target=python, output=./build）
按 Ctrl+C 停止。

[14:32:01] #1 编译成功 — 1 个文件 -> ./build/
[14:32:15] #2 编译成功 — 1 个文件 -> ./build/
[14:32:42] #3 编译成功 — 1 个文件 -> ./build/
^C
已停止。（共 3 次编译）
```

---

## 常用工作流

### 完整开发流程

```bash
# 1. 创建项目
omnic create my-app --type view
cd my-app

# 2. 在 src/main.omni 中编写代码

# 3. 检查问题
omnic analyse

# 4. 运行（视图项目在浏览器中打开）
omnic run src/main.omni

# 5. 生产构建
omnic build --target react --output dist
```

### CI/CD 集成

```bash
# 分析并在有错误时失败
omnic analyse --format json

# 编译到多个目标
for target in python js rust dart; do
    omnic build --target $target --output build/$target
done
```

---

## 故障排除

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| `omnic: command not found` | 包未安装 | 在仓库根目录执行 `pip install -e .` |
| `CompilerUnavailable` | Rust DLL 未编译 | `cd omni-compiler && cargo build` |
| `No .omni files found` | 不在项目目录 | 切换到项目根目录，或指定文件路径 |
| `No source file specified` | 没有 `src/main.omni` | 指定文件：`omnic run app.omni` |
| `Directory already exists` | 目标目录已存在 | 换一个项目名称 |
| `Don't know how to run target` | 不支持的运行目标 | 使用 `omnic build` 代替，然后手动运行 |

## 下一步

- **[12. 配置指南](12-configuration.md)** — `.omnilang.toml` 完整参考
- **[13. 编译目标与输出](13-targets-and-output.md)** — 深入了解每个目标
- **[14. VS Code 与编辑器](14-vscode-and-editor.md)** — IDE 集成
