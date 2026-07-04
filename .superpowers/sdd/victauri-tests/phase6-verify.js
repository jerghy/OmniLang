// Poiesis v2 Phase 6 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase6_verify() {
  console.log("=== Phase 6: Data Persistence ===");

  // 1. 保存项目
  const saveResult = await mcp__victauri__invoke_command({
    command: "save_project", args: {}
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().isDirty === false"
  });
  console.assert(saveResult.success, "保存应成功");

  // 2. 验证 project.json 存在
  const projectJson = await mcp__victauri__invoke_command({
    command: "read_file",
    args: { path: "test/projects/learn-omniview/project.json" }
  });
  console.assert(projectJson !== null, "project.json 应存在");
  const json = JSON.parse(projectJson);
  console.assert(json.version === 2, "version 应为 2");
  console.assert(typeof json.nodes === "object", "nodes 应存在");
  console.assert(Array.isArray(json.wires), "wires 应为数组");
  console.assert(json.architecture !== undefined, "architecture 应存在");

  // 3. .omni 文件含 @pid，不含 @edit
  const omniSource = await mcp__victauri__invoke_command({
    command: "read_file",
    args: { path: "test/projects/learn-omniview/src/main.omni" }
  });
  console.assert(omniSource.includes("// @pid:"), ".omni 应含 @pid");
  console.assert(!omniSource.includes("// @edit"), ".omni 不应含 @edit");

  // 4. JSON 不包含可解析数据
  console.assert(!json.componentDefs, "JSON 不应含 componentDefs");
  console.assert(!json.sourceCode, "JSON 不应含 sourceCode");
  console.assert(!json.signals, "JSON 不应含 signal 定义");

  // 5. 记录当前状态 → 关闭 → 重开 → 恢复
  const beforeState = await mcp__victauri__invoke_command({
    command: "get_store_snapshot", args: {}
  });
  await mcp__victauri__invoke_command({
    command: "close_project", args: {}
  });
  await mcp__victauri__invoke_command({
    command: "open_project",
    args: { path: "test/projects/learn-omniview" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "window.__ZUSTAND_STORE__.getState().elements.length > 0"
  });
  const afterState = await mcp__victauri__invoke_command({
    command: "get_store_snapshot", args: {}
  });

  // 关键状态应一致（忽略运行时差异如 previewUrl）
  console.assert(
    afterState.architecture.mode === beforeState.architecture.mode,
    "架构模式应恢复"
  );
  console.assert(
    afterState.elements.length === beforeState.elements.length,
    `元素数量应一致: ${beforeState.elements.length} → ${afterState.elements.length}`
  );
  console.assert(
    afterState.designPage.openTabs.length === beforeState.designPage.openTabs.length,
    "标签页数量应一致"
  );
  console.assert(
    JSON.stringify(afterState.panels) === JSON.stringify(beforeState.panels),
    "面板尺寸应一致"
  );

  // 6. 节点位置恢复一致
  const beforeNodePos = beforeState.nodes?.["c1"]?.pos;
  const afterNodePos = afterState.nodes?.["c1"]?.pos;
  if (beforeNodePos) {
    console.assert(
      afterNodePos?.x === beforeNodePos.x && afterNodePos?.y === beforeNodePos.y,
      "节点位置应恢复一致"
    );
  }

  // 7. 外部修改 .omni → 检测冲突
  await mcp__victauri__invoke_command({
    command: "write_file",
    args: {
      path: "test/projects/learn-omniview/src/components/TodoItem.omni",
      content: "// externally modified"
    }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelector('.file-change-toast') !== null"
  });
  const toastShown = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "document.querySelector('.file-change-toast') !== null" }
  });
  console.assert(toastShown, "外部修改应触发 toast 提示");

  // 8. Ghost commands 检查
  const ghosts = await mcp__victauri__detect_ghost_commands({});
  const realGhosts = ghosts.filter(g => g.reliability === "confirmed");
  console.assert(realGhosts.length === 0, `ghost commands: ${JSON.stringify(realGhosts)}`);

  console.log("✅ Phase 6 全部验证通过");
}

export { phase6_verify };
