# Poiesis Design Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate Poiesis editor chrome from indigo cool-tone to terracotta warm-tone design system per spec `docs/superpowers/specs/2026-07-24-poiesis-design-migration-design.md`.

**Architecture:** Token-first approach — rewrite `tokens.css` with all new warm-tone values, then sweep all CSS Modules replacing hardcoded hex colors with `var(--ol-*)` references. Delete legacy bridge. Archive Design-System sources for reference.

**Tech Stack:** CSS Custom Properties, CSS Modules, React 19 + TypeScript 6.0, Vite 8.x

## Global Constraints

- Token namespace MUST stay `--ol-*` — no renames, only value changes
- Do NOT touch `design.omni`, `src/core/types/design.ts`, `src/store/slices/design-system.ts`, or any `.tsx` logic code
- ring-shadow replaces all drop-shadows per spec section 3.4 / 3.6
- Hardcoded hex colors in CSS files MUST be replaced with `var(--ol-*)` wherever a matching token exists
- TypeScript `pnpm tsc --noEmit` must pass with zero errors after all changes
- CodeCard semantic colors (let=purple, set=blue, return=red, if=yellow, etc.) are intentionally preserved — they encode code structure semantics, not brand theming

---

### Task 1: Archive Design-System sources to docs/design/

**Files:**
- Create: `docs/design/DESIGN.md`
- Create: `docs/design/guide.md`
- Create: `docs/design/variables.css`
- Create: `docs/design/tokens.default.json`

**Produces:** Reference archive of the original Design-System so future maintainers can trace where the token values came from.

- [ ] **Step 1: Copy files from Design-System to docs/design/**

```bash
mkdir -p docs/design
cp Design-System/DESIGN.md docs/design/DESIGN.md
cp Design-System/guide.md docs/design/guide.md
cp Design-System/system/variables.css docs/design/variables.css
cp Design-System/system/tokens.default.json docs/design/tokens.default.json
```

- [ ] **Step 2: Add a README in docs/design/ explaining what these files are**

Create `docs/design/README.md`:

```markdown
# Poiesis Design Reference

These files are the original Design-System sources (Claude/Anthropic-inspired
brand system) from which the Poiesis `--ol-*` tokens in `src/styles/tokens.css`
were derived.

- `DESIGN.md` — Original design brief
- `guide.md` — Condensed brand guide
- `variables.css` — Original `--brand-*` CSS custom properties
- `tokens.default.json` — Full derived token set (Ant Design format)

**DO NOT edit these files.** They are a historical reference. To change
Poiesis chrome styling, edit `src/styles/tokens.css`.
```

- [ ] **Step 3: Commit**

```bash
git add docs/design/
git commit -m "docs: archive Design-System sources as design reference"
```

---

### Task 2: Rewrite tokens.css — light mode (complete file replacement)

**Files:**
- Modify: `poiesis/src/styles/tokens.css` (entire file)

**Interfaces:**
- Produces: All `--ol-*` CSS custom properties in `:root` with new warm-tone values
- Consumed by: Every CSS Module in the project that references `var(--ol-*)`

- [ ] **Step 1: Write the full rewritten tokens.css**

Replace the entire content of `poiesis/src/styles/tokens.css` with the file below. This is a full-file replacement — the structure (comment sections, variable names, organization) stays identical; only the values change.

```css
/* ═══════════════════════════════════════════════
   Poiesis Design Tokens (--ol-* namespace)
   Derived from Claude/Anthropic-inspired brand system
   Primary: Terracotta #c96442 | Canvas: Parchment #f5f4ed
   Light (default) + Dark ([data-theme="dark"])
   ═══════════════════════════════════════════════ */

