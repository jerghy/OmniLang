/**
 * Test script: verify bubble graph against learn-omniview project.
 *
 * Usage: npx vite-node scripts/verify-bubble-graph.ts
 */

import { readFileSync, readdirSync } from "fs";
import { join, relative, normalize } from "path";

const LEARN_DIR = normalize(
  "D:/space/OmniLang/.claude/worktrees/poiesis-v2-redesign/test/projects/learn-omniview/src",
);

interface OmniFileInfo {
  path: string;
  source: string;
}

function collectOmniFiles(dir: string, baseDir: string): OmniFileInfo[] {
  const files: OmniFileInfo[] = [];
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectOmniFiles(fullPath, baseDir));
    } else if (entry.name.endsWith(".omni")) {
      const relPath = relative(baseDir, fullPath).replace(/\\/g, "/");
      const source = readFileSync(fullPath, "utf-8");
      files.push({ path: `src/${relPath}`, source });
    }
  }
  return files;
}

const omniFiles = collectOmniFiles(LEARN_DIR, LEARN_DIR);
console.log(`Found ${omniFiles.length} .omni files\n`);

// Import and run the function
const { buildBubbleGraph } = await import(
  "../src/services/architecture/bubble-data"
);

const { bubbles, edges } = await buildBubbleGraph("", omniFiles);

// Print summary
console.log("=== Bubble Graph Summary ===\n");
console.log(`Total bubbles: ${bubbles.length}`);
console.log(`Total edges: ${edges.length}\n`);

console.log("--- Top-level (depth 0) ---");
for (const b of bubbles.filter((b) => b.depth === 0)) {
  console.log(`  ${b.label} (${b.id}, type=${b.type})`);
}

console.log("\n--- Depth 1 ---");
for (const b of bubbles.filter((b) => b.depth === 1)) {
  console.log(`  ${b.label} (${b.id}, type=${b.type}, parent=${b.parentId})`);
}

console.log("\n--- Depth 2 ---");
for (const b of bubbles.filter((b) => b.depth === 2)) {
  console.log(
    `  ${b.label} (${b.id}, parent=${b.parentId}, leaf=${b.isLeaf})`,
  );
}

console.log("\n--- Depth 3+ ---");
for (const b of bubbles.filter((b) => b.depth >= 3).sort((a, b) => a.depth - b.depth)) {
  console.log(
    `  [depth ${b.depth}] ${b.label} (${b.id}, parent=${b.parentId}, leaf=${b.isLeaf})`,
  );
}

console.log("\n--- Edges ---");
for (const e of edges) {
  const srcLabel = bubbles.find((b) => b.id === e.source)?.label || e.source;
  const tgtLabel = bubbles.find((b) => b.id === e.target)?.label || e.target;
  console.log(
    `  ${e.edgeType}: ${srcLabel} → ${tgtLabel}${e.label ? ` (${e.label})` : ""}`,
  );
}

console.log("\n--- Leaf Components ---");
for (const b of bubbles) {
  if (b.isLeaf && b.type === "ui") {
    console.log(`  ${b.label} (depth ${b.depth})`);
  }
}

console.log("\n--- Hierarchy ---");
function printHierarchy(bubbleId: string, indent: number, all: typeof bubbles): void {
  const b = all.find((bb) => bb.id === bubbleId);
  if (!b) return;
  console.log(
    `${"  ".repeat(indent)}${b.label}${b.isLeaf ? " [leaf]" : ""} (${b.type})`,
  );
  for (const childId of b.children) {
    printHierarchy(childId, indent + 1, all);
  }
}
for (const b of bubbles.filter((b) => b.depth === 0)) {
  printHierarchy(b.id, 0, bubbles);
}
