// components/design/CodeDrawer.tsx
// Shared code drawer — slides up from bottom to show generated .omni source.
// Used globally across all tab types (design + component).

import styles from "./CodeDrawer.module.css";

interface CodeDrawerProps {
  source: string;
  fileName: string;
  onClose: () => void;
}

const DRAWER_HEIGHT = 260;

export function CodeDrawer({ source, fileName, onClose }: CodeDrawerProps) {
  return (
    <div className={styles.drawer} style={{ height: DRAWER_HEIGHT }}>
      {/* Header */}
      <div className={styles.header}>
        <span className={styles.title}>
          <span className={styles.accent}>{"</>"}</span>
          {fileName}
        </span>
        <button type="button"
          onClick={onClose}
          className={styles.closeBtn}
          title="Close code panel"
        >×</button>
      </div>
      {/* Body */}
      <pre className={styles.codeBlock}>
        {source}
      </pre>
    </div>
  );
}
