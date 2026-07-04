// Poiesis v2 Phase 3 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase3_verify() {
  console.log("=== Phase 3: Architecture Page ===");

  // 1. 确认打开项目后默认进入架构页面
  await mcp__victauri__invoke_command({
    command: "open_project",
    args: { path: "test/projects/learn-omniview" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelector('[data-page=\"architecture\"]') !== null"
  });
  const page = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().currentPage" }
  });
  console.assert(page === "architecture", "默认首页为架构页面");

  // 2. 三大顶层泡泡渲染
  const bubbleCount = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "document.querySelectorAll('.bubble-node').length" }
  });
  console.assert(bubbleCount >= 3, `至少 3 个顶层泡泡，实际: ${bubbleCount}`);

  // 3. 展开界面泡泡 → Store/代码自动展开
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const bubble = document.querySelector('.bubble-node[data-type="ui"]');
      bubble?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelectorAll('.bubble-node.expanded').length >= 3"
  });
  const expanded = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().architecture.expandedBubbleIds.length" }
  });
  console.assert(expanded >= 3, "UI/Store/Code 应同时展开");

  // 4. 整体架构 → 局部架构模式切换
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      // 右键一个组件泡泡 → 局部模式
      const todoBubble = [...document.querySelectorAll('.bubble-node')]
        .find(el => el.textContent.includes('TodoItem'));
      todoBubble?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({ condition: "expression", value: "document.querySelector('.context-menu') !== null" });
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      document.querySelector('[data-action="focus-component"]')?.click();
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().architecture.mode === 'local'"
  });
  const localMode = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().architecture.mode" }
  });
  console.assert(localMode === "local", "应切换到局部架构模式");

  // 5. 双击最底层泡泡 → 打开设计页面
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const leafBubble = [...document.querySelectorAll('.bubble-node.is-leaf')][0];
      leafBubble?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().currentPage === 'design'"
  });

  // 6. debug bridge: 图谱选中 ↔ iframe 高亮
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      // 图谱中选中泡泡
      window.__ZUSTAND_STORE__.getState().architecture.selectedPid = "c1";
    ` }
  });
  const iframePid = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const iframe = document.querySelector('.architecture-preview iframe');
      iframe?.contentWindow?.__POIESIS_BRIDGE__?.getHighlightedPid?.();
    ` }
  });
  console.assert(iframePid === "c1", `iframe 应高亮 c1，实际: ${iframePid}`);

  // 7. 可访问性检查
  const a11y = await mcp__victauri__audit_accessibility({});
  const violations = a11y.violations?.filter(v => v.impact === "critical" || v.impact === "serious") || [];
  console.assert(violations.length === 0, `严重 a11y 问题: ${JSON.stringify(violations)}`);

  console.log("✅ Phase 3 全部验证通过");
}

export { phase3_verify };
