import { useEffect, useCallback } from "react";
import { useEditorStore } from "../../store/editor";
import { useCodeGraphStore } from "../../codegraph/store";
import { parseOmniSource } from "../../services/codegen";
import { api } from "../../services/api";
import { loadComponentDefinitionsFromServer, saveComponentDefinitionsToServer } from "../../services/project/project-service";
import { useComponentRegistry } from "../../store/componentRegistry";
import type { ElementData } from "../../core/types";

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
                (useCodeGraphStore.getState() as any).loadFromData(parseResult.codeTabs);
              }
            }

            if ((loadResult as any).annotations?.viewMarkers) {
              const markers = (loadResult as any).annotations.viewMarkers;
              useEditorStore.setState((prev) => ({
                elements: prev.elements.map((el) => {
                  const marker = markers.find((m: any) => m.elementId === el.id);
                  if (marker) {
                    return {
                      ...el,
                      isFolded: marker.isFolded ?? el.isFolded,
                      renderMode: marker.renderMode || el.renderMode,
                    };
                  }
                  return el;
                }),
              }));
            }

            if ((loadResult as any)?.annotations?.chains?.length) {
              const chains = (loadResult as any).annotations.chains;
              const codeElements: any[] = [];
              const codeWires: any[] = [];
              for (const chain of chains) {
                for (const node of chain.nodes || []) {
                  codeElements.push({
                    id: node.id,
                    tag: node.tag,
                    props: node.attrs || {},
                    position: { x: node.posX, y: node.posY },
                    nodeType: "codeblock",
                    children: [],
                    parentId: null,
                  });
                }
                for (const wire of chain.wires || []) {
                  codeWires.push({
                    id: wire.id,
                    fromId: wire.fromId,
                    fromPort: wire.fromPort,
                    toId: wire.toId,
                    toPort: wire.toPort,
                  });
                }
              }
              if (codeElements.length > 0) {
                (useCodeGraphStore.getState() as any).setElementsAndWires(codeElements, codeWires);
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
            useComponentRegistry.setState({ definitions: merged } as any);
          }
        } catch (e) {
          console.warn("[App] Failed to load component definitions:", e);
        }
      })();

      (async () => {
        try {
          const tomlResult: any = await api.annotation.loadEditorToml();
          if (tomlResult?.state?.open_files?.length > 0) {
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
