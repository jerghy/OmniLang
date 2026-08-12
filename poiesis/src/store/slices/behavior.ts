// poiesis/src/store/slices/behavior.ts
// M3 T2 行为卡落盘：当[事件]→[动作][参数] 提交 → 自动 fn 生成 + 语义事件指向。
// 落盘形状：fn 元素（props {name, params:"", returnType:""}）+ code-block 体元素
// （source = 动作模板输出）+ flow-out→flow-in 线；视图元素事件属性指向 fn 名，
// emitter 以 @<语义事件>={fn名} 落盘（source-emitter 事件发射 + parser-view 归一化
// 保证往返闭环，语法零变更）。
// T2 复审（3 Important + 1 Minor）：
// ① undo 参与——走 setElementsAndWires 快照栈（buildUndoUpdate 入栈，computeInverse
//    双方向还原，rootOrder 由无父元素派生，Ctrl+Z 恢复应用前状态）；
// ② 同事件重复应用——先清理旧自动 fn（名匹配 /^on[A-Z].*Auto$/）+ 其 code-block 体
//    + flow 线再落新 fn，避免孤儿旧 fn 与重复声明；指向用户手写 fn 则 console.warn
//    拒绝覆盖；
// ③ fn/体确定性落位（y 行递增 60，与 materializeFnGraphs 布局约定一致）防画布堆叠；
// ④ BehaviorCommit.elementId 冗余（独立参数已传），移除。
import type { PoiesisState } from "../types";
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { buildUndoUpdate } from "../undo-slice";
import { semanticEventName } from "../../services/codegen/event-name-map";
import { ACTIONS } from "../../components/design/action-verbs";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

/** 自动 fn 名模式（行为卡生成，区别于用户手写 fn——覆盖保护判定用） */
const AUTO_FN_NAME = /^on[A-Z].*Auto$/;

/** 行为卡提交载荷（与 BehaviorCardCommit 结构一致，避免组件 → store 依赖） */
export interface BehaviorCommit {
  event: string;
  action: string;
  params: Record<string, string>;
}

let behaviorSeq = 0;

export function createBehaviorSlice(set: SetFn, get: GetFn) {
  return {
    applyBehaviorCard: (elementId: string, behavior: BehaviorCommit) => {
      const s = get();
      const el = s.elements.find((e) => e.id === elementId);
      const action = ACTIONS[behavior.action];
      if (!el || !action) return;

      // 语义事件名：口语名 → on<Cap> 归一 → 语义映射（click 族 → onPress，其余透传）
      const normalized = "on" + behavior.event.charAt(0).toUpperCase() + behavior.event.slice(1);
      const semantic = semanticEventName(normalized);
      const fnName = "on" + semantic.replace(/^on/i, "") + "Auto";

      // 同事件重复应用：旧自动 fn 先清理（元素 + code-block 体 + flow 线），
      // 避免孤儿旧 fn 与重复声明；指向用户手写 fn 则拒绝覆盖。
      const current =
        "props" in el && typeof el.props[semantic] === "string" ? (el.props[semantic] as string) : null;
      const removedIds = new Set<string>();
      if (current) {
        if (!AUTO_FN_NAME.test(current)) {
          console.warn(`[poiesis] 行为卡：${elementId} 的 ${semantic} 已绑定用户函数 ${current}，不覆盖`);
          return;
        }
        const staleFn = s.elements.find(
          (e) => e.nodeType === "codeblock" && e.tag === "fn" && e.props.name === current,
        );
        if (staleFn) {
          removedIds.add(staleFn.id);
          removedIds.add(`${staleFn.id}-body`);
        }
      }

      // fn 落位：已有 fn 行下方递增 60（与 materializeFnGraphs 布局约定一致）
      let nextFnRow = 0;
      for (const e of s.elements) {
        if (removedIds.has(e.id) || e.nodeType !== "codeblock" || e.tag !== "fn") continue;
        if (e.position?.y !== undefined) {
          nextFnRow = Math.max(nextFnRow, Math.floor(e.position.y / 60) + 1);
        }
      }

      const fnId = `${elementId}-${fnName}-${++behaviorSeq}`;
      const bodyId = `${fnId}-body`;
      const fnEl: CodeblockElementData = {
        id: fnId, nodeType: "codeblock", tag: "fn",
        props: { name: fnName, params: "", returnType: "" },
        source: "", parentId: null,
        position: { x: 0, y: nextFnRow * 60 },
      };
      const bodyEl: CodeblockElementData = {
        id: bodyId, nodeType: "codeblock", tag: "code-block",
        props: {},
        source: action.code(behavior.params),
        parentId: null,
        position: { x: 200, y: nextFnRow * 60 },
      };
      const wire: WireData = {
        id: `${fnId}-w`, fromId: fnId, fromPort: "flow-out",
        toId: bodyId, toPort: "flow-in",
      };

      set((st) => {
        const nextElements = [
          ...st.elements
            .filter((e) => !removedIds.has(e.id))
            .map((e) =>
              e.id === elementId && "props" in e ? { ...e, props: { ...e.props, [semantic]: fnName } } : e,
            ),
          fnEl,
          bodyEl,
        ];
        const nextWires = [
          ...st.wires.filter((w) => !removedIds.has(w.fromId) && !removedIds.has(w.toId)),
          wire,
        ];
        // undo 参与：快照前后态入 setElementsAndWires 栈（computeInverse 双方向还原，
        // rootOrder 由无父元素派生——与 undo-apply 对称，Ctrl+Z 恢复应用前状态）
        const undoUpdate = buildUndoUpdate(st.undoStacks, st.designPage.activeTabId, {
          type: "setElementsAndWires",
          prevElements: [...st.elements],
          prevWires: [...st.wires],
          newElements: nextElements,
          newWires: nextWires,
        });
        return {
          ...undoUpdate,
          elements: nextElements,
          wires: nextWires,
          rootOrder: nextElements.filter((e) => !e.parentId).map((e) => e.id),
          isDirty: true,
        };
      });
    },
  } satisfies Partial<PoiesisState>;
}
