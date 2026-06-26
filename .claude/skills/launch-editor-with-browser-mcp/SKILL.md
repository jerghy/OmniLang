---
name: launch-editor-with-browser-mcp
description: Launch the OmniLang editor (Python backend + Vite frontend) and open it via browser MCP for live visual editing sessions. Trigger: "你用浏览器mcp启动一下editor"
---

# Launch Editor with Browser MCP

## 原则

- **一次 Bash 调用启动一切** — 避免多次 tool call 消耗上下文
- **不截屏** — 导航即可，用户自己看
- **不读日志** — 脚本把日志写文件，只返回最终 URL
- **不轮询 API** — 脚本内部处理等待

## Flowchart

```
用户说触发句
│
└─ [1] 检查浏览器 MCP 是否可用
    ├─ NO → STOP
    └─ YES → Continue
│
└─ [2] 运行 start-editor.sh（单次 Bash，run_in_background）
    └─ 捕获输出：
       │ 最后一行（VITE_URL）→ 浏览器导航
       │ 其余行（日志）→ 丢弃
│
└─ [3] browser_navigate → {VITE_URL}
│
└─ [4] "编辑器已启动"（一行确认，不截屏）
```

## Step-by-Step

### 1. 检查浏览器 MCP

快速检查当前会话是否有 `browser_navigate` 工具。没有则 **STOP**：
> 浏览器 MCP 未连接。请先通过 `/mcp` 连接 Playwright MCP。

### 2. 运行启动脚本

**单次 Bash 调用即可：**

```bash
bash /d/space/OmniLang/.claude/skills/launch-editor-with-browser-mcp/start-editor.sh 2>&1 | tail -1
```

用 `run_in_background: true` + 捕获输出。只取 `tail -1`（实际 Vite URL）。

脚本内部流程（对你是透明的，用户也看不到）：
1. 检查 Python/npm deps + 项目目录
2. 杀掉端口 8000 旧进程
3. 启动后端 (`python -m editor.server.main --port 8000`)
4. 启动前端 (`npm run dev` from `editor/client/`)
5. 从 Vite 日志提取实际端口（自动递增处理）
6. 验证两个服务都存活
7. 输出 `http://localhost:XXXX`

### 3. 打开浏览器

```json
{"url": "{VITE_URL}"}
```

用 `browser_navigate`，**不要 screenshot**，**不要 snapshot**。

### 4. 确认

> ✅ 编辑器已启动！{VITE_URL}

## Obey

- 不要额外读文件（ENVIRONMENT.md、package.json、main.py 等）
- 不要看 Vite 日志内容
- 不要截屏或 snapshot
- 不要检查服务器状态（脚本已经做了）
- 如果命令返回非零或空 URL，告诉用户"启动失败，请检查 /tmp/editor_server.log 和 /tmp/editor_vite.log"
