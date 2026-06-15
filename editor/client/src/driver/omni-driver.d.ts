/** Global type declaration for window.__OMNI__.
 *  Provides type-safe access to OmniDriver in Playwright tests and browser scripts. */

import type {
  PortDescriptor,
  ElementQuery,
  CodeBlockOptions,
  CompileResult,
  EditorSnapshot,
  ComponentInfo,
  EventHandlerOptions,
  ValidationReport,
} from "./types";
import type { ElementData, WireData } from "../store/editor";

interface ElementHandle {
  readonly id: string;
  readonly data: ElementData | undefined;
  exists(): boolean;
  text(): string;
  setProp(prop: string, value: unknown): this;
  getProp(prop: string): unknown;
  getPorts(): PortDescriptor[];
  getEvents(): Record<string, { id: string; description: string; payload: string | null }>;
  getSource(): string | undefined;
  setSource(source: string): this;
  select(): this;
  remove(): void;
  children(): ElementHandle[];
  parent(): ElementHandle | null;
  connectTo(target: ElementHandle, fromPort: string, toPort: string): WireHandle;
  createEventHandler(
    eventName: string,
    options?: EventHandlerOptions,
  ): { handler: ElementHandle; wire: WireHandle };
  waitForLive(timeout?: number): Promise<this>;
}

interface WireHandle {
  readonly id: string;
  readonly data: WireData | undefined;
  disconnect(): void;
  source(): ElementHandle | null;
  target(): ElementHandle | null;
}

interface OmniDriver {
  // ── State ──
  getState(): EditorSnapshot;
  getElements(): ElementData[];
  readonly elements: ElementData[];
  getWires(): WireData[];
  readonly wires: WireData[];
  getMode(): "design" | "code";
  getSelectedElement(): ElementHandle | null;
  getPreviewUrl(): string | null;
  readonly views: ElementHandle[];
  readonly codeBlocks: ElementHandle[];

  // ── Locator ──
  element(id: string): ElementHandle;
  findByTag(tag: string): ElementHandle[];
  find(query: ElementQuery): ElementHandle[];
  findByProp(prop: string, value: unknown): ElementHandle[];

  // ── Creation ──
  component(tag: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  codeBlock(blockType: string, name: string, options?: CodeBlockOptions): ElementHandle;
  button(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  text(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  heading(label: string, level?: string | number, parentId?: string | null): ElementHandle;
  container(props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  input(placeholder?: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  card(props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  flex(direction?: string, gap?: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle;
  image(src: string, alt?: string, parentId?: string | null): ElementHandle;
  list(parentId?: string | null): ElementHandle;
  alert(type?: string, label?: string, parentId?: string | null): ElementHandle;

  // ── Wires ──
  connect(fromId: string, fromPort: string, toId: string, toPort: string, wireId?: string): WireHandle;
  disconnect(wireId: string): void;

  // ── Properties ──
  setProp(id: string, prop: string, value: unknown): void;
  getProp(id: string, prop: string): unknown;
  setSource(id: string, source: string): void;
  selectElement(id: string | null): void;

  // ── Mode & Compile ──
  switchMode(mode: "design" | "code"): void;
  compile(): Promise<CompileResult>;

  // ── Wait ──
  waitForElement(tag: string, timeout?: number): Promise<ElementHandle>;
  waitForWire(fromPort?: string, timeout?: number): Promise<WireHandle>;
  waitFor(predicate: () => boolean, timeout?: number, message?: string): Promise<void>;

  // ── Undo / Redo ──
  undo(): void;
  redo(): void;

  // ── Batch ──
  batch(fn: (driver: OmniDriver) => void): void;

  // ── File ──
  getSourceCode(): string | null;
  setSourceCode(content: string): void;
  getActiveFilePath(): string | null;
  save(): Promise<boolean>;
  readFile(filePath: string): Promise<string | null>;
  writeFile(filePath: string, content: string): Promise<boolean>;
  openFile(filePath: string): Promise<void>;

  // ── Discovery ──
  help(): Record<string, unknown>;
  listComponents(): ComponentInfo[];
  getComponentInfo(tag: string): ComponentInfo | null;

  // ── Management ──
  clearAll(): void;
  removeElement(id: string): void;
  validate(): ValidationReport;
  exportState(): string;
  importState(json: string): void;
  saveSnapshot(): string;
  restoreSnapshot(snapshotToken: string): boolean;
  diff(previous: EditorSnapshot): StateDiff;

  // ── Snippets ──
  snippet(template: string, options?: Record<string, unknown>): ElementHandle;
  registerSnippet(name: string, builder: (options?: Record<string, unknown>) => ElementHandle): void;
  unregisterSnippet(name: string): boolean;
  listSnippets(): { builtin: string[]; custom: string[] };

  // ── Visual Indicators ──
  highlight(elementId: string): void;
  highlightAll(tag: string): void;
  clearHighlights(): void;
}

interface StateDiff {
  added: ElementData[];
  removed: string[];
  modified: Array<{ id: string; changes: Record<string, { from: unknown; to: unknown }> }>;
  addedWires: WireData[];
  removedWires: string[];
}

declare global {
  interface Window {
    __OMNI__: OmniDriver;
  }
}

export type { OmniDriver, ElementHandle, WireHandle, StateDiff };
