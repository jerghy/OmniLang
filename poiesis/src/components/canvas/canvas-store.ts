// poiesis/src/components/canvas/canvas-store.ts
import { create } from "zustand";

export interface DevicePreset {
  name: string;
  width: number;
  height: number; // 0 = auto (fit content)
}

export const DEVICE_PRESETS: DevicePreset[] = [
  { name: "iPhone 15", width: 393, height: 852 },
  { name: "Pixel 8", width: 412, height: 915 },
  { name: "iPad", width: 768, height: 1024 },
  { name: "iPad Pro", width: 1024, height: 1366 },
  { name: "Desktop", width: 1440, height: 900 },
  { name: "Full HD", width: 1920, height: 1080 },
  { name: "Custom", width: 0, height: 0 },
];

export interface CanvasState {
  zoom: number;
  panX: number;
  panY: number;
  devicePreset: DevicePreset;
  artboardScrollTop: number;
  artboardScrollLeft: number;
  panelPosition: { x: number; y: number };
  isPanelCollapsed: boolean;
  setZoom: (zoom: number) => void;
  setPan: (x: number, y: number) => void;
  setDevicePreset: (preset: DevicePreset) => void;
  setArtboardScroll: (top: number, left: number) => void;
  zoomAtPoint: (newZoom: number, cursorX: number, cursorY: number) => void;
  fitToView: (viewportW: number, viewportH: number) => void;
  resetView: () => void;
  setPanelPosition: (x: number, y: number) => void;
  togglePanelCollapsed: () => void;
}

export const useCanvasStore = create<CanvasState>((set, get) => ({
  zoom: 1.0,
  panX: 0,
  panY: 0,
  devicePreset: DEVICE_PRESETS[0], // iPhone 15 default
  artboardScrollTop: 0,
  artboardScrollLeft: 0,
  panelPosition: { x: 0, y: 0 },
  isPanelCollapsed: false,

  setZoom: (zoom) => set({ zoom: Math.max(0.1, Math.min(5.0, zoom)) }),

  setPan: (x, y) => set({ panX: x, panY: y }),

  setDevicePreset: (preset) => set({ devicePreset: preset, artboardScrollTop: 0, artboardScrollLeft: 0 }),

  setArtboardScroll: (top, left) => set({ artboardScrollTop: top, artboardScrollLeft: left }),

  zoomAtPoint: (newZoom, cursorX, cursorY) => {
    const state = get();
    const clamped = Math.max(0.1, Math.min(5.0, newZoom));
    const scale = clamped / state.zoom;
    set({
      zoom: clamped,
      panX: cursorX - scale * (cursorX - state.panX),
      panY: cursorY - scale * (cursorY - state.panY),
    });
  },

  fitToView: (viewportW, viewportH) => {
    const { devicePreset } = get();
    const scaleX = viewportW / devicePreset.width;
    const scaleY = viewportH / (devicePreset.height || 800);
    const zoom = Math.min(scaleX, scaleY) * 0.9;
    const panX = (viewportW - devicePreset.width * zoom) / 2;
    const panY = (viewportH - (devicePreset.height || 800) * zoom) / 2;
    set({ zoom: Math.max(0.1, Math.min(5.0, zoom)), panX, panY });
  },

  resetView: () => set({ zoom: 1.0, panX: 0, panY: 0, artboardScrollTop: 0, artboardScrollLeft: 0 }),

  setPanelPosition: (x, y) => set({ panelPosition: { x, y } }),
  togglePanelCollapsed: () => set((s) => ({ isPanelCollapsed: !s.isPanelCollapsed })),
}));
