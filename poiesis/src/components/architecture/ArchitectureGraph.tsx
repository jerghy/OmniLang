import { useCallback, useEffect, useMemo, useState } from "react";
import { ReactFlow } from "@xyflow/react";
import type { ComponentType } from "react";
import type { NodeProps, EdgeProps, Edge } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../store";
import { buildBubbleGraph, buildDependencyEdges } from "../../services/architecture";
import type { OmniFileInfo } from "../../services/architecture";
import { useBubbleLayout } from "../../hooks/useBubbleLayout";
import { BubbleNode } from "./nodes/BubbleNode";
import { ChildBubbleNode } from "./nodes/ChildBubbleNode";
import { BubbleEdge } from "./edges/BubbleEdge";
import { BubbleContextMenu } from "./context-menu/BubbleContextMenu";
import { api } from "../../commands";
import { createLogger } from "../../lib/utils/logger";

const log = createLogger("ArchitectureGraph");

import type { BubbleNode as ArchBubbleNode, BubbleEdge as ArchBubbleEdge } from "../../core/types/architecture";

// Module-level cache: re-mounts within CACHE_TTL don't re-scan the project
interface CacheData {
  bubbles: ArchBubbleNode[];
  edges: ArchBubbleEdge[];
  projectDir: string;
  timestamp: number;
}
let cache: CacheData | null = null;
const CACHE_TTL = 60000; // 1 minute

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
  const projectDir = usePoiesisStore((s) => s.projectDir);
  const nodeMeta = usePoiesisStore((s) => s.nodeMeta);

  const [contextMenu, setContextMenu] = useState<{
    position: { x: number; y: number };
    bubbleId: string;
  } | null>(null);

  // Content hash of nodeMeta for cache invalidation when dependencies change
  const nodeMetaHash = useMemo(
    () => computeNodeMetaHash(nodeMeta),
    [nodeMeta],
  );

  useEffect(() => {
    let cancelled = false;
    async function load() {
      if (!projectDir) {
        if (!cancelled) {
          setBubblesAndEdges([], []);
          setLoading(false);
        }
        return;
      }

      // Check cache: skip if projectDir + nodeMeta unchanged and within TTL
      if (
        cache &&
        cache.projectDir === projectDir &&
        (cache as CacheData & { nodeMetaHash?: string }).nodeMetaHash === nodeMetaHash &&
        Date.now() - cache.timestamp < CACHE_TTL
      ) {
        if (!cancelled) {
          setBubblesAndEdges(cache.bubbles, cache.edges);
          setLoading(false);
        }
        return;
      }

      const omniFiles: OmniFileInfo[] = [];
      try {
        const result = await api.project.listDir(".");
        if (result.entries) {
          const scans = [result];
          const srcResult = await api.project.listDir("src");
          if (srcResult.entries) scans.push(srcResult);

          const omniEntries = scans.flatMap(scan => scan.entries.filter(e => e.ext === "omni"));
          const files = await Promise.all(omniEntries.map(entry =>
            api.project.readFile(entry.path).then(file => ({ entry, file }))
          ));
          for (const { entry, file } of files) {
            if (file.content && !file.error) {
              omniFiles.push({ path: entry.path, source: file.content });
            }
          }
        }
      } catch (e) {
        log.warn("Failed to read project files for architecture graph:", e);
      }

      const data = await buildBubbleGraph(projectDir, omniFiles);

      // Merge dependency edges from WASM semantic analysis
      const depEdges = buildDependencyEdges(nodeMeta, data.bubbles, data.edges.length);
      const allEdges = [...data.edges, ...depEdges];

      if (!cancelled) {
        cache = {
          bubbles: data.bubbles,
          edges: allEdges,
          projectDir,
          timestamp: Date.now(),
          nodeMetaHash,
        } as CacheData & { nodeMetaHash: string };
        setBubblesAndEdges(data.bubbles, allEdges);
        setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [projectDir, nodeMetaHash, setBubblesAndEdges, setLoading]);

  const positions = useBubbleLayout(bubbles, expandedIds);

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

  const rfEdges = useMemo(() => {
    const expanded = new Set(expandedIds);
    const bubblesById = new Map(bubbles.map((b) => [b.id, b]));
    const result: Edge[] = [];
    for (const e of edges) {
      const sourceBubble = bubblesById.get(e.source);
      if (expanded.has(e.source) || sourceBubble?.depth === 0) {
        result.push({
          id: e.id,
          type: "bubble",
          source: e.source,
          target: e.target,
          data: e as unknown as Record<string, unknown>,
        });
      }
    }
    return result;
  }, [edges, expandedIds, bubbles]);

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
          color: "var(--ol-text-secondary)",
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
      <BubbleContextMenu
        position={contextMenu?.position ?? null}
        bubbleId={contextMenu?.bubbleId ?? null}
        onClose={handleContextMenuClose}
      />
    </div>
  );
}

// ── Helpers ──

/** Content hash of nodeMeta for cache invalidation when dependencies change. */
function computeNodeMetaHash(nodeMeta: Record<string, unknown>): string {
  const depEntries = Object.entries(nodeMeta)
    .filter(([, meta]) => (meta as { dependencies?: unknown }).dependencies)
    .sort(([a], [b]) => a.localeCompare(b));
  if (depEntries.length === 0) return "empty";
  return JSON.stringify(depEntries);
}
