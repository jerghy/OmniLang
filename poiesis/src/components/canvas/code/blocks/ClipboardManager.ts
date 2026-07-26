// poiesis/src/components/canvas/code/blocks/ClipboardManager.ts
// ClipboardManager — JSON serialize/deserialize CodeOp to navigator.clipboard.

import type { CodeOp } from "../../../../core/types/code-op";

export class ClipboardManager {
  async copy(op: CodeOp): Promise<void> {
    const json = JSON.stringify(op);
    await navigator.clipboard.writeText(json);
  }

  async paste(): Promise<CodeOp | null> {
    try {
      const text = await navigator.clipboard.readText();
      return JSON.parse(text) as CodeOp;
    } catch {
      return null;
    }
  }
}
