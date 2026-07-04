// Poiesis v2 Phase 1 Victauri Test Script
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected

async function phase1_verify() {
  console.log("=== Phase 1: PID Annotation System ===");

  // 1. 打开 learn-omniview 项目
  await mcp__victauri__invoke_command({
    command: "open_project",
    args: { path: "test/projects/learn-omniview" }
  });
  await mcp__victauri__wait_for({
    condition: "expression",
    value: "document.querySelector('.project-loaded') !== null"
  });

  // 2. 验证 .omni 文件含 @pid 注释
  const source = await mcp__victauri__invoke_command({
    command: "read_file",
    args: { path: "src/main.omni" }
  });
  console.assert(source.includes("// @pid:c1"), "main.omni 应含 @pid:c1");
  console.assert(!source.includes("// @edit"), "不应再含 @edit 注释");

  // 3. 验证 PID 索引构建
  const pidIndex = await mcp__victauri__invoke_command({
    command: "get_pid_index", args: {}
  });
  console.assert(Object.keys(pidIndex).length > 0, "PID 索引非空");
  console.assert(pidIndex["c1"]?.blockType === "component", "c1 应为 component 类型");

  // 4. 编译后产物不含 @pid 注释（注释已在编译时剥离）
  const buildResult = await mcp__victauri__invoke_command({
    command: "compile_project",
    args: { target: "react" }
  });
  console.assert(buildResult.success, "编译成功");
  console.assert(!buildResult.output.includes("// @pid:"), "编译产物不应含 @pid");

  // 5. round-trip: 解析 → 发射 → 再解析 PID 一致
  const roundtrip = await mcp__victauri__invoke_command({
    command: "verify_pid_roundtrip",
    args: { file: "src/main.omni" }
  });
  console.assert(roundtrip.consistent, "PID round-trip 一致");

  console.log("✅ Phase 1 全部验证通过");
}

export { phase1_verify };
