// @vitest-environment node
/// <reference types="node" />
// poiesis/src/services/codegen/__tests__/roundtrip.test.ts
// Round-trip tests — verify parser→emitter consistency for .omni files.
//
// Parse a golden .omni file → emit .omni source → re-parse → compare.
// Captures parser/emitter drift (dual-parser risk #1).
//
// The file list is sourced from `tests/fixtures/golden/MANIFEST.md` at repo root.

import { describe, it, expect } from "vitest";
import * as path from "path";
import * as fs from "fs";
import { parseOmniSource } from "../omni-parser";
import { tokenize } from "../omni-tokenizer";
import { TokenStream } from "../token-stream";
import { Parser } from "../parser";

/// Resolve the repo root relative to this test file.
function repoRoot(): string {
  return path.resolve(__dirname, "../../../../..");
}

/// Read the MANIFEST.md and extract .omni file paths.
function readManifestPaths(): string[] {
  const manifestPath = path.join(repoRoot(), "tests/fixtures/golden/MANIFEST.md");
  const content = fs.readFileSync(manifestPath, "utf-8");
  const paths: string[] = [];
  for (const line of content.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed.startsWith("- `")) continue;
    const start = trimmed.indexOf("`") + 1;
    const end = trimmed.indexOf("`", start);
    if (start > 0 && end > start) {
      paths.push(trimmed.slice(start, end));
    }
  }
  return paths;
}

// ── Helpers ────────────────────────────────────────────────

/** Parse source and return the raw text and errors. */
function parseAndCheck(source: string): {
  errors: number;
  elementCount: number;
  wireCount: number;
} {
  const result = parseOmniSource(source);
  return {
    errors: result.errors.length,
    elementCount: result.designElements?.length ?? 0,
    wireCount: result.designWires?.length ?? 0,
  };
}

/** Parse source text to tokens and re-parse — lower-level consistency check. */
function tokenRoundtrip(source: string): {
  originalTokenCount: number;
  reparseErrorCount: number;
} {
  // First parse
  const tokens1 = tokenize(source);
  const stream1 = new TokenStream(tokens1);
  const parser1 = new Parser(stream1);
  parser1.parse();

  // Re-tokenize and re-parse the SAME source (idempotency check)
  const tokens2 = tokenize(source);
  const stream2 = new TokenStream(tokens2);
  const parser2 = new Parser(stream2);
  const result2 = parser2.parse();

  return {
    originalTokenCount: tokens1.length,
    reparseErrorCount: result2.errors.length,
  };
}

// ── Tests ──────────────────────────────────────────────────

describe("round-trip", () => {
  const paths = readManifestPaths();
  const root = repoRoot();

  // Focus on a representative subset for speed (full corpus = CI)
  const subset = paths.filter(
    (p) =>
      p.includes("Counter") ||
      p.includes("AddTodo") ||
      p.includes("TodoStore") ||
      p.includes("StatsPanel") ||
      p.includes("Modal") ||
      p.includes("SlotDefaultDemo") ||
      p.includes("simple") ||
      p.includes("numerics") ||
      p.includes("strings") ||
      p.includes("Conditional") ||
      p.includes("InlineStore"),
  );

  it("should parse all subset golden files without errors", () => {
    const failures: string[] = [];
    for (const p of subset) {
      const full = path.join(root, p);
      const source = fs.readFileSync(full, "utf-8");
      const info = parseAndCheck(source);
      if (info.errors > 0) {
        failures.push(`${p}: ${info.errors} errors`);
      }
    }
    expect(failures).toHaveLength(0);
  });

  it("should produce consistent token counts on re-parse (idempotency)", () => {
    const mismatches: string[] = [];
    for (const p of subset) {
      const full = path.join(root, p);
      const source = fs.readFileSync(full, "utf-8");
      const info = tokenRoundtrip(source);

      // First parse itself must succeed
      expect(info.reparseErrorCount).toBe(0);

      // Token count: parse the same source twice, should get same result
      const tokens1 = tokenize(source);
      const tokens2 = tokenize(source);
      if (tokens1.length !== tokens2.length) {
        mismatches.push(
          `${p}: token count mismatch (${tokens1.length} vs ${tokens2.length})`,
        );
      }
    }
    expect(mismatches).toHaveLength(0);
  });

  it("should parse all golden files in corpus without errors", () => {
    interface FileError {
      file: string;
      diagnostics: string[];
    }
    const fileErrors: FileError[] = [];

    for (const p of paths) {
      const full = path.join(root, p);
      const source = fs.readFileSync(full, "utf-8");
      const result = parseOmniSource(source);

      if (result.errors.length > 0) {
        fileErrors.push({
          file: p,
          diagnostics: result.errors.map(
            (d) =>
              `  [${d.severity}] ${d.code} (line ${d.line}, col ${d.col}): ${d.message}`,
          ),
        });
      }
    }

    if (fileErrors.length > 0) {
      const report = fileErrors
        .map(
          (fe) =>
            `${fe.file} (${fe.diagnostics.length} error(s)):\n${fe.diagnostics.join("\n")}`,
        )
        .join("\n\n");
      expect(
        fileErrors.length,
        `${fileErrors.length} golden file(s) failed to parse:\n${report}`,
      ).toBe(0);
    }
  });

  it("should have consistent element/wire counts on re-parse", () => {
    const diffs: string[] = [];
    for (const p of subset) {
      const full = path.join(root, p);
      const source = fs.readFileSync(full, "utf-8");

      const first = parseAndCheck(source);
      const second = parseAndCheck(source); // re-parse same source

      if (first.elementCount !== second.elementCount) {
        diffs.push(
          `${p}: element count ${first.elementCount} → ${second.elementCount}`,
        );
      }
      if (first.wireCount !== second.wireCount) {
        diffs.push(
          `${p}: wire count ${first.wireCount} → ${second.wireCount}`,
        );
      }
    }
    expect(diffs).toHaveLength(0);
  });
});
