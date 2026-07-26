# 01 — 快速入门

## 最小 OmniView 组件

一个完整的 OmniView 组件包含信号（状态）、函数（行为）和视图（UI）：

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

- `import omniview;` — 声明这是 UI 文件
- `signal count: i32 = 0` — 响应式状态
- `fn increment()` — 修改状态的方法
- `view { ... }` — UI 模板
- `{count}` — 表达式插值，自动响应变化

## 运行项目

```bash
# 进入项目目录
cd my-project

# 开发模式（自动编译 + 浏览器预览）
omnic run

# 构建到 OmniView 目标（自包含 HTML/CSS/JS）
omnic build --target omniview

# 构建到 React 目标
omnic build --target react
```

编译产物位于 `build/` 目录：
- `index.html` — 静态 HTML 结构
- `index.css` — 组件样式 + 设计系统 CSS
- `index.js` — 内联信号运行时 + 组件逻辑

## 项目结构

```
my-project/
├── .omnilang.toml         # 项目配置
└── src/
    ├── main.omni          # 入口文件 + App 根组件
    ├── design.omni        # 全局设计系统
    ├── components/        # 可复用组件
    ├── pages/             # 页面级组件
    ├── stores/            # 全局状态管理
    └── lib/               # 纯逻辑库
```

### 入口文件 (main.omni)

```omni
import omniview;
import { Header } from "./components/Header";
import { Dashboard } from "./pages/Dashboard";

component App {
    view {
        <div>
            <Header />
            <Dashboard />
        </div>
    }
}
```

编译器从 `main.omni` 出发，递归发现所有导入的模块，统一编译。

### 配置文件 (.omnilang.toml)

```toml
[project]
name = "my-project"
version = "1.0.0"

[source]
dir = "src"                     # 源文件目录

[output]
dir = "build"                   # 输出目录

[aliases]
"@lib" = "./src/lib"           # 路径别名
```

## 两种文件类型

### UI 文件（以 `import omniview;` 开头）

可以包含 `component`、`store`、`design`、`@motion` 等 UI 构造：

```omni
import omniview;

component MyPage {
    signal title: String = "Hello";
    view { <h1>{title}</h1> }
}
```

### 纯逻辑文件（不写 `import omniview;`）

只包含 `fn`、`struct`、`type` 等纯逻辑定义，可被 UI 文件导入：

```omni
// lib/math.omni —— 纯逻辑文件，不需要 import omniview

pub fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

pub struct Point {
    x: i32,
    y: i32,
}
```

## 设计系统引用

OmniView 的 `design` 块定义全局设计令牌和可复用样式。组件通过 `import { LearnDesign }` 引用：

```omni
import omniview;
import { LearnDesign } from "../design";

component MyCard {
    view {
        <div class: LearnDesign.Card>
            <h3 class: LearnDesign.CardTitle>Title</h3>
            <button variant: LearnDesign.Button(color=primary, size=md)>
                Action
            </button>
        </div>
    }
}
```

详见 [10-设计系统](10-design-system.md)。

## 下一步

- 了解 [类型系统](02-types-and-values.md)
- 学习 [组件](05-components.md) 的全部特性
- 掌握 [模板语法](06-templates.md)
