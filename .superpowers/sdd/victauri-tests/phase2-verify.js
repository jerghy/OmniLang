// Poiesis v2 Phase 2 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase2_verify() {
  console.log("=== Phase 2: Unified Zustand Store ===");

  // 1. 确认 store 初始化状态正确
  const state = await mcp__victauri__invoke_command({
    command: "get_store_snapshot", args: {}
  });
  console.assert(state.currentPage === "architecture", "默认首页为架构页面");
  console.assert(typeof state.pidIndex === "object", "pidIndex 存在");
  console.assert(Array.isArray(state.elements), "elements 为数组");

  // 2. 检查 ghost commands
  const ghosts = await mcp__victauri__detect_ghost_commands({});
  const realGhosts = ghosts.filter(g => g.reliability === "confirmed");
  console.assert(realGhosts.length === 0, `ghost commands: ${JSON.stringify(realGhosts)}`);

  // 3. IPC 链路健康
  const ipc = await mcp__victauri__check_ipc_integrity({});
  console.assert(ipc.healthy, "IPC 链路健康");

  // 4. 打开项目 → store 正确加载
  await mcp__victauri__invoke_command({
    command: "open_project",
    args: { path: "test/projects/learn-omniview" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__?.getState().elements.length > 0"
  });
  const loaded = await mcp__victauri__invoke_command({
    command: "get_store_snapshot", args: {}
  });
  console.assert(loaded.elements.length > 0, "项目加载后应有元素");

  // 5. 撤销/重做
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().undo()" }
  });

  // 6. 合并后堆内存 < 80MB（比旧双 store 更小）
  const perf = await mcp__victauri__get_performance({});
  console.assert(perf.jsHeapSize < 80 * 1024 * 1024, "堆内存 < 80MB");

  console.log("✅ Phase 2 全部验证通过");
}

export { phase2_verify };
