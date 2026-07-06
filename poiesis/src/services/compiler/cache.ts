// editor/client/src/services/compiler/cache.ts
// Caches compiled OmniView output by source content.
// Wraps the omni-gen pipeline (elements -> .omni source) and the
// Rust WASM compiler (.omni source -> JS/HTML/CSS) into a single cache.

import { generateOmniSource } from "../codegen/pipeline";
import { rustCompiler } from "../../commands/rustCompiler";
import type { ElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import type { CodeTab } from "../../services/codegraph/types";
import { Logger } from "../../lib/utils/logger";

const log = new Logger({ prefix: "OmniCompilerCache" });

export interface CompileProjectInput {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
}

export interface CompileProjectOutput {
  srcdoc: string;
  /** 本次编译生成的 .omni 源码（从 elements 转换而来） */
  omniSource?: string;
  errors?: string[];
}

/** Internal cache entry with timestamp for time-based eviction. */
interface CacheEntry {
  output: CompileProjectOutput;
  timestamp: number;
}

class OmniCompilerCache {
  private cache = new Map<string, CacheEntry>();
  private lastOutput: CompileProjectOutput | null = null;
  private lastOutputTimestamp: number = 0;
  private consecutiveFailures: number = 0;
  private static readonly MAX_CACHE_AGE = 5 * 60 * 1000; // 5 minutes
  private static readonly MAX_STALE_FAILURES = 3; // show error after N consecutive failures
  private static readonly MAX_STALE_AGE = 2 * 60 * 1000; // show error if last success > 2 min ago

  /**
   * Compile a project design into an HTML srcdoc for the CanvasPreview iframe.
   * Returns null when no source could be generated (empty project).
   * On failure, returns the previous (stale) output to keep the preview alive.
   */
  compileProject(
    input: CompileProjectInput,
    editorMode: boolean = false,
  ): CompileProjectOutput | null {
    // Validate cache — remove stale entries
    this.validateCache();

    // Generate OmniLang source from design elements
    const genResult = generateOmniSource({
      designElements: input.designElements,
      designWires: input.designWires,
      codeTabs: input.codeTabs,
      componentName: input.componentName,
      rootOrder: input.rootOrder,
    });

    if (!genResult.value) {
      const msgs =
        genResult.errors.length > 0
          ? genResult.errors.map((e) => e.message)
          : ["No OmniLang source generated"];
      this.consecutiveFailures++;
      const staleAge = Date.now() - this.lastOutputTimestamp;
      if (
        this.lastOutput &&
        this.consecutiveFailures < OmniCompilerCache.MAX_STALE_FAILURES &&
        staleAge < OmniCompilerCache.MAX_STALE_AGE
      ) {
        log.warn(
          `Source generation failed (${this.consecutiveFailures}/${OmniCompilerCache.MAX_STALE_FAILURES}), ` +
          `returning stale output (${Math.round(staleAge / 1000)}s old)`,
        );
        return this.lastOutput;
      }
      return this.errorResult(msgs);
    }

    const source = genResult.value;

    // Check cache
    const cached = this.cache.get(source);
    if (cached) return cached.output;

    // Compile through WASM
    const result = rustCompiler.compileToOmniView(source, editorMode);

    if (!result.success || !result.files) {
      const msgs =
        result.errors?.map((e) => e.message) ?? ["WASM compilation failed"];
      this.consecutiveFailures++;
      // Show stale output for transient failures, but escalate to error
      // after N consecutive failures or if the last success is too old.
      const staleAge = Date.now() - this.lastOutputTimestamp;
      if (
        this.lastOutput &&
        this.consecutiveFailures < OmniCompilerCache.MAX_STALE_FAILURES &&
        staleAge < OmniCompilerCache.MAX_STALE_AGE
      ) {
        log.warn(
          `WASM compilation failed (${this.consecutiveFailures}/${OmniCompilerCache.MAX_STALE_FAILURES}), ` +
          `returning stale output (${Math.round(staleAge / 1000)}s old)`,
        );
        return this.lastOutput;
      }
      if (this.consecutiveFailures >= OmniCompilerCache.MAX_STALE_FAILURES) {
        log.error(`WASM compilation failed ${this.consecutiveFailures} times — showing error`);
      }
      // Do NOT cache error results — transient failures should be retried
      return this.errorResult(msgs);
    }

    // Build srcdoc from compiled JS/CSS/HTML
    const srcdoc = this.buildSrcdoc(
      result.files.js,
      result.files.css,
      result.files.html,
    );
    const output: CompileProjectOutput = { srcdoc, omniSource: source };
    this.cache.set(source, { output, timestamp: Date.now() });
    this.lastOutput = output; // Save successful output for stale fallback
    this.lastOutputTimestamp = Date.now();
    this.consecutiveFailures = 0; // Reset failure counter on success
    return output;
  }

  /** Invalidate the entire cache (e.g. when compiler reloads). */
  invalidate(): void {
    this.cache.clear();
    this.lastOutput = null;
    this.lastOutputTimestamp = 0;
    this.consecutiveFailures = 0;
  }

  /**
   * Remove cache entries older than MAX_CACHE_AGE to prevent stale
   * caches from persisting forever.
   */
  private validateCache(): void {
    const now = Date.now();
    for (const [key, entry] of this.cache) {
      if (now - entry.timestamp > OmniCompilerCache.MAX_CACHE_AGE) {
        this.cache.delete(key);
      }
    }
  }

  private buildSrcdoc(js: string, css: string, bodyHtml: string): string {
    return [
      "<!DOCTYPE html>",
      '<html lang="en">',
      "<head>",
      '<meta charset="UTF-8">',
      '<meta name="viewport" content="width=device-width, initial-scale=1.0">',
      "<title>OmniView Preview</title>",
      "<style>",
      "*,*::before,*::after{box-sizing:border-box}",
      "body{margin:0;padding:8px;font-family:system-ui,sans-serif}",
      css,
      "</style>",
      "</head>",
      "<body>",
      bodyHtml,
      "<script>",
      js,
      "</script>",
      "</body>",
      "</html>",
    ].join("\n");
  }

  private errorResult(errors: string[]): CompileProjectOutput {
    const escaped = errors
      .join("\n")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    return {
      srcdoc: [
        "<!DOCTYPE html>",
        '<html lang="en">',
        "<head><meta charset=\"UTF-8\"><title>Error</title></head>",
        '<body style="display:flex;align-items:center;justify-content:center;height:100vh;font-family:sans-serif;padding:20px;box-sizing:border-box;margin:0">',
        '<div style="color:#DC2626;background:#FEF2F2;border:1px solid #FECACA;border-radius:8px;padding:16px;max-width:500px">',
        '<h3 style="margin:0 0 8px;font-size:14px">Compilation Error</h3>',
        `<pre style="font-size:12px;white-space:pre-wrap;margin:0">${escaped}</pre>`,
        "</div>",
        "</body>",
        "</html>",
      ].join("\n"),
      errors,
    };
  }
}

/** Singleton compiler cache instance. */
export const compilerCache = new OmniCompilerCache();
