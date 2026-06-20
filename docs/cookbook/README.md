# 实战食谱

> 实用、可复制的 OmniLang 模式和食谱。

每个食谱都是一个独立示例，可以直接放到你的项目中使用。按类别组织：

---

## 逻辑食谱

详见 [logic-recipes.md](logic-recipes.md)：

| 食谱 | 你将构建什么 |
|------|-------------|
| CLI 参数工具 | 带参数和标志的命令行工具 |
| 斐波那契（递归） | 经典递归算法 |
| 冒泡排序 | 用可变状态的原地数组排序 |
| 字符串格式化 | 用标准库操作字符串 |
| 数学工具 | 用 `omni/math` 执行计算 |
| 随机数生成器 | 用 `random()` 和 `floor()` 做骰子 |
| 表达式求值器 | 在带标签联合体 AST 上做模式匹配 |

## UI 食谱

详见 [ui-recipes.md](ui-recipes.md)：

| 食谱 | 你将构建什么 |
|------|-------------|
| 待办列表 | 完整的待办应用，支持添加、删除和空状态 |
| 购物车 | 产品列表 + 购物车 Store + 添加商品 |
| 博客信息流 | 结构体数据 + 组件组合 + `@for` 渲染 |
| 实时计数器 | 带步长控制、计算属性和副作用的计数器 |
| 搜索过滤 | 实时过滤列表的输入框 |
| 仪表板 KPI 卡片 | 带主题样式的指标卡片网格 |
| 登录表单 | 带验证和条件渲染的表单 |
| 标签页组件 | 动态标签页切换 |

---

## 项目模板

使用 `omnic create` 快速生成：

```bash
omnic create my-cli --type logic      # 纯逻辑项目
omnic create my-app --type view       # 带 OmniView 的视图项目
```

完整流程见[教程 1：快速入门](../tutorial/01-getting-started.md)。

---

## 示例项目

仓库包含 5 个完整的示例项目，位于 `test/projects/`：

| 项目 | 类型 | 演示内容 |
|------|------|---------|
| `logic-cli-tool` | 逻辑 | 函数、参数、条件逻辑、字符串拼接 |
| `logic-counter` | 逻辑 | 结构体、`i32` 算术、函数组合 |
| `mixed-blog` | 逻辑 + 视图 | 带 `pub` 字段的 struct、`pub fn`、组件组合、`@for` + `@empty` |
| `view-ecommerce` | 视图 | 带信号 + getter 的 Store、`@store` 引用、多组件 |
| `view-todo` | 视图 | `bind:value`、事件处理、信号修改、`@for` + `@empty` |

运行任意示例：

```bash
cd test/projects/view-todo
omnic run src/main.omni
```

---

## 下一步

- [教程索引](../README.md) — 完整引导式学习路径
- [标准库](../stdlib.md) — `omni/math`、`omni/string`、`omni/async` 参考
- [语法规范](../omni-grammar-spec.md) — 形式化语言参考
