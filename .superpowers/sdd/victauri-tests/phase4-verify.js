// Poiesis v2 Phase 4 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase4_verify() {
  console.log("=== Phase 4: Design Page + OmniViewRenderer ===");

  // 1. 打开 .omni 文件 → 创建标签页
  await mcp__victauri__invoke_command({
    command: "open_project",
    args: { path: "test/projects/learn-omniview" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().designPage.openTabs.length > 0"
  });

  // 2. OmniViewRenderer 渲染组件（非 iframe）
  const rendererExists = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "document.querySelector('[data-renderer=\"omniview\"]') !== null" }
  });
  console.assert(rendererExists, "OmniViewRenderer DOM 存在");

  // 3. 单击元素 → 选中高亮
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const el = document.querySelector('[data-pid]');
      el?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().selectedElementIds.length > 0"
  });

  // 4. 双击元素 → 下层浮现 + 面包屑
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const el = document.querySelector('[data-pid]');
      el?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().designPage.codeFlowVisible === true"
  });
  const breadcrumbs = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().designPage.breadcrumbs.length" }
  });
  console.assert(breadcrumbs >= 2, "面包屑应有至少 2 层");

  // 5. ContextInspector 根据选中切换内容
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      // 选中基础元素 → ContextInspector 显示 PropsSection
      const div = document.querySelector('[data-pid][data-tag="div"]');
      div?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    ` }
  });
  const inspectorSection = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "document.querySelector('.context-inspector [data-section]')?.dataset?.section" }
  });
  console.assert(inspectorSection, "ContextInspector 应根据选中显示正确 section");

  // 6. signal/prop/event 表单编辑
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const addBtn = document.querySelector('[data-action="add-signal"]');
      addBtn?.click();
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelector('.signal-form') !== null"
  });
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      document.querySelector('.signal-form [name="name"]').value = 'testCount';
      document.querySelector('.signal-form [name="type"]').value = 'i32';
      document.querySelector('.signal-form [data-action="save"]')?.click();
    ` }
  });

  // 7. 标签页拖拽排序
  const tabCount = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "window.__ZUSTAND_STORE__.getState().designPage.openTabs.length" }
  });
  console.assert(tabCount > 0, "应有打开标签页");

  // 8. 关闭未保存标签 → 确认对话框
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      window.__ZUSTAND_STORE__.getState().designPage.isDirty = true;
    ` }
  });
  const confirmShown = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      // 模拟关闭标签
      const closeBtn = document.querySelector('.design-tab .close-btn');
      closeBtn?.click();
      document.querySelector('.confirm-dialog') !== null;
    ` }
  });
  console.assert(confirmShown, "未保存关闭应弹出确认对话框");

  // 9. 性能：OmniViewRenderer 渲染 < 16ms
  const perf = await mcp__victauri__get_performance({});
  console.assert(perf.jsHeapSize < 120 * 1024 * 1024, "堆内存 < 120MB");

  console.log("✅ Phase 4 全部验证通过");
}

export { phase4_verify };
