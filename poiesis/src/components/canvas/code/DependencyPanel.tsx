// DependencyPanel.tsx
// Right-side panel displaying live data dependencies for a code block.

import styles from "./DependencyPanel.module.css";
import type { DependencyInfo, DepEntry, CallEntry } from "../../../core/types/dependency-info";

// Re-export for consumers that already import from here
export type { DependencyInfo, DepEntry, CallEntry };

// ── Component ──

export type AnalysisStatus = "loading" | "ready" | "unavailable";

interface DependencyPanelProps {
  deps: DependencyInfo | null;
  status?: AnalysisStatus;
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

/** Show appropriate placeholder based on analysis status. */
function StatusPlaceholder({ status }: { status?: AnalysisStatus }) {
  switch (status) {
    case "unavailable":
      return (
        <div className={styles.placeholder}>
          <div className={styles.statusIcon}>⚡</div>
          <div className={styles.statusText}>WASM 编译服务未就绪</div>
          <div className={styles.statusHint}>
            依赖分析需要 omnic WASM 二进制文件。请运行 <code>wasm-pack build</code>。
          </div>
        </div>
      );
    case "loading":
    default:
      return (
        <div className={styles.placeholder}>
          解析中...
        </div>
      );
  }
}

export function DependencyPanel({ deps, status }: DependencyPanelProps) {
  // Loading / unavailable state — deps is null
  if (!deps) {
    return (
      <div className={styles.panel}>
        <StatusPlaceholder status={status} />
      </div>
    );
  }

  // Ready state — deps has data (possibly empty)
  return (
    <div className={styles.panel}>
      <div className={styles.scrollArea}>
        {/* Reads Section */}
        {deps.reads.length > 0 && (
          <div className={styles.section}>
            <div className={styles.sectionHeader}>📖 读取</div>
            {deps.reads.map((entry, i) => (
              <div key={`r-${entry.name}-${entry.kind}-${entry.sourceLine}`} className={styles.entry}>
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
              <div key={`w-${entry.name}-${entry.kind}-${entry.sourceLine}`} className={styles.entry}>
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
              <div key={`c-${entry.storeName}-${entry.actionName ?? ""}-${entry.sourceLine}`} className={styles.entry}>
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

        {/* All sections empty — analysis succeeded but no deps found */}
        {deps.reads.length === 0 && deps.writes.length === 0 && deps.calls.length === 0 && (
          <div className={styles.placeholder}>当前代码块无外部依赖</div>
        )}
      </div>
    </div>
  );
}
