# Test-Framework Skill Design Spec

**Date**: 2026-06-13
**Status**: Approved
**Author**: Claude + jerghy

## Overview

Create a reusable Claude Code skill (`test-framework`) that executes a 7-stage closed-loop pipeline to validate the OmniLang framework by:

1. Building a real application in the visual editor (via omni-driver)
2. Transpiling it to React
3. Comparing visual editor output vs actual React runtime
4. Testing code logic
5. Fixing **framework code only** (NOT generated/transpiled code)
6. Looping until all checks pass
7. Git committing and pushing

## Architecture

```
.claude/skills/test-framework/
├── SKILL.md              # Main skill doc — 7-stage closed-loop workflow
├── compare.mjs            # Playwright script: editor→transpile→screenshot→compare
└── helpers/
    ├── transpile.py       # Python: call omniview API to transpile .omni → React
    └── diff-analyzer.mjs  # Helper: structural diff between editor state and React DOM
```

## 7-Stage Closed Loop

### Stage 1: Requirement Clarification (需求澄清)
- AI asks user what real app they want (dialog-driven)
- AI asks about: components, interactions, data flow, edge cases
- Output: app specification (saved as .omni source or written description)

### Stage 2: Visual Implementation (可视化实现)
- AI uses `omni-driver` skill + Browser MCP to build the app in the visual editor
- Uses Progressive Build-Test Loop (validate every 3-5 elements)
- App must include: UI components, event handlers, dataflow wires
- Output: fully functional canvas in visual editor

### Stage 3: Save & Transpile (保存转译)
- Save `.omni` source via `O.setSourceCode()` + `O.save()`
- Save `.omni.json` editor state alongside `.omni` file:
  - Contains `elements[]` (views, codeblocks with ports, props, parentId)
  - Contains `wires[]` (dataflow connections with fromPort/toPort)
  - Contains `components[]` (signals, computeds, fns, view tree)
  - Contains `stores[]`, `canvasLayout` (positions), `searchIndex`
  - Use `/api/export-omni-json` + `/api/save-omni-json` (or `O.exportState()`)
- Call `/api/compile` with target=react to get React code
- Write React files to `build_output/<app-name>/`
- Scaffold Vite project (package.json, vite.config.js, index.html, main.jsx)

### Stage 4: React Runtime & Screenshot (React运行截屏)
- Install npm dependencies
- Start Vite dev server
- Use Playwright (browser MCP) to capture:
  - Full-page screenshot of React app
  - DOM snapshot (accessibility tree)
  - Element inventory (buttons, inputs, headings, test-ids)

### Stage 5: Visual Diff Comparison (差异对比)
- Use Vision MCP (`agent-vision`) to compare:
  - Editor canvas screenshot vs React runtime screenshot
- Analyze differences:
  - Layout differences (position, size, spacing)
  - Missing elements
  - Styling differences (colors, fonts, borders)
  - Text content differences

### Stage 6: Logic Testing (逻辑测试)
- Use Browser MCP to interact with React app:
  - Click buttons, verify state changes
  - Type in inputs, verify updates
  - Check computed/derived values
  - Test edge cases (empty state, max values, error conditions)
- Compare behavior against visual editor's expected behavior

### Stage 7: Framework Fix (框架修复)
- **IRON RULE**: Only modify framework code. NEVER modify generated/transpiled code.
- Framework code locations:
  - `omniview/codegen/react.py` — React code generation
  - `omniview/codegen/css_mapping.py` — CSS/style mapping
  - `omniview/lowering.py` — IR lowering
  - `omniview/parser.py` — .omni parsing
  - `omniview/semantic/` — semantic analysis
  - `omnilang/codegen/` — core codegen
  - `editor/client/src/` — editor frontend
- Fix root causes, not symptoms

### Loop Condition
- If Stage 5 finds visual differences OR Stage 6 finds logic errors:
  - Analyze root cause
  - Fix framework code (Stage 7)
  - Go back to Stage 2 (re-implement/verify)
- If all checks pass: proceed to git commit & push

## Key Design Decisions

### Why Hybrid Mode (AI + Scripts)?
- AI handles contextual decisions (what to ask user, how to fix framework)
- Scripts handle deterministic automation (screenshot, transpile, DOM extraction)
- Vision MCP handles the fuzzy comparison that code can't easily do

### Why NOT modify generated code?
The goal is to test and improve the **framework** (transpiler, codegen, editor).
If we fix generated code, we're just papering over framework bugs.
Every fix must be in the framework so it benefits ALL future transpilations.

### Why Kanban Board as default but user-configurable?
- Kanban Board covers the most framework features (CRUD, drag, store, signals, computed)
- But the skill must be generic — user can specify any app
- AI adapts complexity based on user's request

## Component Coverage Matrix

The skill should ensure the chosen app covers these framework features:

| Feature | Kanban | TaskMgr | Counter |
|---------|--------|---------|---------|
| Component rendering | ✅ | ✅ | ✅ |
| Button events (onPress) | ✅ | ✅ | ✅ |
| Input events (onChange) | ✅ | ✅ | ❌ |
| Signal state | ✅ | ✅ | ✅ |
| Computed values | ✅ | ✅ | ❌ |
| Store/collections | ✅ | ❌ | ❌ |
| Conditional rendering | ✅ | ✅ | ❌ |
| List rendering | ✅ | ✅ | ❌ |
| CSS/Style mapping | ✅ | ✅ | ✅ |
| Form handling | ✅ | ✅ | ❌ |
| Multiple components | ✅ | ✅ | ❌ |

## Success Criteria

1. Visual comparison passes (no significant layout/style differences)
2. All interactive elements work correctly in React
3. Data flow (signal→computed→view) behaves identically
4. Framework code changes are minimal and root-cause targeted
5. No generated code was modified
6. Git commit has clear message describing what framework bug was fixed
