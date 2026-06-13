/** OmniDriver — Playwright-like API for AI-driven editor automation.
 *
 *  Usage:
 *  ```typescript
 *  import { OmniDriver } from "./driver";
 *  const omni = new OmniDriver();
 *  ```
 *
 *  Or via the global window.__OMNI__ (set up automatically in App.tsx):
 *  ```javascript
 *  const omni = window.__OMNI__;
 *  ```
 */

export { OmniDriver, ElementHandle, WireHandle } from "./OmniDriver";
export type {
  ElementData,
  WireData,
  EventMeta,
  PortDescriptor,
  ElementQuery,
  CodeBlockOptions,
  CompileResult,
  EditorSnapshot,
  ComponentInfo,
  EventHandlerOptions,
} from "./types";
