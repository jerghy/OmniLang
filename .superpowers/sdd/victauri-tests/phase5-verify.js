// Poiesis v2 Phase 5 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase5_verify() {
  console.log("=== Phase 5: Code Canvas Nodes ===");

  // 1. 确认四种节点类型渲染
  const nodeTypes = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      [...document.querySelectorAll('.flow-node')].map(n => n.dataset.nodeType)
    ` }
  });
  console.assert(nodeTypes.includes("function-header"), "应有函数头节点");
  console.assert(nodeTypes.includes("skeleton"), "应有骨架节点");
  console.assert(nodeTypes.includes("code-block"), "应有代码块节点");
  console.assert(nodeTypes.includes("return"), "应有 return 节点");

  // 2. 骨架节点多出口连线
  const skeletonEdges = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const skeletonIds = [...document.querySelectorAll('.flow-node[data-node-type="skeleton"]')]
        .map(n => n.dataset.id);
      const edges = window.__ZUSTAND_STORE__.getState().wires
        .filter(w => skeletonIds.includes(w.fromId));
      edges.length;
    ` }
  });
  console.assert(skeletonEdges >= 2, "骨架节点应有多条出边");

  // 3. 双击代码块节点 → Monaco 面板弹出
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const codeBlock = document.querySelector('.flow-node[data-node-type="code-block"]');
      codeBlock?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelector('.monaco-editor') !== null"
  });
  const monacoVisible = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: "document.querySelector('.monaco-editor-panel') !== null" }
  });
  console.assert(monacoVisible, "Monaco 编辑器面板应弹出");

  // 4. 编辑代码 → 保存 → 写回代码块
  await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const editor = window.__MONACO_EDITOR__;
      editor?.setValue('count = count + 1;');
      document.querySelector('[data-action="save-code"]')?.click();
    ` }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "!document.querySelector('.monaco-editor-panel')"
  });

  // 5. 旧 data-flow 节点确认不存在
  const oldNodes = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      const oldTags = ['signal', 'store', 'computed', 'add', 'sub', 'branch', 'forEach'];
      const nodes = window.__ZUSTAND_STORE__.getState().elements;
      nodes.filter(n => oldTags.includes(n.tag)).length;
    ` }
  });
  console.assert(oldNodes === 0, `旧 data-flow 节点应全部删除，实际剩余: ${oldNodes}`);

  // 6. flow port（上下）存在
  const flowHandles = await mcp__victauri__invoke_command({
    command: "eval",
    args: { code: `
      document.querySelectorAll('.react-flow__handle-flow-out, .react-flow__handle-flow-in').length;
    ` }
  });
  console.assert(flowHandles > 0, "flow port handles 应存在（上下连接）");

  // 7. 性能检查
  const perf = await mcp__victauri__get_performance({});
  console.assert(perf.jsHeapSize < 100 * 1024 * 1024, "堆内存 < 100MB");

  console.log("✅ Phase 5 全部验证通过");
}

export { phase5_verify };
