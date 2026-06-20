# 12. 配置指南

> **目标**：掌握 `.omnilang.toml` —— 完整的项目配置参考。

每个 OmniLang 项目根目录都有一个 `.omnilang.toml` 文件，用于控制：
- 项目名称和版本
- 源码目录和输出目录
- 编译目标偏好
- 目标专属代码生成选项
- 静态分析规则

---

## 配置发现机制

`omnic` 从当前目录向上查找 `.omnilang.toml`，使用第一个找到的文件。配置值可以被 CLI 参数覆盖：

```
CLI 参数  >  .omnilang.toml  >  默认值
```

---

## 完整配置

### `[project]` —— 项目信息

```toml
[project]
name = "my-app"             # 项目名称（用于生成的输出中）
version = "0.1.0"           # 语义化版本号
targets = ["rust", "dart", "js", "python"]   # `omnic build` 的默认目标
ui_target = "react"         # 默认 UI 目标（"react"|"vue"|"dioxus"|"flutter"|"omniview"）
```

| 字段 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `name` | string | `""` | 项目名称 |
| `version` | string | `"0.1.0"` | 项目版本 |
| `targets` | string[] | `["rust", "dart", "js", "python"]` | 构建的逻辑目标 |
| `ui_target` | string | `""` | 默认 UI 框架目标 |

> 设置 `ui_target` 后，带 `import omniview` 的视图文件默认编译到该目标。否则默认编译到 `react`。

### `[source]` —— 源码目录

```toml
[source]
dir = "src"                 # .omni 源文件所在目录
```

### `[output]` —— 输出目录

```toml
[output]
dir = "build"               # 编译后的文件写入此目录
```

### `[aliases]` —— 路径别名

```toml
[aliases]
"@components" = "./src/components"
"@lib" = "./src/lib"
"@models" = "./src/models"
```

在导入中使用：

```omni
import { Button } from "@components/Button";
import { User } from "@models/User";
```

---

## 目标配置

### `[target.js]` —— JavaScript / TypeScript

```toml
[target.js]
mode = "plain"              # "plain" | "typescript"
module = "esm"              # "esm" | "commonjs"
private_fields = true       # 使用 #私有 类字段
```

| 字段 | 值 | 默认值 | 效果 |
|------|-----|--------|------|
| `mode` | `"plain"`, `"typescript"` | `"plain"` | 输出 `.js` 还是 `.ts` |
| `module` | `"esm"`, `"commonjs"` | `"esm"` | 模块系统 |
| `private_fields` | `true`, `false` | `true` | 使用 `#field` 语法 |

### `[target.rust]` —— Rust

```toml
[target.rust]
async_runtime = "tokio"     # "tokio" | "async-std"
```

| 字段 | 值 | 默认值 | 效果 |
|------|-----|--------|------|
| `async_runtime` | `"tokio"`, `"async-std"` | `"tokio"` | 生成代码中使用的异步运行时 |

### `[target.python]` —— Python

```toml
[target.python]
min_version = "3.10"        # 影响生成的语法
```

| 字段 | 值 | 默认值 | 效果 |
|------|-----|--------|------|
| `min_version` | `"3.8"`, `"3.9"`, `"3.10"` | `"3.10"` | 控制 `match`/`case` vs `if`/`elif`，以及 `X \| Y` vs `Union[X, Y]` |

### `[target.dart]` —— Dart

```toml
[target.dart]
# 留空，预留给未来 Dart 专属选项
```

---

## 静态分析：`[analyze]`

### `level` —— 最低严重级别

```toml
[analyze]
level = "port"              # "error" | "warning" | "port" | "style"
```

| 级别 | 报告内容 |
|------|---------|
| `"error"` | 仅错误（类型不匹配、语法问题） |
| `"warning"` | 错误 + 警告（未使用变量、死代码） |
| `"port"` | 错误 + 警告 + 可移植性问题（跨目标差异） |
| `"style"` | 以上全部 + 风格建议 |

### 规则覆盖

```toml
[analyze.rules]
P0101 = "allow"             # 忽略 cross_lang_div_behavior
P0110 = "deny"              # 将 potential_divzero 升级为错误
E0101 = "warn"              # 将 type_mismatch 降级为警告
```

| 覆盖 | 效果 |
|------|------|
| `"allow"` | 完全忽略该诊断 |
| `"warn"` | 作为警告报告（即使原本是错误） |
| `"deny"` | 作为错误报告（即使原本是警告） |

### 常用规则代码

| 代码 | 类别 | 检查内容 |
|------|------|---------|
| `E0101` | 错误 | 类型不匹配 |
| `E0104` | 错误 | 需要类型注解 |
| `E0307` | 错误 | 借用冲突 |
| `P0101` | 可移植性 | 跨目标整数除法行为 |
| `P0102` | 可移植性 | 跨目标浮点数比较 |
| `P0103` | 可移植性 | NaN 比较 |
| `P0109` | 可移植性 | 整数溢出行为 |
| `P0110` | 可移植性 | 潜在的除零 |
| `S0101` | 风格 | 未使用的变量 |
| `S0103` | 风格 | 死代码 |
| `S0105` | 风格 | 非惯用命名 |
| `SEC0101` | 安全 | 潜在的 SQL 注入 |
| `SEC0102` | 安全 | 硬编码凭据 |
| `SEC0103` | 安全 | 不安全的 eval 使用 |

---

## 完整示例

### 逻辑项目

```toml
[project]
name = "my-cli-tool"
version = "0.1.0"
targets = ["python", "js"]

[source]
dir = "src"

[output]
dir = "build"

[aliases]
"@lib" = "./src/lib"

[target.python]
min_version = "3.10"

[analyze]
level = "port"
```

### 视图项目

```toml
[project]
name = "my-dashboard"
version = "0.1.0"
targets = []
ui_target = "react"

[source]
dir = "src"

[output]
dir = "dist"

[aliases]
"@components" = "./src/components"
"@stores" = "./src/stores"

[target.js]
mode = "typescript"
module = "esm"

[analyze]
level = "style"

[analyze.rules]
S0101 = "allow"     # 开发阶段不警告未使用变量
```

### 多目标项目

```toml
[project]
name = "my-library"
version = "0.2.0"
targets = ["rust", "dart", "js", "python"]
ui_target = "omniview"

[target.js]
mode = "typescript"

[target.rust]
async_runtime = "tokio"

[target.python]
min_version = "3.10"

[analyze]
level = "port"

[analyze.rules]
P0101 = "allow"
P0103 = "warn"
```

---

## 下一步

- **[11. CLI 参考手册](11-cli-reference.md)** — CLI 参数如何覆盖配置
- **[13. 编译目标与输出](13-targets-and-output.md)** — 目标如何影响输出
- **[14. VS Code 与编辑器](14-vscode-and-editor.md)** — IDE 集成
