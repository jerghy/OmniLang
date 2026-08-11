// 语境面板落位：优先元素右侧，放不下换左侧，两侧都不行落元素下方。
export interface Rect { x: number; y: number; width: number; height: number; }
export interface Size { width: number; height: number; }

export interface PanelPlacement {
  x: number;
  y: number;
  side: "right" | "left" | "bottom";
}

const GAP = 12;

function clampY(top: number, panel: Size, viewport: Size): number {
  return Math.max(0, Math.min(top, viewport.height - panel.height));
}

export function computePanelPlacement(
  elementRect: Rect,
  panelSize: Size,
  viewportSize: Size,
): PanelPlacement {
  const right = elementRect.x + elementRect.width + GAP;
  const left = elementRect.x - panelSize.width - GAP;
  if (right + panelSize.width <= viewportSize.width) {
    return { x: right, y: clampY(elementRect.y, panelSize, viewportSize), side: "right" };
  }
  if (left >= 0) {
    return { x: left, y: clampY(elementRect.y, panelSize, viewportSize), side: "left" };
  }
  const bottom = elementRect.y + elementRect.height + GAP;
  return {
    x: Math.max(0, Math.min(elementRect.x, viewportSize.width - panelSize.width)),
    y: Math.min(bottom, viewportSize.height - panelSize.height),
    side: "bottom",
  };
}
