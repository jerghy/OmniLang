---
name: start-storybook
description: Use when the user asks to start Storybook, launch Poiesis Storybook, view components in isolation, or wants to visually inspect UI components — "启动storybook", "start storybook", "storybook", "组件文档", "component library"
---

# Start Storybook

启动 Poiesis 项目的 Storybook 并在 Playwright 浏览器中自动打开，让用户能可视化查看/调试组件。

## Flowchart

```
用户请求启动 storybook
│
└─ [1] 运行 start-storybook.sh（单次 Bash，run_in_background）
    └─ 脚本内部：杀旧进程 → npx storybook dev → 等端口 6006 就绪
    └─ 成功输出 "Storybook is running on http://localhost:6006"
    └─ 失败输出 "ERROR: ..."
│
└─ [2] Playwright MCP 导航到 http://localhost:6006
│
└─ [3] 截图 + 报告左边栏有哪些组件/Story
│
└─ [4] 确认 → "Storybook 已打开，告诉我要看哪个组件或哪里有 bug"
```

## Step-by-Step

### 1. 运行启动脚本

**单次 Bash 调用即可（不读日志，不轮询）：**

```bash
bash /d/space/OmniLang/.claude/skills/start-storybook/start-storybook.sh
```

用 `run_in_background: true`，等待完成（约 30-60s 首次启动，之后秒级）。

脚本内部（对你透明）：
1. 杀占用端口 6006 的旧进程
2. `npx storybook dev -p 6006 --no-open` 启动
3. 循环等待 `localhost:6006` 返回 HTTP 200（最长 60s）
4. 日志写 `/tmp/storybook.log`

### 2. 用 Playwright MCP 打开

```js
await page.goto('http://localhost:6006/');
```

### 3. 截图确认

截图发给用户，列出左边栏中可见的组件和 Story。

### 4. 确认就绪

> Storybook 已打开，目前有这些组件：xxx。告诉我要看哪个组件，或者哪里有 bug，我截图给你看。

## Obey

- 不要读 `/tmp/storybook.log`（除非报错）
- 不要读 npx 构建输出
- 不要检查进程状态（脚本已确认）
- 启动成功后不要问"还要做什么"，直接导航 + 截图
- 如果脚本返回非零，告诉用户 "启动失败，查看 /tmp/storybook.log"
- Storybook 已运行但用户再次叫你启动 → 跳过脚本，直接 Playwright 导航
