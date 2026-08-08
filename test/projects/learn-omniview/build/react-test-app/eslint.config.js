// ESLint flat config for the OmniLang React output harness.
// Gate scope: generated TSX/TS under src/ (see scripts/verify-react.sh).
//
// Rules:
// - @eslint/js recommended (base)
// - typescript-eslint recommended (TS-aware version of no-unused-vars etc.)
// - react-hooks: classic rules-of-hooks + exhaustive-deps only. The v7
//   "recommended" preset bundles compiler-era rules (static-components,
//   use-memo, purity, …) which are optional and out of scope for the gate.

import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import reactHooks from 'eslint-plugin-react-hooks';

export default tseslint.config(
  {
    ignores: ['dist', 'node_modules', 'test-results', 'playwright-report'],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    plugins: {
      'react-hooks': reactHooks,
    },
    rules: {
      'react-hooks/rules-of-hooks': 'error',
      'react-hooks/exhaustive-deps': 'error',
      // Generated output emits `_`-prefixed identifiers for deliberately
      // unbound positions: `@for` map callbacks emit an unused `_i` index
      // param and the match codegen prefixes unused variant bindings with
      // `_` (`_data`) — the codegen's own convention for "unused by design".
      // argsIgnorePattern honors it for function args so the gate no longer
      // flags them (task 2.11). varsIgnorePattern extends the same `_`
      // convention to LOCALS: the demo's @effect subscription marker
      // `let _ = count;` (OmniView auto-tracking reads the body; the write-
      // only local is deliberately unbound) compiles to `const _ = count;`
      // — a `_`-prefixed local means "intentionally unused" here too
      // (2026-08-08, Task 8).
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },
);
