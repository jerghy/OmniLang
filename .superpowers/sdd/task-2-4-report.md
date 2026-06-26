# Task 2.4: Rename `@code-tab` → `@omni-file` with backward compatibility

## Summary

Renamed the `@code-tab` annotation directive to `@omni-file` across the entire codebase, with full backward compatibility for the old format.

## Changes Made

### TypeScript Source (services/codegen/)

| File | Changes |
|------|---------|
| `source-emitter.ts` | Emits `@omni-file` instead of `@code-tab` for the file-level directive and event tab boundary markers |
| `annotation-data.ts` | `buildAnnotationData()` and `buildAnnotationStrategy()` produce `@omni-file` format |
| `omni-parser.ts` | Accepts both `@omni-file` and `@code-tab` in `parseRootLine()`; updated regex with alternation group `(?:omni-file\|code-tab)` |
| `custom-component-emitter.ts` | Updated all comments referencing `@code-tab` → `@omni-file` |

### TypeScript Tests

| File | Changes |
|------|---------|
| `source-emitter-annotation.test.ts` | Updated assertions and comments to expect `@omni-file` |
| `omni-parser.test.ts` | Updated fixture strings and test names |
| `integration-generate.test.ts` | Updated fixture strings and mock strategy |
| `annotation-data.test.ts` | Updated assertions for `codeTabDirective` value |

### Python Source (editor/server/annotation/)

| File | Changes |
|------|---------|
| `parser.py` | Renamed `_CODE_TAB_RE` → `_OMNI_FILE_RE`; regex accepts both `@omni-file` and `@code-tab`; updated error messages to reference `@omni-file` |
| `generator.py` | Emits `@omni-file` in `_insert_code_tab()`; removal regex strips both new and old formats; updated comments |
| `migrate.py` | Migration produces `@omni-file` directive |
| `validator.py` | `_validate_position_rules()` accepts both `@omni-file` and `@code-tab` as valid first directives |

### Python Tests (tests/annotation/)

| File | Changes |
|------|---------|
| `test_parser.py` | Updated all fixture strings to use `@omni-file` |
| `test_generator.py` | Updated assertions to expect `@omni-file` |
| `test_validator.py` | Updated all fixture strings to use `@omni-file` |
| `test_roundtrip.py` | Updated fixture strings |
| `test_migrate.py` | Updated assertion for migration output |

### Other

| File | Changes |
|------|---------|
| `editor/server/main.py` | Updated docstring comment |
| `editor/.claude/evals/editor-capability-evals.md` | Updated eval checklist |

## Backward Compatibility

The following locations accept both `@omni-file` and `@code-tab`:

1. **omni-parser.ts** — `parseRootLine()` checks for both prefixes
2. **parser.py** — `_OMNI_FILE_RE` regex uses `(?:omni-file|code-tab)` alternation 
3. **generator.py** — `_insert_code_tab()` strips lines matching either format
4. **validator.py** — `_validate_position_rules()` checks for either format

## Verification

- `tsc --noEmit`: Zero errors
- `vitest run`: 192 tests (2 pre-existing failures match baseline)
