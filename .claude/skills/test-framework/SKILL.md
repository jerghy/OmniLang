---
name: test-framework
description: Use when testing OmniLang framework with real-world apps — build in visual editor via OmniDriver API, transpile to React, screenshot-compare with Vision MCP, test logic, fix framework code (never generated code), loop until passing, git push. Triggers: "test framework", "validate transpile", "real case test", "framework test"
---

# Test-Framework — OmniLang Framework Validation via Real-World Apps

## 🛜 MCP Availability Gate (MUST run FIRST)

**Before ANY work, verify MCP tools are connected.** Without these MCP tools the entire pipeline is broken.

```
Checklist:
1. mcp__playwright__browser_navigate — exists? (Browser MCP)
2. mcp__agent-vision__analyze_image — exists? (Vision MCP)
```

**If either is missing:** STOP immediately. Tell the user:
> "Browser MCP and/or Vision MCP are not connected. Please run `/mcp` to reconnect them, then re-invoke test-framework."

**Never proceed without both MCPs.** Do NOT write Playwright scripts as workarounds. Do NOT compare screenshots manually.

## Iron Rules (MUST follow — violating any is a STOP)

### 🔴 Iron Rule 1: Build UI via Visual Editor API ONLY
**Build ALL UI using OmniDriver programmatic API through Browser MCP.** The visual editor IS the thing being tested.

- ✅ `mcp__playwright__browser_evaluate` with `O.button()`, `O.card()`, `O.flex()`, `O.heading()`, `O.text()`, `O.input()`, `O.container()`, `O.codeBlock()`, etc.
- ✅ Use `O.help()` first — ALWAYS (omni-driver Iron Law)
- ✅ Use `O.switchMode("design")` then `O.clearAll()` to start
- ✅ Use `parentId` parameter for nesting: `O.button("OK", {variant:"primary"}, card.id)`
- ✅ Wire dataflow with `createEventHandler()` + `connectTo()`

- ❌ **NEVER write .omni source files directly** (`<Card>...</Card>` syntax in a file)
- ❌ **NEVER use `O.setSourceCode()` + `O.compile()`** to build UI from source
- ❌ **NEVER write Node.js Playwright scripts** to automate the editor
- ❌ **NEVER use Bash to run Playwright scripts** for editor interaction

**The .omni source file is DERIVED from the visual build, not the starting point.**
After building visually, extract the source via `O.getSourceCode()` for transpilation.

**No exceptions:**
- Don't rationalize "source is faster than building visually"
- Don't rationalize "I'll build some parts in source and some visually"
- Don't keep source-first approach as "scaffold" and then switch — delete it, start visually
- `O.setSourceCode()` is ONLY for loading saved state, NEVER for initial build

### 🔴 Iron Rule 2: NEVER Write React Code by Hand
**React output MUST be the direct, unmodified result of `/api/compile` transpilation.**

- ✅ Call `/api/compile` API → get `{ files: { "Component.tsx": "..." } }`
- ✅ Write `compileResult.files["Component.tsx"]` directly to `build_output/<app>/src/`
- ✅ Scaffold Vite project (package.json, index.html, main.tsx, vite.config.ts) — these are infrastructure, not generated code

- ❌ **NEVER hand-edit the generated .tsx file**
- ❌ **NEVER manually type React JSX**
- ❌ **NEVER "fix" the React output** — if it's wrong, fix the FRAMEWORK
- ❌ **NEVER copy-paste sections of transpiled output into a hand-written file**

**The ONLY React code you write is Vite scaffolding** (package.json, index.html, main.tsx entry point, vite.config.ts). These are project infrastructure, not generated code. The entry point `main.tsx` must only import and render the generated component — no other logic.

### 🔴 Iron Rule 3: Browser MCP for ALL Browser Interaction
**Every browser action goes through the MCP tools.** Never bypass them.

- ✅ `mcp__playwright__browser_navigate` — open pages
- ✅ `mcp__playwright__browser_evaluate` — run OmniDriver `O.*` commands
- ✅ `mcp__playwright__browser_click` — click elements
- ✅ `mcp__playwright__browser_type` — type in inputs
- ✅ `mcp__playwright__browser_take_screenshot` — capture screenshots
- ✅ `mcp__playwright__browser_snapshot` — capture DOM snapshots
- ✅ `mcp__agent-vision__analyze_image` — compare screenshots

- ❌ **NEVER write Node.js Playwright scripts** (`import { chromium } from 'playwright'`)
- ❌ **NEVER use Bash to run Playwright** (`node stage2-build.mjs`)
- ❌ **NEVER use Bash `curl` for API calls that can be made via `browser_evaluate`**

### 🔴 Iron Rule 4: Read omni-driver Skill FIRST
**Before any editor interaction, invoke the omni-driver skill.**

