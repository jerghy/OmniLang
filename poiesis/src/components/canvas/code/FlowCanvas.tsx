import { useMemo } from "react";
import { ReactFlow } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { usePoiesisStore } from "../../../store";
import { FunctionHeaderNode } from "./nodes/FunctionHeaderNode";
import { SkeletonNode } from "./nodes/SkeletonNode";
import { CodeBlockNode } from "./nodes/CodeBlockNode";
import { ReturnNode } from "./nodes/ReturnNode";

const nodeTypes = {
  "function-header": FunctionHeaderNode,
  skeleton: SkeletonNode,
  "code-block": CodeBlockNode,
  return: ReturnNode,
};

export function FlowCanvas() {
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const codeFlowElementId = usePoiesisStore(
    (s) => s.designPage.codeFlowElementId,
  );

  // Filter to only code-relevant elements (fn internals)
  const codeElements = useMemo(
    () => elements.filter((e) => e.nodeType === "codeblock"),
    [elements],
  );

  const nodes = useMemo(
    () =>
      codeElements.map((el) => ({
        id: el.id,
        type:
          el.tag === "fn"
            ? "function-header"
            : el.tag === "if" ||
                el.tag === "else" ||
                el.tag === "for" ||
                el.tag === "while"
              ? "skeleton"
              : el.tag === "return"
                ? "return"
                : "code-block",
        position: el.position || { x: 0, y: 0 },
        data: {
          label: el.tag,
          source: el.source,
          fnName: el.props?.name as string,
          skeletonType: el.tag,
          condition: el.props?.condition as string,
        },
      })),
    [codeElements],
  );

  const edges = useMemo(
    () =>
      wires.map((w) => ({
        id: w.id,
        source: w.fromId,
        target: w.toId,
        sourceHandle: w.fromPort,
        targetHandle: w.toPort,
      })),
    [wires],
  );

  return (
    <div style={{ width: "100%", height: "100%" }}>
      {nodes.length === 0 ? (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            color: "#94a3b8",
            fontSize: 13,
          }}
        >
          {codeFlowElementId
            ? "双击组件元素进入代码节点流编辑"
            : "请先选择一个函数元素"}
        </div>
      ) : (
        <ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes} fitView>
          {/* Monaco editor panel will be added in Phase 5 extension */}
        </ReactFlow>
      )}
    </div>
  );
}
