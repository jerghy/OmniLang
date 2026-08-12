// poiesis/src/store/slices/behavior.ts
// M3 T2 行为卡落盘：当[事件]→[动作][参数] 提交 → 自动 fn 生成 + 语义事件指向。
// 落盘形状：fn 元素（props {name, params:"", returnType:""}）+ code-block 体元素
// （source = 动作模板输出）+ flow-out→flow-in 线；视图元素事件属性指向 fn 名，
// emitter 以 @<语义事件>={fn名} 落盘（source-emitter 事件发射 + parser-view 归一化
// 保证往返闭环，语法零变更）。
import type { PoiesisState } from "../types";
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import { semanticEventName } from "../../services/codegen/event-name-map";
import { ACTIONS } from "../../components/design/action-verbs";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

/** 行为卡提交载荷（与 BehaviorCardCommit 结构一致，避免组件 → store 依赖） */
export interface BehaviorCommit {
  elementId: string;
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

      const fnId = `${elementId}-${fnName}-${++behaviorSeq}`;
      const bodyId = `${fnId}-body`;
      const fnEl: CodeblockElementData = {
        id: fnId, nodeType: "codeblock", tag: "fn",
        props: { name: fnName, params: "", returnType: "" },
        source: "", parentId: null,
      };
      const bodyEl: CodeblockElementData = {
        id: bodyId, nodeType: "codeblock", tag: "code-block",
        props: {},
        source: action.code(behavior.params),
        parentId: null,
      };
      const wire: WireData = {
        id: `${fnId}-w`, fromId: fnId, fromPort: "flow-out",
        toId: bodyId, toPort: "flow-in",
      };

      set((st) => ({
        elements: [
          ...st.elements.map((e) =>
            e.id === elementId && "props" in e ? { ...e, props: { ...e.props, [semantic]: fnName } } : e
          ),
          fnEl,
          bodyEl,
        ],
        wires: [...st.wires, wire],
        rootOrder: [...st.rootOrder, fnId, bodyId],
        isDirty: true,
      }));
    },
  } satisfies Partial<PoiesisState>;
}
