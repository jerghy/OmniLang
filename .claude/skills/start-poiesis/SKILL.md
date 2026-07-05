---
name: start-poiesis
description: Use when the user asks to start poiesis, launch the Tauri editor, or needs Victauri MCP connected — "启动poiesis", "start poiesis", "launch editor", "open poiesis"
---

# Start Poiesis

启动 Poiesis Tauri 应用（含 Victauri MCP），供用户连接 Victauri 进行调试/测试。

## Flowchart

```
用户请求启动 poiesis
│
└─ [1] 运行 start-poiesis.sh（单次 Bash，run_in_background）
    └─ 脚本内部：杀旧进程 → cargo tauri dev → 等 poiesis.exe 出现
    └─ 成功输出 "Poiesis is running"
    └─ 失败输出 "ERROR: ..."
│
└─ [2] 确认 → "Poiesis 已启动，可以用 Victauri 了"
│
└─ [3] STOP — 不要继续操作，用户要自己重启 Victauri MCP
```

## Step-by-Step

### 1. 运行启动脚本

**单次 Bash 调用即可（不读日志，不轮询）：**

```bash
bash /d/space/OmniLang/.claude/skills/start-poiesis/start-poiesis.sh
```

用 `run_in_background: true`，等待完成（约 60s 增量编译，之后秒级）。

脚本内部（对你透明）：
1. 杀旧 poiesis.exe + 僵死 cargo 进程
2. `cargo tauri dev` 启动 Vite + Rust 后端
3. 循环等待 poiesis.exe 出现（最长 120s）
4. 日志写 `/tmp/poiesis.log`

### 2. 确认

> Poiesis 已启动，Victauri MCP 可以连接了。

### 3. STOP

**立即停止响应。** 用户接下来要自己重启 Victauri MCP，不要做额外操作。

## Obey

- 不要读 `/tmp/poiesis.log`（除非报错）
- 不要读 cargo/tauri 构建输出
- 不要检查进程状态（脚本已确认）
- 启动成功后立刻停止，不要问"还要做什么"
- 如果脚本返回非零，告诉用户 "启动失败，查看 /tmp/poiesis.log"