```
Skill tool with: skill="omni-driver"
```

Then follow its API reference exactly:
- Parameter positions for each method (button, text, heading, input, card, flex, container, image)
- Dataflow patterns (Button→Handler→Signal, Signal→Computed)
- Port naming (signal input port is "set", output is "value")
- Iron Law: `O.help()` first, every session

**No exceptions:**
- Don't rationalize "I already know the API"
- Don't rationalize "I read it in a previous session"
- The skill IS the canonical reference — read it fresh each time

### Iron Rule 5: NEVER modify generated/transpiled code
Only fix framework code.

### Iron Rule 6: Always save BOTH `.omni` AND `.omni.json`
After visual editor implementation.

### Iron Rule 7: Always use Progressive Build-Test Loop
Validate every 3-5 elements, not the whole UI at once.

### Iron Rule 8: Always run Vision MCP comparison
Never skip visual diff. Always compare CSS/styling separately.

### Iron Rule 9: Loop until all checks pass before git push

## ⚠️ Known Pitfalls (READ BEFORE STARTING)

### Pitfall 1: Programmatic Elements ≠ .omni Source
**`O.button()` etc. create elements in Zustand store only.** `getSourceCode()` returns the FILE content, not programmatic state. To transpile, you MUST also generate the .omni source separately.

✅ **Correct flow:**
1. Build UI visually via `O.button()`, `O.card()` etc. in design mode
2. Screenshot the visual result
3. Write the corresponding `.omni` source file based on what was built visually
4. Save `.omni.json` from `O.getState()`
5. Transpile the `.omni` source via `/api/compile`

### Pitfall 2: Python Bytecode Cache
After editing `omniview/codegen/*.py` or `omnilang/**/*.py`, the running server WON'T pick up changes because Python caches `.pyc` files. You MUST:
```bash
# Find and kill backend
PID=$(netstat -ano | grep ":5173 " | grep LISTENING | awk '{print $NF}')
taskkill //F //PID "$PID"
# Clear ALL caches
find . -name "__pycache__" -type d -exec rm -rf {} + 2>/dev/null
# Restart
python -m editor.server.main --dir build/test --port 5173 &
```

### Pitfall 3: Worktree vs Main Repo
If using a git worktree, Python imports from the **worktree** path, not the main repo. Edit the worktree's copy of framework files. Verify with:
```bash
python -c "import omniview.codegen.react; import inspect; print(inspect.getfile(omniview.codegen.react))"
```

### Pitfall 4: useCallback Stale Closure (CRITICAL)
Generated React code uses `useCallback(() => { setX(x + 1) }, [])`. The empty deps `[]` + direct state read `(x + 1)` causes STALE CLOSURE — x is always the initial value (0).

**This is the #1 framework bug to check.** Fix: all component signal mutations must use functional updaters:
- ❌ `setX(x + 1)` — reads stale closure value
- ✅ `setX(prev => prev + 1)` — always reads latest state

### Pitfall 5: Screenshot Staleness
After reloading the editor or changing the React app, **always retake screenshots**. Vision MCP may analyze cached/old files. Take fresh screenshots immediately before comparison.

### Pitfall 6: Port Conflicts
Vite auto-increments port if occupied (5201→5202→5203...). Always check the actual port Vite started on and use it for screenshots.

### Pitfall 7: Vision MCP Needs Explicit Prompts
Don't ask "compare these screenshots". Instead, ask SEPARATE targeted questions:
1. **Elements:** "List ALL visible UI elements with exact text"
2. **CSS:** "Describe button colors, card shadow, border radius, padding, text color"
3. **Layout:** "Describe flex direction, gap, alignment of product rows"

## Framework Code Locations (these you CAN modify)

| Path | Responsibility |
|------|---------------|
| `omniview/codegen/react.py` | React code generation |
| `omniview/codegen/css_mapping.py` | CSS/style mapping |
| `omniview/lowering.py` | IR lowering |
| `omniview/parser.py` | .omni parsing |
| `omniview/semantic/` | Semantic analysis |
| `omnilang/codegen/` | Core codegen |
| `editor/client/src/` | Editor frontend |

## Generated Code Locations (these you CANNOT modify)

- `build_output/<app-name>/` — All React output files
- Any `.jsx`/`.tsx`/`.css` produced by transpilation

## 7-Stage Closed Loop

### Stage 1: Requirement Clarification

Ask the user what real app to build. Cover:
- What does the app do?
- What components? (Button, Input, List, Card, etc.)
- What interactions? (click, type, submit)
- What data flow? (signals, computed values, stores)
- Edge cases? (empty state, error handling)

### Stage 2: Visual Implementation

> 🔴 **MUST use OmniDriver API via Browser MCP** (Iron Rules 1, 3, 4)

