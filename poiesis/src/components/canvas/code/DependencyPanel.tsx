// DependencyPanel.tsx
// Right-side panel displaying live data dependencies for a code block.

import styles from "./DependencyPanel.module.css";

// ── Public Types ──

export interface DependencyInfo {
  codeBlockPid: string;
  reads:  DepEntry[];
  writes: DepEntry[];
  calls:  CallEntry[];
}

export interface DepEntry {
  name: string;
  kind: "param" | "signal" | "computed" | "prop" | "store" | "local" | "function" | "unknown";
  sourceLine: number;
}

export interface CallEntry {
  storeName: string;
  actionName?: string;
  sourceLine: number;
}

// ── Component ──

interface DependencyPanelProps {
  deps: DependencyInfo | null;
}

const KIND_LABELS: Record<DepEntry["kind"], string> = {
  param:    "参数",
  signal:   "Signal",
  computed: "Computed",
  prop:     "Prop",
  store:    "Store",
  local:    "局部变量",
  function: "函数",
  unknown:  "未知",
};

const KIND_CLASS: Record<DepEntry["kind"], string> = {
  param:    styles.kindParam,
  signal:   styles.kindSignal,
  computed: styles.kindComputed,
  prop:     styles.kindProp,
  store:    styles.kindStore,
  local:    styles.kindLocal,
  function: styles.kindFunction,
  unknown:  styles.kindUnknown,
};

function KindBadge({ kind }: { kind: DepEntry["kind"] }) {
  return (
    <span className={`${styles.kindBadge} ${KIND_CLASS[kind]}`}>
      {KIND_LABELS[kind]}
    </span>
  );
}

export function DependencyPanel({ deps }: DependencyPanelProps) {
  if (!deps) {
    return (
      <div className={styles.panel}>
        <div className={styles.placeholder}>解析中...</div>
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      <div className={styles.scrollArea}>
        {/* Reads Section */}
        {deps.reads.length > 0 && (
          <div className={styles.section}>
            <div className={styles.sectionHeader}>📖 读取</div>
            {deps.reads.map((entry, i) => (
              <div key={`r-${entry.name}-${i}`} className={styles.entry}>
                <span className={styles.entryName}>{entry.name}</span>
                <KindBadge kind={entry.kind} />
                <span className={styles.sourceLine}>L{entry.sourceLine}</span>
              </div>
            ))}
          </div>
        )}

        {/* Writes Section */}
        {deps.writes.length > 0 && (
          <div className={styles.section}>
            <div className={styles.sectionHeader}>✏️ 写入</div>
            {deps.writes.map((entry, i) => (
              <div key={`w-${entry.name}-${i}`} className={styles.entry}>
                <span className={styles.entryName}>{entry.name}</span>
                <KindBadge kind={entry.kind} />
                <span className={styles.sourceLine}>L{entry.sourceLine}</span>
              </div>
            ))}
          </div>
        )}

        {/* Calls Section */}
        {deps.calls.length > 0 && (
          <div className={styles.section}>
            <div className={styles.sectionHeader}>📞 调用</div>
            {deps.calls.map((entry, i) => (
              <div key={`c-${entry.storeName}-${i}`} className={styles.entry}>
                <span className={styles.entryName}>
                  {entry.actionName
                    ? `${entry.storeName}.${entry.actionName}`
                    : entry.storeName}
                </span>
                <span className={styles.sourceLine}>L{entry.sourceLine}</span>
              </div>
            ))}
          </div>
        )}

        {/* All sections empty */}
        {deps.reads.length === 0 && deps.writes.length === 0 && deps.calls.length === 0 && (
          <div className={styles.placeholder}>无依赖</div>
        )}
      </div>
    </div>
  );
}
