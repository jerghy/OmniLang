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
  FnParam,
  ParamBinding,
  EventBindingOptions,
  EventBindingRecord,
  TabInfo,
  FunctionValidationReport,
  FunctionSignature,
  TabExportData,
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

  // Event binding v2 (replaces createEventHandler)
  bindEvent(eventName: string, fnName: string, options?: EventBindingOptions): this;
  unbindEvent(eventName: string): this;
  getAvailableEvents(): string[];
  getEventBindings(): Array<{ eventName: string; fnName: string; paramBindings: Record<string, ParamBinding> }>;

  // Custom function operations
  addParam(name: string, type: string, defaultValue?: unknown): this;
  setReturnType(type: string): this;
  setDescription(desc: string): this;

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

  // ── Event Binding v2 ──
  listAllEventBindings(): EventBindingRecord[];
  findMatchingFunctions(tabId: string, eventName: string): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string }>;

  // ── Code Tab Management ──
  createTab(type: "event" | "logic", label?: string, icon?: string): { id: string };
  switchTab(tabId: string): void;
  removeTab(tabId: string): void;
  listTabs(): TabInfo[];
  getActiveTab(): TabInfo | null;
  getTab(tabId: string): TabInfo | null;
  cloneTab(tabId: string, newLabel?: string): { id: string } | null;
  exportTab(tabId: string): string | null;
  importTab(json: string): { id: string } | null;
  addVariable(tabId: string, name: string, type: string, defaultValue?: unknown, description?: string): void;
  removeVariable(tabId: string, name: string): void;
  getVariables(tabId: string): Array<{ name: string; type: string; defaultValue?: unknown; description?: string }>;
  getTabElements(tabId: string): ElementData[];
  getTabWires(tabId: string): WireData[];

  // ── Custom Function Operations ──
  customFunction(fnName: string, fnParams: FnParam[], fnReturnType: string): ElementHandle;
  returnNode(parentFnId: string): ElementHandle;
  validateFunction(fnId: string): FunctionValidationReport;
  listCustomFunctions(): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string }>;
  findFunctionByName(fnName: string): { fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string; nodeId: string } | null;
  checkFunctionNameConflict(fnName: string): Array<{ fnName: string; tabId: string }>;
  getFunctionSignature(fnId: string): FunctionSignature | null;

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