1. **Invoke omni-driver skill** (`Skill` tool with `skill="omni-driver"`)
2. `mcp__playwright__browser_navigate` to `http://localhost:5174`
3. `mcp__playwright__browser_evaluate`:
   - `O.help()` — ALWAYS first
   - `O.switchMode("design")` + `O.clearAll()`
   - Build UI using `O.card()`, `O.button()`, `O.flex()`, `O.heading()`, `O.text()`, `O.input()`, `O.container()`, `O.codeBlock()`
   - Wire events: `el.createEventHandler("onPress", { source: "..." })` + `handler.connectTo(signal, "result", "set")`
4. `mcp__playwright__browser_take_screenshot` — save editor screenshot
5. **Progressive:** repeat steps 3-4 for each chunk of 3-5 elements

### Stage 3: Save & Transpile

> 🔴 **.omni source is DERIVED from visual build** (Iron Rule 1). **React code is transpiler output only** (Iron Rule 2).

**3a. Write .omni source** — derive from the visual elements just built:
- Map each `O.button("Label", ...)` → `<Button ...>Label</Button>`
- Map each `O.codeBlock("signal", "name", ...)` → `signal name: Type = value`
- Map each handler → `fn handlerName() { ... }`
- Write to `build/test/<app-name>/<AppName>.omni`

**3b. Save .omni.json** — via Browser MCP `browser_evaluate`:
```javascript
const state = O.getState();
const src = O.getSourceCode() || "<derived .omni source from 3a>";
const resp = await fetch("/api/omni-json/export", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ source: src, filepath: "Component.omni", target: "react" }),
});
const { document: doc } = await resp.json();
doc.elements = state.elements;
doc.wires = state.wires;
await fetch("/api/omni-json/save", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ filePath: "build/test/<app>/Component.omni.json", document: doc }),
});
```

**3c. Transpile to React** — via Browser MCP `browser_evaluate`:
```javascript
const resp = await fetch("/api/compile", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ source: src, target: "react" }),
});
const { files } = await resp.json();
// files["Component.tsx"] is the generated React code
```

**3d. Write React output** — write files directly:
- `build_output/<app>/src/Component.tsx` ← `files["Component.tsx"]` (DO NOT EDIT)
- `build_output/<app>/src/main.tsx` ← entry point (scaffolding, you write this)
- `build_output/<app>/index.html` ← HTML shell (scaffolding, you write this)
- `build_output/<app>/package.json` ← dependencies (scaffolding, you write this)
- `build_output/<app>/vite.config.ts` ← Vite config (scaffolding, you write this)

### Stage 4: React Runtime Screenshot

> 🔴 **Use Browser MCP only** (Iron Rule 3)

1. Start Vite via Bash: `cd build_output/<app> && npx vite --port 5201 --host &`
2. Wait for server, check actual port
3. `mcp__playwright__browser_navigate` to `http://localhost:<port>`
4. `mcp__playwright__browser_take_screenshot` — save React screenshot
5. Capture `mcp__playwright__browser_snapshot` for element inventory

### Stage 5: Visual Diff Comparison

> 🔴 **Use Vision MCP only** (Iron Rule 3). Compare BOTH structure AND CSS separately (Pitfall 7).

**5a. Structure comparison:**
`mcp__agent-vision__analyze_image` with: "List ALL visible UI elements with exact text content"

**5b. CSS/Style comparison (SEPARATE call!):**
`mcp__agent-vision__analyze_image` with specific questions about:
- Card: background color? border-radius? box shadow? padding?
- Primary buttons: background? text color? border? border-radius?
- Outline buttons: background? border color?
- Layout: flex direction? gap? alignment?
- Text: color? font weight? font size?

**5c. Common CSS bugs checklist:**
- [ ] `variant` prop → inline style or useless `data-variant` attribute?
- [ ] Card `background` prop → NOT overridden by Card default styles?
- [ ] Button primary/outline visually distinct?
- [ ] `borderRadius: "50%"` → preserved as percentage, not converted to px?
- [ ] Multi-value CSS (padding, border-radius) correctly rendered?

### Stage 6: Logic Testing

> 🔴 **Use Browser MCP only** (Iron Rule 3)

Use `mcp__playwright__browser_click` and `browser_type` to interact:
- Click every button, verify state change via screenshot + Vision MCP
- Click multiple buttons, verify ACCUMULATED state (stale closure check — Pitfall 4)
- Type in inputs, verify updates
- Test edge cases: empty state, max/min, clear/reset
- **Repeat clicks after reset** to verify state works post-reset

### Stage 7: Framework Fix

**ONLY if Stage 5 or 6 found issues:**

1. Analyze root cause — use Known Pitfalls checklist
2. **Verify which Python path is active** (Pitfall 3)
3. Fix the framework code (NOT generated code — Iron Rule 5)
4. **Clear bytecode cache + restart backend** (Pitfall 2)
5. Verify fix: re-transpile → Vite auto-reloads → retake screenshots → re-compare with Vision MCP
6. Repeat until all checks pass

