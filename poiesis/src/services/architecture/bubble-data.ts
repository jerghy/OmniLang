/**
 * Service that scans .omni files and builds the bubble graph data.
 *
 * Algorithm:
 * 1. For each .omni file, parse with parseOmniSource() + raw source scanning
 * 2. Extract component/store/fn info from each file
 * 3. Build BubbleNode tree:
 *    - Top-level (depth 0): 3 fixed bubbles — 界面, Store, 代码
 *    - Depth 1: entry component from main.omni, store files, lib files
 *    - Depth 2+: nested components from view template composition
 * 4. Detect isLeaf: component has NO PascalCase child component tags → isLeaf=true
 * 5. Build edges: composition (parent→child), store-read (component→store),
 *    store-write (component→store action)
 */

import type { BubbleNode, BubbleEdge } from "../../core/types/architecture";

// ── Types ──

export interface OmniFileInfo {
  path: string;
  source: string;
}

interface ParsedComponent {
  name: string;
  filePath: string;
  /** PascalCase component tags used in view (custom component refs) */
  viewChildren: string[];
  /** @store references */
  storeRefs: string[];
  /** View contains basic HTML elements (div/button/span/...) */
  hasBasicElements: boolean;
  /** View contains custom component tags (PascalCase) */
  hasPascalCaseChildren: boolean;
}

interface ParsedStore {
  name: string;
  filePath: string;
  signals: string[];
  getters: string[];
  actions: string[];
}

interface ParsedLib {
  filePath: string;
  functions: string[];
}

// ── Constants ──

const BASIC_HTML_TAGS = new Set([
  "div", "span", "button", "input", "p", "h1", "h2", "h3", "h4", "h5", "h6",
  "img", "a", "ul", "li", "form", "section", "header", "main", "footer", "nav",
  "article", "ol", "table", "tr", "td", "th", "label", "textarea", "select",
  "option", "br", "hr", "strong", "em", "i", "b", "small", "sup", "sub",
  "code", "pre", "blockquote", "figure", "figcaption", "aside", "details",
  "summary", "time", "mark", "del", "ins", "sub", "sup",
]);

/** Tags that appear only in PascalCase (leaf check ignores these as basic) */
const PSEUDO_BASIC_TAGS = new Set([
  "If", "For", "Slot", "@if", "@for", "@slot",
]);

// ── Helpers: tag classification ──

function isBasicHtmlTag(tag: string): boolean {
  // Case-sensitive check: only lowercase tags are HTML elements.
  // PascalCase tags (e.g., "Header") are custom OmniLang components.
  return tag === tag.toLowerCase() && BASIC_HTML_TAGS.has(tag);
}

function isComponentTag(tag: string): boolean {
  return (
    /^[A-Z]/.test(tag) &&
    !isBasicHtmlTag(tag) &&
    !PSEUDO_BASIC_TAGS.has(tag)
  );
}

// ── Helpers: source text extraction ──

function extractComponentNames(source: string): string[] {
  const names: string[] = [];
  const re = /^\s*\bcomponent\s+(\w+)/gm;
  let m;
  while ((m = re.exec(source)) !== null) {
    names.push(m[1]);
  }
  return names;
}

function extractStoreName(source: string): string | null {
  // Only match at line start (with optional leading whitespace) to avoid comments
  const m = source.match(/^\s*\bstore\s+(\w+)/m);
  return m ? m[1] : null;
}

function extractStoreSignals(source: string, withinStoreBlock: string): string[] {
  const names: string[] = [];
  const re = /^\s*\bsignal\s+(\w+)\s*:/gm;
  let m;
  while ((m = re.exec(withinStoreBlock)) !== null) {
    names.push(m[1]);
  }
  return names;
}

function extractStoreGetters(source: string, withinStoreBlock: string): string[] {
  const names: string[] = [];
  const re = /^\s*\bgetter\s+(\w+)\s*:/gm;
  let m;
  while ((m = re.exec(withinStoreBlock)) !== null) {
    names.push(m[1]);
  }
  return names;
}

function extractStoreActions(source: string, withinStoreBlock: string): string[] {
  const names: string[] = [];
  const re = /^\s*\baction\s+(\w+)/gm;
  let m;
  while ((m = re.exec(withinStoreBlock)) !== null) {
    names.push(m[1]);
  }
  return names;
}

function extractPubFns(source: string): string[] {
  const names: string[] = [];
  const re = /^\s*\bpub\s+fn\s+(\w+)/gm;
  let m;
  while ((m = re.exec(source)) !== null) {
    names.push(m[1]);
  }
  return names;
}

