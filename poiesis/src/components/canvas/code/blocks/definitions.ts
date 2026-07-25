// poiesis/src/components/canvas/code/blocks/definitions.ts
import * as Blockly from "blockly";
import { registerIfBlockMutator } from "./mutations";

// ── Color palette (Zelos/Scratch 3.0 defaults) ──
const C = {
  CONTROL:  "#FFAB19",  // if, while, for
  VARIABLE: "#FF8C1A",  // declare_var, set_var
  FUNCTION: "#995BA5",  // return, call_fn
  ASYNC:    "#5B67B0",  // await
  BREAK:    "#CF8B17",  // break, continue
  COMMENT:  "#5CB85C",  // comment
  RAW:      "#AAAAAA",  // raw
} as const;

// ── Simple statement blocks ──

const declareVarDef = {
  type: "declare_var",
  message0: "%1 %2 : %3 = %4",
  args0: [
    { type: "field_dropdown", name: "MUT", options: [["let", "let"], ["let mut", "mut"]] },
    { type: "input_value", name: "NAME", check: "Text" },
    { type: "input_value", name: "VARTYPE", check: "Text" },
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.VARIABLE,
  tooltip: "Declare a variable",
};

const setVarDef = {
  type: "set_var",
  message0: "%1 %2 = %3",
  args0: [
    { type: "field_dropdown", name: "OP", options: [["set", "set"], ["+=", "+="], ["-=", "-="], ["*=", "*="], ["/=", "/="], ["%=", "%="]] },
    { type: "input_value", name: "NAME", check: "Text" },
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.VARIABLE,
};

const returnDef = {
  type: "return_stmt",
  message0: "return %1",
  args0: [
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.FUNCTION,
};

const callFnDef = {
  type: "call_fn",
  message0: "call %1 ( %2 )",
  args0: [
    { type: "input_value", name: "FN_NAME", check: "Text" },
    { type: "input_value", name: "ARGS", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.FUNCTION,
  // ARGS is a single input; multi-arg support is deferred
};

const awaitDef = {
  type: "await_expr",
  message0: "await %1",
  args0: [
    { type: "input_value", name: "VALUE", check: "Expr" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.ASYNC,
};

const breakDef = {
  type: "break_stmt",
  message0: "break",
  previousStatement: null,
  nextStatement: null,
  colour: C.BREAK,
};

const continueDef = {
  type: "continue_stmt",
  message0: "continue",
  previousStatement: null,
  nextStatement: null,
  colour: C.BREAK,
};

const commentDef = {
  type: "comment_stmt",
  message0: "// %1",
  args0: [
    { type: "field_input", name: "TEXT", text: "" },
  ],
  inputsInline: true,
  previousStatement: null,
  nextStatement: null,
  colour: C.COMMENT,
};

const rawDef = {
  type: "raw_stmt",
  message0: "raw %1",
  args0: [
    { type: "field_multilinetext", name: "SOURCE", text: "" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.RAW,
};

// ── C-shaped blocks ──

const ifBlockDef = {
  type: "if_block",
  message0: "if %1",
  args0: [
    { type: "input_value", name: "CONDITION", check: "Expr" },
  ],
  message1: "do %1",
  args1: [
    { type: "input_statement", name: "BODY", check: "Statement" },
  ],
  message2: "else %1",
  args2: [
    { type: "input_statement", name: "ELSE_BODY", check: "Statement" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.CONTROL,
  // Mutation for elif/else
  mutator: "if_block_mutator",
  tooltip: "If / elif / else block",
};

const whileDef = {
  type: "while_loop",
  message0: "while %1",
  args0: [
    { type: "input_value", name: "CONDITION", check: "Expr" },
  ],
  message1: "do %1",
  args1: [
    { type: "input_statement", name: "BODY", check: "Statement" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.CONTROL,
};

const forInDef = {
  type: "for_in",
  message0: "for %1 in %2",
  args0: [
    { type: "input_value", name: "ITEM", check: "Text" },
    { type: "input_value", name: "COLLECTION", check: "Expr" },
  ],
  message1: "do %1",
  args1: [
    { type: "input_statement", name: "BODY", check: "Statement" },
  ],
  previousStatement: null,
  nextStatement: null,
  colour: C.CONTROL,
};

export const BLOCK_DEFINITIONS: Record<string, unknown> = {
  declare_var: declareVarDef,
  set_var: setVarDef,
  return_stmt: returnDef,
  call_fn: callFnDef,
  await_expr: awaitDef,
  break_stmt: breakDef,
  continue_stmt: continueDef,
  comment_stmt: commentDef,
  raw_stmt: rawDef,
  if_block: ifBlockDef,
  while_loop: whileDef,
  for_in: forInDef,
};

export function registerAllBlocks(): void {
  for (const [name, def] of Object.entries(BLOCK_DEFINITIONS)) {
    Blockly.Blocks[name] = {
      init(this: Blockly.Block) {
        this.jsonInit(def as Blockly.BlockDefinition);
      },
    };
  }
  registerIfBlockMutator();
}