### Completion: Git Push

```bash
git add omniview/codegen/react.py  # or whichever framework file
git commit -m "fix(framework): <describe root cause and fix>"
git push
```

## Self-Check Checklist

- [ ] 🛜 MCP Availability Gate passed (both Browser + Vision MCP connected)
- [ ] omni-driver skill invoked and read (Stage 2, step 1)
- [ ] Asked user what app to build (Stage 1)
- [ ] Built UI via O.button()/O.card()/O.flex() etc. in design mode (Stage 2)
- [ ] Derived .omni source from visual build (Stage 3a)
- [ ] Saved BOTH .omni and .omni.json (Stage 3b)
- [ ] Transpiled via /api/compile — React code written unmodified (Stage 3c-3d)
- [ ] Cleared __pycache__ after any Python edits (Pitfall 2)
- [ ] Verified Python import path (Pitfall 3)
- [ ] Vision MCP: structure comparison done (Stage 5a)
- [ ] Vision MCP: CSS comparison done SEPARATELY (Stage 5b)
- [ ] Checked useCallback functional updaters (Pitfall 4)
- [ ] Checked variant→style mapping (Pitfall 5c)
- [ ] Logic testing: multi-click accumulation verified (Stage 6)
- [ ] NO generated code was modified (Iron Rule 5)
- [ ] NO React code was hand-written beyond scaffolding (Iron Rule 2)
- [ ] NO Playwright bash scripts were used (Iron Rule 3)
- [ ] Framework fix is root-cause, not symptom-patching
- [ ] All checks pass before git push

## Red Flags — STOP and Check

### Tool/MCP Red Flags
- **"I'll write a Playwright script to save time"** → STOP. Use Browser MCP (Iron Rule 3).
- **"Browser MCP isn't connected, I'll work around it"** → STOP. Tell user to run `/mcp` (MCP Gate).
- **"I'll just run node script.mjs to automate this"** → STOP. Use MCP tools (Iron Rule 3).

### Visual Editor Red Flags
- **"I'll write the .omni source first, it's faster"** → STOP. Build visually with O.button() (Iron Rule 1).
- **"Let me use setSourceCode() + compile() to scaffold"** → STOP. Build in design mode (Iron Rule 1).
- **"The source approach and visual approach are equivalent"** → STOP. They are NOT equivalent — only visual tests the framework (Iron Rule 1).
- **"I already know the omni-driver API"** → STOP. Invoke the skill fresh each time (Iron Rule 4).
- **"Let me skip O.help(), I know what I'm doing"** → STOP. O.help() reveals port UUIDs and critical rules not visible in source.

### React Code Red Flags
- **"I'll just fix the generated React code"** → STOP. Fix the framework (Iron Rule 5).
- **"Let me edit the TSX to fix a rendering issue"** → STOP. Fix the framework (Iron Rule 5).
- **"The transpiled output is close, let me tweak it"** → STOP. Write unmodified output (Iron Rule 2).
- **"I'll write the React component by hand based on the editor state"** → STOP. Use /api/compile output ONLY (Iron Rule 2).

### Process Red Flags
- **"The screenshots look close enough"** → STOP. Run Vision MCP for precise diff (Iron Rule 8).
- **"I'll skip .omni.json — .omni is enough"** → STOP. Both must be saved (Iron Rule 6).
- **"I'll build the whole UI then validate"** → STOP. Progressive Build-Test Loop only (Iron Rule 7).
- **"The Python fix is in the file but API still returns old code"** → STOP. Clear __pycache__ + restart (Pitfall 2).
- **"Vision MCP says everything matches" after one prompt** → STOP. Run separate CSS prompt (Pitfall 7).
- **"useCallback deps are empty but it should be fine"** → STOP. Check Pitfall 4 — this IS a bug.

## Common Rationalizations and Their Counters

| Rationalization | Counter |
|----------------|---------|
| "Writing .omni source is faster than building visually" | Speed isn't the goal — testing the framework IS. Visual build = test. Source = bypass. |
| "I can scaffold with source then switch to visual" | Delete source. Start with visual. Scaffolding creates frameworks you don't test. |
| "Playwright scripts are more reliable than MCP" | MCP is the canonical interface. Scripts bypass the skill's checks and balances. |
| "I read omni-driver last session, I remember it" | API evolves. Port UUIDs change. Read it fresh. |
| "The React output just needs one small fix" | Every "small fix" masks a framework bug. Fix the root cause. |
| "Browser MCP is slow, let me use curl" | MCP provides visual feedback you need. Speed isn't the priority. |
| "I'll build in my head and write source directly" | You're testing your mental model, not the framework. Build visually. |