:root {
  /* ═══════════════════════════════════════════════
     Color: Gray Scale (50–950) — warm yellow-brown undertone
     ═══════════════════════════════════════════════ */
  --ol-gray-50:  #f1f0e9;
  --ol-gray-100: #ecebe4;
  --ol-gray-200: #e7e7e0;
  --ol-gray-300: #d3d2cc;
  --ol-gray-400: #bdbcb7;
  --ol-gray-500: #908f8b;
  --ol-gray-600: #63625f;
  --ol-gray-700: #2f2f2d;
  --ol-gray-800: #141413;
  --ol-gray-900: #141413;
  --ol-gray-950: #141413;

  /* ═══════════════════════════════════════════════
     Color: Semantic — Primary (Terracotta #c96442)
     ═══════════════════════════════════════════════ */
  --ol-color-primary:         #c96442;
  --ol-color-primary-hover:   #d68869;
  --ol-color-primary-active:  #a3472e;
  --ol-color-primary-subtle:  #fff6f0;
  --ol-color-primary-muted:   #f0d2c2;

  /* ── Success ── */
  --ol-color-success:         #52c41a;
  --ol-color-success-hover:   #72d13d;
  --ol-color-success-active:  #389e0d;
  --ol-color-success-subtle:  #f6ffed;
  --ol-color-success-muted:   #b6eb8f;

  /* ── Warning ── */
  --ol-color-warning:         #faad14;
  --ol-color-warning-hover:   #ffc53d;
  --ol-color-warning-active:  #d48806;
  --ol-color-warning-subtle:  #fffbe6;
  --ol-color-warning-muted:   #ffe58f;

  /* ── Danger ── */
  --ol-color-danger:          #ff4d4f;
  --ol-color-danger-hover:    #ff7875;
  --ol-color-danger-active:   #d9363d;
  --ol-color-danger-subtle:   #fff2f0;
  --ol-color-danger-muted:    #ffccc7;

  /* ── Info ── */
  --ol-color-info:            #c96442;
  --ol-color-info-hover:      #d68869;
  --ol-color-info-active:     #a3472e;
  --ol-color-info-subtle:     #fff6f0;
  --ol-color-info-muted:      #f0d2c2;

  /* ═══════════════════════════════════════════════
     Color: Surface & Border & Text
     ═══════════════════════════════════════════════ */
  --ol-surface:               #f5f4ed;
  --ol-surface-hover:         #ecebe4;
  --ol-surface-active:        #fff6f0;
  --ol-surface-raised:        #ecebe4;
  --ol-surface-overlay:       #f5f4ed;
  --ol-surface-inverse:       #141413;

  --ol-border:                #e8e5dc;
  --ol-border-hover:          #d3d2cc;
  --ol-border-active:         #c96442;
  --ol-border-subtle:         #f0eee6;
  --ol-border-strong:         #908f8b;

  --ol-text:                  #141413;
  --ol-text-secondary:        #5e5d59;
  --ol-text-muted:            #908f8b;
  --ol-text-inverse:          #f5f4ed;
  --ol-text-link:             #c96442;
  --ol-text-link-hover:       #d68869;

  /* ═══════════════════════════════════════════════
     Spacing (4px base grid)
     ═══════════════════════════════════════════════ */
  --ol-space-0:   0px;
  --ol-space-0p5: 2px;
  --ol-space-1:   4px;
  --ol-space-1p5: 6px;
  --ol-space-2:   8px;
  --ol-space-2p5: 10px;
  --ol-space-3:   12px;
  --ol-space-3p5: 14px;
  --ol-space-4:   16px;
  --ol-space-5:   20px;
  --ol-space-6:   24px;
  --ol-space-7:   28px;
  --ol-space-8:   32px;
  --ol-space-9:   36px;
  --ol-space-10:  40px;
  --ol-space-12:  48px;
  --ol-space-14:  56px;
  --ol-space-16:  64px;
  --ol-space-20:  80px;
  --ol-space-24:  96px;

  /* ═══════════════════════════════════════════════
     Typography
     ═══════════════════════════════════════════════ */
  --ol-font-family-ui:   'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  --ol-font-family-mono: 'JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', 'Courier New', monospace;

  --ol-font-size-10: 10px;
  --ol-font-size-11: 11px;
  --ol-font-size-12: 12px;
  --ol-font-size-13: 13px;
  --ol-font-size-14: 14px;
  --ol-font-size-15: 15px;
  --ol-font-size-16: 16px;
  --ol-font-size-18: 18px;
  --ol-font-size-20: 20px;
  --ol-font-size-24: 24px;
  --ol-font-size-28: 28px;
  --ol-font-size-32: 32px;
  --ol-font-size-36: 36px;

  --ol-font-weight-400: 400;
  --ol-font-weight-500: 500;
  --ol-font-weight-600: 600;
  --ol-font-weight-700: 700;

  --ol-line-height-tight:   1.25;
  --ol-line-height-normal:  1.5;
  --ol-line-height-relaxed: 1.75;

  /* ═══════════════════════════════════════════════
     Border Radius
     ═══════════════════════════════════════════════ */
  --ol-radius-none: 0px;
  --ol-radius-sm:   4px;
  --ol-radius-md:   8px;
  --ol-radius-lg:   8px;
  --ol-radius-xl:   12px;
  --ol-radius-2xl:  16px;
  --ol-radius-full: 9999px;

  /* ═══════════════════════════════════════════════
     Shadows — ring-shadow depth system (warm rings, min 6% opacity)
     ═══════════════════════════════════════════════ */
  --ol-shadow-sm:      0 0 0 1px rgba(0, 0, 0, 0.06);
  --ol-shadow-md:      0 0 0 1px rgba(0, 0, 0, 0.08);
  --ol-shadow-lg:      0 0 0 1px rgba(0, 0, 0, 0.10);
  --ol-shadow-xl:      0 0 0 1px rgba(0, 0, 0, 0.12);
  --ol-shadow-2xl:     0 0 0 2px rgba(0, 0, 0, 0.10);
  --ol-shadow-popover:  0 0 0 2px rgba(0, 0, 0, 0.10), 0 4px 12px rgba(0, 0, 0, 0.06);
  --ol-shadow-modal:    0 0 0 2px rgba(0, 0, 0, 0.12), 0 8px 32px rgba(0, 0, 0, 0.08);
  --ol-shadow-drawer:   0 0 0 2px rgba(0, 0, 0, 0.10);
  --ol-shadow-dropdown: 0 0 0 2px rgba(0, 0, 0, 0.08), 0 2px 8px rgba(0, 0, 0, 0.06);

  /* ═══════════════════════════════════════════════
     Z-Index
     ═══════════════════════════════════════════════ */
  --ol-z-base:     0;
  --ol-z-dropdown: 100;
  --ol-z-sticky:   200;
  --ol-z-popover:  300;
  --ol-z-tooltip:  400;
  --ol-z-overlay:  500;
  --ol-z-modal:    600;
  --ol-z-toast:    700;

  /* ═══════════════════════════════════════════════
     Animation
     ═══════════════════════════════════════════════ */
  --ol-duration-fast:   100ms;
  --ol-duration-normal: 200ms;
  --ol-duration-slow:   300ms;
  --ol-duration-slide:  400ms;

  --ol-ease-default: cubic-bezier(0.645, 0.045, 0.355, 1);
  --ol-ease-in:       cubic-bezier(0.4, 0, 1, 1);
  --ol-ease-out:      cubic-bezier(0.215, 0.61, 0.355, 1);
  --ol-ease-in-out:   cubic-bezier(0.645, 0.045, 0.355, 1);
  --ol-ease-spring:   cubic-bezier(0.34, 1.56, 0.64, 1);
  --ol-ease-bounce:   cubic-bezier(0.34, 1.56, 0.64, 1);

  /* ═══════════════════════════════════════════════
     Focus Ring
     ═══════════════════════════════════════════════ */
  --ol-focus-ring-width:  2px;
  --ol-focus-ring-offset: 2px;
  --ol-focus-ring-color:  rgba(201, 100, 66, 0.35);

  /* ═══════════════════════════════════════════════
     Layout
     ═══════════════════════════════════════════════ */
  --ol-header-height:    44px;
  --ol-toolbar-height:   36px;
  --ol-statusbar-height: 24px;
  --ol-sidebar-width:    240px;
  --ol-inspector-width:  280px;
}

