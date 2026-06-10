# OmniLang 外部验证报告

> **期间：** 2026-06-10  
> **基于：** Council R11 外部验证清单  
> **测试框架：** pytest (Python) + Playwright (Browser)  
> **截图：** 8 张  

---

## §1 执行摘要

Tier 0-1 全部完成。3 个真实 .omni 应用（Todo + Dashboard + Form）编译运行，在浏览器中零 console error。发现 20 个缺口（9 已修复，含 7 个 CRITICAL）。333 测试通过，零回归。

**核心发现：** 真实应用验证投入产出比极高——3 个应用发现了 7 个 333 个合成测试从未触及的 CRITICAL bugs。

---

## §2 进度

| Tier | 内容 | 状态 | 关键产出 |
|------|------|------|---------|
| 0.1 | B3: Style 格式 | ✅ | CSS→React 对象转换 |
| 0.2 | E1: 错误消息 | ✅ | `or`→`||` 友好提示 |
| 0.3 | G1: 类型映射 | ✅ | `Array<String>`→`string[]` |
| 1.1 | Todo App | ✅ | 添加/完成/过滤 流程通过 |
| 1.2 | Dashboard App | ✅ | 4 KPI 计算正确，多 store |
| 1.3 | Form App | ✅ | bind:value + 验证 + 提交 |
| 1.4 | 跨目标一致性 | ✅ | React + Vue 对等输出 |
| 1.5 | CSS Showcase | ✅ | 19 规则全正确，shadow/font/border/color/layout |
| 2.1 | 陌生人测试 | ⏳ | 需外部开发者 |
| 2.2 | 错误恢复 | ✅ | 3/5 优秀，2/5 可改进 |
| 2.3 | 文档自检 | ⏳ | 延后 |
| 3.x | 生产就绪 | ⏳ | 延后 |

---

## §3 缺口统计

| 严重度 | 发现 | 已修复 | 剩余 |
|--------|------|--------|------|
| CRITICAL | 7 | **7** (B1-B7) | 0 |
| HIGH | 2 | 0 | B3, B8 |
| MEDIUM | 4 | 2 (G1, E1) | G2, G6 |
| LOW | 7 | 0 | G3-G5, G7-G8, E2 |
| LOW | 8 | 0 | G3-G5, G7-G9, E2 |
| **总计** | **21** | **9** | **12** |

---

## §4 CRITICAL 修复详情

| Bug | 描述 | 文件 | 行 |
|-----|------|------|-----|
| B1 | Store getter `tasks.length` → `state.tasks.length` | `react.py` | 893-903 |
| B2 | 内联 handler 丢失 `(args)` 调用 | `lowering.py` | 391-396 |
| B4 | `@if` 生成 `({...})` Babel 崩溃 | `react.py` | 520-521 |
| B5 | 多 store 字段全部重复声明 | `react.py` | 304-349 |
| B6 | 多 store action 路由到错误 store | `react.py` | 265-277 |
| B7 | `@if` 多子元素 Fragment 包裹 | `react.py` | 559-561 |
| B8 | 嵌套 `else { if }` 解析丢失 | `parser.py` | 383-415 |

---

## §5 应用验证结果

### Todo App
- **文件：** `todo-app.omni` (80 行)
- **编译：** ✅ 3 文件输出 (TSX + CSS + Store)
- **运行：** 零 console error
- **功能：** ✅ 添加/完成/清除/过滤 全部通过
- **截图：** `todo-app-working.png`, `todo-app-after-add.png`, `todo-app-after-complete.png`, `todo-app-final.png`

### Dashboard App
- **文件：** `dashboard.omni` (100 行)
- **编译：** ✅ 4 文件输出 (2 Store + TSX + CSS)
- **运行：** 零 console error
- **KPI 计算：** ✅ 67.44% active, $146.15 AOV, 36.48% conversion
- **多 store：** ✅ 字段隔离 + action 路由正确
- **截图：** `dashboard-after-load.png`

### Form App
- **文件：** `form-app.omni` (95 行)
- **编译：** ✅ 3 文件输出
- **运行：** 零 console error
- **功能：** ✅ bind:value / 验证 / 提交成功 / toggle
- **截图：** `form-app-success.png`, `form-app-submitted.png`

---

## §6 测试指标

| 指标 | 值 |
|------|-----|
| Python 测试 | **333 passed**，零回归 |
| E2E 测试 | 4 个（编译 + 结构验证 + 已知缺口） |
| xfailed (已知缺口) | 1 |
| xpassed (修复验证) | 3 |
| 浏览器验证 | 零 console error |
| Playwright 截图 | 8 张 |

---

## §7 建议下一步

1. **Tier 2.1 陌生人测试** —— 找 1 个非团队开发者用 30 分钟写 .omni 功能
2. **Tier 3.1 Bundle 大小** —— 测量 Todo app min+gzip
3. **Tier 3.2 真实部署** —— Dashboard 部署到 Vercel
4. **G2 空 action body 警告** —— 编译时 warning
5. **G6 bind:value 支持 store 字段** —— 拓宽语法
