---
name: test-framework
description: Use when testing OmniLang framework with real-world apps — build in visual editor via OmniDriver API, transpile to React/Flutter, screenshot-compare with Vision MCP, test logic, fix framework code (never generated code), loop until passing, git push. Triggers: "test framework", "validate transpile", "real case test", "framework test", "flutter test"
---

# Test-Framework — OmniLang Framework Validation via Real-World Apps

## 🛜 MCP Availability Gate (MUST run FIRST)

**Before ANY work, verify MCP tools are connected.** Without these MCP tools the entire pipeline is broken.

```
Checklist:
1. mcp__playwright__browser_navigate — exists? (Browser MCP)
2. mcp__agent-vision__analyze_image — exists? (Vision MCP)
3. mcp__flutter-skill__launch_app — exists? (Flutter MCP — for Flutter target testing)
```

**If any required MCP is missing:** STOP immediately. Tell the user:
> "Browser MCP and/or Vision MCP are not connected. Please run `/mcp` to reconnect them, then re-invoke test-framework."

**For Flutter testing** (target=flutter in Stage 3), also check flutter-skill MCP is available. If missing, Flutter testing stages will be skipped and only React comparison will run.

**Never proceed without both Browser MCP and Vision MCP.** Do NOT write Playwright scripts as workarounds. Do NOT compare screenshots manually.

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
| `omniview/codegen/flutter.py` | Flutter/Dart code generation |
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
git add omniview/codegen/react.py omniview/codegen/flutter.py  # or whichever framework file
git commit -m "fix(framework): <describe root cause and fix>"
git push
```

---
## Flutter Testing Pipeline (target=flutter)

> **Prerequisite:** flutter-skill MCP must be available (checked in MCP Gate).
> **Single reusable project:** All Flutter tests share ONE project at `build_output/omni_flutter_app/`. Each test overwrites the component file and `lib/main.dart`.

### Stage F1: Flutter Transpile

**F1a. Compile to Flutter/Dart** — via Browser MCP `browser_evaluate`:
```javascript
const resp = await fetch("/api/compile", {
  method: "POST", headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ source: src, target: "flutter" }),
});
const { files } = await resp.json();
// files["Component.dart"] is the generated Flutter code
```

**F1b. Write to shared Flutter project:**
- `build_output/omni_flutter_app/lib/<component_snake>.dart` ← `files["Component.dart"]` (DO NOT EDIT)
- `build_output/omni_flutter_app/lib/main.dart` ← test harness (update import + home widget)

**F1c. Run dart analyze** — verify no compilation errors:
```bash
cd build_output/omni_flutter_app && dart analyze lib/
```
- `error` level issues → fix framework (flutter.py), NOT generated code
- `info` level issues (child ordering, unnecessary braces) → acceptable, note for later

### Stage F2: Flutter Launch & Screenshot

**F2a. Launch Flutter web app** — via Browser MCP:
```bash
cd build_output/omni_flutter_app && flutter run -d edge --web-port=8080 &
```
- Wait ~12s for "Flutter run key commands" prompt
- Verify with `curl http://localhost:8080/`

**F2b. Navigate & enable accessibility** — via Browser MCP:
```javascript
// Navigate
mcp__playwright__browser_navigate → http://localhost:8080

// Enable Flutter accessibility (exposes semantics tree as DOM)
document.querySelector('flt-semantics-placeholder').click()
```

**F2c. Capture screenshot:**
`mcp__playwright__browser_take_screenshot` → save as `flutter-<app-name>.png`

### Stage F3: Flutter Visual Diff Comparison

> 🔴 Use Vision MCP. Compare Flutter vs React rendering SEPARATELY (structure + CSS).

**F3a. Structure comparison:**
`mcp__agent-vision__analyze_image` with: "List ALL visible UI elements with exact text content"

**F3b. CSS/Style comparison (SEPARATE call!):**
`mcp__agent-vision__analyze_image` with specific questions about:
- Button types: ElevatedButton (filled/primary) vs OutlinedButton (outline) vs TextButton (ghost)?
- Are `variant="primary"` buttons filled and `variant="outline"` buttons outlined?
- Card background color, border-radius, padding
- Text alignment (centered vs left-aligned)
- Typography: font sizes, font weights

**F3c. Flutter-specific CSS bugs checklist:**
- [ ] `variant` prop → correct Flutter button type? (primary→ElevatedButton, outline→OutlinedButton, ghost→TextButton)
- [ ] Card with multiple children → wrapped in `Column` (not raw `children: [...]`)?
- [ ] `$` signs in text properly escaped for Dart string interpolation?
- [ ] Handler mutations wrapped in `setState()` for Flutter rebuild?

