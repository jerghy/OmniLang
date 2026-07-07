---
name: recover-victauri
description: Use when Victauri MCP returns 401 errors, connection lost, or bridge needs reset — "恢复victauri", "victauri连不上", "restore victauri", "reconnect victauri", "重置victauri", "victauri断了"
---

# Recover Victauri MCP Bridge

恢复 Victauri MCP 桥接连接。当 Victauri 工具返回 401 Unauthorized（`Victauri returned 401`）或 `not connected` 时，杀掉旧桥接进程让 Claude 自动重连。

## Flowchart

```
用户请求恢复 victauri
│
└─ [1] 运行恢复脚本（单次 Bash）
    └─ 脚本内部：
        ├─ 找最新 Poiesis PID 和 temp token
        ├─ 验证 token（REST API health check）
        ├─ 杀掉所有 stale victauri.exe 桥接进程
        └─ 输出恢复结果
    └─ 成功输出 "Victauri recovered"
    └─ 失败输出 "ERROR: ..."
│
└─ [2] 验证 Victauri 是否正常（调用 get_plugin_info）
    ├─ 成功 → "Victauri 已恢复"
    └─ 失败 → 告诉用户手动重启 Victauri MCP
│
└─ [3] STOP — 不继续测试或操作
```

## Step-by-Step

### 1. 运行恢复脚本

```bash
bash /d/space/OmniLang/.claude/skills/recover-victauri/recover-victauri.sh
```

脚本内部：
1. 通过 `tasklist` 找到最新 poiesis.exe PID
2. 读取对应 temp 目录的 port 和 token
3. 用 REST API 测试健康状态 (`POST /api/tools/eval_js`)
4. 杀掉所有 `victauri.exe` 桥接进程（强制 MCP 重连）
5. 确认 port 仍被 Poiesis 持有

### 2. 验证连接

调用一次 Victauri MCP 工具确认恢复：
```
get_plugin_info → 应返回 app.identifier = "com.omnilang.poiesis"
```

### 3. 结果

- **成功** → "Victauri 已恢复 🟢，可以继续测试了"
- **失败** → "恢复失败，请手动在 Claude 设置中重启 Victauri MCP"

### 4. STOP

恢复成功后立即停止，不要继续测试或其他操作。

## Obey

- 不要启动/重启 Poiesis（只恢复桥接）
- 验证只调用一次 `get_plugin_info`
- 恢复成功后立刻停止
- 如果脚本返回非零，告诉用户手动重启
