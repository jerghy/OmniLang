# Poiesis v2 Victauri Test Scripts

## Prerequisites

- Poiesis app running in Tauri debug mode
- Victauri MCP connected

## How to Run

Start Poiesis in debug mode with Victauri, then execute these scripts via Victauri MCP tools or browser console.

## Scripts

| File | Phase | Description |
|------|-------|-------------|
| `phase0-verify.js` | 0 | Type System + Cleanup — IPC health, backend command, memory baseline |
| `phase1-verify.js` | 1 | PID Annotation System — PID annotations, round-trip, compile check |
| `phase2-verify.js` | 2 | Unified Zustand Store — store snapshot, ghost commands, undo, memory |
| `phase3-verify.js` | 3 | Architecture Page — bubble rendering, linked expansion, mode switch, debug bridge, a11y |
| `phase4-verify.js` | 4 | Design Page + OmniViewRenderer — Z-axis, breadcrumbs, ContextInspector, tab behavior |
| `phase5-verify.js` | 5 | Code Canvas Nodes — 4 node types, skeleton edges, Monaco popup, old nodes removed, flow ports |
| `phase6-verify.js` | 6 | Data Persistence — save/load round-trip, project.json format, state restoration, file change detection |
| `phase7-verify.js` | 7 | End-to-End Regression — full flow, edge cases, final health check |
| `run-all.js` | All | Runner that imports all 8 phases and runs them sequentially |

## Usage

Each phase `<number>-verify.js` is self-contained. Run individually or use `run-all.js` to execute all phases sequentially.

```javascript
// Import and run a single phase in the browser console:
import { phase3_verify } from './phase3-verify.js';
await phase3_verify();

// Or run all phases:
import './run-all.js';
```
