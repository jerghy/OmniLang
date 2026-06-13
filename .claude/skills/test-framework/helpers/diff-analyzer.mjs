#!/usr/bin/env node
/**
 * Structural diff analyzer — compare editor state vs React DOM.
 *
 * Usage:
 *   node diff-analyzer.mjs --editor-state <file.omni.json> --react-url <url>
 */

import * as fs from "fs";
import { chromium } from "playwright";

async function loadEditorState(omniJsonPath) {
  const doc = JSON.parse(fs.readFileSync(omniJsonPath, "utf-8"));
  const views = (doc.elements || []).filter(e => e.nodeType === "view");

  return {
    totalElements: doc.elements?.length || 0,
    totalWires: doc.wires?.length || 0,
    views: views.map(e => ({
      tag: e.tag,
      id: e.id,
      label: (e.props && (e.props.label || e.props.text || e.props.placeholder)) || "",
      testId: (e.props && e.props["test-id"]) || "",
      parentId: e.parentId,
    })),
    tagCounts: views.reduce((acc, e) => {
      acc[e.tag] = (acc[e.tag] || 0) + 1;
      return acc;
    }, {}),
    codeblocks: (doc.elements || []).filter(e => e.nodeType === "codeblock").map(cb => ({
      tag: cb.tag,
      typeAnnotation: cb.typeAnnotation || "",
      ports: cb.ports || [],
    })),
  };
}

const TAG_MAP = {
  Container: "div", Flex: "div", Grid: "div", Card: "div",
  Text: "span", Heading: "h2", Image: "img", Link: "a",
  Button: "button", Input: "input", TextArea: "textarea",
  Checkbox: "input", Select: "select", List: "ul", ListItem: "li",
  Spinner: "div", Progress: "progress", Alert: "div", Form: "form",
};

async function captureReactDOM(reactUrl) {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  try {
    await page.goto(reactUrl, { waitUntil: "networkidle", timeout: 15000 });
    await page.waitForTimeout(1000);

    const dom = await page.evaluate(() => {
      const allElements = document.querySelectorAll("*");
      const tagCounts = {};
      const interactive = [];
      const testIdElements = [];
      const headings = [];

      allElements.forEach(el => {
        const tag = el.tagName.toLowerCase();
        tagCounts[tag] = (tagCounts[tag] || 0) + 1;

        if (["button", "input", "select", "textarea"].includes(tag)) {
          interactive.push({
            tag,
            text: (el.textContent || "").trim().substring(0, 50),
            type: el.getAttribute("type") || "",
            placeholder: el.getAttribute("placeholder") || "",
            testId: el.getAttribute("data-testid") || "",
          });
        }

        if (el.hasAttribute("data-testid")) {
          testIdElements.push({
            tag,
            testId: el.getAttribute("data-testid"),
            text: (el.textContent || "").trim().substring(0, 40),
          });
        }
      });

      for (let i = 1; i <= 6; i++) {
        document.querySelectorAll(`h${i}`).forEach(h => {
          headings.push({ level: i, text: h.textContent?.trim() || "" });
        });
      }

      return { tagCounts, interactive, testIdElements, headings };
    });

    return dom;
  } finally {
    await browser.close();
  }
}

