# Phase 0 Task 1: Poiesis v2 Type System

**Status:** DONE

**Created files (5):**
- `poiesis/src/core/types/pid.ts` — PID annotation system types
- `poiesis/src/core/types/architecture.ts` — Architecture page bubble graph types
- `poiesis/src/core/types/design-page.ts` — Design page tabbed editor types
- `poiesis/src/core/types/project-json.ts` — Project JSON persistence types
- `poiesis/src/core/types/node-flow.ts` — Code canvas node flow types

**Modified files (1):**
- `poiesis/src/core/types/index.ts` — Barrel export updated to re-export v2 types + legacy `element`, `wire`, `style`

**Verification:** All new files compile cleanly — zero syntax/type errors. The ~70 pre-existing errors are all from old code still importing removed legacy exports (`AnnotatedLoadResponse`, `CodeTab`, etc.), which is expected and will be resolved in subsequent phases.

**Concerns:** None.
