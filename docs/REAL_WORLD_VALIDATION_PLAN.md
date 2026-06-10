# OmniLang 真实应用验证计划

> **基于：** Council R10 全票通过——停止内部架构优化，用真实应用找出实际缺口
> **日期：** 2026-06-10
> **原则：** 不新增框架能力，只用现有功能写应用。痛点才是下一阶段的执行计划。

---

## §1 验证应用：Todo App

选择 Todo App 作为首个验证应用——覆盖 CRUD、状态管理、列表渲染、条件显示、输入绑定。

### 功能清单

| 功能 | 测试的框架能力 |
|------|---------------|
| 添加任务 | store action, input bind:value, event handler |
| 显示任务列表 | @for 循环, key, Array<T> store |
| 切换完成状态 | store action with param, event handler with arg |
| 删除任务 | store action, array mutation |
| 过滤器（全部/活跃/已完成） | signal, @if 条件渲染 |
| 计数显示 | store getter, 插值 `{store.activeCount}` |
| 清除已完成 | store action, computed |
| 样式 | style block, CSS 输出 |

### 不做的（延后到 v2）

- 编辑任务文字（双击编辑）
- 拖拽排序
- 持久化（localStorage）
- 动画
- 嵌套组件（TodoItem 独立组件）

---

## §2 执行步骤

| 步骤 | 内容 | 预计 |
|------|------|------|
| S1 | 编写 `todo-app.omni` (~80 行) | 30 min |
| S2 | `omnic build --target react` 编译 | 5 min |
| S3 | 创建 HTML 包装页面加载编译产物 | 10 min |
| S4 | Playwright E2E 测试（添加/切换/过滤/删除/计数） | 20 min |
| S5 | 截图 + 记录所有缺口 | 15 min |

---

## §3 缺口分类标准

| 级别 | 含义 |
|------|------|
| **BLOCKER** | 编译失败 / 运行时崩溃 |
| **BUG** | 输出不正确 / 行为与预期不符 |
| **GAP** | 功能缺失 / 语法不支持 |
| **ERGO** | 人机工程问题 / 语法别扭 |
| **PERF** | 性能问题 |

---

## §4 成功标准

- [ ] todo-app.omni 编译成功（`omnic build` 退出码 0）
- [ ] 生成的 React 代码在浏览器中可运行
- [ ] 核心流程通过 Playwright 测试：添加 → 切换 → 过滤 → 删除
- [ ] 所有缺口已记录到 `docs/REAL_WORLD_GAPS.md`
