import { useEffect, useCallback } from "react";
import { usePoiesisStore } from "../store";
import { useCodeGraphStore, type CodeGraphStoreState } from "../services/codegraph/store";
import { parseOmniSource } from "../services/codegen/omni-parser";
import { parseStyleValue } from "../services/codegen/design-value-parser";
import { uid } from "../core/utils/id";
import { api } from "../commands";
import { loadComponentDefinitionsFromServer, saveComponentDefinitionsToServer } from "../services/project/component-service";
import { computeAutoLayout } from "../components/canvas/shared/layoutUtils";
import type { ElementData, WireData, ViewElementData, GroupElementData } from "../core/types";
import type { TokenType } from "../core/types/design";
import { tokenTypeToCategory } from "../core/types/design";
import { createLogger } from "../lib/utils/logger";

const log = createLogger("init");

/** Parse a design.omni source string and load it into the design store. */
async function loadDesignFromSource(source: string): Promise<void> {
  const result = parseOmniSource(source);
  if ((result.designTokens?.length ?? 0) > 0
      || (result.designClasses?.length ?? 0) > 0
      || (result.designVariants?.length ?? 0) > 0) {
    const { loadDesign } = usePoiesisStore.getState();
    loadDesign({
      name: result.designName || "AppDesign",
      tokens: (result.designTokens ?? []).map((t) => ({
        id: t.id, name: t.name, category: tokenTypeToCategory(t.tokenType), tokenType: t.tokenType as TokenType, value: t.value,
      })),
      classes: (result.designClasses ?? []).map((c) => ({
        id: c.id,
        name: c.name,
        properties: (c.properties ?? []).map((p) => ({
          id: uid("sp"),
          name: p.name,
          segments: parseStyleValue(p.value),
        })),
        pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
          id: uid("ps"),
          pseudo: ps.pseudo,
          properties: ps.properties.map((p) => ({
            id: uid("sp"),
            name: p.name,
            segments: parseStyleValue(p.value),
          })),
        })),
      })),
      variants: (result.designVariants ?? []).map((v) => ({
        id: v.id,
        name: v.name,
        dimensions: (v.dimensions ?? []).map((d) => ({
          name: d.name,
          values: d.values.map((dv) => dv.name),
        })),
        defaults: v.defaults ?? {},
        baseProperties: (v.baseProperties ?? []).map((p) => ({
          id: uid("sp"),
          name: p.name,
          segments: parseStyleValue(p.value),
        })),
        branches: (v.branches ?? []).map((b) => {
          const dotIdx = b.name.indexOf(".");
          const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
          const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
          const props = b.properties.map((p) => ({
            id: uid("sp"),
            name: p.name,
            segments: parseStyleValue(p.value),
          }));
          return { id: uid("vb"), dimension: dim, value: val, properties: props };
        }),
      })),
    });
  }
}

