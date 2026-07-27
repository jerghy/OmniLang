// @vitest-environment node
/// <reference types="node" />
// poiesis/src/services/codegen/__tests__/golden.test.ts
// Golden file parse tests — verify the TypeScript parser successfully parses
// a representative corpus of .omni files, establishing a baseline for
// dual-parser (Rust <-> TypeScript) consistency checking.
//
// The file list is sourced from `tests/fixtures/golden/MANIFEST.md` at repo root.

import { describe, it, expect } from "vitest";
import * as path from "path";
import * as fs from "fs";
import { parseOmniSource } from "../omni-parser";

/// Resolve the repo root relative to this test file.
function repoRoot(): string {
  // poiesis/src/services/codegen/__tests__ -> repo root
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

describe("golden files", () => {
  const paths = readManifestPaths();

  it("should list at least 20 golden files", () => {
    expect(paths.length).toBeGreaterThanOrEqual(20);
  });

  it("should have all golden files present on disk", () => {
    const root = repoRoot();
    const missing: string[] = [];
    for (const p of paths) {
      const full = path.join(root, p);
      if (!fs.existsSync(full)) {
        missing.push(p);
      }
    }
    expect(missing, `Missing golden files:\n${missing.join("\n")}`).toHaveLength(0);
  });

  it("should parse all golden files without errors", () => {
    const root = repoRoot();
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
            (d) => `  [${d.severity}] ${d.code} (line ${d.line}, col ${d.col}): ${d.message}`,
          ),
        });
      }
    }

    // Report all errors in bulk (not fail-fast on first error)
    if (fileErrors.length > 0) {
      const report = fileErrors
        .map(
          (fe) => `${fe.file} (${fe.diagnostics.length} error(s)):\n${fe.diagnostics.join("\n")}`,
        )
        .join("\n\n");
      expect(
        fileErrors.length,
        `${fileErrors.length} golden file(s) failed to parse:\n${report}`,
      ).toBe(0);
    }
  });
});
