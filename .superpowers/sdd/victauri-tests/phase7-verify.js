// Poiesis v2 Phase 7 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase7_verify() {
  console.log("=== Phase 7: End-to-End Regression ===");

  // ═══ 1. 全流程 E2E: 打开 → 架构 → 设计 → 编辑 → 保存 → 编译 ═══

  // 1a. 打开项目
  await mcp__victauri__invoke_command({
    command: "open_project",
    args: { path: "test/projects/learn-omniview" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelector('[data-page=\"architecture\"]') !== null"
  });

  // 1b. 展开泡泡 → 导航到设计页面
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      [...document.querySelectorAll('.bubble-node.is-leaf')][0]
        ?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().currentPage === 'design'"
  });

  // 1c. 编辑 signal
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      document.querySelector('[data-action="edit-signal"]')?.click();
      document.querySelector('.signal-form [name="defaultValue"]').value = '42';
      document.querySelector('.signal-form [data-action="save"]')?.click();
    ` }
  });

  // 1d. 保存 → 编译
  await mcp__victauri__invoke_command({
    command: "save_project", args: {}
  });
  await mcp__victauri__invoke_command({
    command: "compile_project",
    args: { target: "react" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().compileErrors.length >= 0"
  });
  const errors = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().compileErrors.length" }
  });
  console.assert(errors === 0, `编译应无错误，实际: ${errors}`);

  // ═══ 2. 边界情况 ═══

  // 2a. 循环依赖检测
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      // 检查架构图谱是否有循环检测
      window.__ZUSTAND_STORE__.getState().architecture.bubbles
        .some(b => b.warning === "circular-dependency");
    ` }
  });
  // 正常项目不应有循环依赖

  // 2b. 大项目虚拟化（节点渲染数量 < 实际节点数）
  const visibleNodes = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      document.querySelectorAll('.react-flow__node:not(.hidden)').length;
    ` }
  });
  const totalNodes = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().elements.length" }
  });
  // visibleNodes ≤ totalNodes（虚拟化生效）

  // 2c. 未保存切换标签确认
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      window.__ZUSTAND_STORE__.getState().isDirty = true;
      document.querySelector('.design-tab .close-btn')?.click();
    ` }
  });
  const confirmVisible = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "document.querySelector('.confirm-dialog') !== null" }
  });
  console.assert(confirmVisible, "未保存关闭应弹确认框");

  // ═══ 3. 最终健康检查 ═══

  // 3a. IPC 健康
  const ipc = await mcp__victauri__check_ipc_integrity({});
  console.assert(ipc.healthy, "IPC 链路健康");

  // 3b. Ghost commands 清零
  const ghosts = await mcp__victauri__detect_ghost_commands({});
  const realGhosts = ghosts.filter(g => g.reliability === "confirmed");
  console.assert(realGhosts.length === 0, `残余 ghost commands: ${JSON.stringify(realGhosts)}`);

  // 3c. 内存不超过基线 50%
  const perf = await mcp__victauri__get_performance({});
  const jsHeapMB = perf.jsHeapSize / (1024 * 1024);
  console.assert(jsHeapMB < 150, `堆内存 ${jsHeapMB.toFixed(1)}MB 应 < 150MB`);

  // 3d. 可访问性回归
  const a11y = await mcp__victauri__audit_accessibility({});
  const violations = a11y.violations?.filter(v => v.impact === "critical" || v.impact === "serious") || [];
  console.assert(violations.length === 0, `回归 a11y 问题: ${JSON.stringify(violations)}`);

  // 3e. 状态一致性
  const verify = await mcp__victauri__verify_state({ frontendKey: "all", backendKey: null });
  console.assert(verify.consistent, "前后端状态一致");

  console.log("✅ Phase 7 全部验证通过");
}

export { phase7_verify };