export function useProjectInit(setShowWelcome: (v: boolean) => void) {
  // Initial load
  useEffect(() => {
    (async () => {
      const proj = await api.project.getProject();
      if (proj.dir) usePoiesisStore.getState().setProjectDir(proj.dir);
      await api.project.listDir(".");
    })();
  }, [setShowWelcome]);

  // Component def auto-save
  useEffect(() => {
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    const unsub = usePoiesisStore.subscribe((state) => {
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveTimer = null;
        const defs = state.componentDefs;
        saveComponentDefinitionsToServer((defs ?? {}) as unknown as Parameters<typeof saveComponentDefinitionsToServer>[0]).catch((e) => log.error("Failed to save component defs:", e));
      }, 2000);
    });
    return () => {
      unsub();
      if (saveTimer) clearTimeout(saveTimer);
    };
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      api.compile.stopRun().catch((e) => log.warn("Failed to stop run:", e));
      api.disconnectDebug();
    };
  }, []);

  const handleProjectReady = useCallback(
    (dir: string, name: string) => {
      usePoiesisStore.getState().setProjectDir(dir);
      usePoiesisStore.getState().newProject();
      usePoiesisStore.setState({ projectName: name, projectDir: dir });

      (async () => {
        try {
          const res = await api.project.readFile("src/main.omni");
          if (res.content) {
            usePoiesisStore.setState({ activeFileContent: res.content, activeFilePath: "src/main.omni" });
            const parseResult = parseOmniSource(res.content);

            // Populate componentDefs from parsed props/signals/computeds/events
            if (parseResult.props || parseResult.signals || parseResult.computeds || parseResult.events) {
              const existing = usePoiesisStore.getState().componentDefs["main"] || {};
              usePoiesisStore.getState().setComponentDef("main", {
                ...existing,
                name: "main",
                filePath: "src/main.omni",
                signals: (parseResult.signals || []).map((s) => ({ pid: uid("sgn"), name: s.name, type: s.type, defaultValue: s.defaultValue })),
                props: (parseResult.props || []).map((p) => ({ pid: uid("p"), name: p.name, type: p.type, defaultValue: p.defaultValue })),
                computeds: (parseResult.computeds || []).map((c) => ({ pid: uid("c"), name: c.name, returnType: c.type, expression: c.expression })),
                events: (parseResult.events || []).map((e) => ({ pid: uid("evt"), name: e.name, payloadType: "" })),
                fns: existing.fns || [],
              });
            }

            if (parseResult.designElements) {
              const rootOrder = parseResult.designElements
                .filter((e: ElementData) => !e.parentId)
                .map((e: ElementData) => e.id);
              // Compute auto-layout positions for all elements
              const layout = computeAutoLayout(parseResult.designElements || []) as unknown as Record<string, { x: number; y: number }>;
              const positionedElements = parseResult.designElements.map((el) => ({
                ...el,
                position: layout[el.id] ?? ("position" in el ? el.position : undefined) ?? { x: 100, y: 100 },
              }));
              usePoiesisStore.setState({
                elements: positionedElements,
                wires: parseResult.designWires || [],
                rootOrder,
              });
              if (parseResult.codeTabs?.length) {
                (useCodeGraphStore.getState() as unknown as CodeGraphStoreState).loadFromData(parseResult.codeTabs);
              }
            }

            // Load inline store definitions from parsed component file
            if (parseResult.inlineStoreDefs) {
              const { loadInlineStoreDef } = usePoiesisStore.getState();
              for (const def of Object.values(parseResult.inlineStoreDefs)) {
                const dotIdx = def.name.indexOf(".");
                const storeName = dotIdx >= 0 ? def.name.slice(dotIdx + 1) : def.name;
                const rekeyed = { ...def, name: `main.${storeName}` };
                loadInlineStoreDef(rekeyed);
              }
            }
          }
        } catch (e) {
          log.warn("Failed to load main.omni:", e);
        }
      })();

      // ── Also load design.omni (canonical src/, fallback root) ──
      (async () => {
        try {
          // Prefer canonical src/design.omni; fall back to root design.omni
          const srcRes = await api.project.readFile("src/design.omni");
          if (srcRes.content && !srcRes.error) {
            await loadDesignFromSource(srcRes.content);
            // Clean up old root design.omni if src/ exists
            const rootRes = await api.project.readFile("design.omni");
            if (rootRes.content && !rootRes.error) {
              api.project.deleteFile("design.omni").catch((e) => { log.warn("Failed to delete design.omni", e); });
            }
          } else {
            const rootRes = await api.project.readFile("design.omni");
            if (rootRes.content && !rootRes.error) {
              await loadDesignFromSource(rootRes.content);
            }
          }
        } catch (e) {
          log.warn("Failed to load design.omni:", e);
        }
      })();

      (async () => {
        try {
          const merged = await loadComponentDefinitionsFromServer();
          if (Object.keys(merged).length > 0) {
            usePoiesisStore.setState({ componentDefs: merged as unknown as Record<string, import("../core/types/project-json").ComponentDef> });
          }
        } catch (e) {
          log.warn("Failed to load component definitions:", e);
        }
      })();

      setShowWelcome(false);
    },
    [setShowWelcome],
  );

  return { handleProjectReady };
}
