#!/usr/bin/env node
/**
 * Full pipeline: Editor → Transpile → React → Screenshot → Compare
 *
 * Usage:
 *   node compare.mjs --omni <file.omni> --app-name <name>
 *   node compare.mjs --omni <file.omni> --app-name <name> --skip-install
 *
 * Output:
 *   build_output/<app-name>/compare-editor.png
 *   build_output/<app-name>/compare-react.png
 *   build_output/<app-name>/pipeline-summary.json
 */

import { chromium } from "playwright";
import * as fs from "fs";
import * as path from "path";
import { execSync, spawn } from "child_process";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const EDITOR_URL = process.env.OMNI_EDITOR_URL || "http://localhost:5174";
const REACT_PORT = parseInt(process.env.REACT_PORT || "5201", 10);
const PROJECT_DIR = path.resolve(__dirname, "..", "..", "..");

async function sleep(ms) {
  return new Promise(r => setTimeout(r, ms));
}

async function stage1_loadEditor(browser, omniSourcePath) {
  console.log("─".repeat(50));
  console.log("Stage 1: Loading app in visual editor...");

  const omniSource = fs.readFileSync(omniSourcePath, "utf-8");
  const omniJsonPath = omniSourcePath.replace(/\.omni$/, ".omni.json");
  const appName = path.basename(omniSourcePath, ".omni");

  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  try {
    await page.goto(EDITOR_URL, { waitUntil: "networkidle", timeout: 30000 });
  } catch {
    console.log("  WARNING: networkidle timeout, continuing...");
  }
  await page.waitForFunction(() => window.__OMNI__);

  const compileResult = await page.evaluate(async (source) => {
    const O = window.__OMNI__;
    O.help();
    O.switchMode("code");
    O.setSourceCode(source);
    const result = await O.compile();
    O.switchMode("design");
    return {
      success: result.success,
      errors: (result.errors || []).length,
      elements: O.elements.length,
      wires: O.wires.length,
    };
  }, omniSource);

  console.log(`  Compile: ${compileResult.success ? "OK" : "FAIL"}, errors: ${compileResult.errors}`);
  console.log(`  Canvas: ${compileResult.elements} elements, ${compileResult.wires} wires`);

  await page.waitForTimeout(1000);

  const outDir = path.join(PROJECT_DIR, "build_output", appName);
  fs.mkdirSync(outDir, { recursive: true });

  const editorScreenshotPath = path.join(outDir, "compare-editor.png");
  await page.screenshot({ path: editorScreenshotPath, fullPage: false });

  // Save .omni.json
  const saveResult = await page.evaluate(async (jsonPath) => {
    const O = window.__OMNI__;
    const state = O.getState();
    const activeFile = O.getActiveFilePath() || "project.omni";
    const jsonFilePath = jsonPath || activeFile.replace(/\.omni$/, ".omni.json");

    try {
      const exportResp = await fetch("/api/omni-json/export", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          source: O.getSourceCode(),
          filepath: activeFile,
          target: "react",
        }),
      });
      const exportData = await exportResp.json();
      if (exportData.success && exportData.document) {
        const doc = exportData.document;
        doc.elements = state.elements;
        doc.wires = state.wires;
        await fetch("/api/omni-json/save", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ filePath: jsonFilePath, document: doc }),
        });
        return { saved: true, path: jsonFilePath };
      }
    } catch (e) {
      // Fallback: save state via fs/write API
    }

    // Fallback: write directly
    const doc = {
      schemaVersion: "1.0.0",
      meta: {
        projectName: "test", omniVersion: "0.4.0",
        createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(),
        isHandWritten: false, conversionWarnings: [],
      },
      components: [], stores: [], themes: [], imports: [],
      elements: state.elements,
      wires: state.wires,
      ownershipRegions: [], searchIndex: { entries: [], version: 1, lastBuilt: new Date().toISOString() },
      canvasLayout: { positions: {} },
    };
    await fetch("/api/fs/write", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filePath: jsonFilePath, content: JSON.stringify(doc, null, 2) }),
    });
    return { saved: true, path: jsonFilePath, fallback: true };
  }, omniJsonPath);

  console.log(`  .omni.json: ${saveResult.saved ? "saved" : "FAILED"} → ${saveResult.path}`);
  console.log(`  Screenshot: ${editorScreenshotPath}`);

  return { page, editorScreenshotPath, omniJsonPath: saveResult.path, compileResult, appName };
}

async function stage2_transpile(omniSourcePath, appName) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 2: Transpiling to React...");

  const outputDir = path.join(PROJECT_DIR, "build_output", appName);
  const transpileScript = path.join(__dirname, "helpers", "transpile.py");

  try {
    execSync(
      `python "${transpileScript}" --source "${omniSourcePath}" --target react --output "${outputDir}" --port ${REACT_PORT}`,
      { stdio: "pipe", encoding: "utf-8", shell: process.platform === "win32" }
    );
    console.log("  Transpile OK");
  } catch (e) {
    console.error("  Transpile FAILED:", e.stderr || e.message);
    return null;
  }

  return outputDir;
}

async function stage3_startReact(outputDir) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 3: Starting React app...");

  const viteProc = spawn("npx", ["vite", "--port", String(REACT_PORT), "--host"], {
    cwd: outputDir,
    stdio: "pipe",
    shell: process.platform === "win32",
  });

  await sleep(4000);
  console.log(`  Vite started on port ${REACT_PORT}`);
  return viteProc;
}

