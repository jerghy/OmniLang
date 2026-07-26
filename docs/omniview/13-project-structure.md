# 13 — 项目结构与模块

## 文件类型

OmniLang 源文件使用 `.omni` 扩展名，分为两类：

### UI 文件

以 `import omniview;` 开头，可包含 `component`、`store`、`design`、`@motion` 等 UI 构造：

```omni
import omniview;

component MyApp {
    view { <div>Hello</div> }
}
```

### 纯逻辑文件

不写 `import omniview;`，只包含 `fn`、`struct`、`type` 等逻辑定义：

```omni
// lib/math.omni —— 不需要 import omniview

pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

pub struct Point { x: i32, y: i32 }
```

## import 导入系统

### 命名导入

```omni
import { TodoStore } from "./stores/TodoStore";
import { Header } from "./components/Header";
import { add, multiply } from "./lib/math";
```

### 命名空间导入

将整个模块作为命名空间：

```omni
import "./lib/stats" as StatsLib;
// 使用：StatsLib.computeAverage(data)

import "./lib/types" as Types;
// 使用：Types.StatisticsData
```

### 类型导入

仅导入类型定义，编译后不产生运行时代码：

```omni
import type { StatisticsData, UserRecord } from "./lib/types";
```

### 库导入

导入语言内置或标准库模块：

```omni
import omniview;                              // OmniView UI 框架（必需）
import { http_get } from "std/http";          // 标准库
import { json_parse } from "std/json";
import { console_log } from "std/console";
```

## export 重导出

```omni
export { Percentage, Score } from "./lib/types";
```

## pub 可见性

默认所有定义模块私有。`pub` 标记可被导入的项：

```omni
fn helper() -> i32 { return 42; }            // 私有

pub fn calculate(x: i32) -> i32 { ... }      // 公共
pub struct User { name: String, ... }         // 公共
pub type Percentage = f64;                    // 公共
```

`pub` 适用于 `fn`、`struct`、`type`。`component`、`store`、`design`、`@motion`、`@timeline` 默认即可被导入（不需要 `pub`）。

## 路径别名 (.omnilang.toml)

在项目配置中定义路径别名：

```toml
[aliases]
"@lib" = "./src/lib"
"@shared" = "./src/shared"
```

使用别名导入：
```omni
import { StatisticsData } from "@lib/types";
```

## 推荐项目结构

```
my-project/
├── .omnilang.toml           # 项目配置
└── src/
    ├── main.omni            # 入口 + App 根组件
    ├── design.omni          # 全局设计系统
    ├── lib.omni             # 入口级工具函数
    ├── lib/                 # 纯逻辑库
    │   ├── types.omni       # pub struct + pub type
    │   ├── validation.omni  # 验证逻辑
    │   └── api.omni         # API 调用
    ├── stores/              # 全局状态
    │   ├── TodoStore.omni
    │   └── ThemeStore.omni
    ├── components/          # 可复用 UI 组件
    │   ├── Header.omni
    │   └── Counter.omni
    └── pages/               # 页面级组件
        ├── Dashboard.omni
        └── Settings.omni
```

## 编译入口

应用从 `main.omni` 编译，编译器递归发现所有导入的模块：

```omni
// src/main.omni
import omniview;

// 导入 Store
import { TodoStore } from "./stores/TodoStore";

// 导入组件
import { Header } from "./components/Header";
import { Dashboard } from "./pages/Dashboard";

// 导入纯逻辑
import type { StatisticsData } from "./lib/types";
import "./lib/stats" as StatsLib;

component App {
    @store TodoStore;

    view {
        <div class: LearnDesign.Page>
            <Header />
            <Dashboard />
        </div>
    }
}
```

## 多目标编译

OmniLang 源代码（`.omni`）编译为 IR（中间表示）后，可输出到多个目标：

```
.omni → IR → OmniView (HTML/CSS/JS)
           → React (TSX + CSS Modules)
           → Flutter (Dart)
           → Vue (SFC)
           → Dioxus (Rust)
           → JavaScript / Python / Dart / Rust (纯逻辑)
```

`UiCodegen` trait 提供统一接口，各后端实现自己的代码生成策略。
