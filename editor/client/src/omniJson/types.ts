/**
 * OmniLang editor native format (.omni.json) — v1 TypeScript type definitions.
 *
 * Mirrors editor/server/omni_json/schema.py dataclass definitions.
 * Keys use camelCase (matching JSON serialization from Python serializer).
 */

// ── Core Document ──────────────────────────────────────────

export interface OmniMetadata {
  projectName: string;
  omniVersion: string;
  createdAt: string;      // ISO 8601
  updatedAt: string;      // ISO 8601
  isHandWritten: boolean;
  conversionWarnings: string[];
}

export interface OmniJsonDocument {
  schemaVersion: string;  // "1.0.0"
  meta: OmniMetadata;
  components: ComponentSnapshot[];
  stores: StoreSnapshot[];
  themes: ThemeSnapshot[];
  imports: string[];
  elements: ElementEntry[];
  wires: WireEntry[];
  ownershipRegions: OwnershipRegionEntry[];
  searchIndex: SearchIndexSnapshot;
  canvasLayout: CanvasLayoutSnapshot;
}

// ── Component / Store / Theme Snapshots ────────────────────

export interface ComponentSnapshot {
  name: string;
  genericParams: string[];
  props: Record<string, string>;              // name → type
  signals: Record<string, { type: string; init?: unknown }>;
  computeds: Record<string, { type: string; body: string; deps: string[] }>;
  events: string[];
  refs: string[];
  storeRefs: string[];                        // @store names
  lifecycles: Record<string, string[]>;        // { mount: [body], unmount: [body] }
  fns: Array<{ name: string; params: Array<{ name: string; type: string }>; returnType: string; body: string }>;
  view: ViewNodeSnapshot | null;
  style: Record<string, Record<string, string>>; // { className: { prop: value } }
}

export interface StoreSnapshot {
  name: string;
  signals: Record<string, string>;            // name → type
  signalDefaults: Record<string, unknown>;     // name → default value
  getters: Record<string, { returnType: string; body: string }>;
  actions: Array<{ name: string; params: Array<{ name: string; type: string }>; body: string }>;
}

export interface ThemeSnapshot {
  name: string;
  categories: Record<string, Record<string, string>>;
}

// ── View Tree (recursive, _kind-discriminated) ─────────────

export type ViewNodeKind = 'element' | 'text' | 'interpolation' | 'if' | 'for' | 'slot' | 'native' | 'fragment';

export interface ViewNodeSnapshot {
  _kind: ViewNodeKind;
  testId: string;
  // Element-specific
  tag: string;
  props: Record<string, unknown>;
  bindings: Record<string, string>;          // prop → signalName
  events: Record<string, { kind: string; ref: string }>;
  styleClasses: string[];
  inlineStyle: Record<string, string>;
  refName: string;
  // Children (recursive)
  children: ViewNodeSnapshot[];
  // Text-specific
  text: string;
  // Interpolation
  expression: string;
  // If-specific
  condition: string;
  branches: Array<{ condition: string; body: ViewNodeSnapshot[] }>;
  fallback: ViewNodeSnapshot[] | null;
  // For-specific
  iterable: string;
  itemBinding: string;
  keyExpr: string;
  emptyBody: ViewNodeSnapshot[] | null;
  // Slot-specific
  slotName: string;
  // Native-specific
  target: string;
  nativeCode: string;
  nativePorts: Record<string, string>;
}

// ── Editor Runtime State ───────────────────────────────────

export interface ElementEntry {
  id: string;
  nodeType: 'view' | 'codeblock' | 'group';
  tag: string;
  props: Record<string, unknown>;
  children: string[];
  parentId: string | null;
  ports: Array<{ name: string; direction: 'in' | 'out'; type: string }>;
  renderMode: 'dom' | 'canvas';
  isFolded: boolean;
  typeAnnotation: string;
  storeName: string;
}

export interface WireEntry {
  id: string;
  fromId: string;
  fromPort: string;
  toId: string;
  toPort: string;
  isCollapsed: boolean;
  collapseGroupId: string;
}

export interface CanvasLayoutSnapshot {
  positions: Record<string, { x: number; y: number; w: number; h: number }>;
}

// ── Ownership Regions ──────────────────────────────────────

export interface OwnershipRegionEntry {
  regionType: 'managed' | 'handwritten';
  nodeId: string;
  nodeType: string;
  fieldPath: string;
  sourceLineStart: number;
  sourceLineEnd: number;
}

// ── Search Index ───────────────────────────────────────────

export type SearchIndexSource = 'file' | 'component' | 'snippet' | 'connector' | 'token' | 'wire' | 'command';

export interface SearchIndexEntry {
  id: string;
  source: SearchIndexSource;
  title: string;
  subtitle: string;
  keywords: string[];
  icon: string;
  actionType: string;
  actionPayload: string;
}

export interface SearchIndexSnapshot {
  entries: SearchIndexEntry[];
  version: number;
  lastBuilt: string;
}
