import type { Preview } from '@storybook/react-vite'
import '../src/styles/tokens.css';

/**
 * Minimal essential design tokens injected directly into the preview head.
 * Full tokens.css is imported above for Vite bundling;
 * this inline block guarantees variables are defined in the iframe
 * even if the CSS import tree-shaking strips the :root block.
 */
const DESIGN_TOKEN_INLINE = `
<style data-poiesis-tokens>
:root {
  --ol-surface: #f5f4ed;
  --ol-surface-hover: #ecebe4;
  --ol-surface-raised: #ecebe4;
  --ol-border: #e8e5dc;
  --ol-border-hover: #d3d2cc;
  --ol-border-subtle: #f0eee6;
  --ol-text: #141413;
  --ol-text-secondary: #5e5d59;
  --ol-text-muted: #908f8b;
  --ol-color-primary: #c96442;
  --ol-color-primary-active: #a3472e;
  --ol-color-primary-subtle: #fff6f0;
  --ol-color-primary-muted: #f0d2c2;
  --ol-color-success: #52c41a;
  --ol-color-success-active: #389e0d;
  --ol-color-success-subtle: #f6ffed;
  --ol-color-success-muted: #b6eb8f;
  --ol-color-warning: #faad14;
  --ol-color-warning-active: #d48806;
  --ol-color-warning-muted: #ffe58f;
  --ol-color-info: #c96442;
  --ol-color-info-active: #a3472e;
  --ol-color-info-muted: #f0d2c2;
  --ol-color-danger: #ff4d4f;
  --ol-color-danger-active: #d9363d;
  --ol-color-danger-muted: #ffccc7;
  --ol-gray-200: #e7e7e0;
  --ol-gray-300: #d3d2cc;
  --ol-gray-700: #2f2f2d;
  --ol-font-family-ui: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  --ol-font-family-mono: 'JetBrains Mono', 'Fira Code', 'Consolas', 'Monaco', 'Courier New', monospace;
  --ol-font-size-10: 10px; --ol-font-size-11: 11px; --ol-font-size-12: 12px;
  --ol-font-size-13: 13px; --ol-font-size-14: 14px; --ol-font-size-15: 15px;
  --ol-font-size-16: 16px; --ol-font-size-18: 18px; --ol-font-size-20: 20px;
  --ol-font-size-24: 24px; --ol-font-size-28: 28px;
  --ol-font-weight-400: 400; --ol-font-weight-500: 500; --ol-font-weight-600: 600; --ol-font-weight-700: 700;
  --ol-line-height-normal: 1.5; --ol-line-height-relaxed: 1.75; --ol-line-height-tight: 1.25;
  --ol-space-0: 0px; --ol-space-0p5: 2px; --ol-space-1: 4px;
  --ol-space-1p5: 6px; --ol-space-2: 8px; --ol-space-3: 12px;
  --ol-space-4: 16px; --ol-space-5: 20px; --ol-space-6: 24px;
  --ol-radius-sm: 4px; --ol-radius-md: 8px; --ol-radius-lg: 8px;
  --ol-shadow-modal: 0 0 0 2px rgba(0,0,0,0.12), 0 8px 32px rgba(0,0,0,0.08);
  --ol-z-modal: 600;
  --ol-duration-fast: 100ms; --ol-duration-normal: 200ms; --ol-duration-slow: 300ms;
  --ol-ease-default: cubic-bezier(0.645, 0.045, 0.355, 1);
  --ol-ease-in-out: cubic-bezier(0.645, 0.045, 0.355, 1);
  --ol-focus-ring-width: 2px; --ol-focus-ring-offset: 2px;
  --ol-focus-ring-color: rgba(201,100,66,0.35);
  --ol-scrollbar-size: 6px;
  --ol-scrollbar-track: transparent;
  --ol-scrollbar-thumb: rgba(0,0,0,0.12);
  --ol-scrollbar-thumb-hover: rgba(0,0,0,0.22);
}
</style>`;

const preview: Preview = {
  parameters: {
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    docs: { toc: true },
  },
};

// Inject token <style> directly into the preview's <head>
// to guarantee --ol-* variables are defined for all stories.
const originalDocument = typeof document !== 'undefined' ? document : null;
if (originalDocument) {
  const existing = originalDocument.querySelector('style[data-poiesis-tokens]');
  if (!existing) {
    originalDocument.head.insertAdjacentHTML('beforeend', DESIGN_TOKEN_INLINE);
  }
}

export default preview;
