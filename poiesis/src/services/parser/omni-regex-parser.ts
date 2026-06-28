// services/parser/omni-regex-parser.ts
// Lightweight regex-based .omni source parser — replaces Python FastAPI endpoints
// /api/parse-view, /api/parse-codeblocks, /api/parse-native-ports
//
// These are quick regex extractions used after compilation to populate the canvas,
// NOT a replacement for the full codegen parseOmniSource().
//
// Python original: omnilang/api.py (_extract_view_tree_regex, _extract_blocks_regex, parse_native_ports)

// ── Types ──

export interface ViewTreeNode {
  id: string;
  tag: string;
  props: Record<string, unknown>;
  children: ViewTreeNode[];
  test_id?: string;
  source_line: number;
}

export interface CodeBlockNode {
  id: string;
  blockType: "store" | "signal" | "computed" | "action" | "fn" | "onPress" | "onChange";
  name: string;
  parentStore?: string;
  typeAnnotation?: string;
  sourceLine: number;
}

export interface NativePortInfo {
  id: string;
  target: string;
  inputs: Array<{ name: string; type: string }>;
  outputs: Array<{ name: string; type: string }>;
  source_line: number;
}

export interface ParseViewResult {
  success: boolean;
  nodes: ViewTreeNode[];
}

export interface ParseCodeBlocksResult {
  success: boolean;
  blocks: CodeBlockNode[];
}

export interface ParseNativePortsResult {
  success: boolean;
  ports: NativePortInfo[];
}

// ── UI mode detection ──

/**
 * Detect whether source is an OmniView UI file.
 * A file is a "view file" iff it contains 'import omniview'.
 */
export function detectUiMode(source: string): boolean {
  return source.includes("import omniview");
}

// ── View tree extraction ──

/**
 * Parse .omni source and extract view element tree (for canvas rendering).
 * Uses regex-based extraction as a lightweight fallback.
 */
export function parseViewTree(source: string): ViewTreeNode[] {
  return extractViewTreeRegex(source);
}

