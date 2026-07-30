#!/usr/bin/env node
/// OmniView runtime smoke test for CI.
///
/// Reads a generated HTML file (the full HTML document with inline <script>),
/// loads it in jsdom, then verifies:
///   1. #root element exists
///   2. #root has non-empty innerHTML
///   3. A [data-component] element is present
///   4. No runtime errors were recorded (window.__omni_runtimeErrors)
///   5. Signal text binding rendered initial value (body text contains "0")
///
/// Usage: node smoke_test.js <path-to-html>
///
/// Requires jsdom (npm install jsdom). Skips gracefully if unavailable.

"use strict";

const fs = require("fs");

const htmlPath = process.argv[2];
if (!htmlPath) {
  console.error("Usage: node smoke_test.js <path-to-html>");
  process.exit(1);
}

const html = fs.readFileSync(htmlPath, "utf-8");

// ---- jsdom availability check ----
let JSDOM;
try {
  JSDOM = require("jsdom").JSDOM;
} catch (_e) {
  console.log("SKIP: jsdom not available (install with: npm install jsdom)");
  process.exit(0);
}

// ---- Load HTML in jsdom ----
let dom;
try {
  dom = new JSDOM(html, {
    runScripts: "dangerously",
    resources: "usable",
    url: "http://localhost",
  });
} catch (e) {
  console.error("FAIL: jsdom load error:", e.message);
  process.exit(1);
}

const win = dom.window;
const doc = win.document;

// ---- Run checks ----
const checks = [];

// Check 1: root element exists
const root = doc.getElementById("root");
checks.push({ name: "root element exists", pass: root !== null });

// Check 2: root has non-empty innerHTML
checks.push({
  name: "root has content",
  pass: root !== null && root.innerHTML.trim().length > 0,
});

// Check 3: component rendered with [data-component] attribute
const compEl = doc.querySelector("[data-component]");
checks.push({
  name: "component rendered",
  pass: compEl !== null,
});

// Check 4: no runtime errors
const runtimeErrors = win.__omni_runtimeErrors || [];
checks.push({
  name: "no runtime errors",
  pass: runtimeErrors.length === 0,
});

// Check 5: signal text binding rendered initial value ("0" from count: i32 = 0)
const bodyText = doc.body.textContent || "";
checks.push({
  name: "signal value rendered",
  pass: bodyText.indexOf("0") >= 0,
});

// ---- Report ----
let allPass = true;
for (const c of checks) {
  const status = c.pass ? "  PASS" : "  FAIL";
  if (!c.pass) allPass = false;
  console.log(status + ": " + c.name);
}

if (runtimeErrors.length > 0) {
  console.error("");
  console.error("Runtime errors (" + runtimeErrors.length + "):");
  runtimeErrors.forEach(function (e) {
    console.error(
      "  [" + e.severity + "] " + e.code + ": " + e.message
    );
  });
}

if (allPass) {
  console.log("");
  console.log("  ✓ OmniView runtime smoke test passed");
  process.exit(0);
} else {
  console.log("");
  console.log("  ✗ OmniView runtime smoke test FAILED");
  process.exit(1);
}