/**
 * Extract the `store ... { ... }` block content from source.
 * Uses brace counting to find the matching closing `}`.
 */
function extractStoreBlock(source: string): string | null {
  const match = source.match(/^\s*\bstore\s+\w+\s*\{/m);
  if (!match) return null;
  const startIdx = match.index! + match[0].length - 1; // position of the `{`
  let depth = 1;
  let endIdx = startIdx + 1;
  while (depth > 0 && endIdx < source.length) {
    const ch = source[endIdx];
    if (ch === "{") depth++;
    else if (ch === "}") depth--;
    endIdx++;
  }
  if (depth !== 0) return null;
  return source.slice(match.index!, endIdx);
}

/**
 * Extract PascalCase component tags from raw source view block.
 * This catches references that the parser may miss due to @if/@for blocks.
 */
/**
 * Extract the view block content using brace counting.
 * Handles arbitrarily nested braces (e.g., from @if/@for blocks).
 */
function extractViewBlock(source: string): string | null {
  const viewMatch = source.match(/\bview\s*\{/);
  if (!viewMatch) return null;

  const startIdx = viewMatch.index! + viewMatch[0].length - 1; // position of the `{`
  let depth = 1;
  let endIdx = startIdx + 1;
  while (depth > 0 && endIdx < source.length) {
    const ch = source[endIdx];
    if (ch === "{") depth++;
    else if (ch === "}") depth--;
    endIdx++;
  }
  if (depth !== 0) return null;
  return source.slice(viewMatch.index!, endIdx);
}

/**
 * Extract PascalCase component tags from raw source view block.
 * This catches references that the parser may miss due to @if/@for blocks.
 */
function extractComponentTagsFromSource(source: string): {
  pascalCaseChildren: string[];
  basicHtmlTags: string[];
} {
  const pascalCaseChildren: string[] = [];
  const basicHtmlTags: string[] = [];

  // Extract the view block content using brace counting
  const viewContent = extractViewBlock(source) || source;

  // Find all HTML-like tags: <TagName ...> or </TagName>
  const tagRe = /<\s*(\/?)\s*([\w]+)/g;
  let m;
  while ((m = tagRe.exec(viewContent)) !== null) {
    const isClosing = m[1] === "/";
    if (isClosing) continue; // skip closing tags, avoid double-count
    const tag = m[2];
    if (isComponentTag(tag)) {
      if (!pascalCaseChildren.includes(tag)) {
        pascalCaseChildren.push(tag);
      }
    } else if (isBasicHtmlTag(tag)) {
      if (!basicHtmlTags.includes(tag)) {
        basicHtmlTags.push(tag);
      }
    }
  }

  return { pascalCaseChildren, basicHtmlTags };
}

/**
 * Extract @store references from source text.
 */
function extractStoreRefsFromSource(source: string): string[] {
  const refs: string[] = [];
  const re = /@store\s+(\w+)/g;
  let m;
  while ((m = re.exec(source)) !== null) {
    if (!refs.includes(m[1])) {
      refs.push(m[1]);
    }
  }
  return refs;
}

// ── Store analysis ──

function analyzeStore(name: string, filePath: string, source: string): ParsedStore {
  const block = extractStoreBlock(source) || source;
  return {
    name,
    filePath,
    signals: extractStoreSignals(source, block),
    getters: extractStoreGetters(source, block),
    actions: extractStoreActions(source, block),
  };
}

// ── Bubble position helpers ──

/** Simple deterministic layout: stacked rows by depth */
function nextBubblePosition(
  depth: number,
  siblingIndex: number,
): { x: number; y: number } {
  // depth 0: fixed top positions
  if (depth === 0) {
    return { x: 400, y: 100 };
  }

  // For depth 1+, use a grid-like layout
  const xBase = 120;
  const xSpread = 140;
  const yBase = 180;
  const yStep = 120;

  return {
    x: xBase + siblingIndex * xSpread,
    y: yBase + (depth - 1) * yStep,
  };
}

// ── ID helpers ──

let bubbleCounter = 0;
let edgeCounter = 0;

function resetCounters(): void {
  bubbleCounter = 0;
  edgeCounter = 0;
}

function bubbleId(prefix: string): string {
  return `${prefix}-${bubbleCounter++}`;
}

function edgeId(): string {
  return `e${edgeCounter++}`;
}

// ── Demo data fallback ──

function buildDemoData(): { bubbles: BubbleNode[]; edges: BubbleEdge[] } {
  const bubbles: BubbleNode[] = [
    { id: "ui", pid: "", type: "ui", label: "界面", depth: 0, children: ["page-main"], parentId: null, expanded: false, position: { x: 400, y: 100 }, isLeaf: false },
    { id: "store", pid: "", type: "store", label: "Store", depth: 0, children: ["store-todo"], parentId: null, expanded: false, position: { x: 100, y: 350 }, isLeaf: false },
    { id: "code", pid: "", type: "code", label: "代码", depth: 0, children: [], parentId: null, expanded: false, position: { x: 700, y: 350 }, isLeaf: false },
    { id: "page-main", pid: "c1", type: "ui", label: "MainPage", depth: 1, children: ["comp-header", "comp-todopage"], parentId: "ui", expanded: false, position: { x: 350, y: 220 }, isLeaf: false },
    { id: "comp-header", pid: "c2", type: "ui", label: "Header", depth: 2, children: [], parentId: "page-main", expanded: false, position: { x: 250, y: 340 }, isLeaf: true },
    { id: "comp-todopage", pid: "c3", type: "ui", label: "TodoPage", depth: 2, children: ["comp-todolist", "comp-filterbar"], parentId: "page-main", expanded: false, position: { x: 450, y: 340 }, isLeaf: false },
    { id: "comp-todolist", pid: "c4", type: "ui", label: "TodoList", depth: 3, children: ["comp-todoitem"], parentId: "comp-todopage", expanded: false, position: { x: 400, y: 460 }, isLeaf: false },
    { id: "comp-todoitem", pid: "c5", type: "ui", label: "TodoItem", depth: 4, children: [], parentId: "comp-todopage", expanded: false, position: { x: 350, y: 580 }, isLeaf: true },
    { id: "comp-filterbar", pid: "c6", type: "ui", label: "FilterBar", depth: 3, children: [], parentId: "comp-todopage", expanded: false, position: { x: 550, y: 460 }, isLeaf: true },
    { id: "store-todo", pid: "s1", type: "store", label: "TodoStore", depth: 1, children: ["sig-items", "act-add", "act-remove"], parentId: "store", expanded: false, position: { x: 100, y: 470 }, isLeaf: false },
    { id: "sig-items", pid: "s1_a1", type: "store", label: "items: Todo[]", depth: 2, children: [], parentId: "store-todo", expanded: false, position: { x: 30, y: 590 }, isLeaf: true, layer: "internal" as const },
    { id: "act-add", pid: "s1_a2", type: "store", label: "add(text)", depth: 2, children: [], parentId: "store-todo", expanded: false, position: { x: 100, y: 590 }, isLeaf: true, layer: "internal" as const },
    { id: "act-remove", pid: "s1_a3", type: "store", label: "remove(id)", depth: 2, children: [], parentId: "store-todo", expanded: false, position: { x: 170, y: 590 }, isLeaf: true, layer: "internal" as const },
  ];

  const edges: BubbleEdge[] = [
    { id: "e1", source: "ui", target: "store", edgeType: "composition" },
    { id: "e2", source: "ui", target: "code", edgeType: "composition" },
    { id: "e3", source: "comp-todolist", target: "store-todo", label: "items", edgeType: "store-read" },
    { id: "e4", source: "comp-todoitem", target: "act-remove", label: "remove()", edgeType: "store-write" },
  ];

  return { bubbles, edges };
}

// ── Tree building ──

/**
 * Recursively build component subtree.
 */
function buildComponentSubtree(
  compName: string,
  components: Map<string, ParsedComponent>,
  stores: Map<string, ParsedStore>,
  bubbles: BubbleNode[],
  edges: BubbleEdge[],
  parentBubbleId: string,
  depth: number,
  visited: Set<string>,
  siblingIndex: { value: number },
): string | null {
  if (visited.has(compName)) return null; // circular ref guard
  const comp = components.get(compName);
  if (!comp) {
    // Component not found in parsed files — might be a system component
    return null;
  }

  visited.add(compName);

  const bid = bubbleId(compName.toLowerCase());
  const isLeaf = !comp.hasPascalCaseChildren;

  // Position based on depth and sibling order
  const position = nextBubblePosition(depth, siblingIndex.value);

  const bubble: BubbleNode = {
    id: bid,
    pid: bid,
    type: "ui",
    label: compName,
    depth,
    children: [],
    parentId: parentBubbleId,
    expanded: false,
    position,
    isLeaf,
  };

  bubbles.push(bubble);

  // Composition edge from parent
  if (parentBubbleId !== "ui") {
    edges.push({
      id: edgeId(),
      source: parentBubbleId,
      target: bid,
      edgeType: "composition",
    });
  }

  // Store-read edges
  for (const storeRef of comp.storeRefs) {
    const store = stores.get(storeRef);
    if (store) {
      // Find store bubble ID
      const storeBubble = bubbles.find(
        (b) => b.label === storeRef && b.type === "store" && b.depth === 1,
      );
      if (storeBubble) {
        edges.push({
          id: edgeId(),
          source: bid,
          target: storeBubble.id,
          label: storeRef,
          edgeType: "store-read",
        });
      }
    }
  }

  // Recurse into child components
  const childSiblingIndex = { value: 0 };
  for (const childName of comp.viewChildren) {
    const childBid = buildComponentSubtree(
      childName, components, stores,
      bubbles, edges, bid, depth + 1,
      new Set(visited), childSiblingIndex,
    );
    if (childBid) {
      bubble.children.push(childBid);
      childSiblingIndex.value++;
    }
  }

  return bid;
}

/**
 * Build store bubble subtree.
 */
function buildStoreBubbles(
  stores: Map<string, ParsedStore>,
  bubbles: BubbleNode[],
  edges: BubbleEdge[],
): void {
  let storeIndex = 0;
  for (const [, store] of stores) {
    const storeBid = bubbleId(`store-${store.name.toLowerCase()}`);
    const position = nextBubblePosition(1, storeIndex);

    const storeBubble: BubbleNode = {
      id: storeBid,
      pid: storeBid,
      type: "store",
      label: store.name,
      depth: 1,
      children: [],
      parentId: "store",
      expanded: false,
      position,
      isLeaf: false,
    };
    bubbles.push(storeBubble);

    // Edge from Store top-level
    edges.push({
      id: edgeId(),
      source: "store",
      target: storeBid,
      edgeType: "composition",
    });

    // Signal children
    let sigIndex = 0;
    for (const sig of store.signals) {
      const sigBid = bubbleId(`sig-${sig}`);
      const sigPos = nextBubblePosition(2, sigIndex);
      bubbles.push({
        id: sigBid,
        pid: sigBid,
        type: "store",
        label: `${sig}`,
        depth: 2,
        children: [],
        parentId: storeBid,
        expanded: false,
        position: sigPos,
        isLeaf: true,
        layer: "internal",
      });
      storeBubble.children.push(sigBid);
      sigIndex++;
    }

    // Getter children
    for (const getter of store.getters) {
      const getBid = bubbleId(`get-${getter}`);
      const getPos = nextBubblePosition(2, sigIndex);
      bubbles.push({
        id: getBid,
        pid: getBid,
        type: "store",
        label: `${getter}`,
        depth: 2,
        children: [],
        parentId: storeBid,
        expanded: false,
        position: getPos,
        isLeaf: true,
        layer: "internal",
      });
      storeBubble.children.push(getBid);
      sigIndex++;
    }

    // Action children
    for (const act of store.actions) {
      const actBid = bubbleId(`act-${act}`);
      const actPos = nextBubblePosition(2, sigIndex);
      bubbles.push({
        id: actBid,
        pid: actBid,
        type: "store",
        label: `${act}()`,
        depth: 2,
        children: [],
        parentId: storeBid,
        expanded: false,
        position: actPos,
        isLeaf: true,
        layer: "internal",
      });
      storeBubble.children.push(actBid);
      sigIndex++;
    }

    storeIndex++;
  }
}

/**
 * Build lib/code bubble subtree.
 */
function buildCodeBubbles(
  libs: ParsedLib[],
  bubbles: BubbleNode[],
): void {
  let libIndex = 0;
  for (const lib of libs) {
    // Derive module name from file path
    const moduleName = lib.filePath
      .replace(/\.omni$/i, "")
      .split(/[/\\]/)
      .pop() || "lib";

    const libBid = bubbleId(`lib-${moduleName}`);
    const position = nextBubblePosition(1, libIndex);

    const libBubble: BubbleNode = {
      id: libBid,
      pid: libBid,
      type: "code",
      label: moduleName,
      depth: 1,
      children: [],
      parentId: "code",
      expanded: false,
      position,
      isLeaf: lib.functions.length === 0,
    };
    bubbles.push(libBubble);

    // Fn children
    let fnIndex = 0;
    for (const fn of lib.functions) {
      const fnBid = bubbleId(`fn-${fn}`);
      const fnPos = nextBubblePosition(2, fnIndex);
      bubbles.push({
        id: fnBid,
        pid: fnBid,
        type: "code",
        label: `${fn}()`,
        depth: 2,
        children: [],
        parentId: libBid,
        expanded: false,
        position: fnPos,
        isLeaf: true,
        layer: "internal",
      });
      libBubble.children.push(fnBid);
      fnIndex++;
    }

    libIndex++;
  }
}

// ── Main entry ──

export async function buildBubbleGraph(
  _projectDir: string,
  omniFiles?: OmniFileInfo[],
): Promise<{ bubbles: BubbleNode[]; edges: BubbleEdge[] }> {
  // Fall back to demo data if no files provided
  if (!omniFiles || omniFiles.length === 0) {
    return buildDemoData();
  }

  resetCounters();

  // ── Phase 1: Parse all files ──
  const components = new Map<string, ParsedComponent>();
  const stores = new Map<string, ParsedStore>();
  const libs: ParsedLib[] = [];

  for (const file of omniFiles) {
    // Check if it's a component file (use source scanning first, avoid parser crashes)
    const compNames = extractComponentNames(file.source);
    if (compNames.length > 0) {
      for (const compName of compNames) {
        if (components.has(compName)) continue; // dedup

        const tags = extractComponentTagsFromSource(file.source);
        const storeRefs = extractStoreRefsFromSource(file.source);
        const pChildren = [...tags.pascalCaseChildren];
        const pBasic = tags.basicHtmlTags.length > 0;

        components.set(compName, {
          name: compName,
          filePath: file.path,
          viewChildren: pChildren,
          storeRefs,
          hasBasicElements: pBasic,
          hasPascalCaseChildren: pChildren.length > 0,
        });
      }
      continue;
    }

    // Check if it's a store file
    const storeName = extractStoreName(file.source);
    if (storeName) {
      stores.set(storeName, analyzeStore(storeName, file.path, file.source));
      continue;
    }

    // Check if it's a lib file (has pub fn)
    if (/\bpub\s+fn\s/.test(file.source)) {
      const fns = extractPubFns(file.source);
      if (fns.length > 0) {
        libs.push({ filePath: file.path, functions: fns });
      }
    }
  }

  // ── Phase 2: Build bubble graph ──
  const bubbles: BubbleNode[] = [];
  const edges: BubbleEdge[] = [];

  // Top-level 3 bubbles
  const uiBubble: BubbleNode = { id: "ui", pid: "", type: "ui", label: "界面", depth: 0, children: [], parentId: null, expanded: false, position: { x: 400, y: 100 }, isLeaf: false };
  const storeBubble: BubbleNode = { id: "store", pid: "", type: "store", label: "Store", depth: 0, children: [], parentId: null, expanded: false, position: { x: 100, y: 350 }, isLeaf: false };
  const codeBubble: BubbleNode = { id: "code", pid: "", type: "code", label: "代码", depth: 0, children: [], parentId: null, expanded: false, position: { x: 700, y: 350 }, isLeaf: false };
  bubbles.push(uiBubble, storeBubble, codeBubble);

  // Find entry component (from main.omni, or first component found)
  const entryFile = omniFiles.find(
    (f) => f.path === "src/main.omni" || f.path.endsWith("/main.omni") || f.path.endsWith("\\main.omni"),
  );
  const entryNames = entryFile ? extractComponentNames(entryFile.source) : [];
  let entryCompName: string | null = entryNames.length > 0 ? entryNames[0] : null;

  // If no main.omni component, use the first component found
  if (!entryCompName && components.size > 0) {
    entryCompName = components.keys().next().value as string;
  }

  // Build component tree from entry point
  if (entryCompName && components.has(entryCompName)) {
    const siblingIndex = { value: 0 };
    const bid = buildComponentSubtree(
      entryCompName, components, stores,
      bubbles, edges, "ui", 1,
      new Set(), siblingIndex,
    );
    if (bid) {
      uiBubble.children.push(bid);
    }
  }

  // Edges: ui → store, ui → code (always present for context)
  edges.push(
    { id: edgeId(), source: "ui", target: "store", edgeType: "composition" },
    { id: edgeId(), source: "ui", target: "code", edgeType: "composition" },
  );

  // Build store bubbles
  buildStoreBubbles(stores, bubbles, edges);

  // Build code bubbles
  buildCodeBubbles(libs, bubbles);

  return { bubbles, edges };
}
