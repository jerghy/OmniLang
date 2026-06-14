# OmniLang Visual Scripting — Design Specification

**Date:** 2026-06-14
**Status:** Approved
**Target:** Transform the node editor into a visual programming system (Unity Visual Scripting style)

## 1. Overview

### 1.1 Problem

The current "code interface" is a code-block wire canvas with only 1 block type (action). All logic is written as text code in Monaco editor. There are no visual flow control nodes, no visual operators, no drag-and-drop visual programming. The BlockPalette, StatePanel, and FunctionLibrary are fragmented panels that don't form a cohesive visual programming experience.

### 1.2 Vision

Transform the node editor into a **hybrid visual scripting system**: simple UI interaction logic is built by connecting visual nodes; complex computation logic remains in text code blocks. Similar to Unity Visual Scripting + C# scripts coexistence.

### 1.3 Key Design Decisions

| Decision | Choice |
|----------|--------|
| Execution model | Flow + Data separation (white flow arrows, colored data wires) |
| Library | `@xyflow/react` v12 (already installed, replacing Rete.js) |
| Execution engine | Self-built topological sort (~200 lines) |
| Code block integration | Black-box nodes with auto-generated ports from source code |
| Node discovery | Right-click canvas → search popup (Unity VS style) |
| Complex logic | Text code blocks |
| Simple interaction | Visual node connections |

## 2. Architecture

### 2.1 System Layers

```
┌────────────────────────────────────────────┐
│  UI Layer (React)                          │
│  NodePalette | FlowCanvas | VariablePanel  │
│  NodeConfigPanel | CodeEditorPopup         │
├────────────────────────────────────────────┤
│  State Layer (Zustand)                     │
│  ElementData (enhanced) | WireData |       │
│  Flow Engine (topological executor)        │
├────────────────────────────────────────────┤
│  Compile Layer (Python)                    │
│  Visual Graph → .omni AST → React TSX      │
└────────────────────────────────────────────┘
```

### 2.2 Library: @xyflow/react v12

Already installed (`^12.11.0`). Replaces Rete.js for the visual scripting canvas.

**Mapping to our architecture:**

| React Flow Concept | Visual Scripting Mapping |
|-------------------|--------------------------|
| `Node` (React component) | Flow/Event/Action/Data node |
| `Handle` (port) | Flow In/Out + Data In/Out ports |
| `Edge` | Flow Edge (white, triangle arrow) / Data Edge (colored, circle dot) |
| `node.data` | `ElementData.props` + `config` |
| `useNodesData()` | Data flow propagation |
| `useHandleConnections()` | Connection state tracking |
| `onConnect` | → `addWire` (reuse existing Zustand store) |

### 2.3 Panel Changes

| Action | Panel | Reason |
|--------|-------|--------|
| **Remove** | BlockPalette | Replaced by NodePalette |
| **Remove** | StatePanel | Replaced by VariablePanel |
| **Remove** | ConnectorPanel | No longer needed |
| **Keep** | FunctionLibrary | fn definitions still text-based |
| **Keep** | ComponentPalette | Design mode unchanged |
| **Keep** | PropertyPanel | Inspector unchanged |
| **Add** | NodePalette | Right-click search popup |
| **Add** | VariablePanel | Visual variable management |
| **Add** | NodeConfigPanel | Click node → configure params |
| **Replace** | FlowCanvas (Rete) | FlowCanvas (@xyflow/react) |

## 3. Data Model

### 3.1 Enhanced ElementData

```typescript
interface Port {
  name: string;
  direction: "in" | "out";
  portType: "flow" | "data";       // NEW: distinguishes flow vs data
  type: string;                     // data type (i32, f64, bool, String, any, void)
  label?: string;
  configurable?: boolean;           // NEW: configurable on node
  defaultValue?: unknown;           // NEW: default value
}

interface ElementData {
  // ... existing fields ...
  nodeCategory: "flow" | "event" | "action" | "data" | "variable" | "code";  // NEW
  config?: Record<string, unknown>;  // NEW: node-specific config
  flowOrder?: number;                // NEW: execution order (topological)
}
```

### 3.2 WireData (unchanged)

WireData structure remains the same. The `portType` on ports distinguishes flow wires from data wires for rendering purposes.

## 4. Node Taxonomy

### 4.1 Flow Nodes (execution control)

| Node | Description | Flow Out |
|------|-------------|----------|
| `Branch` | if/else — one flow in, True/False out | 2 |
| `Sequence` | Execute N actions in order | N |
| `ForEach` | Loop over array | Loop body, Completed |
| `While` | Loop while condition true | Loop body, Completed |
| `Wait` | Delay execution | 1 (after delay) |
| `Switch` | Multi-branch on value | N |

### 4.2 Event Nodes (UI event sources)

| Node | Description | Data Out |
|------|-------------|----------|
| `OnPress` | Button click | event payload |
| `OnChange` | Input change | value |
| `OnHover` | Mouse hover | — |
| `OnMount` | Component mounted | — |

### 4.3 Action Nodes (state mutation)

| Node | Description | Config |
|------|-------------|--------|
| `SetVariable` | Set variable value | variable name, value/expression |
| `CallFunction` | Call fn from FunctionLibrary | fn name, args |
| `RunCodeBlock` | Execute a code block | code block reference |
| `UpdateUIProperty` | Change UI element property | element, property, value |