function compare(editor, react) {
  const issues = [];

  for (const [omniTag, count] of Object.entries(editor.tagCounts)) {
    const htmlTag = TAG_MAP[omniTag] || "div";
    const reactCount = react.tagCounts[htmlTag] || 0;

    if (omniTag === "Input") {
      const inputs = react.interactive.filter(e => e.tag === "input");
      if (inputs.length !== count) {
        issues.push({
          type: "COUNT_MISMATCH",
          element: omniTag,
          editor: count,
          react: inputs.length,
          detail: `Input: editor has ${count}, React has ${inputs.length} <input> elements`,
        });
      }
      continue;
    }

    if (omniTag === "Heading") {
      const headingCount = react.headings.length;
      if (headingCount !== count) {
        issues.push({
          type: "COUNT_MISMATCH",
          element: omniTag,
          editor: count,
          react: headingCount,
          detail: `Heading: editor has ${count}, React has ${headingCount} headings`,
        });
      }
      continue;
    }

    if (count > 0 && reactCount === 0 && !["Container", "Flex", "Grid", "Card"].includes(omniTag)) {
      issues.push({
        type: "MISSING_IN_REACT",
        element: omniTag,
        editor: count,
        react: 0,
        detail: `${omniTag}: ${count} in editor, 0 in React`,
      });
    }
  }

  const editorButtons = (editor.views || []).filter(v => v.tag === "Button").length;
  const reactButtons = react.interactive.filter(e => e.tag === "button").length;
  if (editorButtons !== reactButtons) {
    issues.push({
      type: "BUTTON_COUNT",
      editor: editorButtons,
      react: reactButtons,
      detail: `Buttons: editor has ${editorButtons}, React has ${reactButtons}`,
    });
  }

  const editorTestIds = (editor.views || []).filter(v => v.testId).map(v => v.testId).sort();
  const reactTestIds = react.testIdElements.map(e => e.testId).sort();

  const missingTestIds = editorTestIds.filter(id => !reactTestIds.includes(id));
  const extraTestIds = reactTestIds.filter(id => !editorTestIds.includes(id));

  if (missingTestIds.length > 0) {
    issues.push({
      type: "MISSING_TEST_IDS",
      ids: missingTestIds,
      detail: `Test IDs in editor but missing in React: ${missingTestIds.join(", ")}`,
    });
  }
  if (extraTestIds.length > 0) {
    issues.push({
      type: "EXTRA_TEST_IDS",
      ids: extraTestIds,
      detail: `Test IDs in React but not in editor: ${extraTestIds.join(", ")}`,
    });
  }

  return {
    match: issues.length === 0,
    issues,
    summary: {
      editor: { elements: editor.totalElements, wires: editor.totalWires, views: editor.views.length },
      react: { tagCounts: react.tagCounts, interactiveCount: react.interactive.length, headingsCount: react.headings.length },
    },
  };
}

async function main() {
  const args = process.argv.slice(2);
  const getArg = (name) => {
    const idx = args.indexOf(name);
    return idx >= 0 ? args[idx + 1] : null;
  };

  const omniJsonPath = getArg("--editor-state");
  const reactUrl = getArg("--react-url");

  if (!omniJsonPath) {
    console.error("ERROR: --editor-state <path> is required");
    process.exit(1);
  }
  if (!reactUrl) {
    console.error("ERROR: --react-url <url> is required");
    process.exit(1);
  }

  console.log("═".repeat(60));
  console.log("STRUCTURAL DIFF: Editor State vs React DOM");
  console.log("═".repeat(60));

  console.log(`\n📋 Editor state: ${omniJsonPath}`);
  const editor = await loadEditorState(omniJsonPath);
  console.log(`   Elements: ${editor.totalElements} (${editor.views.length} views + ${editor.codeblocks.length} codeblocks)`);
  console.log(`   Wires: ${editor.totalWires}`);
  console.log(`   Tags: ${JSON.stringify(editor.tagCounts)}`);

  console.log(`\n🌐 React DOM: ${reactUrl}`);
  const react = await captureReactDOM(reactUrl);
  console.log(`   Tags: ${JSON.stringify(react.tagCounts)}`);
  console.log(`   Interactive: ${react.interactive.length}`);
  console.log(`   Headings: ${react.headings.length}`);

  console.log(`\n🔍 Comparison:`);
  const result = compare(editor, react);

  if (result.match) {
    console.log("   ✅ MATCH — no structural differences found");
  } else {
    console.log(`   ❌ ${result.issues.length} issue(s) found:`);
    for (const issue of result.issues) {
      console.log(`   - [${issue.type}] ${issue.detail}`);
    }
  }

  console.log("\n" + "═".repeat(60));
  console.log(JSON.stringify(result.summary, null, 2));
  console.log("═".repeat(60));

  process.exit(result.match ? 0 : 1);
}

main().catch(e => { console.error(e); process.exit(1); });
