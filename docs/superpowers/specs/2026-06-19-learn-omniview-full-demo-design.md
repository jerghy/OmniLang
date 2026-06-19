# Learn OmniView — Full Framework Demo Design

> 将 learn-omniview 从 12 文件教学项目升级为 ~50 文件的全框架演示平台。
>
> 目标：在 OmniView 编译器中实现但尚未在项目中展示的 73 项功能，全部纳入演示。

## 项目结构（目标）

```
test/projects/learn-omniview/
├── .omnilang.toml
├── src/
│   ├── main.omni                    # App 根组件 + 路由
│   ├── lib.omni                     # 工具函数（已存在，保留）
│   ├── lib/                         # [新增] 纯逻辑模块目录
│   │   ├── types.omni               # struct + tagged union + type alias
│   │   ├── stats.omni               # 统计引擎（纯逻辑）
│   │   └── validation.omni          # 输入验证（使用 tagged union）
│   ├── pages/                       # [新增] 页面级组件
│   │   ├── Dashboard.omni           # 仪表盘页面
│   │   ├── TodoPage.omni            # 待办页面（现有功能重构）
│   │   ├── SettingsPage.omni        # 设置页面（theme + 配置）
│   │   └── AboutPage.omni           # 关于页面
│   ├── components/                  # UI 组件（扩展现有 + 新增）
│   │   ├── Header.omni              # [已有] 静态标题
│   │   ├── Counter.omni             # [已有] signal + computed
│   │   ├── FeatureList.omni         # [已有] @if + @for
│   │   ├── StatsPanel.omni          # [已有] prop + if 表达式
│   │   ├── TodoList.omni            # [已有] @for/@empty + 组合
│   │   ├── TodoItem.omni            # [已有] prop + event + signal
│   │   ├── AddTodo.omni             # [已有] bind:value + 生命周期
│   │   ├── FeatureCard.omni         # [新增] struct/union 展示 + @else if
│   │   ├── StatsDashboard.omni      # [新增] ref + @unmount + const
│   │   ├── ValidationInput.omni     # [新增] let + while + action
│   │   ├── Timer.omni               # [新增] async + interval pattern
│   │   ├── NotificationsPanel.omni  # [新增] slot + optional event
│   │   ├── FilterBar.omni           # [新增] store filter + multi events
│   │   ├── ThemeSwitcher.omni       # [新增] theme 切换
│   │   ├── Modal.omni               # [新增] slot + @unmount + keydown
│   │   ├── DataTable.omni           # [新增] Map 字面量 + 闭包
│   │   ├── FormPage.omni            # [新增] 全事件类型 + inline style
│   │   └── TabPanel.omni            # [新增] 自定义标签 + class map
│   ├── stores/                      # Store（扩展现有）
│   │   ├── TodoStore.omni           # [已有] 升级：multi-param action + async
│   │   ├── ThemeStore.omni          # [新增] theme 状态
│   │   └── NotificationStore.omni   # [新增] 通知队列
│   └── shared/                      # [新增] 共享基础设施
│       ├── theme.omni               # theme 定义（颜色 + 间距 + 排版）
│       └── constants.omni           # const $prefixed 常量
├── build/                           # 编译输出
└── .omnilang.toml
```

---

## 第 1 阶段：基础与类型系统（~8 个新文件）

**目标：** 首次引入纯逻辑概念——struct、tagged union、type alias、nominal struct。

### 新增文件

| 文件 | 覆盖功能 |
|------|----------|
| `src/lib/types.omni` | `struct`、`nominal struct`、`tagged union`、`type` alias、`pub struct` |
| `src/lib/stats.omni` | 带 `let`/`while`/`return` 的 `pub fn`、`&&`/`\|\|`、`!=`、比较运算符 |
| `src/lib/validation.omni` | `for..in`、`return` 提前返回、带标签联合体使用 |
| `src/components/FeatureCard.omni` | struct 字面量、`@else if` 链、`@onFocus`/`@onBlur` |
| `src/components/StatsDashboard.omni` | `ref`、`@unmount`、`const`/`$prefixed`、`+=`/`-=`、`%`、`arr[i]` |
| `src/components/ValidationInput.omni` | `let`、`while`、组件内 `action`、三元运算符、多值 CSS |

### 关键设计决策
- `lib/` 文件使用 `pub` 导出，使它们成为框架中其他模块的共享依赖
- 带标签联合体为后续阶段中的组件引入 `Result<T, E>` 风格模式
- 所有新组件均为独立可测试的单元

---

## 第 2 阶段：组件系统进阶（~6 个新文件）

**目标：** 引入 `ref`、`@unmount`、`const`/`$prefixed`、组件局部 `action`、`async fn`、可选 event `?`。

