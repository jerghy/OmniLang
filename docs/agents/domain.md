# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root if it exists, or
- **`CONTEXT-MAP.md`** at the repo root if it exists — it points at one `CONTEXT.md` per context. Read each one relevant to the topic.
- **`.architecture/decisions/adrs/`** — this repo's 74+ ADRs (named `ADR-0NN-<slug>.md`, e.g. `ADR-075-semantic-event-intent-phase1.md`). Read ADRs that touch the area you're about to work in. Deferred items are tracked in `.architecture/decisions/deferrals.md`.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The `/domain-modeling` skill (reached via `/grill-with-docs` and `/improve-codebase-architecture`) creates them lazily when terms or decisions actually get resolved.

## File structure

Single-context repo (this repo):

```
/
├── CONTEXT.md                          ← not yet created; lazily created by /domain-modeling
├── .architecture/decisions/adrs/       ← ADR-001 … ADR-075 (system-wide decisions)
├── .architecture/decisions/deferrals.md
├── omnilang/                           ← the OmniLang compiler (Rust workspace)
├── poiesis/                            ← the Tauri visual editor (TypeScript/React)
└── packages/omni-animation/            ← shared animation package
```

Multi-context repo (presence of `CONTEXT-MAP.md` at the root):

```
/
├── CONTEXT-MAP.md
├── docs/adr/                          ← system-wide decisions
└── src/
    ├── ordering/
    │   ├── CONTEXT.md
    │   └── docs/adr/                  ← context-specific decisions
    └── billing/
        ├── CONTEXT.md
        └── docs/adr/
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/domain-modeling`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0075 (semantic event intent phase 1) — but worth reopening because…_