### Stage F4: Flutter Logic Testing

> 🔴 Use Browser MCP to click Flutter semantics elements.

**F4a. Find button elements:**
```javascript
const buttons = document.querySelectorAll('flt-semantics[role="button"]');
// buttons[0] = first "Add to Cart", buttons[N-1] = "Clear Cart"
```

**F4b. Click and verify state:**
```javascript
// Click button
buttons[0].dispatchEvent(new MouseEvent('click', { bubbles: true }));

// Read state from accessibility group
const group = document.querySelector('flt-semantics[role="group"]');
const label = group.getAttribute('aria-label');
// label updates asynchronously — wait ~1s after click before reading
```

**F4c. Test sequence:**
- Click each action button individually → verify state update
- Click same button multiple times → verify accumulation (stale closure check)
- Click Clear/Reset → verify state returns to initial
- Click after reset → verify state still works post-reset

### Flutter-Specific Pitfalls

### Flutter Pitfall 1: No setState in fn handlers
Named handler functions (`fn increment()`) generate methods WITHOUT `setState()` if the body has no `.` character access. Flutter **requires** `setState()` for any mutation to trigger rebuild.

**Fix location:** `omniview/codegen/flutter.py` → `_gen_component` handler method generation (~line 169-186)

**Fix:** Always wrap handler bodies in `setState(() { ... })`, not conditionally.

### Flutter Pitfall 2: Card uses `children` instead of `child`
Flutter `Card` widget accepts a single `child` parameter. Multiple children must be wrapped in `Column`.

**Fix location:** `omniview/codegen/flutter.py` → `_build_widget` (~line 534)

**Fix:** When a non-multi-child widget has >1 child, wrap children in `Column(children: [...])` and pass as `child:`.

### Flutter Pitfall 3: Dollar signs in Dart strings
Dart treats `$` as string interpolation prefix. Literal `$` signs (currency) must be escaped as `\$`.

**Fix location:** `omniview/codegen/flutter.py` → `_expr_to_dart` LiteralExpr path (~line 743)

**Fix:** Replace `$` with `\$` in string literal values before wrapping in quotes.

### Flutter Pitfall 4: Button variant ignored
The Flutter codegen always generates `ElevatedButton` regardless of the `variant` prop.

**Fix location:** `omniview/codegen/flutter.py` → Button element handler (~line 466)

**Fix:** Map `variant="primary"` → `ElevatedButton`, `variant="outline"` → `OutlinedButton`, `variant="ghost"` → `TextButton`.

### Flutter Pitfall 5: Accessibility required for testing
Flutter web renders into `<flt-glass-pane>` canvas by default. DOM-based testing requires enabling accessibility mode via clicking the `flt-semantics-placeholder` element. Without this, only coordinate-based clicking is possible.

### Flutter Pitfall 6: Semantics tree async update
After clicking a button in Flutter, the accessibility tree (`aria-label` on `flt-semantics[role="group"]`) updates asynchronously. Always `browser_wait_for` ~1s before reading state after interactions.

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
- [ ] (Flutter) dart analyze passes with no errors
- [ ] (Flutter) Button variant→type mapping verified (primary=ElevatedButton, outline=OutlinedButton)
- [ ] (Flutter) Card children wrapped in Column (not raw children: [...])
- [ ] (Flutter) All handler mutations use setState() wrapper
- [ ] (Flutter) Dollar signs in strings escaped for Dart
- [ ] (Flutter) Logic testing: multi-click accumulation verified via semantics tree
- [ ] (Flutter) Vision MCP: Flutter vs React structural comparison done
- [ ] (Flutter) Vision MCP: Flutter vs React CSS comparison done SEPARATELY
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

### Flutter Red Flags
- **"Dart errors are just linter warnings"** → STOP. Fix framework for all `error` level issues. `info` level only are acceptable.
- **"The Flutter button looks the same whether variant=primary or outline"** → STOP. Fix variant→type mapping in flutter.py.
- **"Card is empty / shows nothing"** → STOP. Check if Card has `children:` instead of `child: Column(children: [...])`.
- **"Flutter doesn't rebuild on button click"** → STOP. Check if handler methods use `setState()`. Flutter Pitfall 1.
- **"Playwright can't find buttons in Flutter web"** → STOP. Enable accessibility mode first (Flutter Pitfall 5).

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
