import { useCallback, useEffect, useMemo, useState } from "react";
import { ReactFlow } from "@xyflow/react";
import type { ComponentType } from "react";
import type { NodeProps, EdgeProps, Edge } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../store";
import { buildBubbleGraph } from "../../services/architecture";
import type { OmniFileInfo } from "../../services/architecture";
import { useBubbleLayout } from "../../hooks/useBubbleLayout";
import { BubbleNode } from "./nodes/BubbleNode";
import { ChildBubbleNode } from "./nodes/ChildBubbleNode";
import { BubbleEdge } from "./edges/BubbleEdge";
import { BubbleContextMenu } from "./context-menu/BubbleContextMenu";
import { api } from "../../commands";
import { createLogger } from "../../lib/utils/logger";

const log = createLogger("ArchitectureGraph");

// @xyflow/react v12 uses broad Node/Edge types; cast to our specific component types
const nodeTypes = {
  bubble: BubbleNode as unknown as ComponentType<NodeProps>,
  childBubble: ChildBubbleNode as unknown as ComponentType<NodeProps>,
};

const edgeTypes = {
  bubble: BubbleEdge as unknown as ComponentType<EdgeProps & { data: any; type: any }>,
};

export function ArchitectureGraph() {
  const [loading, setLoading] = useState(true);

  const bubbles = usePoiesisStore((s) => s.architecture.bubbles);
  const edges = usePoiesisStore((s) => s.architecture.edges);
  const expandedIds = usePoiesisStore(
    (s) => s.architecture.expandedBubbleIds
  );
  const setBubblesAndEdges = usePoiesisStore((s) => s.setBubblesAndEdges);
  const expandBubble = usePoiesisStore((s) => s.expandBubble);
  const openTab = usePoiesisStore((s) => s.openTab);

  // Context menu state
  const [contextMenu, setContextMenu] = useState<{
    position: { x: number; y: number };
    bubbleId: string;
  } | null>(null);

  // Load bubble data by scanning project .omni files
  useEffect(() => {
    let cancelled = false;
    async function load() {
      const projectDir = usePoiesisStore.getState().projectDir;
      if (!projectDir) {
        if (!cancelled) {
          setBubblesAndEdges([], []);
          setLoading(false);
        }
        return;
      }

      // Scan project for .omni files
      const omniFiles: OmniFileInfo[] = [];
      try {
        const result = await api.project.listDir(".");
        if (result.entries) {
          // Also scan src/ subdirectory
          const scans = [result];
          const srcResult = await api.project.listDir("src");
          if (srcResult.entries) scans.push(srcResult);

          for (const scan of scans) {
            for (const entry of scan.entries) {
              if (entry.ext === "omni") {
                const file = await api.project.readFile(entry.path);
                if (file.content && !file.error) {
                  omniFiles.push({ path: entry.path, source: file.content });
                }
              }
            }
          }
        }
      } catch (e) {
        log.warn("Failed to read project files for architecture graph:", e);
      }

      const data = await buildBubbleGraph(projectDir, omniFiles);
      if (!cancelled) {
        setBubblesAndEdges(data.bubbles, data.edges);
        setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [setBubblesAndEdges]);

  // Compute positions via layout hook
  const positions = useBubbleLayout(bubbles, expandedIds);

  // Filter visible bubbles and map to ReactFlow nodes
  const rfNodes = useMemo(() => {
    const expanded = new Set(expandedIds);
    const visible = bubbles.filter(
      (b) => b.depth === 0 || (b.parentId && expanded.has(b.parentId))
    );

    return visible.map((b) => {
      const pos = positions.get(b.id) || b.position;
      return {
        id: b.id,
        type: b.depth === 0 ? "bubble" : "childBubble",
        position: pos,
        data: {
          pid: b.pid,
          type: b.type,
          label: b.label,
          depth: b.depth,
          children: b.children,
          expanded: expanded.has(b.id),
          isLeaf: b.isLeaf,
          parentId: b.parentId,
          layer: b.layer,
          childCount: b.children.length,
        },
      };
    });
  }, [bubbles, expandedIds, positions]);

  // Filter visible edges
  const rfEdges = useMemo(() => {
    const expanded = new Set(expandedIds);
    return edges
      .filter(
        (e) =>
          expanded.has(e.source) ||
          bubbles.find((b) => b.id === e.source)?.depth === 0
      )
      .map((e): Edge => ({
        id: e.id,
        type: "bubble",
        source: e.source,
        target: e.target,
        data: e as unknown as Record<string, unknown>,
      }));
  }, [edges, expandedIds, bubbles]);

  // Double-click: expand bubble or navigate to design page
  const handleNodeDoubleClick = useCallback(
    (_event: React.MouseEvent, node: { id: string }) => {
      const bubble = bubbles.find((b) => b.id === node.id);
      if (!bubble) return;

      if (bubble.children.length > 0) {
        expandBubble(bubble.id);
      } else if (bubble.isLeaf && bubble.pid) {
        const entry = usePoiesisStore.getState().pidIndex[bubble.pid];
        if (entry) {
          openTab(entry.sourceFile, bubble.label);
        }
      }
    },
    [bubbles, expandBubble, openTab]
  );

  // Context menu
  const handleNodeContextMenu = useCallback(
    (event: React.MouseEvent, node: { id: string }) => {
      event.preventDefault();
      setContextMenu({
        position: { x: event.clientX, y: event.clientY },
        bubbleId: node.id,
      });
    },
    []
  );

  const handleContextMenuClose = useCallback(() => {
    setContextMenu(null);
  }, []);

  // Handle pane click to close context menu
  const handlePaneClick = useCallback(() => {
    setContextMenu(null);
  }, []);

  if (loading) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          color: "#6b7280",
          fontSize: 14,
        }}
      >
        加载关系图谱...
      </div>
    );
  }

  return (
    <div style={{ width: "100%", height: "100%", position: "relative" }}>
      <ReactFlow
        nodes={rfNodes}
        edges={rfEdges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        fitView
        minZoom={0.1}
        maxZoom={2}
        onNodeDoubleClick={handleNodeDoubleClick}
        onNodeContextMenu={handleNodeContextMenu}
        onPaneClick={handlePaneClick}
      />
      {contextMenu && (
        <BubbleContextMenu
          position={contextMenu.position}
          bubbleId={contextMenu.bubbleId}
          onClose={handleContextMenuClose}
        />
      )}
    </div>
  );
}
