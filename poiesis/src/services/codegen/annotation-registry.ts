// Unified annotation parsing and emission for all // @... comments in .omni source.
// Single source of truth for annotation format across the entire codebase.

// ── Regex constants ─────────────────────────────────────────────────────────

/** Matches `// @pid:<id>` — the sole identity annotation format in .omni. */
export const PID_REGEX = /(?:\/\/|^)\s*@pid:(\S+)/;

/** Matches `// @<type>` — manual type annotations for fn/action body nodes. */
export const TYPE_REGEX = /^\s*\/\/\s*@(if|elif|else|return|code-block|for|while)\s*$/;

/** The set of all valid type annotation values. */
export const VALID_TYPES = new Set(["if", "elif", "else", "return", "code-block", "for", "while"]);

// ── Parsing ─────────────────────────────────────────────────────────────────

/**
 * Parse `// @pid:<id>` from a source line.
 * Returns the pid or null if the line does not match.
 */
export function parsePidAnnotation(line: string): { pid: string } | null {
  const match = line.match(PID_REGEX);
  if (!match) return null;
  return { pid: match[1] };
}

/**
 * Parse `// @<type>` from a source line.
 * Returns the nodeType or null if the line does not match one of the valid types.
 */
export function parseTypeAnnotation(line: string): { nodeType: string } | null {
  const match = line.match(TYPE_REGEX);
  if (!match) return null;
  return { nodeType: match[1] };
}

/**
 * Parse any annotation from a source line.
 * Returns pid, nodeType, or null.
 */
export function parseAnyAnnotation(line: string): { pid?: string; nodeType?: string } | null {
  const pidResult = parsePidAnnotation(line);
  if (pidResult) return pidResult;

  const typeResult = parseTypeAnnotation(line);
  if (typeResult) return typeResult;

  return null;
}

// ── Classification ──────────────────────────────────────────────────────────

/** True if the line contains a `// @pid:` annotation. */
export function isPidAnnotation(line: string): boolean {
  return PID_REGEX.test(line);
}

/** True if the line contains a `// @<type>` annotation. */
export function isTypeAnnotation(line: string): boolean {
  return TYPE_REGEX.test(line);
}

/** True if the line contains any // @... annotation. */
export function isAnyAnnotation(line: string): boolean {
  return isPidAnnotation(line) || isTypeAnnotation(line);
}

// ── Emission ────────────────────────────────────────────────────────────────

/**
 * Emit a `// @pid:<id>` annotation line.
 * This is the ONLY format ever emitted into .omni source.
 */
export function emitPidAnnotation(pid: string): string {
  return `// @pid:${pid}`;
}
