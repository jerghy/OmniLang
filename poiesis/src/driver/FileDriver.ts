/** FileDriver — source code accessors and file read/write/save operations.
 *  Extracted from OmniDriver god class. */

import { getDesignStore } from "./store";
import { usePoiesisStore } from "../store";
import { api } from "../commands";

// ═══════════════════════════════════════════════════════
// FileDriver
// ═══════════════════════════════════════════════════════

export class FileDriver {
  // ── Source Code ──────────────────────────────────

  /** Get the active file content (the .omni source). */
  getSourceCode(): string | null {
    return getDesignStore().activeFileContent;
  }

  /** Set the active file content. */
  setSourceCode(content: string): void {
    usePoiesisStore.setState({ activeFileContent: content });
  }

  /** Get the active file path. */
  getActiveFilePath(): string | null {
    return getDesignStore().activeFilePath;
  }

  /** Write the current source to disk. */
  async save(): Promise<boolean> {
    const path = getDesignStore().activeFilePath;
    const content = getDesignStore().activeFileContent;
    if (!path || !content) return false;
    const result = await api.project.writeFile(path, content);
    return result.ok;
  }

  // ── File Operations ──────────────────────────────

  /** Read a file from the project. */
  async readFile(filePath: string): Promise<string | null> {
    const result = await api.project.readFile(filePath);
    return result.content || null;
  }

  /** Write content to a file in the project. */
  async writeFile(filePath: string, content: string): Promise<boolean> {
    const result = await api.project.writeFile(filePath, content);
    return result.ok;
  }

  /** Open a file in the editor. */
  async openFile(filePath: string): Promise<void> {
    const result = await api.project.readFile(filePath);
    if (result.content) {
      usePoiesisStore.getState().openFile(filePath, result.content);
    }
  }
}
