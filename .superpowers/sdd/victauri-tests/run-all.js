// Poiesis v2 Victauri Test Runner
// Prerequisites: Poiesis app running in Tauri debug mode, Victauri MCP connected
// Run this script to execute all 8 phases sequentially

import { phase0_verify } from './phase0-verify.js';
import { phase1_verify } from './phase1-verify.js';
import { phase2_verify } from './phase2-verify.js';
import { phase3_verify } from './phase3-verify.js';
import { phase4_verify } from './phase4-verify.js';
import { phase5_verify } from './phase5-verify.js';
import { phase6_verify } from './phase6-verify.js';
import { phase7_verify } from './phase7-verify.js';

const phases = [
  { name: "Phase 0", fn: phase0_verify },
  { name: "Phase 1", fn: phase1_verify },
  { name: "Phase 2", fn: phase2_verify },
  { name: "Phase 3", fn: phase3_verify },
  { name: "Phase 4", fn: phase4_verify },
  { name: "Phase 5", fn: phase5_verify },
  { name: "Phase 6", fn: phase6_verify },
  { name: "Phase 7", fn: phase7_verify },
];

let passed = 0;
let failed = 0;
const failures = [];

for (const phase of phases) {
  console.log(`\n═══════════════════════════════════════════`);
  console.log(`  Running ${phase.name}...`);
  console.log(`═══════════════════════════════════════════\n`);

  try {
    await phase.fn();
    console.log(`\n✓ ${phase.name} PASSED`);
    passed++;
  } catch (error) {
    console.error(`\n✗ ${phase.name} FAILED:`);
    console.error(`  ${error.message || error}`);
    failed++;
    failures.push({ name: phase.name, error: error.message || error });
  }
}

console.log(`\n═══════════════════════════════════════════`);
console.log(`  Poiesis v2 Test Summary`);
console.log(`═══════════════════════════════════════════\n`);
console.log(`  Total: ${phases.length}`);
console.log(`  Passed: ${passed}`);
console.log(`  Failed: ${failed}`);

if (failures.length > 0) {
  console.log(`\n  Failed phases:`);
  for (const f of failures) {
    console.log(`    - ${f.name}: ${f.error}`);
  }
}

console.log(`\n${'═'.repeat(47)}`);
