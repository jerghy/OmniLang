## CodeCard Editor v2: Fine-Grained Card Editor
> branch: main | start: 2026-07-24
> plan: docs/superpowers/plans/2026-07-23-codecard-editor-v2.md
> BASE: 9a8b536
Task 1: complete (commit 9a8b536..9f81a17, review clean — Spec ✅ Quality ✅, 4 Minor notes)
Task 2: complete (commit 9f81a17..000e9cd, 1 fix round, 14/14 tests, 4 MEDIUM+3 LOW fixed)
Task 3: complete (commit 000e9cd..bd22f80, 1 fix round CRITICAL+5 HIGH, 6/6 tests, 0 TS errors)
Task 4: complete (commit 152cc21..a9957c6, InputSlotNode, 6/6 tests)
Task 5: complete (commit a9957c6..7924e22, CardNode, 6/6 tests) 
Task 6: complete (commit a9957c6, ContainerNode+BlockNode+BodyLineNode, 22/22 tests)
Task 7: complete (commit 7924e22..e4aaabf, nodes/index.ts + 172 lines CSS, 0 TS errors)
Task 8: complete (commit e4aaabf..fa437d9, autocomplete rules rewrite, 10/10 tests)
Task 9: complete (commit fa437d9..1f208ca, KeyboardPlugin v2, 6/6 tests)
Task 10: complete (commit 1f208ca..a32d3d8, AutocompletePlugin v2, 0 new TS errors)
Task 11: complete (commit a32d3d8..dd65e6c, CodeOpSyncPlugin → EditorNode sync)
Task 12: complete (commit dd65e6c..9ba8739, CodeCardEditor shell + FlowCanvas import)
Task 13: complete (commit 9ba8739..6e66036, card-editor/ deleted, 21 files removed)
Task 14: complete (commit 9ba8739..b8141b7, integration tests 6/6)
Task 15: complete (commit 6e66036..e01804b, 7 TS fixes, 283 tests, 89 TS errors)

FINAL: 14 commits, 283 tests pass, TS 96→89 (7 fixed), old card-editor/ removed

FIX ROUND (post-review):
H1: Branch roundtrip preserved through Lexical (BodyLineNode.__branchIndex + BlockNode.__branches)
H2: Block head context detection fixed (getParentContextKind checks BlockElementNode)
H3: Autocomplete menu positioned at cursor (window.getSelection bounding rect)
TS fix: Import path + Branch type in CodeOpSyncPlugin
Final: 17 commits, 283 tests pass, TS 89 (baseline, no regressions)
Task 1: complete (commits 153be3b..b9ae334, review clean)
Task 2: complete (commits b9ae334..c858ca2, review clean after 2 fix rounds)
Task 3: complete (commits c858ca2..ba3f824, review clean, 3 Minor)
Task 4: complete (commits ba3f824..3f1087d, review approved, 1 Important+2 Minor)
Task 5: complete (commits 3f1087d..cd653c3, empty commit, roundtrip passes)
Task 6: complete (commits cd653c3..2ee8a87, review approved, 1 Important+5 Minor)
Task 7: complete (commits 2ee8a87..283c22e, 6/6 tests)
Task 8: complete (commits 283c22e..b3d4da4, slot-operations.ts created)
Task 9: complete (commits b3d4da4..904ee4d, \ delegate, 279 tests)
Task 10: complete (commits 904ee4d..c1c4519, CSS cursor states added)