/* ═══════════════════════════════════════════════════════════════════
   Dark Mode
   ═══════════════════════════════════════════════════════════════════ */
[data-theme="dark"] {
  /* ── Gray Scale (warm-undertone darks) ── */
  --ol-gray-50:  #1d1d1d;
  --ol-gray-100: #272727;
  --ol-gray-200: #303030;
  --ol-gray-300: #3e3e3e;
  --ol-gray-400: #4f4f4f;
  --ol-gray-500: #7e7e7e;
  --ol-gray-600: #adadad;
  --ol-gray-700: #dcdcdc;
  --ol-gray-800: #dcdcdc;
  --ol-gray-900: #dcdcdc;
  --ol-gray-950: #ffffff;

  /* ── Primary (Terracotta, lifted for dark bg) ── */
  --ol-color-primary:         #b9775c;
  --ol-color-primary-hover:   #ce9e87;
  --ol-color-primary-active:  #925f4b;
  --ol-color-primary-subtle:  rgba(185, 119, 92, 0.15);
  --ol-color-primary-muted:   rgba(185, 119, 92, 0.3);

  /* ── Success ── */
  --ol-color-success:         #64b537;
  --ol-color-success-hover:   #87ca5c;
  --ol-color-success-active:  #518f2f;
  --ol-color-success-subtle:  rgba(82, 196, 26, 0.15);
  --ol-color-success-muted:   rgba(100, 181, 55, 0.3);

  /* ── Warning ── */
  --ol-color-warning:         #dcaa37;
  --ol-color-warning-hover:   #e8c35e;
  --ol-color-warning-active:  #ad872f;
  --ol-color-warning-subtle:  rgba(250, 173, 20, 0.15);
  --ol-color-warning-muted:   rgba(220, 170, 55, 0.3);

  /* ── Danger ── */
  --ol-color-danger:          #dc6966;
  --ol-color-danger-hover:    #e89590;
  --ol-color-danger-active:   #ad5553;
  --ol-color-danger-subtle:   rgba(255, 77, 79, 0.15);
  --ol-color-danger-muted:    rgba(220, 105, 102, 0.3);

  /* ── Info ── */
  --ol-color-info:            #b9775c;
  --ol-color-info-hover:      #ce9e87;
  --ol-color-info-active:     #925f4b;
  --ol-color-info-subtle:     rgba(185, 119, 92, 0.15);
  --ol-color-info-muted:      rgba(185, 119, 92, 0.3);

  /* ── Surface & Border & Text ── */
  --ol-surface:               #141414;
  --ol-surface-hover:         #272727;
  --ol-surface-active:        rgba(185, 119, 92, 0.15);
  --ol-surface-raised:        #222222;
  --ol-surface-overlay:       #272727;
  --ol-surface-inverse:       #f5f4ed;

  --ol-border:                #343434;
  --ol-border-hover:          #3e3e3e;
  --ol-border-active:         #b9775c;
  --ol-border-subtle:         #303030;
  --ol-border-strong:         #4f4f4f;

  --ol-text:                  #dcdcdc;
  --ol-text-secondary:        #adadad;
  --ol-text-muted:            #7e7e7e;
  --ol-text-inverse:          #141413;
  --ol-text-link:             #b9775c;
  --ol-text-link-hover:       #ce9e87;

  /* ── Shadows (light rings on dark bg, min 8% opacity) ── */
  --ol-shadow-sm:      0 0 0 1px rgba(255, 255, 255, 0.08);
  --ol-shadow-md:      0 0 0 1px rgba(255, 255, 255, 0.10);
  --ol-shadow-lg:      0 0 0 1px rgba(255, 255, 255, 0.12);
  --ol-shadow-xl:      0 0 0 1px rgba(255, 255, 255, 0.14);
  --ol-shadow-2xl:     0 0 0 2px rgba(255, 255, 255, 0.12);
  --ol-shadow-popover:  0 0 0 2px rgba(255, 255, 255, 0.12), 0 4px 12px rgba(0, 0, 0, 0.3);
  --ol-shadow-modal:    0 0 0 2px rgba(255, 255, 255, 0.14), 0 8px 32px rgba(0, 0, 0, 0.4);
  --ol-shadow-drawer:   0 0 0 2px rgba(255, 255, 255, 0.12);
  --ol-shadow-dropdown: 0 0 0 2px rgba(255, 255, 255, 0.10), 0 2px 8px rgba(0, 0, 0, 0.3);

  /* ── Focus Ring (brighter in dark) ── */
  --ol-focus-ring-color: rgba(185, 119, 92, 0.4);
}
```

- [ ] **Step 2: Verify token count — every old variable name still exists**

```bash
# Extract old variable names from git
git show HEAD:poiesis/src/styles/tokens.css | grep -oP -- '--ol-[\w-]+' | sort -u > /tmp/old-tokens.txt
# Extract new variable names
grep -oP -- '--ol-[\w-]+' poiesis/src/styles/tokens.css | sort -u > /tmp/new-tokens.txt
# Diff — should be empty (all old names preserved)
diff /tmp/old-tokens.txt /tmp/new-tokens.txt
```

Expected: No output (no missing or extra tokens).

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/styles/tokens.css
git commit -m "feat: migrate --ol-* tokens to warm terracotta design system"
```

