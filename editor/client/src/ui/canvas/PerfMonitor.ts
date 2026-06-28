/**
 * Performance monitor — tracks canvas render performance metrics.
 *
 * Measures:
 *   - Frame time (ms per canvas render)
 *   - FPS (frames per second, rolling average)
 *   - Element count
 *   - Wire count
 *
 * Performance gates (design doc §6 Tier 0-3):
 *   - 50 components > 30fps (≤33ms per frame)
 *   - 200 components + 300 wires < 50ms property edit latency
 */

import { Logger } from "../../utils/logger";

const log = new Logger({ prefix: "Perf" });

interface PerfSample {
  timestamp: number;
  frameTime: number;   // ms
  elementCount: number;
  wireCount: number;
}

const MAX_SAMPLES = 120;  // 2 seconds at 60fps

class PerfMonitorImpl {
  private samples: PerfSample[] = [];
  private frameStart = 0;
  private listeners: Set<(metrics: PerfMetrics) => void> = new Set();

  /** Call at the START of a canvas render frame. */
  beginFrame(): void {
    this.frameStart = performance.now();
  }

  /** Call at the END of a canvas render frame. */
  endFrame(elementCount: number, wireCount: number): void {
    const frameTime = performance.now() - this.frameStart;
    this.samples.push({
      timestamp: Date.now(),
      frameTime,
      elementCount,
      wireCount,
    });
    if (this.samples.length > MAX_SAMPLES) {
      this.samples.shift();
    }
    // Notify listeners
    const metrics = this.getMetrics();
    for (const listener of this.listeners) {
      listener(metrics);
    }
  }

  /** Get current performance metrics. */
  getMetrics(): PerfMetrics {
    if (this.samples.length === 0) {
      return { fps: 0, avgFrameTime: 0, maxFrameTime: 0, elementCount: 0, wireCount: 0, meetsGate: true };
    }

    // FPS from recent samples (last second)
    const now = Date.now();
    const recent = this.samples.filter(s => now - s.timestamp < 1000);
    const fps = recent.length > 0 ? recent.length : 0;

    // Rolling average of frame times
    const all = this.samples;
    const avgFrameTime = all.reduce((sum, s) => sum + s.frameTime, 0) / all.length;
    const maxFrameTime = Math.max(...all.map(s => s.frameTime));

    const last = all[all.length - 1];
    const elementCount = last.elementCount;
    const wireCount = last.wireCount;

    // Performance gate check
    const meetsGate = fps >= 30 && avgFrameTime <= 33;

    return { fps, avgFrameTime, maxFrameTime, elementCount, wireCount, meetsGate };
  }

  /** Subscribe to metrics updates. */
  subscribe(listener: (metrics: PerfMetrics) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /** Mark a specific operation's duration. */
  measure<T>(label: string, fn: () => T): T {
    const start = performance.now();
    const result = fn();
    const duration = performance.now() - start;
    // Only log slow operations (>16ms — one frame budget)
    if (duration > 16) {
      log.debug(`${label}: ${duration.toFixed(1)}ms`);
    }
    return result;
  }
}

export interface PerfMetrics {
  fps: number;
  avgFrameTime: number;  // ms
  maxFrameTime: number;   // ms
  elementCount: number;
  wireCount: number;
  meetsGate: boolean;
}

/** Singleton perf monitor instance. */
export const perfMonitor = new PerfMonitorImpl();

/** React hook: subscribe to perf metrics. */
export function usePerfMetrics(): PerfMetrics {
  const [metrics, setMetrics] = React.useState<PerfMetrics>(() => perfMonitor.getMetrics());
  React.useEffect(() => {
    return perfMonitor.subscribe(setMetrics);
  }, []);
  return metrics;
}

import React from "react";
