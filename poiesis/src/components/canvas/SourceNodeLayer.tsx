// 数据区（M2）：醒态 + [数据节点] 开关门控的源节点卡片层。
// 画布右上角自动排布（纵向堆叠）；卡片可拖（@dnd-kit，嵌套 DndContext——子 context 优先，
// 对自身 draggables 有优先权；activationConstraint distance:5 与宿主一致，点击/拖拽分离）。
// 点卡片 → 编辑态（T5 渲染 ValueEditor；本任务为指示占位）。
// 防御性门控：仅组件 tab（activeTabType === "component"）显示（挂载点已在组件中心区，双保险）。
import { useMemo, useState } from "react";
import { DndContext, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { usePoiesisStore } from "../../store";
import { collectSourceNodes, type SourceNode } from "./source-nodes";
import { SourceNodeCard } from "./SourceNodeCard";
import styles from "./SourceNodeLayer.module.css";

export function SourceNodeLayer() {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const dataNodesVisible = usePoiesisStore((s) => s.designPage.dataNodesVisible);
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);
  const inlineStoreDefs = usePoiesisStore((s) => s.inlineStoreDefs);
  const storeReferences = usePoiesisStore((s) => s.storeReferences);
  const debugValues = usePoiesisStore((s) => s.debugValues);

  // 拖动位置：相对堆叠槽位的偏移（slot 包裹卡片，translate 不压垮 flex 布局）
  const [positions, setPositions] = useState<Record<string, { x: number; y: number }>>({});
  const [editing, setEditing] = useState<SourceNode | null>(null);
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }));

  const activeTab = openTabs.find((t) => t.id === activeTabId);
  const componentName = activeTab?.tabType === "component" ? (activeTab.componentName ?? null) : null;

  const nodes = useMemo(
    () => collectSourceNodes({ componentName, componentDefs, storeDefs, inlineStoreDefs, storeReferences, debugValues }),
    [componentName, componentDefs, storeDefs, inlineStoreDefs, storeReferences, debugValues],
  );

  if (!awake || !dataNodesVisible || !componentName) return null;

  const handleDragEnd = (e: DragEndEvent) => {
    const id = String(e.active.id);
    const { x, y } = e.delta;
    if (!x && !y) return;
    setPositions((prev) => ({
      ...prev,
      [id]: { x: (prev[id]?.x ?? 0) + x, y: (prev[id]?.y ?? 0) + y },
    }));
  };

  return (
    <div className={styles.layer} data-testid="source-node-layer" aria-label="数据区">
      <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
        {nodes.map((n) => {
          const pos = positions[n.id];
          return (
            <div
              key={n.id}
              className={styles.slot}
              style={pos ? { transform: `translate(${pos.x}px, ${pos.y}px)` } : undefined}
            >
              <SourceNodeCard node={n} onEdit={setEditing} />
            </div>
          );
        })}
      </DndContext>
      {editing && (
        <div className={styles.editing} data-testid="editing-indicator">
          编辑中：{editing.name}（T5 换 ValueEditor）
        </div>
      )}
    </div>
  );
}
