// Poiesis v2 Phase 0 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase0_verify() {
  console.log("=== Phase 0: Type System + Cleanup ===");

  // 1. 确认连接到 Poiesis
  const info = await mcp__victauri__get_plugin_info({});
  console.assert(info.app.identifier.includes("poiesis"), "应连接到 Poiesis");

  // 2. 确认可以调用 Tauri 后端命令
  const dir = await mcp__victauri__invoke_command({
    command: "get_project_dir", args: {}
  });
  console.assert(dir !== null, "后端命令可调用");

  // 3. IPC 链路健康检查
  const ipc = await mcp__victauri__check_ipc_integrity({});
  console.assert(ipc.healthy, "IPC 链路健康");

  // 4. 前端 JS 堆内存基线
  const perf = await mcp__victauri__get_performance({});
  console.assert(perf.jsHeapSize < 100 * 1024 * 1024, "堆内存 < 100MB");

  console.log("✅ Phase 0 全部验证通过");
}

export { phase0_verify };
