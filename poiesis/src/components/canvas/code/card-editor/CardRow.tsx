// poiesis/src/components/canvas/code/card-editor/CardRow.tsx
import React from "react";
import type { CodeOp } from "../../../../core/types/code-op";
import { DeclareVarCard } from "./cards/DeclareVarCard";
import { SetVarCard } from "./cards/SetVarCard";
import { IfBlockCard } from "./cards/IfBlockCard";
import { WhileLoopCard } from "./cards/WhileLoopCard";
import { ForLoopCard } from "./cards/ForLoopCard";
import { ReturnExprCard } from "./cards/ReturnExprCard";
import { CallFnCard } from "./cards/CallFnCard";
import { AwaitExprCard } from "./cards/AwaitExprCard";
import { BreakStmtCard } from "./cards/BreakStmtCard";
import { ContinueStmtCard } from "./cards/ContinueStmtCard";
import { CommentCard } from "./cards/CommentCard";
import { RawCodeCard } from "./cards/RawCodeCard";

interface Props {
  op: CodeOp;
  indent: number;
  isFocused: boolean;
  onUpdate: (op: CodeOp) => void;
  onFocus: () => void;
  onInsertBelow: () => void;
  onDelete: () => void;
}

export const CardRow: React.FC<Props> = ({ op, indent, isFocused, onUpdate, onFocus, onInsertBelow, onDelete }) => {
  const shared = { op, isFocused, onUpdate, onFocus, onInsertBelow, onDelete };

  return (
    <div style={{ paddingLeft: indent * 20 }}>
      {renderCard(op, shared)}
    </div>
  );
};

function renderCard(op: CodeOp, p: Omit<Props, "indent">): React.ReactNode {
  switch (op.kind) {
    case "declare_var": return <DeclareVarCard {...p} op={op} />;
    case "set_var": return <SetVarCard {...p} op={op} />;
    case "if_block": return <IfBlockCard {...p} op={op} />;
    case "while": return <WhileLoopCard {...p} op={op} />;
    case "for_in": return <ForLoopCard {...p} op={op} />;
    case "return": return <ReturnExprCard {...p} op={op} />;
    case "call_fn": return <CallFnCard {...p} op={op} />;
    case "await": return <AwaitExprCard {...p} op={op} />;
    case "break": return <BreakStmtCard {...p} op={op} />;
    case "continue": return <ContinueStmtCard {...p} op={op} />;
    case "comment": return <CommentCard {...p} op={op} />;
    case "raw": return <RawCodeCard {...p} op={op} />;
  }
}
