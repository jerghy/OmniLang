// editor/client/src/omni-compiler-cache.ts
// Compilation cache bridging omni-gen -> WASM compiler -> iframe srcdoc.
// Provides full-project compile, incremental per-component recompile,
// and cache invalidation with stale-output fallback on errors.

import { rustCompiler } from './api/rustCompiler';
import type { OmniViewResult, ComponentCompileResult } from './api/rustCompiler';
import { generateOmniSource } from './omni-gen/pipeline';
import type { ElementData, WireData } from './store/editor';
import type { CodeTab } from './codegraph/types';

// ── Types ──

export interface CompileOptions {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
}

interface ComponentCacheEntry {
  omniFragment: string;
  compiledJS: string;
  compiledHTML: string;
  compiledCSS: string;
  signalNames: string[];
  lastCompiled: number;
}

export interface CompiledOutput {
  srcdoc: string;
}

// ── OmniCompilerCache ──

class OmniCompilerCache {
  private components = new Map<string, ComponentCacheEntry>();
  private runtimeJS = '';
  private lastFullCompile = 0;
  private lastOutput: CompiledOutput | null = null;

  /**
   * Full project compile.
   * Runs omni-gen pipeline to produce OmniLang source, then WASM compile to
   * JS/HTML/CSS, assembles an iframe srcdoc, and caches per-component results.
   * On error, returns the previous (stale) output to avoid blank iframe.
   */
  compileProject(
    options: CompileOptions,
    editorMode: boolean = true,
  ): CompiledOutput | null {
    const genResult = generateOmniSource(options);
    if (genResult.errors?.length) {
      console.error('[OmniCompilerCache] omni-gen errors:', genResult.errors);
      return this.lastOutput;
    }

    const omniSource = genResult.value;
    if (omniSource === null) {
      console.error('[OmniCompilerCache] omni-gen returned null value');
      return this.lastOutput;
    }

    const wasmResult: OmniViewResult = rustCompiler.compileToOmniView(
      omniSource,
      editorMode,
    );

    if (!wasmResult.success || !wasmResult.files) {
      console.error('[OmniCompilerCache] WASM errors:', wasmResult.errors);
      return this.lastOutput;
    }

    // Build srcdoc from compiled output
    const srcdoc = buildSrcdoc(
      wasmResult.files.js,
      wasmResult.files.html,
      wasmResult.files.css,
    );

    this.lastFullCompile = Date.now();
    this.lastOutput = { srcdoc };

    return this.lastOutput;
  }

  /**
   * Incremental per-component recompile.
   * Runs the full omni-gen pipeline (Task 9 will add generateOmniFragment for
   * true incremental), checks if the fragment changed, and skips WASM if not.
   */
  recompileComponent(
    name: string,
    options: CompileOptions,
    editorMode: boolean = true,
  ): { js: string } | null {
    const genResult = generateOmniSource(options);
    if (genResult.errors?.length) {
      console.error('[OmniCompilerCache] omni-gen errors:', genResult.errors);
      return null;
    }

    const fragment = genResult.value;
    if (fragment === null) {
      console.error('[OmniCompilerCache] omni-gen returned null value');
      return null;
    }

    const cached = this.components.get(name);

    // Skip WASM if the fragment hasn't changed
    if (cached && cached.omniFragment === fragment) {
      return { js: cached.compiledJS };
    }

    const wasmResult: ComponentCompileResult =
      rustCompiler.compileOmniViewComponent(fragment, name, editorMode);

    if (!wasmResult.success || !wasmResult.component) {
      console.error(
        '[OmniCompilerCache] Component compile errors:',
        wasmResult.errors,
      );
      return null;
    }

    const componentJS = wasmResult.component.js;
    this.components.set(name, {
      omniFragment: fragment,
      compiledJS: componentJS,
      compiledHTML: wasmResult.component.html || '',
      compiledCSS: wasmResult.component.css || '',
      signalNames: extractSignalNames(componentJS),
      lastCompiled: Date.now(),
    });

    return { js: componentJS };
  }

  /** Remove a single component from the cache. */
  invalidate(name: string): void {
    this.components.delete(name);
  }

  /** Clear all cached state. */
  invalidateAll(): void {
    this.components.clear();
    this.runtimeJS = '';
    this.lastFullCompile = 0;
    this.lastOutput = null;
  }
}

// ── Helpers ──

/**
 * Assemble a full HTML document for the iframe from compiled JS/HTML/CSS files.
 */
function buildSrcdoc(js: string, html: string, css: string): string {
  return `<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>${css}</style>
</head>
<body>
  <div id="root">${html}</div>
  <script>${js}<\/script>
</body>
</html>`;
}

/**
 * Extract signal names from compiled JS by matching __omni_signal("name") calls.
 */
function extractSignalNames(js: string): string[] {
  const names: string[] = [];
  const re = /__omni_signal\("([^"]+)"/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(js)) !== null) {
    names.push(m[1]);
  }
  return names;
}

// ── Singleton export ──

export const compilerCache = new OmniCompilerCache();
