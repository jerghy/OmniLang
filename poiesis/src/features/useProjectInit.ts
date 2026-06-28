import { useEffect, useCallback } from "react";
import { useEditorStore } from "../../store/editor";
import { useCodeGraphStore } from "../../codegraph/store";
import { parseOmniSource } from "../../services/codegen";
import { api } from "../../services/api";
import { loadComponentDefinitionsFromServer, saveComponentDefinitionsToServer } from "../../services/project/component-service";
import { useComponentRegistry } from "../../store/componentRegistry";
import type { ElementData, WireData } from "../../core/types";
import type { ComponentDefinition } from "../../core/state/componentRegistry";

export function useProjectInit(setShowWelcome: (v: boolean) => void) {
  // Initial load
  useEffect(() => {
    (async () => {
      const proj = await api.project.getProject();
      if (proj.dir) useEditorStore.getState().setProjectDir(proj.dir);
      const list = await api.project.listDir(".");
      if (list.entries) useEditorStore.getState().loadFiles(list.entries);
      setShowWelcome(true);
    })();
  }, [setShowWelcome]);

  // Component def auto-save
  useEffect(() => {
    let saveTimer: ReturnType<typeof setTimeout> | null = null;
    const unsub = useComponentRegistry.subscribe((state) => {
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveTimer = null;
        saveComponentDefinitionsToServer(state.definitions).catch(() => {});
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
      api.compile.stopRun().catch(() => {});
      api.disconnectDebug();
    };
  }, []);

  const handleProjectReady = useCallback(
    (dir: string, name: string) => {
      useEditorStore.getState().setProjectDir(dir);
      useEditorStore.getState().newProject();
      useEditorStore.setState({ projectName: name, projectPath: dir });

      (async () => {
        try {
          const loadResult = await api.annotation.annotatedLoad("src/main.omni");
          if (loadResult.success && loadResult.source) {
            useEditorStore.setState({
              activeFileContent: loadResult.source,
              activeFilePath: "src/main.omni",
            });

            const parseResult = parseOmniSource(loadResult.source);
            if (parseResult.designElements) {
              const rootOrder = parseResult.designElements
                .filter((e: ElementData) => !e.parentId)
                .map((e: ElementData) => e.id);
              useEditorStore.setState({
                elements: parseResult.designElements,
                wires: parseResult.designWires || [],
                rootOrder,
              });
              if (parseResult.codeTabs?.length) {
                useCodeGraphStore.getState().loadFromData(parseResult.codeTabs);
              }
            }

            if (loadResult.annotations && typeof loadResult.annotations === "object" && "viewMarkers" in loadResult.annotations) {
              const annotations = loadResult.annotations as Record<string, unknown>;
              const markers = annotations.viewMarkers as Array<{ elementId: string; isFolded?: boolean; renderMode?: string }> | undefined;
              if (markers) {
                useEditorStore.setState((prev) => ({
                  elements: prev.elements.map((el) => {
                    const marker = markers.find((m) => m.elementId === el.id);
                    if (marker) {
                      return {
                        ...el,
                        isFolded: marker.isFolded ?? el.isFolded,
                        renderMode: marker.renderMode || el.renderMode,
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
                    children: [],
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
              useEditorStore.setState({ activeFileContent: res.content, activeFilePath: "src/main.omni" });
              const parseResult = parseOmniSource(res.content);
              if (parseResult.designElements) {
                const rootOrder = parseResult.designElements
                  .filter((e: ElementData) => !e.parentId)
                  .map((e: ElementData) => e.id);
                useEditorStore.setState({
                  elements: parseResult.designElements,
                  wires: parseResult.designWires || [],
                  rootOrder,
                });
              }
            }
          }
        } catch (e) {
          console.warn("[App] Failed to load main.omni:", e);
        }
      })();

      (async () => {
        try {
          const merged = await loadComponentDefinitionsFromServer();
          if (Object.keys(merged).length > 0) {
            useComponentRegistry.setState({ definitions: merged as Record<string, ComponentDefinition> });
          }
        } catch (e) {
          console.warn("[App] Failed to load component definitions:", e);
        }
      })();

      (async () => {
        try {
          const tomlResult = await api.annotation.loadEditorToml();
          if (tomlResult.state?.open_files?.length) {
            useEditorStore.setState({ openTabs: tomlResult.state.open_files });
          }
        } catch {
          /* non-critical */
        }
      })();

      setShowWelcome(false);
    },
    [setShowWelcome],
  );

  return { handleProjectReady };
}
