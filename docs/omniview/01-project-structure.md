# 01 — 项目结构与模块

## 源文件

OmniLang 源文件使用 `.omni` 扩展名。每个文件是一个**模块**（Module），编译为一个独立的单元。

### 文件类型

**UI 文件**：包含 UI 构造（component、store、design 等），必须以 `import omniview;` 开头：

```omni
import omniview;

component MyApp {
    // ...
}
```

**纯逻辑文件**：只包含函数、类型定义、结构体等，**不写** `import omniview;`：

```omni
// 纯逻辑文件，不需要 import omniview;

pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

pub struct Point {
    x: i32,
    y: i32,
}
```

## import 导入系统

OmniLang 支持四种导入方式：

### 1. 命名导入（Named Import）

导入指定名称的导出项：

```omni
// 导入组件、store、函数等
import { TodoStore } from "./stores/TodoStore";
import { Header } from "./components/Header";
import { add, multiply } from "./lib/math";
```

### 2. 命名空间导入（Namespace Import）

将整个模块作为命名空间导入：

```omni
// 将模块作为命名空间导入
import "./lib/stats" as StatsLib;
import "./shared/theme" as ThemeTokens;

// 使用：StatsLib.computeAverage(data)
```

### 3. 类型导入（Type-Only Import）

仅导入类型定义（编译后不产生运行时代码）：

```omni
import type { StatisticsData, UserRecord } from "./lib/types";
```

### 4. 库导入（Lib Import）

导入语言内置或标准库模块：

```omni
import omniview;                          // OmniView UI 框架
import { http_get } from "std/http";      // 标准库 HTTP 模块
import { json_parse } from "std/json";    // 标准库 JSON 模块
import { console_log } from "std/console"; // 标准库控制台模块
```

## export 重导出

将其他模块的内容重新导出：

```omni
// 从另一个模块重新导出指定名称
export { Percentage, Score } from "./lib/types";
```

## pub 可见性

默认情况下，所有定义都是模块私有的。使用 `pub` 关键字标记可被其他模块导入的项：

```omni
// 私有（仅当前模块可见）
fn helper() -> i32 { return 42; }

// 公共（可被其他模块导入）
pub fn calculate(x: i32) -> i32 { return x * 2; }

pub struct User {
    name: String,
    age: i32,
}

pub type Percentage = f64;
```

`pub` 适用于：
- `fn` 函数
- `struct` 结构体
- `type` 类型别名

## 项目结构推荐

```
src/
├── main.omni              # 入口文件 + App 根组件
├── design.omni            # 全局设计系统
├── lib.omni               # 入口级工具函数
├── lib/                   # 纯逻辑库模块
│   ├── types.omni         # pub struct + pub type 定义
│   ├── reexports.omni     # export { X } from "..." 重导出
│   ├── validation.omni    # 验证逻辑
│   ├── transform.omni     # 数据转换
│   └── api.omni           # API 类型 + HTTP 工具
├── stores/                # 状态管理 Store
│   ├── TodoStore.omni
│   ├── ThemeStore.omni
│   └── NotificationStore.omni
├── shared/                # 共享模块
│   ├── theme.omni         # 主题令牌
│   └── constants.omni     # 常量
├── components/            # 可复用 UI 组件
│   ├── Header.omni
│   ├── Counter.omni
│   ├── Modal.omni
│   └── ...
└── pages/                 # 页面级组件
    ├── Dashboard.omni
    ├── SettingsPage.omni
    └── ...
```

## 模块入口

应用通常从 `main.omni` 编译，该文件导入所有需要的模块并定义根 `App` 组件：

```omni
import omniview;

// 导入 Store
import { TodoStore } from "./stores/TodoStore";

// 导入页面和组件
import { Header } from "./components/Header";
import { Dashboard } from "./pages/Dashboard";

// 导入类型和工具
import type { StatisticsData } from "./lib/types";
import "./lib/stats" as StatsLib;

component App {
    @store TodoStore;

    view {
        <div>
            <Header />
            <Dashboard />
        </div>
    }
}
```
