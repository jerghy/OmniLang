# 14. VS Code 与编辑器

> **目标**：搭建 OmniLang 开发环境 —— VS Code 扩展和可视化编辑器。

---

## VS Code 扩展

OmniLang VS Code 扩展提供语法高亮、代码片段和 LSP 驱动的诊断。

### 安装

扩展以 `.vsix` 格式在仓库中提供：

```bash
cd vscode-extension

# 方式 A：从预构建的 .vsix 安装
code --install-extension omnilang-vscode-0.2.1.vsix

# 方式 B：自行打包安装
npm install
npx vsce package
code --install-extension omnilang-vscode-0.2.0.vsix
```

安装后，`.omni` 文件自动获得语言支持。

### 功能

#### 语法高亮

扩展为 `.omni` 文件提供以下作用域的高亮：

- **关键字**：`fn`、`let`、`mut`、`if`、`else`、`while`、`for`、`return`、`component`、`signal`、`store`、`view`、`style`
- **类型**：`i32`、`f64`、`bool`、`String`、`Array`、`Map`
- **指令**：`@if`、`@for`、`@else`、`@empty`、`@slot`、`@store`
- **字符串**、**注释**、**数字**、**运算符**

#### 代码片段

| 前缀 | 片段内容 |
|------|---------|
| `comp` | 带 `view` 块的组件 |
| `store` | 带信号的 Store |
| `fn` | 带返回类型的函数 |
| `view` | View 块 |
| `sig` | 信号声明 |
| `if` | `@if` 指令 |
| `for` | `@for` 指令 |

输入前缀后按 `Tab` 展开。

#### LSP 功能

扩展启动 `omnic lsp` 作为语言服务器，提供：

| 功能 | 描述 |
|------|------|
| **诊断** | 实时错误和警告——红色/黄色波浪线 |
| **补全** | 关键字 + 符号建议 |
| **悬停信息** | 悬停时显示类型和文档 |
| **跳转到定义** | 跳转到符号声明（支持跨文件） |
| **查找引用** | 列出符号的所有使用位置 |
| **重命名** | 跨文件重命名符号（`F2`） |
| **文档符号** | 大纲视图：组件、Store、函数 |
| **折叠** | 按大括号折叠代码块 |

> LSP 需要 Rust 编译器 DLL 已编译。如果看到 "language server not started"，运行 `cargo build -p omnilang`。

#### 语言配置

- **行注释**：`//`
- **块注释**：`/* */`
- **自动闭合对**：`{}`, `[]`, `()`, `""`, `''`
- **包围对**：同上

---

## 可视化编辑器

OmniLang 包含一个基于浏览器的可视化编辑器，使用节点/画布界面可视化构建 UI。

### 启动编辑器

```bash
omnic serve --dir ./my-project
```

选项：

| 参数 | 默认值 | 描述 |
|------|--------|------|
| `--dir` | （必需） | 项目目录 |
| `--port` | `5173` | 服务器端口 |
| `--vite-port` | `5174` | Vite 开发服务器端口 |
| `--dev` | false | 强制 Vite 开发模式 |
| `--no-frontend` | false | 跳过自动启动 Vite |

### 编辑器功能

- **可视化画布**——拖拽式节点 UI 构建器
- **实时编译**——编辑时实时查看输出
- **文件树浏览器**——浏览项目文件
- **WebSocket 实时重载**——更改即时同步
- **`.omni.json` 导入/导出**——保存/加载画布状态
- **组件和 Store 连线**——可视化连接数据流

### API 端点（供工具使用）

编辑器服务器暴露以下 REST 端点：

| 端点 | 用途 |
|------|------|
| `POST /api/compile` | 编译 `.omni` 源码 |
| `WS /api/watch` | 通过 WebSocket 实时编译 |
| `GET /api/project` | 项目信息 |
| `POST /api/fs/read` | 读取项目文件 |
| `POST /api/fs/write` | 写入项目文件 |
| `POST /api/parse-view` | 提取 UI 元素树 |
| `POST /api/parse-codeblocks` | 提取 Store/Signal/Action 块 |
| `POST /api/omni-json/export` | 导出画布状态 |
| `POST /api/omni-json/load` | 导入画布状态 |

---

## 推荐工作流

### 新项目

```bash
# 1. 创建项目
omnic create my-app --type view
cd my-app

# 2. 在 VS Code 中打开（需安装扩展）
code .

# 3. 开始编码——使用代码片段和 LSP
# 编辑 src/main.omni

# 4. 检查问题
omnic analyse

# 5. 运行并在浏览器中查看
omnic run src/main.omni
```

### 快速原型

```bash
# 1. 启动可视化编辑器
omnic serve --dir ./my-app

# 2. 在浏览器中打开 http://localhost:5173
# 3. 在画布上可视化构建 UI
# 4. 导出 .omni 源码
# 5. 在 VS Code 中细化
```

### 自动重新编译开发

```bash
# 终端 1：监听并重新编译
omnic watch src/main.omni --target omniview --output dist

# 终端 2：提供输出文件
npx serve dist
```

---

## 故障排除

| 问题 | 解决方案 |
|------|---------|
| 没有语法高亮 | 安装 `.vsix`：`code --install-extension omnilang-vscode-0.2.1.vsix` |
| LSP 未启动 | 编译 Rust 编译器：`cargo build -p omnilang` |
| 代码片段不出现 | 确保文件保存为 `.omni`——片段只对 `omnilang` 语言 ID 激活 |
| 编辑器无法启动 | 检查 `--dir` 指向包含 `.omnilang.toml` 的目录 |
| 端口冲突 | 用 `--port 3000` 切换服务器端口 |

---

## 下一步

- **[11. CLI 参考手册](11-cli-reference.md)** — 完整的 `omnic` 命令参考
- **[7. OmniView 组件](07-omniview-components.md)** — 开始构建 UI
- **[实战食谱](../cookbook/README.md)** — 实用模式和示例