---

### Task 3: Replace hardcoded hex colors in CodeCardEditor.module.css

**Files:**
- Modify: `poiesis/src/components/canvas/code/CodeCardEditor.module.css`

**Interfaces:**
- Consumes: `var(--ol-*)` tokens defined in Task 2
- Produces: No semantic change — visual rendering only

**Constraint:** Semantic code colors (let=purple #a78bfa, set=blue #60a5fa, return=red #f87171, if=yellow #fbbf24, while=orange #f97316, for=green #34d399) are preserved — they encode code structure meaning. Only shell/ chrome colors are replaced.

- [ ] **Step 1: Replace shell chrome colors in CodeCardEditor.module.css**

Edit `poiesis/src/components/canvas/code/CodeCardEditor.module.css`, applying these replacements:

| Line(s) | Old value | New value | Token |
|---------|-----------|-----------|-------|
| 15 | `background: #fff` | `background: var(--ol-surface)` | Surface |
| 28 | `border-bottom: 1px solid #e2e8f0` | `border-bottom: 1px solid var(--ol-border)` | Border |
| 44 | `background: #6366f1` | `background: var(--ol-color-primary)` | Primary |
| 45 | `color: #fff` | `color: var(--ol-text-inverse)` | Text inverse |
| 54 | `background: #e2e8f0` | `background: var(--ol-gray-200)` | Gray 200 |
| 55 | `color: #475569` | `color: var(--ol-text-secondary)` | Text secondary |
| 65 | `color: #6366f1` | `color: var(--ol-color-primary)` | Primary |
| 66 | `border: 1px solid #6366f1` | `border: 1px solid var(--ol-color-primary)` | Primary |
| 164 | `outline: 2px solid #6366f1` | `outline: 2px solid var(--ol-color-primary)` | Primary |
| 170 | `outline: 2px solid #4c4cff` | `outline: 2px solid var(--ol-color-primary-hover)` | Primary hover |
| 177, 182 | `border-right: 2px solid #4c4cff` | `border-right: 2px solid var(--ol-color-primary-hover)` | Primary hover |
| 188 | `#4c4cff` | `var(--ol-color-primary-hover)` | Primary hover |
| 276 | `background: #fff` | `background: var(--ol-surface)` | Surface |
| 277 | `border: 1px solid #cbd5e1` | `border: 1px solid var(--ol-border-hover)` | Border hover |
| 287 | `border-color: #6366f1` | `border-color: var(--ol-color-primary)` | Primary |
| 312 | `background: #fafbfc` | `background: var(--ol-surface-hover)` | Surface hover |
| 313 | `border: 1px solid #e2e8f0` | `border: 1px solid var(--ol-border)` | Border |

- [ ] **Step 2: Replace placeholder text colors in input slots**

| Line(s) | Old value | New value | Token |
|---------|-----------|-----------|-------|
| 292 | `color: #fca5a5` | `color: var(--ol-color-danger-muted)` | Danger muted |
| 293-296 | `color: #cbd5e1` | `color: var(--ol-text-muted)` | Text muted |

- [ ] **Step 3: Keep semantic code colors intact — verify these are NOT changed**

These colors must remain hardcoded (they encode code structure semantics, not brand theming):

```
.code-card--let          → background: #a78bfa  (purple — declaration)
.code-card--set          → background: #60a5fa  (blue — assignment)
.code-card--return       → background: #f87171  (red — return)
.code-card--await/call   → background: #22d3ee  (cyan — async/call)
.code-card--if           → background: #fbbf24  (yellow — conditional)
.code-card--while        → background: #f97316  (orange — loop)
.code-card--for          → background: #34d399  (green — iteration)
.code-card--mut          → background: #fde68a  (amber — mutation)
.code-card--continue     → background: #e2e8f0  (gray — structural)
.code-card--operator     → background: #e2e8f0  (gray — structural)
.code-card--type         → background: #cffafe  (cyan — type)
.code-card--var_ref      → background: #d1fae5  (green — reference)
.code-card--literal      → background: #d1fae5  (green — value)
```

Confirm via grep that none of these semantic colors were accidentally changed:

```bash
grep -n 'code-card--' poiesis/src/components/canvas/code/CodeCardEditor.module.css | grep -v 'var(--ol'
```

Expected: Still shows the semantic hex colors above (no false positives).

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/components/canvas/code/CodeCardEditor.module.css
git commit -m "refactor: replace hardcoded chrome colors in CodeCardEditor with --ol-* tokens"
```

---

### Task 4: Fix ComponentDefPanel.css hardcoded shadow

**Files:**
- Modify: `poiesis/src/components/design/ComponentDefPanel.css`

- [ ] **Step 1: Replace hardcoded box-shadow with --ol-* token**

Edit line 9:

```css
/* OLD */
box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);