### 4.4 Data Nodes (computation)

| Node | Description | Inputs | Outputs |
|------|-------------|--------|---------|
| `GetVariable` | Read variable value | — | value |
| `Literal` | Constant value | — | value |
| `Add` / `Sub` / `Mul` / `Div` | Arithmetic | A, B | result |
| `Compare` | ==, !=, >, <, >=, <= | A, B | bool |
| `And` / `Or` / `Not` | Logic gates | A, B | bool |
| `Concat` | String concatenation | A, B | string |
| `ToString` / `ToNumber` | Type conversion | value | converted |

## 5. Wire Visualization

### 5.1 Flow Wires

- Color: `#9CA3AF` (gray)
- Width: 3px
- Style: solid with triangle arrowhead marker
- Port shape: triangle `▶`
- Semantics: execution order only

### 5.2 Data Wires

- Color: by data type (i32=blue, f64=green, bool=orange, String=pink, any=gray)
- Width: 2px
- Style: solid with circle dot marker
- Port shape: circle `●`
- Semantics: data value passing

### 5.3 Port Positions

```
        Data In (●)        Data Out (●)
             ↓                  ↓
    ┌────────────────────────────┐
    │  Flow In (▶)    Node  Flow Out (▶)  │
    └────────────────────────────┘
             ↑                  ↑
        Data In (●)        Data Out (●)
```

- Flow ports: left (in) / right (out) sides of node
- Data ports: top (in) / bottom (out) of node, OR left/right alongside flow ports

## 6. Node Palette (Right-Click Search)

### 6.1 Interaction

1. Right-click empty canvas → search popup appears at cursor position
2. Type to fuzzy-filter node list
3. Arrow keys to navigate, Enter to select, Escape to close
4. Node appears at click position

### 6.2 Search Index

All node types registered with: `{ name, category, keywords, icon }`. Fuzzy match against name + keywords. Results grouped by category.
Categories in search result order: Event → Action → Flow → Data → Code.

## 7. Flow Engine

### 7.1 Algorithm

Topological sort of flow-connected nodes starting from event nodes:

```
1. Find all Event nodes (execution roots)
2. For each Event node, follow flow-out edges to build execution DAG
3. Topological sort the DAG
4. Execute nodes in order:
   - Gather data inputs from connected Data edges
   - Execute node logic (configured operation)
   - Set data outputs
   - Follow flow-out to next node(s)
5. Handle Branch: evaluate condition → follow True or False flow edge
```

### 7.2 Execution Scope

- Execution is **component-local** (scoped to current component)
- Variables are scoped to the component
- Code blocks have access to all signals in the component
- Cross-component communication via props (not in Round 1-7)

## 8. 7-Round Implementation Plan

### Round 1: Flow Engine Core + @xyflow/react migration
- Replace FlowCanvas.tsx with @xyflow/react-based canvas
- Implement ElementData enhancements (nodeCategory, portType, config)
- Implement Flow Engine (topological sort executor)
- Dual-color wire rendering (flow vs data)
- Port shape differentiation (triangle vs circle)

### Round 2: Basic Nodes + NodePalette
- Event nodes: OnPress, OnChange
- Action nodes: SetVariable, RunCodeBlock
- Data nodes: GetVariable, Add/Sub/Mul/Div
- NodePalette (right-click search popup)
- Node registration system (defineNode)

### Round 3: Flow Control + Full Data Nodes
- Flow nodes: Branch, Sequence, ForEach
- Data nodes: Compare, And/Or/Not, Concat
- NodeConfigPanel (click to configure)
- Literal node

### Round 4: Variable System + UI Property Nodes
- VariablePanel (visual variable CRUD)
- GetVariable / SetVariable deep integration
- UpdateUIProperty node
- Variable drag-to-canvas → auto-create Get node

### Round 5: Code Block Deep Integration
- CodeBlockNode: auto-generate input ports from fn params
- Auto-generate output ports from return type
- CodeEditorPopup integration (edit code → ports update)
- Seamless visual-to-code handoff

### Round 6: Code Generation
- Visual Graph → .omni source compilation
- .omni → React TSX (extend existing react.py codegen)
- Bidirectional sync (edit source → update graph; edit graph → update source)

### Round 7: Real App Validation
- Rebuild Smart Task Board's interaction logic visually
- Compare: hand-written code vs visual graph generated code
- E2E testing via test-framework pipeline
- Performance benchmarks

## 9. Scope Boundaries

### In Scope (Round 1-7)
- Flow + Data visual nodes as specified
- Right-click search palette
- Flow execution engine
- Variable system
- Code block integration
- .omni code generation
- React TSX code generation

### Out of Scope (Future)
- State Machine graphs
- Visual debugger (breakpoints, step-through)
- Subgraphs / nested graphs
- Custom compound nodes (user-defined macros)
- Cross-component visual scripting
- Undo/redo for visual graph edits (already supported via existing undo system)

## 10. Compatibility Guarantees

- Existing `.omni` source compilation works unchanged
- Existing code blocks (signal/computed/action/store) continue to work
- Design mode (ComponentPalette) unchanged
- OmniDriver API maintains backward compatibility
- `window.__OMNI__` API extended with visual node methods, not replaced
- Existing `.omni.json` save/load format preserved