async function stage4_captureReact(browser, appName) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 4: Capturing React app...");

  const reactUrl = `http://localhost:${REACT_PORT}`;
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  try {
    await page.goto(reactUrl, { waitUntil: "networkidle", timeout: 15000 });
  } catch {
    console.log("  WARNING: networkidle timeout, continuing...");
  }
  await page.waitForTimeout(1000);

  const outDir = path.join(PROJECT_DIR, "build_output", appName);
  const reactScreenshotPath = path.join(outDir, "compare-react.png");
  await page.screenshot({ path: reactScreenshotPath, fullPage: true });

  const domSnapshot = await page.evaluate(() => {
    const buttons = Array.from(document.querySelectorAll("button")).map(b => ({
      text: (b.textContent || "").trim().substring(0, 30),
      testId: b.getAttribute("data-testid") || "",
    }));
    const inputs = Array.from(document.querySelectorAll("input")).map(i => ({
      placeholder: i.getAttribute("placeholder") || "",
      type: i.getAttribute("type") || "text",
      testId: i.getAttribute("data-testid") || "",
    }));
    const headings = Array.from(document.querySelectorAll("h1,h2,h3,h4,h5,h6")).map(h => ({
      level: parseInt(h.tagName[1], 10),
      text: (h.textContent || "").trim().substring(0, 50),
    }));
    const testIds = Array.from(document.querySelectorAll("[data-testid]")).map(e => ({
      testId: e.getAttribute("data-testid"),
      tag: e.tagName.toLowerCase(),
    }));
    return {
      buttons, inputs, headings, testIds,
      buttonCount: buttons.length, inputCount: inputs.length,
      headingCount: headings.length, testIdCount: testIds.length,
    };
  });

  console.log(`  Screenshot: ${reactScreenshotPath}`);
  console.log(`  DOM: ${domSnapshot.buttonCount} buttons, ${domSnapshot.inputCount} inputs, ${domSnapshot.headingCount} headings`);

  await page.close();
  return { reactScreenshotPath, domSnapshot, reactUrl };
}

async function stage5_structuralDiff(omniJsonPath, appName) {
  console.log("\n" + "─".repeat(50));
  console.log("Stage 5: Structural diff...");

  const diffScript = path.join(__dirname, "helpers", "diff-analyzer.mjs");
  const reactUrl = `http://localhost:${REACT_PORT}`;

  try {
    const result = execSync(
      `node "${diffScript}" --editor-state "${omniJsonPath}" --react-url "${reactUrl}"`,
      { stdio: "pipe", encoding: "utf-8", shell: process.platform === "win32" }
    );
    console.log(result);
    return { match: true, issues: [] };
  } catch (e) {
    console.log(e.stdout || e.message);
    return { match: false, output: e.stdout || "" };
  }
}

async function main() {
  const args = process.argv.slice(2);
  const getArg = (name) => {
    const idx = args.indexOf(name);
    return idx >= 0 ? args[idx + 1] : null;
  };

  const omniPath = getArg("--omni");
  const appName = getArg("--app-name");

  if (!omniPath || !appName) {
    console.error("Usage: node compare.mjs --omni <file.omni> --app-name <name> [--skip-install]");
    process.exit(1);
  }

  console.log("╔══════════════════════════════════════════════╗");
  console.log("║  OmniLang Framework Test — Compare Pipeline ║");
  console.log("╚══════════════════════════════════════════════╝");
  console.log(`  Source: ${omniPath}`);
  console.log(`  App: ${appName}`);
  console.log(`  Editor: ${EDITOR_URL}`);
  console.log(`  React port: ${REACT_PORT}`);

  const browser = await chromium.launch({ headless: true });
  let viteProc = null;

  try {
    // Stage 1: Load in editor
    const { page: editorPage, editorScreenshotPath, omniJsonPath, compileResult } =
      await stage1_loadEditor(browser, omniPath);

    if (!compileResult.success) {
      console.error("ERROR: Compile failed — cannot continue");
      await browser.close();
      process.exit(1);
    }

    // Stage 2: Transpile
    const outputDir = await stage2_transpile(omniPath, appName);
    if (!outputDir) {
      await browser.close();
      process.exit(1);
    }

    // Stage 3: Start React
    viteProc = await stage3_startReact(outputDir);

    // Stage 4: Capture React
    const { reactScreenshotPath, domSnapshot } = await stage4_captureReact(browser, appName);

    // Stage 5: Structural diff
    const diffResult = await stage5_structuralDiff(omniJsonPath, appName);

    // Summary
    console.log("\n" + "═".repeat(60));
    console.log("PIPELINE COMPLETE");
    console.log("═".repeat(60));
    console.log(`  Editor screenshot: ${editorScreenshotPath}`);
    console.log(`  React screenshot:  ${reactScreenshotPath}`);
    console.log(`  .omni.json:        ${omniJsonPath}`);
    console.log(`  Structural diff:   ${diffResult.match ? "✅ MATCH" : "❌ ISSUES FOUND"}`);

    if (!diffResult.match) {
      console.log("\n⚠ Run Vision MCP comparison next:");
      console.log(`  Compare: ${editorScreenshotPath}`);
      console.log(`  vs:      ${reactScreenshotPath}`);
    }

    // Write summary JSON
    const outDir = path.join(PROJECT_DIR, "build_output", appName);
    const summaryPath = path.join(outDir, "pipeline-summary.json");
    fs.writeFileSync(summaryPath, JSON.stringify({
      timestamp: new Date().toISOString(),
      appName,
      editor: { screenshot: editorScreenshotPath, omniJson: omniJsonPath, elements: compileResult.elements, wires: compileResult.wires },
      react: { screenshot: reactScreenshotPath, port: REACT_PORT, ...domSnapshot },
      diff: diffResult,
    }, null, 2));
    console.log(`  Summary: ${summaryPath}`);

  } finally {
    if (viteProc) viteProc.kill();
    await browser.close();
    console.log("\nDone!");
  }
}

main().catch(e => { console.error(e); process.exit(1); });