function extractViewTreeRegex(source: string): ViewTreeNode[] {
  const result: ViewTreeNode[] = [];
  const nodeId = { current: 0 };

  // Match component/view blocks: component Name { view { <tag>...</tag> } }
  const componentPattern = /component\s+(\w+)\s*\{/g;
  let match: RegExpExecArray | null;

  while ((match = componentPattern.exec(source)) !== null) {
    const name = match[1];
    result.push({
      id: `comp-${name}`,
      tag: "Container",
      props: { "data-component": name },
      children: [],
      source_line: source.slice(0, match.index).split("\n").length,
    });
  }

  // Also match top-level view blocks: view { ... }
  const viewPattern = /view\s*\{/g;
  while ((match = viewPattern.exec(source)) !== null) {
    const id = `view-${++nodeId.current}`;
    result.push({
      id,
      tag: "Container",
      props: {},
      children: [],
      source_line: source.slice(0, match.index).split("\n").length,
    });
  }

  // Match JSX-like elements: <Tag>, <Tag attr="val">, etc.
  const elementPattern = /<(\w+)([^>]*?)>/g;
  while ((match = elementPattern.exec(source)) !== null) {
    const tag = match[1];
    if (["component", "view", "import", "/"].includes(tag)) continue;
    const attrs = match[2] || "";
    const id = `el-${++nodeId.current}`;
    const props: Record<string, unknown> = {};

    // Extract attributes
    const attrPattern = /(\w+)=["']([^"']*?)["']/g;
    let attrMatch: RegExpExecArray | null;
    while ((attrMatch = attrPattern.exec(attrs)) !== null) {
      props[attrMatch[1]] = attrMatch[2];
    }

    // Detect test-id
    const testIdMatch = attrs.match(/test-id=["']([^"']*?)["']/);
    const testId = testIdMatch ? testIdMatch[1] : undefined;

    result.push({
      id,
      tag,
      props,
      children: [],
      test_id: testId,
      source_line: source.slice(0, match.index).split("\n").length,
    });
  }

  return result;
}

// ── Code block extraction ──

/**
 * Parse .omni source and extract store/signal/computed/action/fn declarations.
 * Uses regex-based extraction (mirrors Python's _extract_blocks_regex).
 */
export function parseCodeBlocks(source: string): CodeBlockNode[] {
  return extractBlocksRegex(source);
}

function extractBlocksRegex(source: string): CodeBlockNode[] {
  const blocks: CodeBlockNode[] = [];
  let nodeId = 0;

  // Match store definitions: store Name { ... }
  const storePattern = /store\s+(\w+)\s*\{/g;
  let match: RegExpExecArray | null;
  while ((match = storePattern.exec(source)) !== null) {
    nodeId++;
    blocks.push({
      id: `cb-store-${match[1]}`,
      blockType: "store",
      name: match[1],
      sourceLine: source.slice(0, match.index).split("\n").length,
    });
  }

  // Match component definitions: component Name { ... }
  const compPattern = /component\s+(\w+)\s*\{/g;
  while ((match = compPattern.exec(source)) !== null) {
    nodeId++;
    blocks.push({
      id: `cb-store-${match[1]}`,
      blockType: "store",
      name: match[1],
      sourceLine: source.slice(0, match.index).split("\n").length,
    });
  }

  // Match signal declarations: signal name: Type or let mut name: Type
  const signalPattern = /(?:signal|let\s+mut)\s+(\w+)(?:\s*:\s*(\w+))?/g;
  while ((match = signalPattern.exec(source)) !== null) {
    nodeId++;
    blocks.push({
      id: `cb-signal-${nodeId}`,
      blockType: "signal",
      name: match[1],
      typeAnnotation: match[2] || undefined,
      sourceLine: source.slice(0, match.index).split("\n").length,
    });
  }

  // Match computed/getter declarations: computed name => expr
  const computedPattern = /(?:computed|get)\s+(\w+)(?:\s*:\s*(\w+))?\s*[=:]/g;
  while ((match = computedPattern.exec(source)) !== null) {
    nodeId++;
    blocks.push({
      id: `cb-computed-${nodeId}`,
      blockType: "computed",
      name: match[1],
      typeAnnotation: match[2] || undefined,
      sourceLine: source.slice(0, match.index).split("\n").length,
    });
  }

  // Match action/fn declarations: fn name(...)
  const fnPattern = /(?:fn|action)\s+(\w+)\s*\(/g;
  while ((match = fnPattern.exec(source)) !== null) {
    nodeId++;
    blocks.push({
      id: `cb-action-${nodeId}`,
      blockType: "action",
      name: match[1],
      sourceLine: source.slice(0, match.index).split("\n").length,
    });
  }

  return blocks;
}

// ── Native port extraction ──

/**
 * Parse @native blocks and extract @in/@out comment annotations for canvas ports.
 *
 * Scans for patterns like:
 *     // @in: videoUrl (String)
 *     // @out: isPlaying (bool)
 *     @native(react) { ... }
 */
export function parseNativePorts(source: string): NativePortInfo[] {
  const results: NativePortInfo[] = [];
  let nodeId = 0;

  const lines = source.split("\n");
  let currentIns: Array<{ name: string; type: string }> = [];
  let currentOuts: Array<{ name: string; type: string }> = [];

  for (let i = 0; i < lines.length; i++) {
    const stripped = lines[i].trim();

    // Detect @in annotation
    const inMatch = stripped.match(/^\/\/\s*@in:\s*(\w+)\s*\((\w+)\)/);
    if (inMatch) {
      currentIns.push({ name: inMatch[1], type: inMatch[2] });
      continue;
    }

    // Detect @out annotation
    const outMatch = stripped.match(/^\/\/\s*@out:\s*(\w+)\s*\((\w+)\)/);
    if (outMatch) {
      currentOuts.push({ name: outMatch[1], type: outMatch[2] });
      continue;
    }

    // Detect @native block start
    const nativeMatch = stripped.match(/^@native\((\w+)\)\s*\{/);
    if (nativeMatch) {
      nodeId++;
      const target = nativeMatch[1];
      results.push({
        id: `native-${target}-${nodeId}`,
        target,
        inputs: [...currentIns],
        outputs: [...currentOuts],
        source_line: i + 1,
      });
      currentIns = [];
      currentOuts = [];
      continue;
    }

    // Reset annotations on blank lines or non-comment lines
    if (!stripped || (!stripped.startsWith("//") && !stripped.startsWith("@"))) {
      currentIns = [];
      currentOuts = [];
    }
  }

  return results;
}

// ── Path mapping (for cross-file import resolution) ──

/**
 * Derive import path mapping from source file paths.
 * Maps relative import paths to actual file paths.
 */
export function derivePathMapping(sources: Record<string, string>): Record<string, string> {
  const mapping: Record<string, string> = {};

  for (const filepath of Object.keys(sources)) {
    // Direct path → self
    mapping[filepath] = filepath;

    // Basename
    const basename = filepath.split("/").pop() || filepath;
    if (!(basename in mapping)) {
      mapping[basename] = filepath;
    }

    // Stem (without extension)
    const stem = basename.includes(".") ? basename.slice(0, basename.lastIndexOf(".")) : basename;
    if (!(stem in mapping)) {
      mapping[stem] = filepath;
    }
  }

  // For every pair of files, compute relative-path keys
  for (const importPath of Object.keys(sources)) {
    const importerParts = importPath.split("/");

    for (const targetPath of Object.keys(sources)) {
      if (importPath === targetPath) continue;

      // Compute relative path
      const targetParts = targetPath.split("/");
      let commonLen = 0;
      while (
        commonLen < importerParts.length - 1 &&
        commonLen < targetParts.length &&
        importerParts[commonLen] === targetParts[commonLen]
      ) {
        commonLen++;
      }

      const upCount = importerParts.length - 1 - commonLen;
      const relParts: string[] = [];
      for (let u = 0; u < upCount; u++) relParts.push("..");
      for (let t = commonLen; t < targetParts.length; t++) relParts.push(targetParts[t]);
      const rel = relParts.join("/");

      // Map "./Component.omni" → full path
      mapping["./" + rel] = targetPath;

      // Map "../lib" (without ./ prefix)
      mapping[rel] = targetPath;

      // Map "./Component" (without .omni extension) → full path
      if (rel.endsWith(".omni")) {
        const relNoExt = rel.slice(0, -5);
        mapping["./" + relNoExt] = targetPath;
        mapping[relNoExt] = targetPath;
      }
    }
  }

  return mapping;
}

// ── Convenience: parse both view and code blocks at once ──

export function parseAll(source: string): {
  viewNodes: ViewTreeNode[];
  codeBlocks: CodeBlockNode[];
  nativePorts: NativePortInfo[];
  isUI: boolean;
} {
  return {
    viewNodes: parseViewTree(source),
    codeBlocks: parseCodeBlocks(source),
    nativePorts: parseNativePorts(source),
    isUI: detectUiMode(source),
  };
}
