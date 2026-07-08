// components/design/cards/BoxModelPreview.tsx
// Box-model preview diagram with Poiesis color palette:
// margin (warm orange) → padding (accent purple) → content (white).

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
      style={{
        position: "relative",
        width: size,
        height: size,
        margin: "0 auto",
        opacity: hasAnySpacing ? 1 : 0.3,
      }}
    >
      {/* Margin layer (outermost) — warm orange */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          background: "rgba(249, 115, 22, 0.12)",
          border: "1px solid rgba(249, 115, 22, 0.3)",
          borderRadius: 3,
        }}
      />

      {/* Padding layer (middle) — accent purple */}
      <div
        style={{
          position: "absolute",
          top: paddingBox.top,
          right: paddingBox.right,
          bottom: paddingBox.bottom,
          left: paddingBox.left,
          background: "rgba(99, 102, 241, 0.1)",
          border: "1px solid rgba(99, 102, 241, 0.3)",
          borderRadius: 2,
        }}
      />

      {/* Content area (innermost) */}
      <div
        style={{
          position: "absolute",
          top: contentInset.top,
          left: contentInset.left,
          width: contentW,
          height: contentH,
          background: "var(--dp-surface, #fff)",
          border: "1px solid var(--dp-border, #e8e5f2)",
          borderRadius: 2,
        }}
      />

      {/* Dimension labels */}
      {mTop > 4 && (
        <div
          style={{
            position: "absolute",
            top: 1,
            left: 0,
            right: 0,
            height: mTop - 2,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 9,
            fontFamily: "var(--dp-font-mono, monospace)",
            color: "#c2410c",
            fontWeight: 600,
            pointerEvents: "none",
          }}
        >
          M{mTop}
        </div>
      )}
      {mBottom > 4 && (
        <div
          style={{
            position: "absolute",
            bottom: 1,
            left: 0,
            right: 0,
            height: mBottom - 2,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 9,
            fontFamily: "var(--dp-font-mono, monospace)",
            color: "#c2410c",
            fontWeight: 600,
            pointerEvents: "none",
          }}
        >
          M{mBottom}
        </div>
      )}
      {pTop > 4 && (
        <div
          style={{
            position: "absolute",
            top: mTop + 1,
            left: 0,
            right: 0,
            height: pTop - 2,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 9,
            fontFamily: "var(--dp-font-mono, monospace)",
            color: "var(--dp-accent, #6366f1)",
            fontWeight: 600,
            pointerEvents: "none",
          }}
        >
          P{pTop}
        </div>
      )}

      {/* Legend */}
      {hasAnySpacing && (
        <div
          style={{
            position: "absolute",
            bottom: -18,
            left: 0,
            right: 0,
            display: "flex",
            justifyContent: "center",
            gap: 12,
            fontSize: 9,
          }}
        >
          <span style={{ display: "flex", alignItems: "center", gap: 3, color: "var(--dp-text-muted, #a09db8)" }}>
            <span style={{ width: 8, height: 8, borderRadius: 1, background: "rgba(249,115,22,0.3)", border: "1px solid rgba(249,115,22,0.5)" }} />
            Margin
          </span>
          <span style={{ display: "flex", alignItems: "center", gap: 3, color: "var(--dp-text-muted, #a09db8)" }}>
            <span style={{ width: 8, height: 8, borderRadius: 1, background: "rgba(99,102,241,0.2)", border: "1px solid rgba(99,102,241,0.4)" }} />
            Padding
          </span>
        </div>
      )}
    </div>
  );
}