/* NEW */
box-shadow: var(--ol-shadow-popover);
```

Edit line 272:

```css
/* OLD */
box-shadow: 0 0 0 1px rgba(124, 58, 237, 0.2);

/* NEW */
box-shadow: 0 0 0 1px var(--ol-focus-ring-color);
```

- [ ] **Step 2: Commit**

```bash
git add poiesis/src/components/design/ComponentDefPanel.css
git commit -m "refactor: replace hardcoded shadows in ComponentDefPanel with --ol-* tokens"
```

---

### Task 5: Delete legacy-bridge.css

**Files:**
- Delete: `poiesis/src/styles/legacy-bridge.css`
- Modify: `poiesis/src/App.tsx` (remove import)

- [ ] **Step 1: Remove the import from App.tsx**

Edit `poiesis/src/App.tsx`, delete line 18:

```typescript
// DELETE this line:
import "./styles/legacy-bridge.css";
```

- [ ] **Step 2: Delete the file**

```bash
rm poiesis/src/styles/legacy-bridge.css
```

- [ ] **Step 3: Commit**

```bash
git add poiesis/src/App.tsx
git rm poiesis/src/styles/legacy-bridge.css
git commit -m "refactor: remove legacy-bridge.css — old --color-*/--dp-* token mappings no longer needed"
```

---

### Task 6: Verify — TypeScript check

- [ ] **Step 1: Run TypeScript compilation**

```bash
cd poiesis && pnpm tsc --noEmit
```

Expected: Zero errors. The design migration should have no impact on TypeScript since it only changes CSS values and removes one CSS import.

If errors appear, they must be pre-existing — do NOT fix them in this migration. Report them and skip.

- [ ] **Step 2: Verify no remaining hex colors outside tokens.css**

```bash
# Find hardcoded hex colors, excluding tokens.css and semantic code-card colors
grep -rn '#[0-9a-fA-F]\{3,6\}' poiesis/src --include='*.css' | grep -v 'tokens.css' | grep -v 'code-card--'
```

Expected: Minimal output — only semantic code-card colors in CodeCardEditor.module.css (verified in Task 3) and possibly pre-existing values in `.module.css` files that already use `var(--ol-*)` for colors but may have hex in `rgba()` shadows or other contexts. Review output and only replace if they are chrome-related values with a matching `--ol-*` token.

---

### Task 7: Verify — Visual check

**Prerequisite:** Poiesis must be runnable (Tauri dev environment set up).

- [ ] **Step 1: Start Poiesis**

```bash
cd poiesis && pnpm tauri dev
```

- [ ] **Step 2: Check light mode**

Open Poiesis, ensure theme is set to "Light". Verify:
- Background is warm parchment tone (not pure white)
- Primary buttons/elements use terracotta orange-brown (not indigo blue)
- Panel borders are subtle warm rings (not cold gray lines)
- No visual breakage — all UI elements are readable and properly spaced

- [ ] **Step 3: Switch to dark mode**

Toggle theme to "Dark". Verify:
- Background is near-black with warm undertone (not cold zinc gray)
- Text is soft white, readable
- Primary accent is lifted terracotta (#b9775c)
- Ring-shadows visible as subtle light borders around panels
- No visual breakage

- [ ] **Step 4: Take screenshots for record**

Save screenshots of light and dark mode to `docs/design/screenshots/`:

```bash
mkdir -p docs/design/screenshots
# Use Victauri MCP screenshot, or OS screenshot tool
# Save as: docs/design/screenshots/light-mode.png
# Save as: docs/design/screenshots/dark-mode.png
```


