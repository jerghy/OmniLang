# OmniLang Code Review Fix Progress v2
> branch: worktree-code-review-fixes-v2 | start: 2026-07-06
> based on: main

## Setup
- 6988f6e1: chore: fix duplicate [dev-dependencies] and move [profile.release] to workspace root

## Tasks

### Phase 1: Quick Wins
- [ ] Task 1: Codegen P1 shared utility extraction
- [ ] Task 2: CSP/Security hardening
- [ ] Task 3: #[must_use] audit + annotation
- [ ] Task 4: TS global type declarations

### Phase 2A: Core Refactors
- [ ] Task 5: format_json_value unification
- [x] Task 6: Parser infrastructure + Error AST nodes
- [ ] Task 7: PoiesisState missing methods

### Phase 2B: Deeper Changes
- [ ] Task 8: binary_op_to_* cleanup + camel_to_separated
- [ ] Task 9: Statement-level error recovery
- [ ] Task 10: OmniDriver/ElementHandle window.__* migration

## Verification
- cargo check: PASS | cargo test: PASS | cargo clippy: PASS

- [x] Task 1: ba401f2c refactor: extract shared codegen utilities (review: PASS/Approved)

- [x] Task 2: 93600150 fix: security hardening (review: PASS/Approved)

- [x] Task 3: 5d0c6a24 refactor: add #[must_use] (31 annotations, 9 modules, review pending)
- [x] Task 4: 9995732e fix: restore type checking (review: PASS/Approved))

- [x] Task 5: 3dce5243 refactor: unify format_json_value (review: PASS/Approved with fix a28ffe78)

- [x] Task 6: Parser infrastructure + Error AST nodes
