import { useEffect, useCallback } from "react";
import { usePoiesisStore } from "../store";
import { useCodeGraphStore, type CodeGraphStoreState } from "../services/codegraph/store";
import { parseOmniSource } from "../services/codegen";
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
        id: c.id, name: c.name, properties: [], pseudoStates: [],
      })),
      variants: (result.designVariants ?? []).map((v) => ({
        id: v.id, name: v.name, dimensions: [], defaults: {}, baseProperties: [], branches: [],
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
          const loadResult = await (api.annotation as unknown as { annotatedLoad: (path: string) => Promise<{ success: boolean; source?: string; annotations?: Record<string, unknown> }> }).annotatedLoad("src/main.omni");
          if (loadResult.success && loadResult.source) {
            usePoiesisStore.setState({
              activeFileContent: loadResult.source,
              activeFilePath: "src/main.omni",
            });

            const parseResult = parseOmniSource(loadResult.source);
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

            if (loadResult.annotations && typeof loadResult.annotations === "object" && "viewMarkers" in loadResult.annotations) {
              const annotations = loadResult.annotations as Record<string, unknown>;
              const markers = annotations.viewMarkers as Array<{ elementId: string; isFolded?: boolean; renderMode?: string }> | undefined;
              if (markers) {
                usePoiesisStore.setState((prev) => ({
                  elements: prev.elements.map((el) => {
                    const marker = markers.find((m) => m.elementId === el.id);
                    if (marker) {
                      return {
                        ...el,
                        ...("isFolded" in el ? { isFolded: marker.isFolded ?? (el as GroupElementData).isFolded } : {}),
                        ...("renderMode" in el ? { renderMode: marker.renderMode || (el as ViewElementData).renderMode } : {}),
                      } as ElementData;
                    }
                    return el;
                  }),
                }));
              }
            }

            if (loadResult.annotations && typeof loadResult.annotations === "object" && "chains" in loadResult.annotations) {
              const annotations = loadResult.annotations as Record<string, unknown>;
              const chains = annotations.chains as Array<{ nodes?: Array<{ id: string; tag: string; attrs?: Record<string, unknown>; posX?: number; posY?: number }>; wires?: Array<{ id: string; fromId: string; fromPort: string; toId: string; toPort: string }> }> | undefined;
              const codeElements: ElementData[] = [];
              const codeWires: WireData[] = [];
              for (const chain of chains ?? []) {
                for (const node of chain.nodes || []) {
                  codeElements.push({
                    id: node.id,
                    tag: node.tag,
                    props: node.attrs || {},
                    position: { x: node.posX ?? 0, y: node.posY ?? 0 },
                    nodeType: "codeblock" as const,
                    source: "",
                    parentId: null,
                  } as ElementData);
                }
                for (const wire of chain.wires || []) {
                  codeWires.push({
                    id: wire.id,
                    fromId: wire.fromId,
                    fromPort: wire.fromPort,
                    toId: wire.toId,
                    toPort: wire.toPort,
                  } as WireData);
                }
              }
              if (codeElements.length > 0) {
                useCodeGraphStore.getState().setElementsAndWires(codeElements, codeWires);
              }
            }
          } else {
            const res = await api.project.readFile("src/main.omni");
            if (res.content) {
              usePoiesisStore.setState({ activeFileContent: res.content, activeFilePath: "src/main.omni" });
              const parseResult = parseOmniSource(res.content);
              if (parseResult.designElements) {
                const rootOrder = parseResult.designElements
                  .filter((e: ElementData) => !e.parentId)
                  .map((e: ElementData) => e.id);
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
