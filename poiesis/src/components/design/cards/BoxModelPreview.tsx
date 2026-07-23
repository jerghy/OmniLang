// components/design/cards/BoxModelPreview.tsx
// Box-model preview diagram with Poiesis color palette:
// margin (warm orange) → padding (accent purple) → content (white).

import styles from "./BoxModelPreview.module.css";

interface SpacingValue {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

interface BoxModelPreviewProps {
  margin: SpacingValue;
  padding: SpacingValue;
  size?: number;
}

export function BoxModelPreview({
  margin,
  padding,
  size = 120,
}: BoxModelPreviewProps) {
  const maxSide = Math.floor(size / 2 - 4);
  const clamp = (v: number) => Math.min(Math.max(v, 0), maxSide);

  const mTop = clamp(margin.top);
  const mRight = clamp(margin.right);
  const mBottom = clamp(margin.bottom);
  const mLeft = clamp(margin.left);

  const pTop = clamp(padding.top);
  const pRight = clamp(padding.right);
  const pBottom = clamp(padding.bottom);
  const pLeft = clamp(padding.left);

  const paddingBox = {
    top: mTop,
    right: mRight,
    bottom: mBottom,
    left: mLeft,
  };

  const contentInset = {
    top: mTop + pTop,
    right: mRight + pRight,
    bottom: mBottom + pBottom,
    left: mLeft + pLeft,
  };

  const contentW = Math.max(size - contentInset.left - contentInset.right, 4);
  const contentH = Math.max(size - contentInset.top - contentInset.bottom, 4);

  const hasAnySpacing =
    mTop + mRight + mBottom + mLeft + pTop + pRight + pBottom + pLeft > 0;

  return (
    <div
      className={styles.wrapper}
      style={{ width: size, height: size, opacity: hasAnySpacing ? 1 : 0.3 }}
    >
      {/* Margin layer (outermost) — warm orange */}
      <div className={styles.marginLayer} />

      {/* Padding layer (middle) — accent purple */}
      <div
        className={styles.paddingLayer}
        style={{ top: paddingBox.top, right: paddingBox.right, bottom: paddingBox.bottom, left: paddingBox.left }}
      />

      {/* Content area (innermost) */}
      <div
        className={styles.contentArea}
        style={{ top: contentInset.top, left: contentInset.left, width: contentW, height: contentH }}
      />

      {/* Dimension labels */}
      {mTop > 4 && (
        <div
          className={styles.label}
          style={{ top: 1, left: 0, right: 0, height: mTop - 2 }}
        >
          M{mTop}
        </div>
      )}
      {mBottom > 4 && (
        <div
          className={styles.label}
          style={{ bottom: 1, left: 0, right: 0, height: mBottom - 2 }}
        >
          M{mBottom}
        </div>
      )}
      {pTop > 4 && (
        <div
          className={styles.label}
          style={{ top: mTop + 1, left: 0, right: 0, height: pTop - 2 }}
        >
          P{pTop}
        </div>
      )}

      {/* Legend */}
      {hasAnySpacing && (
        <div className={styles.legend}>
          <span className={styles.legendItem}>
            <span className={styles.legendSwatch} style={{ background: "rgba(249,115,22,0.3)", border: "1px solid rgba(249,115,22,0.5)" }} />
            Margin
          </span>
          <span className={styles.legendItem}>
            <span className={styles.legendSwatch} style={{ background: "rgba(99,102,241,0.2)", border: "1px solid rgba(99,102,241,0.4)" }} />
            Padding
          </span>
        </div>
      )}
    </div>
  );
}
