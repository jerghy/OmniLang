/** Canvas2D 连线叠加层。绘制贝塞尔连线 + 端口圆点 + 连线折叠 + 搜索过滤。ResizeObserver + rAF 同步。*/

import { useRef, useEffect, useCallback, useState } from "react";
import { useEditorStore, type WireData } from "../store/editor";
import { perfMonitor } from "./PerfMonitor";

// ── 连线折叠 ──────────────────────────────────────

interface FoldedGroup {
  fromId: string;
  toId: string;
  count: number;
}

function foldWires(wires: WireData[], maxVisible = 5): {
  visible: WireData[];
  folded: FoldedGroup[];
} {
  const groups = new Map<string, WireData[]>();
  for (const w of wires) {
    const key = `${w.fromId}→${w.toId}`;
    const list = groups.get(key) || [];
    list.push(w);
    groups.set(key, list);
  }

  const visible: WireData[] = [];
  const folded: FoldedGroup[] = [];

  for (const [, list] of groups) {
    if (list.length <= 1) {
      visible.push(list[0]);
    } else if (list.length <= maxVisible) {
      visible.push(...list);
    } else {
      visible.push(...list.slice(0, maxVisible));
      folded.push({
        fromId: list[0].fromId,
        toId: list[0].toId,
        count: list.length - maxVisible,
      });
    }
  }

  return { visible, folded };
}

// ── 连线搜索 ──────────────────────────────────────

function searchWires(wires: WireData[], query: string): WireData[] {
  const q = query.toLowerCase();
  return wires.filter(
    (w) =>
      w.fromId.toLowerCase().includes(q) ||
      w.toId.toLowerCase().includes(q) ||
      w.fromPort.toLowerCase().includes(q) ||
      w.toPort.toLowerCase().includes(q)
  );
}

// ── 端口提取 ──────────────────────────────────────

interface PortPoint {
  id: string;
  x: number;
  y: number;
  dir: "in" | "out";
}

