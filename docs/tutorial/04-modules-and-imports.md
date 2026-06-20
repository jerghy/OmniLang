# 4. 模块与导入

> **目标**：用导入系统跨多个文件组织 OmniLang 代码。

---

## 模块系统

每个 `.omni` 文件都是一个**模块**。文件可以用 `import` 语句从其他文件导入符号（函数、结构体、类型）。

```
my-project/
├── src/
│   ├── main.omni          # 入口点 — 从 lib.omni 导入
│   └── lib.omni           # 库 — 导出工具函数
└── .omnilang.toml
```

---

## 导入语法

OmniLang 支持四种导入风格：

### 1. 命名导入

从模块中导入特定符号：

```omni
// main.omni
import { add, multiply } from "./lib";

fn main() {
    let result = add(2, 3);
    println(result.to_string());
}
```

```omni
// lib.omni
pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

pub fn multiply(a: i32, b: i32) -> i32 {
    return a * b;
}
```

> **重要**：只有 `pub` 符号才能被其他模块导入。私有符号仅限文件内部使用。

### 2. 命名空间导入

将整个模块导入到一个命名空间下：

```omni
// main.omni
import "./lib" as lib;

fn main() {
    let result = lib.add(2, 3);
    println(result.to_string());
}
```

### 3. 类型专用导入

只导入类型（不导入值）——对于避免名称冲突很有用：

```omni
import type { User, Product } from "./models";
```

### 4. 库导入（从标准库）

从内置标准库模块导入：

```omni
import { abs, floor, PI } from "omni/math";
import { to_upper, trim } from "omni/string";
import { sleep } from "omni/async";
```

> 库导入使用裸模块名（不带 `./` 前缀）。所有可用模块见[标准库](../stdlib.md)。

---

## 导出

符号**默认私有**。使用 `pub` 导出：

```omni
// lib.omni
fn private_helper() -> String {    // 其他文件不可访问
    return "内部";
}

pub fn public_api() -> String {    // 其他文件可访问
    return "公开";
}

pub struct User {                  // 结构体可访问
    name: String,                  // 字段是私有的
    pub email: String,             // 字段可访问
}
```

### 重新导出

从另一个模块转发符号：

```omni
// api.omni — 从内部模块重新导出所有符号
export { login, logout } from "./auth";
export { getPosts } from "./posts";
```

---

## 可见性规则

| 声明 | 默认 | 加 `pub` 后 |
|------|------|------------|
| `fn` | 文件私有 | 其他文件可访问 |
| `struct` | 文件私有 | 其他文件可访问 |
| `struct` 字段 | 私有 | 其他文件可访问 |
| `type` 别名 | 文件私有 | 其他文件可访问 |
| `component` | 始终公开 | — |
| `store` | 始终公开 | — |

> 组件和 Store **始终公开**，因为它们构成应用的 UI 层。

---

## 路径别名

在 `.omnilang.toml` 中可以定义路径别名，避免冗长的相对路径：

```toml
[aliases]
"@components" = "./src/components"
"@lib" = "./src/lib"
"@models" = "./src/models"
```

然后在导入中使用别名：

```omni
import { Button } from "@components/Button";
import { User } from "@models/User";
import { format } from "@lib/format";
```

---

## 循环导入

编译器会检测循环导入并报告错误：

```omni
// a.omni
import { b } from "./b";   // 错误：检测到循环导入

// b.omni
import { a } from "./a";   // 错误：检测到循环导入
```

**解决方案**：将共享的类型或函数提取到第三个模块，两个文件都导入它：

```omni
// shared.omni
pub struct User { name: String, age: i32 }

// a.omni
import { User } from "./shared";

// b.omni
import { User } from "./shared";
```

---

## 标准库

> 🚧 **即将推出**：`omni/math`、`omni/string`、`omni/async` 标准库模块正在开发中。当前请使用 OmniLang 内置的类型方法（`.to_string()`、`.len()`、`.push()` 等）。

> 完整参考：[标准库](../stdlib.md)

---

## 模块布局惯例

```
src/
├── main.omni              # 入口点 — 导入和组合
├── lib.omni               # 共享逻辑工具
├── models/                # 数据类型
│   ├── User.omni
│   └── Product.omni
├── components/            # UI 组件（视图项目）
│   ├── App.omni
│   ├── Header.omni
│   └── Button.omni
└── stores/                # 状态 Store（视图项目）
    └── AppStore.omni
```

---

## 下一步

- **[5. 所有权与闭包](05-ownership-and-closures.md)** — 移动语义、借用、闭包和 async
- **[标准库](../stdlib.md)** — `omni/math`、`omni/string`、`omni/async` 完整参考
- **[3. 结构体与数据类型](03-structs-and-data.md)** — 自定义类型和泛型
