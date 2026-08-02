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
    },
  },
);