export function WireOverlay() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const elements = useEditorStore((s) => s.elements);
  const wires = useEditorStore((s) => s.wires);
  const selectedId = useEditorStore((s) => s.selectedElementId);
  const [wireSearch, setWireSearch] = useState("");

  const getPorts = useCallback((): PortPoint[] => {
    const portList: PortPoint[] = [];
    const canvas = canvasRef.current;
    if (!canvas) return portList;
    const cr = canvas.getBoundingClientRect();
    for (const el of elements) {
      const dom = document.querySelector(`[data-omni-id="${el.id}"]`);
      if (!dom) continue;
      const r = dom.getBoundingClientRect();
      if (el.nodeType === "view") {
        portList.push({ id: el.id, x: r.right - cr.left, y: r.top + r.height / 2 - cr.top, dir: "out" });
        portList.push({ id: el.id, x: r.left - cr.left, y: r.top + r.height / 2 - cr.top, dir: "in" });
      }
      if (el.nodeType === "codeblock") {
        const isStore = el.tag === "store";
        if (!isStore) {
          portList.push({ id: el.id, x: r.left - cr.left, y: r.top + r.height / 2 - cr.top, dir: "in" });
        }
        portList.push({ id: el.id, x: r.right - cr.left, y: r.top + r.height / 2 - cr.top, dir: "out" });
      }
    }
    return portList;
  }, [elements]);

  const draw = useCallback(() => {
    perfMonitor.beginFrame();
    const c = canvasRef.current;
    if (!c) return;
    const ctx = c.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const w = c.parentElement!.clientWidth;
    const h = c.parentElement!.clientHeight;
    c.width = w * dpr;
    c.height = h * dpr;
    c.style.width = `${w}px`;
    c.style.height = `${h}px`;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const ports = getPorts();

    // Determine visible wires: selection filter → search filter
    let candidateWires = selectedId
      ? wires.filter((wr) => wr.fromId === selectedId || wr.toId === selectedId)
      : wires;
    if (wireSearch.trim()) {
      candidateWires = searchWires(candidateWires, wireSearch.trim());
    }

    // Fold wires with same (fromId, toId)
    const { visible: visibleWires, folded: foldedGroups } = foldWires(candidateWires);

    // Draw visible wires
    for (const wr of visibleWires) {
      const from = ports.find((p) => p.id === wr.fromId && p.dir === "out");
      const to = ports.find((p) => p.id === wr.toId && p.dir === "in");
      if (!from || !to) continue;
      ctx.beginPath();
      ctx.moveTo(from.x, from.y);
      const cp = Math.abs(to.x - from.x) * 0.5;
      ctx.bezierCurveTo(from.x + cp, from.y, to.x - cp, to.y, to.x, to.y);
      ctx.strokeStyle = "#4F46E5";
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Draw folded group indicators (thicker lines with count badge)
    for (const fg of foldedGroups) {
      const from = ports.find((p) => p.id === fg.fromId && p.dir === "out");
      const to = ports.find((p) => p.id === fg.toId && p.dir === "in");
      if (!from || !to) continue;
      ctx.beginPath();
      ctx.moveTo(from.x, from.y);
      const cp = Math.abs(to.x - from.x) * 0.5;
      ctx.bezierCurveTo(from.x + cp, from.y, to.x - cp, to.y, to.x, to.y);
      ctx.strokeStyle = "#8B5CF6";
      ctx.lineWidth = 3;
      ctx.setLineDash([8, 4]);
      ctx.stroke();
      ctx.setLineDash([]);

      // Count badge
      const mx = (from.x + to.x) / 2;
      const my = (from.y + to.y) / 2;
      const label = `+${fg.count}`;
      ctx.font = "bold 11px system-ui";
      const metrics = ctx.measureText(label);
      const pw = metrics.width + 8;
      const ph = 16;
      ctx.fillStyle = "#8B5CF6";
      ctx.beginPath();
      if (typeof (ctx as any).roundRect === "function") {
        (ctx as any).roundRect(mx - pw / 2, my - ph / 2, pw, ph, 8);
      } else {
        // Fallback: manual rounded rectangle with arcTo
        const rx = mx - pw / 2, ry = my - ph / 2, r = 8;
        ctx.moveTo(rx + r, ry);
        ctx.lineTo(rx + pw - r, ry);
        ctx.arcTo(rx + pw, ry, rx + pw, ry + r, r);
        ctx.lineTo(rx + pw, ry + ph - r);
        ctx.arcTo(rx + pw, ry + ph, rx + pw - r, ry + ph, r);
        ctx.lineTo(rx + r, ry + ph);
        ctx.arcTo(rx, ry + ph, rx, ry + ph - r, r);
        ctx.lineTo(rx, ry + r);
        ctx.arcTo(rx, ry, rx + r, ry, r);
      }
      ctx.fill();
      ctx.fillStyle = "#FFF";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(label, mx, my);
    }

    // Port dots
    for (const p of ports) {
      if (selectedId && p.id !== selectedId) continue;
      ctx.beginPath();
      ctx.arc(p.x, p.y, 4, 0, Math.PI * 2);
      ctx.fillStyle = p.dir === "out" ? "#4F46E5" : "#10B981";
      ctx.fill();
    }

    perfMonitor.endFrame(elements.length, wires.length);
  }, [elements, wires, selectedId, wireSearch, getPorts]);

  useEffect(() => {
    let raf: number;
    let timer: number;
    const debounced = () => {
      clearTimeout(timer);
      timer = window.setTimeout(() => {
        raf = requestAnimationFrame(draw);
      }, 50);
    };
    const ro = new ResizeObserver(debounced);
    const parent = canvasRef.current?.parentElement;
    if (parent) ro.observe(parent);
    debounced();
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
      clearTimeout(timer);
    };
  }, [draw]);

  return (
    <>
      {/* Wire search bar */}
      {wires.length > 0 && (
        <div
          style={{
            position: "absolute",
            top: 8,
            right: 8,
            zIndex: 11,
            display: "flex",
            alignItems: "center",
            gap: 4,
            background: "#FFF",
            border: "1px solid #E5E7EB",
            borderRadius: 6,
            padding: "4px 8px",
            fontSize: 11,
            pointerEvents: "auto",
          }}
        >
          <span style={{ color: "#9CA3AF" }}>🔍</span>
          <input
            value={wireSearch}
            onChange={(e) => setWireSearch(e.target.value)}
            placeholder="搜索连线..."
            style={{
              border: "none",
              outline: "none",
              fontSize: 11,
              width: 120,
              background: "transparent",
            }}
          />
          {wireSearch && (
            <button
              onClick={() => setWireSearch("")}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                fontSize: 10,
                color: "#9CA3AF",
                padding: 0,
              }}
            >
              ✕
            </button>
          )}
        </div>
      )}
      <canvas
        ref={canvasRef}
        style={{
          position: "absolute",
          top: 0,
          left: 0,
          pointerEvents: "none",
          zIndex: 10,
        }}
      />
    </>
  );
}