| 文件 | 覆盖功能 |
|------|----------|
| `src/components/Timer.omni` | `async fn`、可选 `event ...?`、`@unmount` 清理 |
| `src/components/NotificationsPanel.omni` | slot 投影、`ref` 用于滚动容器、`@effect` 用于通知计数 |
| `src/stores/NotificationStore.omni` | 带 `return` 的 store action、多参数 action、异步 action |
| `src/components/TabPanel.omni` | 自定义标签（`Card`、`Button`）、`class:` 映射、内联 `style:` |

---

## 第 3 阶段：视图模板精通（~8 个新文件）

**目标：** `@slot`、内联 style/class、布尔属性、全部 10+ 种事件类型、自定义标签、自闭合元素、HTML 注释。

| 文件 | 覆盖功能 |
|------|----------|
| `src/components/Modal.omni` | `@slot`（header/body/footer）、`@onKeyDown`（Escape 关闭）、`@onMouseEnter`/`@onMouseLeave` |
| `src/components/FormPage.omni` | `@onChange`、`@onSubmit`、`@onFocus`/`@onBlur`、`@onScroll`、布尔属性 `disabled`/`required`/`readonly`、连字符属性 `aria-label`、`data-*` |
| `src/components/DataTable.omni` | `@onScroll`、自定义标签（`Table`/`List`）、自闭合 `<img />`/`<br />`/`<hr />`、HTML 注释 |

---

## 第 4 阶段：表达式与控制流（~5 个新文件）

**目标：** Map/Struct 字面量、闭包 `\|x\| x*2`、三元运算符、`let` 声明、`while`/`for..in`、`break`/`continue`、复合赋值。

| 文件 | 覆盖功能 |
|------|----------|
| `src/components/DataTable.omni`（升级） | Map 字面量 `{id: 1, name: "A"}`、闭包 `\|row\| row.name` |
| `src/components/ValidationInput.omni`（升级） | `break`/`continue` 在循环中、复合赋值 `*=` `/=` `%=` |
| `src/lib/transform.omni` | `for..in` 循环 + `break`/`continue` 在纯逻辑中 |

---

## 第 5 阶段：Store 进阶模式（~4 个新文件 / 升级）

**目标：** 多参数 action、带条件分支的 getter、驱动过滤 UI 的 store 状态、带返回类型的异步 action。

| 文件 | 覆盖功能 |
|------|----------|
| `src/stores/TodoStore.omni`（升级） | 带 `filter` 信号驱动过滤、多参数 `add(title, priority, due)`、异步 `async action` |
| `src/components/FilterBar.omni` | `@onChange` 用于过滤、`@onClick` 用于排序、computed 过滤列表 |

---

## 第 6 阶段：Theme 系统（~5 个新文件）

**目标：** 完整的主题系统——颜色、间距、排版分类；深色/浅色模式切换。

| 文件 | 覆盖功能 |
|------|----------|
| `src/shared/theme.omni` | `theme` 定义，包含 `colors`、`spacing`、`typography` 分类 |
| `src/shared/constants.omni` | `const $logoUrl`、`const APP_NAME`、`const $apiBase` |
| `src/stores/ThemeStore.omni` | `signal mode: "light"`、`computed` 当前主题、`action toggle()` |
| `src/components/ThemeSwitcher.omni` | 使用 theme token 切换深色/浅色模式 |
| `src/pages/SettingsPage.omni` | 将 ThemeSwitcher + `AboutPage` 组合为设置页面 |

---

## 第 7 阶段：模块系统与结构（~4 个文件 / 最终适配）

**目标：** 仅类型导入、命名空间导入、重新导出、多源目录、高级 CSS。

| 变更 | 覆盖功能 |
|------|----------|
| `src/lib/reexports.omni` | `export { StatisticsData } from "./types"` |
| `.omnilang.toml` 更新 | 多源目录（`src/lib` 作为第二个源） |
| 所有 `.omni` 文件 CSS 更新 | 多值 CSS（`margin: 0 auto`、`border: 1 solid #333`）、`vh`/`vw` 单位、`#fff` 简写十六进制 |
| `src/main.omni` 升级 | 命名空间 import、仅类型 import |

---

## 成功标准

每完成一个阶段：
1. `omnic build --target omniview` 成功
2. 该阶段的每个功能均在至少一个 `.omni` 文件中可见
3. 无回归——所有现有测试保持通过（`cargo test -p omni-compiler`）
4. 新组件可独立阅读（清晰的注释，语义化的名称）

## 累积覆盖率

| 阶段 | 新增文件 | 累积文件数 | 覆盖功能数 |
|------|---------|-----------|-----------|
| 1 | 6 | ~16 | 26 |
| 2 | 6 | ~22 | +15 = 41 |
| 3 | 8 | ~30 | +20 = 61 |
| 4 | 5 | ~35 | +5 = 66 |
| 5 | 4 | ~39 | +4 = 70 |
| 6 | 5 | ~44 | +3 = 73 |
| 7 | 4 | ~48 | +0（最终适配） |

---

*设计文档 · 2026-06-19 · 7 个阶段，~50 个文件，73 项功能*
