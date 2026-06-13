/** Rete.js v2 wire canvas — data-flow code blocks with typed ports + wire labels. */

import { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { NodeEditor, ClassicPreset, getUID } from "rete";
import type { GetSchemes } from "rete";
import { AreaPlugin } from "rete-area-plugin";
import { ReactPlugin, Presets } from "rete-react-plugin";
import type { ReactArea2D } from "rete-react-plugin";
import { ConnectionPlugin } from "rete-connection-plugin";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { BLOCK_TYPES, createBlock } from "../panels/BlockPalette";
import { COMPONENT_PROTOCOL } from "../panels/ComponentPalette";
import { EventCreationPanel, type EventSource, type CreationAction } from "../panels/EventCreationPanel";
import { parseFnSignature } from "../panels/FunctionLibrary";
import { isTypeCompatible, typeMismatchMessage, TYPE_COLORS, EVENT_PORT_COLOR } from "./socketTypes";
import { AddWire } from "../commands/AddWire";
import { RemoveWire } from "../commands/RemoveWire";
import { AddElement } from "../commands/AddElement";
import { SetElementsAndWires } from "../commands/SetElementsAndWires";
import type { CommandContext } from "../commands/types";
import { AutoArrangePlugin, Presets as ArrangePresets } from "rete-auto-arrange-plugin";

// ── Types ────────────────────────────────────────

type Node_ = ClassicPreset.Node;
type Conn = ClassicPreset.Connection<Node_, Node_>;
type Schemes = GetSchemes<Node_, Conn>;
type AreaExtra = ReactArea2D<Schemes>;

// ── Socket ───────────────────────────────────────

const ANY = new ClassicPreset.Socket("any");

// ── Color/icon maps ──────────────────────────────

const BLOCK_ICONS: Record<string, string> = {
  store: "🗄️", signal: "⚡", computed: "🔄", action: "▶️",
  fn: "📦", "fn-call": "📞",
};

const BLOCK_COLORS: Record<string, { bg: string; border: string }> = {
  store:    { bg: "#F5F3FF", border: "#7C3AED" },
  signal:   { bg: "#FEF3C7", border: "#D97706" },
  computed: { bg: "#DBEAFE", border: "#2563EB" },
  action:   { bg: "#FCE7F3", border: "#DB2777" },
  fn:       { bg: "#ECFDF5", border: "#059669" },
  "fn-call":{ bg: "#F0FDF4", border: "#10B981" },
};

// ── Props ────────────────────────────────────────

interface FlowCanvasProps {
  showViews?: boolean;
  highlightedWireId?: string | null;
  onNodeDoubleClick?: (nodeId: string, nodeData: ElementData) => void;
  ctx: CommandContext;
  /** Increment to trigger auto-layout from outside (Feature 2). */
  layoutTrigger?: number;
}

// ── Helpers ──────────────────────────────────────

function getPorts(el: ElementData): { name: string; dir: "in" | "out"; type: string; label?: string }[] {
  const isC = el.nodeType === "codeblock";
  if (isC && el.ports?.length) return el.ports.map(p => ({ name: p.name, dir: p.direction as "in" | "out", type: p.type, label: p.label }));
  if (isC) {
    const ports: { name: string; dir: "in" | "out"; type: string; label?: string }[] = [{ name: "value", dir: "out", type: "any", label: "输出" }];
    if (el.tag !== "store") ports.unshift({ name: "call", dir: "in", type: "any", label: "调用" });
    return ports;
  }
  // View elements — generate event ports from Component Protocol metadata
  const ports: { name: string; dir: "in" | "out"; type: string; label?: string }[] = [
    { name: "in", dir: "in", type: "any", label: "输入" },
    { name: "out", dir: "out", type: "any", label: "输出" },
  ];
  const meta = COMPONENT_PROTOCOL[el.tag];
  if (meta?.events) {
    for (const [eventName, eventMeta] of Object.entries(meta.events)) {
      ports.push({
        name: eventMeta.id,                  // stable UUID for wire storage
        dir: "out",
        type: eventMeta.payload || "void",   // type for socket compatibility
        label: `${eventName}: ${eventMeta.payload || "无"}`,
      });
    }
  }
  return ports;
}

function parseFnFromString(paramsStr: string): Array<{ name: string; type: string }> {
  if (!paramsStr.trim()) return [];
  const result: Array<{ name: string; type: string }> = [];
  for (const part of paramsStr.split(",")) {
    const [pName, pType] = part.split(":").map(s => s.trim());
    if (pName && pType) result.push({ name: pName, type: pType });
  }
  return result;
}

// ── Component ────────────────────────────────────

export function FlowCanvas({ showViews = true, highlightedWireId = null, onNodeDoubleClick, ctx, layoutTrigger = 0 }: FlowCanvasProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<NodeEditor<Schemes> | null>(null);
  const areaRef = useRef<AreaPlugin<Schemes, AreaExtra> | null>(null);
  const autoArrangeRef = useRef<AutoArrangePlugin<Schemes> | null>(null);
  const nodeMapRef = useRef<Map<string, { reteNode: Node_; el: ElementData }>>(new Map());
  const [, forceTick] = useState(0);
  const [ready, setReady] = useState(false);
  const [contextMenu, setContextMenu] = useState<{
    x: number; y: number;
    nodeId?: string; tag?: string; events?: Array<[string, { id: string; description: string; payload: string | null }]>;
  } | null>(null);
  const [eventSource, setEventSource] = useState<EventSource | null>(null);

  const elements = useEditorStore((s) => s.elements);
  const wires = useEditorStore((s) => s.wires);
  const undoExec = useUndoStore((s) => s.execute);

  // ── Init Rete ────────────────────────────────────

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const editor = new NodeEditor<Schemes>();
    const area = new AreaPlugin<Schemes, AreaExtra>(container);
    const render = new ReactPlugin<Schemes, AreaExtra>({ createRoot });
    const connection = new ConnectionPlugin<Schemes>();

    render.addPreset(Presets.classic.setup());

    editor.use(area);
    area.use(render);
    area.use(connection);

    // Auto-arrange plugin (Rete official, elkjs-based layout)
    const autoArrange = new AutoArrangePlugin<Schemes>();
    autoArrange.addPreset(ArrangePresets.classic.setup());
    area.use(autoArrange);
    autoArrangeRef.current = autoArrange;

    // Attach dblclick handlers to Rete node DOM elements via MutationObserver
    const attachedNodes = new Set<string>();
    const attachDblClick = () => {
      if (!onNodeDoubleClick) return;
      const nodeEls = container.querySelectorAll('[data-testid="node"]');
      nodeEls.forEach((el) => {
        // Find the Rete node that matches this DOM element
        // Rete nodes don't have a direct ID attribute, so we match by content
        for (const [elId, entry] of nodeMapRef.current) {
          if (attachedNodes.has(elId)) continue;
          const name = (entry.el.props.name || entry.el.props.label || entry.el.tag) as string;
          if (el.textContent?.includes(name)) {
            attachedNodes.add(elId);
            el.addEventListener("dblclick", (e) => {
              e.stopPropagation();
              if (entry.el.nodeType === "codeblock") {
                onNodeDoubleClick(entry.el.id, entry.el);
              }
            });
            break;
          }
        }
      });
    };
    // Observe for new nodes
    const observer = new MutationObserver(() => attachDblClick());
    observer.observe(container, { childList: true, subtree: true });
    // Initial attach after nodes render
    setTimeout(attachDblClick, 300);

    // Pipe: connection creation → add wire to store
    const cmdCtx = ctx; // capture CommandContext before pipe shadowing
    editor.addPipe((pipeCtx) => {
      if (pipeCtx.type === "connectioncreate") {
        const data = pipeCtx.data as Conn & { source: string; target: string; sourceOutput?: string; targetInput?: string };
        const fromEntry = [...nodeMapRef.current.values()].find(v => v.reteNode.id === data.source);
        const toEntry = [...nodeMapRef.current.values()].find(v => v.reteNode.id === data.target);
        if (fromEntry && toEntry) {
          const fromPort = data.sourceOutput || "out";
          const toPort = data.targetInput || "in";

          // ── Type validation ──
          const sourcePorts = getPorts(fromEntry.el);
          const targetPorts = getPorts(toEntry.el);
          const sourcePort = sourcePorts.find(p => p.name === fromPort && p.dir === "out");
          const targetPort = targetPorts.find(p => p.name === toPort && p.dir === "in");
          if (sourcePort && targetPort) {
            if (!isTypeCompatible(sourcePort.type, targetPort.type)) {
              alert(typeMismatchMessage(sourcePort.type, targetPort.type));
              return; // reject the connection
            }
          }

          // Prevent duplicate wires
          const exists = useEditorStore.getState().wires.some(
            w => w.fromId === fromEntry.el.id && w.fromPort === fromPort && w.toId === toEntry.el.id && w.toPort === toPort
          );
          if (!exists) {
            const wireData = {
              id: `w-${getUID()}`, fromId: fromEntry.el.id, fromPort,
              toId: toEntry.el.id, toPort,
            };
            useUndoStore.getState().execute(new AddWire(wireData), ctx);
          }
        }
      }
      if (pipeCtx.type === "connectionremove") {
        const data = pipeCtx.data as Conn;
        const store = useEditorStore.getState();
        const wire = store.wires.find(w => {
          const fe = [...nodeMapRef.current.values()].find(v => v.reteNode.id === data.source);
          const te = [...nodeMapRef.current.values()].find(v => v.reteNode.id === data.target);
          return fe && te && w.fromId === fe.el.id && w.toId === te.el.id;
        });
        if (wire) useUndoStore.getState().execute(new RemoveWire(wire.id), cmdCtx);
      }
      return pipeCtx;
    });

    // Right-click handler — different menus for empty canvas vs view nodes
    const onCtx = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const nodeEl = target.closest('[data-testid="node"]');
      const connEl = target.closest('[data-testid="connection"]');
      if (connEl) return; // skip connections
      e.preventDefault();
      const rect = container.getBoundingClientRect();
      const canvasX = e.clientX - rect.left;
      const canvasY = e.clientY - rect.top;

      if (nodeEl) {
        // Right-click on a node → check if it's a view element with events
        for (const [elId, entry] of nodeMapRef.current) {
          const name = (entry.el.props.name || entry.el.props.label || entry.el.tag) as string;
          if (nodeEl.textContent?.includes(name) && entry.el.nodeType === "view") {
            const meta = COMPONENT_PROTOCOL[entry.el.tag];
            if (meta?.events) {
              const eventList = Object.entries(meta.events);
              if (eventList.length === 1) {
                // Single event → open creation panel directly
                const [evName, evMeta] = eventList[0];
                setEventSource({
                  elementId: entry.el.id,
                  componentTag: entry.el.tag,
                  eventName: evName,
                  eventMeta: evMeta,
                  x: canvasX, y: canvasY,
                });
              } else if (eventList.length > 1) {
                // Multiple events → show event picker menu first
                setContextMenu({
                  x: canvasX, y: canvasY,
                  nodeId: entry.el.id,
                  tag: entry.el.tag,
                  events: eventList,
                });
              }
            }
            break;
          }
        }
        return;
      }
      // Right-click on empty canvas → add-block menu
      setContextMenu({ x: canvasX, y: canvasY });
    };
    container.addEventListener("contextmenu", onCtx);

    editorRef.current = editor;
    areaRef.current = area;
    forceTick(1);
    // Defer ready to let ReactPlugin fully initialize before sync effect adds nodes
    setTimeout(() => setReady(true), 50);

    return () => {
      container.removeEventListener("contextmenu", onCtx);
      observer.disconnect();
      editor.clear();
      area.destroy();
    };
  }, []);

  // ── Fn-call node creation (from function library drop) ──

  const createFnCallNode = (
    fnData: { fnId: string; fnName: string; params: string; returnType: string },
    cx: number, cy: number,
  ) => {
    const pports = parseFnFromString(fnData.params);
    const id = `cb-fncall-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const ports: Array<{ name: string; direction: "in" | "out"; type: string; label?: string }> = [];
    for (const p of pports) {
      ports.push({ name: p.name, direction: "in", type: p.type, label: `${p.name}:${p.type}` });
    }
    ports.push({ name: "result", direction: "out", type: fnData.returnType, label: `返回: ${fnData.returnType}` });

    const el: ElementData = {
      id, nodeType: "codeblock", tag: "fn-call",
      props: { name: fnData.fnName, fnId: fnData.fnId },
      children: [], parentId: null,
      ports,
      isFolded: false,
      source: `// 📞 调用节点：调用函数库中的 ${fnData.fnName}(${fnData.params}) → ${fnData.returnType}`,
    };

    ctx.addElement(el);
    // Position node after sync loop picks it up
    setTimeout(() => {
      const entry = nodeMapRef.current.get(id);
      const area = areaRef.current;
      if (entry && area) {
        area.translate(entry.reteNode.id, { x: cx, y: cy });
        forceTick(t => t + 1);
      }
    }, 300);
  };

  // ── Add block handler ────────────────────────────

  const handleAddBlock = (tag: string) => {
    const el = createBlock(tag);
    useUndoStore.getState().execute(new AddElement(el), ctx);
    setContextMenu(null);
  };

  // ── Event creation handler ─────────────────────────

  const handleEventCreation = (action: CreationAction) => {
    if (!eventSource) return;
    const store = useEditorStore.getState();
    const undoStore = useUndoStore.getState();

    if (action.type === "new-code-block") {
      const blockName = action.blockName || `handle${eventSource.eventName}`;
      const payloadType = eventSource.eventMeta.payload || "void";
      const source = `// 💻 代码块：由事件 ${eventSource.eventName} 触发
action ${blockName}(event: ${payloadType}) {
    // 在此编写事件处理逻辑

}`;
      const blockId = `cb-action-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
      const el: ElementData = {
        id: blockId, nodeType: "codeblock", tag: "action",
        props: { name: blockName },
        children: [], parentId: null,
        ports: [
          { name: "call", direction: "in", type: payloadType, label: `${eventSource.eventName}: ${payloadType}` },
          { name: "result", direction: "out", type: "any", label: "返回结果" },
        ],
        isFolded: false,
        source,
      };
      undoStore.execute(new AddElement(el), ctx);

      // Create wire from event port to new block
      const wireData = {
        id: `w-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
        fromId: eventSource.elementId,
        fromPort: eventSource.eventMeta.id,
        toId: blockId,
        toPort: "call",
      };
      setTimeout(() => undoStore.execute(new AddWire(wireData), ctx), 150);

      // Optionally create a signal
      if (action.createSignal && action.signalName) {
        const sigId = `cb-signal-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
        const sigSource = `// ⚡ 状态：由事件 ${eventSource.eventName} 更新
signal ${action.signalName}: ${payloadType} = ${defaultValueForType(payloadType)};

// 值由事件更新
set(newValue) { value = newValue; }`;
        const sigEl: ElementData = {
          id: sigId, nodeType: "codeblock", tag: "signal",
          props: { name: action.signalName, typeAnnotation: payloadType },
          children: [], parentId: null,
          ports: [
            { name: "value", direction: "out", type: payloadType, label: `${action.signalName}:${payloadType}` },
            { name: "set", direction: "in", type: payloadType, label: `设置 ${action.signalName}` },
          ],
          isFolded: false,
          source: sigSource,
        };
        setTimeout(() => undoStore.execute(new AddElement(sigEl), ctx), 100);

        // Wire block result → signal set
        const sigWire = {
          id: `w-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
          fromId: blockId, fromPort: "result",
          toId: sigId, toPort: "set",
        };
        setTimeout(() => undoStore.execute(new AddWire(sigWire), ctx), 300);
      }
    }

    setEventSource(null);
    setContextMenu(null);
  };

  // ── Close context menu on any click ──────────────

  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  // ── Sync state → Rete ────────────────────────────

  useEffect(() => {
    const editor = editorRef.current;
    const area = areaRef.current;
    if (!editor || !area) return;
    if (!ready) return;

    const nodeMap = nodeMapRef.current;
    const wiredIds = new Set<string>();
    if (!showViews) for (const w of wires) { wiredIds.add(w.fromId); wiredIds.add(w.toId); }

    const visible = new Set<string>();
    const toAdd: ElementData[] = [];
    for (const el of elements) {
      if ((el.nodeType === "codeblock" && el.tag !== "fn") || showViews || wiredIds.has(el.id)) {
        visible.add(el.id);
        const existing = nodeMap.get(el.id);
        if (!existing) {
          toAdd.push(el);
        } else {
          // Detect port changes (e.g. after CodeEditorPopup save changed type)
          const latestPorts = JSON.stringify(el.ports);
          const cachedPorts = JSON.stringify(existing.el.ports);
          if (latestPorts !== cachedPorts) {
            // Ports changed → remove old Rete node → re-create
            editor.removeNode(existing.reteNode.id).catch(() => {});
            nodeMap.delete(el.id);
            toAdd.push(el);
          }
        }
      }
    }

    // Remove stale nodes
    for (const [elId, entry] of nodeMap) {
      if (!visible.has(elId)) { editor.removeNode(entry.reteNode.id).catch(() => {}); nodeMap.delete(elId); }
    }

    // Remove stale connections
    const existingConns = editor.getConnections();
    for (const c of existingConns) {
      const fromEntry = [...nodeMap.values()].find(v => v.reteNode.id === c.source);
      const toEntry = [...nodeMap.values()].find(v => v.reteNode.id === c.target);
      const hasWire = wires.some(w => w.fromId === fromEntry?.el.id && w.toId === toEntry?.el.id && w.fromPort === c.sourceOutput && w.toPort === c.targetInput);
      if (!hasWire) editor.removeConnection(c.id).catch(() => {});
    }

    let codeIdx = 0, viewIdx = 0;
    const addNodePromises: Promise<void>[] = [];
    for (const el of toAdd) {
      const name = (el.props.name || el.props.label || el.tag) as string;
      const icon = BLOCK_ICONS[el.tag] || "📦";
      const isC = el.nodeType === "codeblock";
      const colors = BLOCK_COLORS[el.tag] || { bg: "#F3F4F6", border: "#9CA3AF" };

      const node = new ClassicPreset.Node(name);

      // Custom control showing block info + runtime value
      const valueStr = el.outputValue !== undefined ? ` → ${JSON.stringify(el.outputValue)}` : "";
      const typeAnn = (el.props.typeAnnotation as string) || "";
      const labelText = `${icon} ${name}${typeAnn ? ` : ${typeAnn}` : ""}${valueStr}`;
      node.addControl("label", new ClassicPreset.InputControl("text", { initial: labelText, readonly: true }));

      // Add typed ports
      const ports = getPorts(el);
      for (const p of ports) {
        const socket = new ClassicPreset.Socket(p.type);
        const displayLabel = p.label || p.name;
        if (p.dir === "in") node.addInput(p.name, new ClassicPreset.Input(socket, displayLabel));
        else node.addOutput(p.name, new ClassicPreset.Output(socket, displayLabel));
      }

      // Style the node with block-type color
      // Use el.position if set (by auto-layout or loaded canvas state), otherwise default offset
      const defaultPos = { x: isC ? 50 : 550, y: isC ? 50 + codeIdx * 150 : 50 + viewIdx * 150 };
      const pos = el.position || defaultPos;
      const p = editor.addNode(node).then(() => {
        area.translate(node.id, pos);
        forceTick(t => t + 1);
      });
      addNodePromises.push(p);

      nodeMap.set(el.id, { reteNode: node, el });
      if (isC) codeIdx++; else viewIdx++;
    }

    // Update existing node labels (for runtime value changes)
    for (const [elId, entry] of nodeMap) {
      const latest = elements.find(e => e.id === elId);
      if (!latest) continue;

      if (latest.outputValue !== entry.el.outputValue) {
        const name = (latest.props.name || latest.props.label || latest.tag) as string;
        const icon = BLOCK_ICONS[latest.tag] || "📦";
        const valueStr = latest.outputValue !== undefined ? ` → ${JSON.stringify(latest.outputValue)}` : "";
        const typeAnn = (latest.props.typeAnnotation as string) || "";
        const ctrl = entry.reteNode.controls["label"] as ClassicPreset.InputControl<"text">;
        if (ctrl) ctrl.setValue(`${icon} ${name}${typeAnn ? ` : ${typeAnn}` : ""}${valueStr}`);
        entry.el = latest; // sync stored el ref
      }

      // Sync positions for existing nodes (from auto-layout or loaded canvas state)
      if (latest.position && (
        !entry.el.position ||
        latest.position.x !== entry.el.position?.x ||
        latest.position.y !== entry.el.position?.y
      )) {
        area.translate(entry.reteNode.id, latest.position);
        entry.el = latest; // sync stored el ref
      }
    }

    // Sync connections — wait for all nodes to be registered first
    if (wires.length > 0) {
      Promise.all(addNodePromises).then(() => {
        const latestConns = editor.getConnections();
        const existingKeys = new Set(latestConns.map(c => `${c.source}:${c.sourceOutput}:${c.target}:${c.targetInput}`));
        for (const w of wires) {
          const key = `${w.fromId}:${w.fromPort}:${w.toId}:${w.toPort}`;
          if (existingKeys.has(key)) continue;
          const fromEntry = nodeMap.get(w.fromId);
          const toEntry = nodeMap.get(w.toId);
          if (!fromEntry || !toEntry) continue;

          const fromNode = fromEntry.reteNode;
          const toNode = toEntry.reteNode;
          editor.addConnection(new ClassicPreset.Connection(fromNode, w.fromPort, toNode, w.toPort)).catch(() => {});

          // Evaluate initial data flow for this connection (deferred to avoid render loop)
          const fromEl = elements.find(e => e.id === w.fromId);
          if (fromEl?.outputValue !== undefined) {
            setTimeout(() => evaluateDataFlow(w.fromId, w.fromPort, w.toId, w.toPort), 0);
          }
        }
      });
    }

    // Highlight wire
    if (highlightedWireId) {
      setTimeout(() => {
        const hw = wires.find(w => w.id === highlightedWireId);
        if (hw) {
          const fe = nodeMap.get(hw.fromId);
          const te = nodeMap.get(hw.toId);
          if (fe && te) {
            area.translate(fe.reteNode.id, { x: 250, y: 150 });
            area.translate(te.reteNode.id, { x: 500, y: 150 });
            forceTick(t => t + 1);
          }
        }
      }, 200);
    }
  }, [elements, wires, showViews, ready]);

  // ── Auto-layout (Feature 2) ─────────────────────
  // Uses Rete's official auto-arrange plugin (elkjs-based, replaces custom dagre).
  // elkjs produces a proper left-to-right hierarchical layout, handling
  // port positions, node dimensions, and edge routing correctly.

  const elementsRef = useRef(elements);
  elementsRef.current = elements;
  const wiresRef = useRef(wires);
  wiresRef.current = wires;
  const ctxRef = useRef(ctx);
  ctxRef.current = ctx;

  useEffect(() => {
    const trigger = layoutTrigger ?? 0;
    if (trigger <= 0) return;

    const area = areaRef.current;
    const editor = editorRef.current;
    const autoArrange = autoArrangeRef.current;
    if (!area || !editor || !autoArrange) return;

    // Wait for any pending node additions to settle
    const timer = setTimeout(async () => {
      try {
        // Read ACTUAL rendered dimensions from the DOM so elkjs gets correct sizes
        const allNodes = editor.getNodes();
        for (const node of allNodes) {
          if (!node.width || node.width < 10 || !node.height || node.height < 10) {
            const nodeView = area.nodeViews.get(node.id);
            if (nodeView) {
              const rect = nodeView.element.getBoundingClientRect();
              node.width = rect.width || 200;
              node.height = rect.height || 170;
            } else {
              node.width = node.width || 200;
              node.height = node.height || 170;
            }
          }
        }

        // Run elkjs layout WITHOUT the StandardApplier (which would resize nodes).
        // Instead, we manually translate nodes to preserve their React-rendered sizes.
        const layoutResult = await autoArrange.layout();

        // Manually apply positions only (no resize!)
        const applyPositions = (elkNodes: any[], offsetX = 0, offsetY = 0) => {
          for (const n of elkNodes) {
            if (typeof n.x === 'number' && typeof n.y === 'number') {
              const nodeView = area.nodeViews.get(n.id);
              if (nodeView) {
                nodeView.translate(offsetX + n.x, offsetY + n.y);
              }
            }
            if (n.children?.length) {
              applyPositions(n.children, offsetX + (n.x || 0), offsetY + (n.y || 0));
            }
          }
        };
        applyPositions(layoutResult.result.children || [layoutResult.result]);

        // Read new positions from Rete node views after layout
        const newPositions: Record<string, { x: number; y: number }> = {};
        for (const [elId, entry] of nodeMapRef.current) {
          const nodeView = area.nodeViews.get(entry.reteNode.id);
          if (nodeView) {
            newPositions[elId] = { x: nodeView.position.x, y: nodeView.position.y };
          }
        }

        // Bake positions into elements (immutable: spread each element)
        const currentElements = elementsRef.current;
        const currentWires = wiresRef.current;
        const positionedElements = currentElements.map(el => ({
          ...el,
          position: newPositions[el.id] || el.position,
        }));

        // Execute via Command pattern for undo/redo support
        const cmd = new SetElementsAndWires(
          positionedElements,
          currentWires,
          "📐 自动布局",
        );

        const currentCtx = ctxRef.current;
        useUndoStore.getState().execute(cmd, currentCtx);

        forceTick(t => t + 1);
      } catch (err) {
        console.error("Auto-arrange failed:", err);
      }
    }, 300); // 300ms delay to let Rete finish adding new nodes

    return () => clearTimeout(timer);
  }, [layoutTrigger]); // ONLY depend on layoutTrigger, not elements/wires

  // ── Data flow evaluation ─────────────────────────

  function evaluateDataFlow(fromId: string, _fromPort: string, toId: string, _toPort: string) {
    const store = useEditorStore.getState();
    const fromEl = store.elements.find(e => e.id === fromId);
    const toEl = store.elements.find(e => e.id === toId);
    if (!fromEl || !toEl) return;

    // If source has an output value, propagate it to target
    if (fromEl.outputValue !== undefined) {
      store.setBlockValue(toId, fromEl.outputValue);
    }
    // If source is a view element with props, propagate relevant data
    else if (fromEl.nodeType === "view") {
      const props = fromEl.props;
      // Pass the element's key props as data
      const data = {
        id: fromEl.id,
        tag: fromEl.tag,
        value: props.value,
        label: props.label,
        checked: props.checked,
      };
      store.setBlockValue(toId, data);
    }
  }

  // ── Render ───────────────────────────────────────

  // Collect existing code blocks for "bind existing" dropdown
  const existingCodeBlocks = elements
    .filter(e => e.nodeType === "codeblock" && e.tag === "action")
    .map(e => ({ id: e.id, name: (e.props.name as string) || e.tag }));

  // Collect existing store actions (placeholder — will be expanded in Phase C)
  const existingStoreActions: Array<{ storeId: string; storeName: string; actionName: string }> = [];

  return (
    <div ref={containerRef} style={{ width: "100%", height: "100%", minHeight: 400, overflow: "hidden", position: "relative" }}
      onDragOver={(e) => {
        if (e.dataTransfer.types.includes("application/omni-state") ||
            e.dataTransfer.types.includes("application/omni-fn")) {
          e.preventDefault();
          e.dataTransfer.dropEffect = "copy";
        }
      }}
      onDrop={(e) => {
        const stateRaw = e.dataTransfer.getData("application/omni-state");
        const fnRaw = e.dataTransfer.getData("application/omni-fn");
        if (!stateRaw && !fnRaw) return;
        e.preventDefault();
        const rect = containerRef.current?.getBoundingClientRect();
        const area = areaRef.current;
        if (!rect || !area) return;
        const x = e.clientX - rect.left;
        const y = e.clientY - rect.top;

        if (stateRaw) {
          try {
            const dropped = JSON.parse(stateRaw) as { id: string; kind: string; name: string; typeAnnotation: string };
            const el = useEditorStore.getState().elements.find(e => e.id === dropped.id);
            if (el) {
              setTimeout(() => {
                const entry = nodeMapRef.current.get(dropped.id);
                if (entry) {
                  area.translate(entry.reteNode.id, { x, y });
                  forceTick(t => t + 1);
                }
              }, 200);
            }
          } catch { /* ignore */ }
        }

        if (fnRaw) {
          try {
            const dropped = JSON.parse(fnRaw) as { fnId: string; fnName: string; params: string; returnType: string };
            createFnCallNode(dropped, x, y);
          } catch { /* ignore */ }
        }
      }}
    >
      {/* Context menu — add-block (empty canvas) or event-picker (multi-event node) */}
      {contextMenu && !contextMenu.events && (
        <div style={{
          position: "absolute", left: contextMenu.x, top: contextMenu.y, zIndex: 1000,
          background: "#fff", borderRadius: 8, boxShadow: "0 4px 16px rgba(0,0,0,0.15)", border: "1px solid #E5E7EB",
          minWidth: 220, padding: "6px 0", fontSize: 12,
        }}>
          <div style={{ padding: "4px 12px", color: "#9CA3AF", fontSize: 10, textTransform: "uppercase", fontWeight: 600 }}>添加代码块</div>
          {BLOCK_TYPES.map(b => (
            <div
              key={b.tag}
              onClick={() => handleAddBlock(b.tag)}
              style={{
                padding: "7px 12px", cursor: "pointer", display: "flex", alignItems: "center", gap: 8,
                transition: "background 0.1s",
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "#F3F4F6")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              <span style={{ fontSize: 16 }}>{b.icon}</span>
              <div style={{ flex: 1 }}>
                <div style={{ fontWeight: 600, color: "#374151" }}>{b.label}</div>
                <div style={{ fontSize: 10, color: "#9CA3AF" }}>
                  {b.desc}
                </div>
                <div style={{ fontSize: 9, color: "#9CA3AF", marginTop: 2, display: "flex", gap: 6 }}>
                  <span title="输出端口">出: {b.ports.filter(p => p.direction === "out").map(p => `${p.label}(${p.type})`).join(", ") || "—"}</span>
                  <span title="输入端口">入: {b.ports.filter(p => p.direction === "in").map(p => `${p.label}(${p.type})`).join(", ") || "—"}</span>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Context menu — event picker (multi-event view node) */}
      {contextMenu && contextMenu.events && (
        <div style={{
          position: "absolute", left: contextMenu.x, top: contextMenu.y, zIndex: 1000,
          background: "#fff", borderRadius: 8, boxShadow: "0 4px 16px rgba(0,0,0,0.15)", border: "1px solid #E5E7EB",
          minWidth: 200, padding: "6px 0", fontSize: 12,
        }}>
          <div style={{ padding: "4px 12px", color: "#9CA3AF", fontSize: 10, textTransform: "uppercase", fontWeight: 600 }}>
            🔌 {contextMenu.tag} 事件
          </div>
          {contextMenu.events.map(([evName, evMeta]) => (
            <div
              key={evMeta.id}
              onClick={() => {
                setEventSource({
                  elementId: contextMenu.nodeId!,
                  componentTag: contextMenu.tag!,
                  eventName: evName,
                  eventMeta: evMeta,
                  x: contextMenu.x, y: contextMenu.y,
                });
                setContextMenu(null);
              }}
              style={{
                padding: "7px 12px", cursor: "pointer", display: "flex", alignItems: "center", gap: 8,
                transition: "background 0.1s",
              }}
              onMouseEnter={e => (e.currentTarget.style.background = "#F3F4F6")}
              onMouseLeave={e => (e.currentTarget.style.background = "transparent")}
            >
              <span style={{ color: EVENT_PORT_COLOR, fontSize: 14 }}>●</span>
              <div>
                <div style={{ fontWeight: 600, color: "#374151" }}>{evName}</div>
                <div style={{ fontSize: 10, color: "#9CA3AF" }}>
                  事件数据: {evMeta.payload || "无"} — {evMeta.description}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Event creation panel */}
      {eventSource && (
        <EventCreationPanel
          source={eventSource}
          existingCodeBlocks={existingCodeBlocks}
          existingStoreActions={existingStoreActions}
          onConfirm={handleEventCreation}
          onClose={() => setEventSource(null)}
        />
      )}
    </div>
  );
}

// ── Helpers ─────────────────────────────────────

function defaultValueForType(type: string): string {
  switch (type) {
    case "String": return '""';
    case "bool": return "false";
    case "i32": case "f64": return "0";
    default: return "null";
  }
}
