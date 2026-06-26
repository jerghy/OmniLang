## EVAL DEFINITION: omni-editor-capability-evals

### Eval Type: Capability Evals

Covers 9 core subsystems of the OmniLang Editor, defining what each must do correctly.

---

### E1: omni-gen Pipeline — Code Generation

**Priority:** P0 (Core)

The omni-gen pipeline must transform visual design elements + code tabs into compilable OmniLang source code.

**Success Criteria:**
- [ ] Phase A (ViewTreeBuilder): Converts flat `ElementData[]` into nested `ViewNode` tree with correct parent/child relationships
- [ ] Phase A (WireAnalyzer): Extracts signal bindings, store refs, computed deps from `WireData[]`
- [ ] Phase A (ImportCollector): Detects PascalCase tags and store refs, generates correct import statements
- [ ] Phase B (StoreExtractor): Separates signals into stores/standalone by parentStore attribute; groups actions/getters under correct store
- [ ] Phase B (ExpressionLowering): Transforms code block wire graphs into inline expressions; detects wire cycles
- [ ] Phase B (ChainEmitter): Emits chain blocks from code tab graphs with function bindings
- [ ] Phase C (StyleResolver): Resolves CSS rules per view node, generates scoped class names
- [ ] Phase C (AnnotationData): Builds annotation metadata for round-trip editing
- [ ] Phase C (SourceEmitter): Emits full `*.omni` document with imports, stores, signals, computeds, view tree, style blocks
- [ ] generateOmniFiles: Produces multi-file output (`src/main.omni` + `src/stores/*.omni`)
- [ ] Pipeline roundtrip: parse → generate → re-parse produces equivalent AST
- [ ] Fatal errors stop pipeline with clear error messages
- [ ] Non-fatal errors (warnings) collected and surfaced without aborting

**Expected Output:** Valid OmniLang source that can be parsed back and compiled to target (React/Flutter).

---

### E2: Canvas/Flow Editor — Visual Programming

**Priority:** P0 (Core)

The canvas subsystem must support node-based visual editing with drag-drop, wiring, and layout.

**Success Criteria:**
- [ ] FlowEngine.executeFlow: BFS from event nodes produces ordered execution steps with correct data input gathering
- [ ] FlowEngine.executeFlow: Detects unreachable code blocks (no path from any event root)
- [ ] FlowEngine.validateFlow: DFS cycle detection finds all flow cycles
- [ ] FlowEngine.validateFlow: Identifies orphan flow-in ports (action/event nodes with unconnected flow-in)
- [ ] Connection validators: Enforces port-type compatibility (flow→flow, data→data)
- [ ] Connection validators: Prevents invalid connections (output→output, incompatible types)
- [ ] Node palette: Lists available code blocks and view elements by category
- [ ] Layout engine: Auto-positions nodes with dagre layout (or equivalent)
- [ ] Data edges / flow edges: Renders different edge styles for data vs flow connections
- [ ] Wire overlay: Visual feedback during connection drag
- [ ] Multi-tab canvas: Switch between component/design canvases preserving state
- [ ] Element registration: New node types can be registered via node registry

**Expected Output:** Functional visual programming surface with drag-drop, wiring, cycle detection, and layout.

---

### E3: Command System — Undo/Redo

**Priority:** P0 (Core)

The command system must provide reliable undo/redo for all editor mutations.

**Success Criteria:**
- [ ] `Command` interface: All 9 command types implement execute/undo (AddElement, RemoveElement, AddWire, RemoveWire, SetProp, SetElementsAndWires, MoveElement, ReorderElement, CompositeCommand)
- [ ] AddElement: execute adds element to store; undo removes it (including from parent's children)
- [ ] RemoveElement: execute snapshots element data (plus children) then removes; undo restores snapshot
- [ ] RemoveElement: no-op when element does not exist (idempotent guard)
- [ ] SetProp: execute stores old value and sets new; undo restores old value
- [ ] CopyPaste: deep-clones elements with new IDs, preserving parent/child structure and wires
- [ ] CompositeCommand: batches multiple commands with single undo
- [ ] UndoStore: caps undo stack at 200 entries (memory bound)
- [ ] UndoStore: clears redo stack on new execute (standard UX)
- [ ] UndoStore.undo: no-op on empty stack (no crash)
- [ ] UndoStore.redo: no-op on empty stack (no crash)
- [ ] UndoStore.clear: clears both stacks (on project switch)
- [ ] moveElement: updates parentId and both source/target children arrays atomically

**Expected Output:** Reliable undo/redo across all editor operations with proper memory management.

---

### E4: Editor Store (Zustand) — State Management

**Priority:** P0 (Core)

The Zustand store must correctly manage editor state for design mode, code mode, project lifecycle, and tab system.

**Success Criteria:**
- [ ] addElement: Normalizes props; updates parent's children array; updates rootOrder for root elements
- [ ] removeElement: Removes element from elements array; updates parent's children; removes descendant elements
- [ ] setProp: Immutable update with spread; triggers isDirty = true
- [ ] setElementsAndWires: Atomically replaces all elements/wires; clears selection
- [ ] selectElement: null clears selection; multi toggles element in selection set
- [ ] selectAll / clearSelection: Selects all / selects none
- [ ] addWire / removeWire: Adds/removes wire; triggers isDirty = true
- [ ] reorderElement: Handles both root-level (rootOrder) and container-level (parent.children) reordering
- [ ] reorderElement: Guard for mismatched parentIds (cannot reorder across different parents)
- [ ] moveElement: Updates parentId; modifies old parent's children and new parent's children
- [ ] setBlockValue: No-op (returns same state) when value hasn't changed
- [ ] newProject: Resets all state to defaults (projectPath, elements, wires, tabs, selection)
- [ ] openProject: Restores design elements, wires, config, project name, rootOrder
- [ ] Tab system: switchTab saves current canvas state; restores target tab state
- [ ] Tab system: closeTab removes tab from openTabs; deletes tabState; returns to backup state
- [ ] Component canvas: enterComponentCanvas saves backup and loads component definition
- [ ] Component canvas: exitComponentCanvas saves component state and restores backup
- [ ] isDirty: Set to true on mutations; markClean sets to false
- [ ] Exposed on window.__ZUSTAND_EDITOR_STORE__ for E2E testing
- [ ] Config: target and autoSave persistable via setConfig

**Expected Output:** Correct state transitions for all editor operations, no stale or orphaned state.

---

### E5: API Client — Server Communication

**Priority:** P1 (Integration)

The API client must communicate with the Python backend over HTTP and WebSocket.

**Success Criteria:**
- [ ] HTTP compile: POST `/api/compile` with source+target returns CompileResponse
- [ ] File system: listDir, readFile, writeFile return correct entries with error handling
- [ ] Project info: GET `/api/project` returns ProjectInfo (dir, target, mainFile)
- [ ] OmniJson export: POST `/api/omni-json/export` produces valid .omni.json document
- [ ] OmniJson load: POST `/api/omni-json/load` restores document from file path
- [ ] OmniJson save: POST `/api/omni-json/save` writes document to disk
- [ ] Hand-written detection: POST `/api/omni-json/detect-handwritten` returns isHandWritten + warnings
- [ ] Component definitions: save/load component definitions to server
- [ ] Run bridge: startRun, stopRun, getRunStatus, writeRunFiles for runtime preview
- [ ] WebSocket connect: Connects to `/api/watch`, receives "compiled" and "error" messages
- [ ] WebSocket auto-reconnect: Reconnects on close with 3s delay
- [ ] WebSocket heartbeat: Sends ping every 5s
- [ ] Debug WebSocket: Separate connection to `/api/debug/ws` for runtime debug bridge
- [ ] Annotation API: annotatedSave and annotatedLoad for editor annotation persistence
- [ ] Error handling: Returns `{ error: string }` on server errors, never throws raw

**Expected Output:** Reliable client-server communication with auto-reconnect and full endpoint coverage.

---

### E6: OmniDriver — Programmatic Editor API

**Priority:** P1 (Integration)

OmniDriver must provide a Playwright-like API for AI/test scripts to control the editor programmatically.

**Success Criteria:**
- [ ] component(name, props): Creates a component Node (view element) with given tag and props
- [ ] codeBlock(type, source): Creates a code block with tag, source, and ports
- [ ] findByTag(tag): Returns ElementHandles matching tag
- [ ] findByProp(name, value): Returns ElementHandles matching prop value
- [ ] findById(id): Returns ElementHandle; null if not found
- [ ] ElementHandle.setProp(name, value): Updates prop via store.setProp
- [ ] ElementHandle.bindEvent(eventName, handler): Binds event to function handler
- [ ] ElementHandle.removeEvent(eventName): Removes event binding
- [ ] Wire creation: connect(fromId, fromPort, toId, toPort) adds wire to store
- [ ] Wire disconnection: disconnect(wireId) removes wire
- [ ] compile(): Calls server compile, returns CompileResult with errors/files
- [ ] exportOmniJson(): Exports current design as .omni.json
- [ ] getSnapshot(): Returns full EditorSnapshot (elements, wires, metadata)
- [ ] getStateDiff(): Returns StateDiff between two snapshots
- [ ] validate(): Returns ValidationReport (cycle detection, orphan ports, unreachable nodes)
- [ ] Undo/redo: Calls store undo/redo through the driver
- [ ] Component definitions: registerComponent, getComponent, listComponents
- [ ] Error handling: Methods return structured errors, never throw from store access
- [ ] Console access: Driver exposed on `window.__OMNI__` for browser console / E2E use

**Expected Output:** A complete programmatic interface to the editor, enabling AI agents and tests to drive the editor without UI interaction.

---

### E7: Panels — UI Side Panels

**Priority:** P1 (UI)

Editor panels must provide functional UI for property editing, component selection, and navigation.

**Success Criteria:**
- [ ] ActivityBar: Shows icon bar with panel toggles (file explorer, component palette, code, settings)
- [ ] BlockPalette: Lists available view elements (Text, Button, Image, etc.) with drag-to-canvas
- [ ] ComponentPalette: Lists registered custom components with drag-to-canvas; exposes COMPONENT_PROTOCOL for metadata
- [ ] FileExplorer: Lists project files; supports file open, directory expand/collapse
- [ ] PropertyPanel: Shows properties of selected element(s); supports type-based editors (text, number, boolean, select)
- [ ] CodePropertyPanel: Shows code block properties when code mode is active
- [ ] StatePanel: Shows signals, stores, and computeds for the current component
- [ ] TokenPanel: Debug view for internal token/annotation data
- [ ] EventCreationPanel: UI for binding events to function handlers
- [ ] SnippetPanel: Lists and inserts code snippets
- [ ] ConflictPanel: Shows merge conflicts during omni-json operations
- [ ] FunctionLibrary: Lists available functions with drag-to-graph
- [ ] ConnectorPanel: Shows connection wizard for wires
- [ ] AnimationImport: Panel for importing animation definitions
- [ ] ReadOnlyBanner: Shows banner when editing auto-generated code
- [ ] Error states: Panels gracefully handle missing selection, empty data, server errors

**Expected Output:** All panels render and function correctly; no blank panels or crash-on-empty states.

---

### E8: Editor Integration — Monaco / Code Editor

**Priority:** P2 (UX Enhancement)

The Monaco editor integration must provide syntax-aware editing for `.omni` files.

**Success Criteria:**
- [ ] MonacoEditor: Renders Monaco with correct language configuration for .omni files
- [ ] omniLangTokens: Provides Monarch tokenizer for OmniLang syntax (keywords: component, view, signal, store, etc.)
- [ ] omniCompletion: Provides completion provider for OmniLang keywords and component tags
- [ ] CodeEditorPopup: Shows inline popup for quick code edits (small code blocks)
- [ ] Syntax highlighting: Highlights keywords (component, view, signal, store, getter, action, @omni-file, @edit)
- [ ] Error markers: Shows compile error markers on the gutter and inline
- [ ] Read-only mode: Disables editing for auto-generated code with ReadOnlyBanner
- [ ] Responsive: Editor resizes with the panel; maintains scroll position on re-render
- [ ] Error handling: Gracefully handles Monaco load failure (fallback textarea)

**Expected Output:** Monaco editor with OmniLang syntax highlighting and completion; no Monaco dependency errors.

---

### E9: Search — Code Search

**Priority:** P2 (UX Enhancement)

Search subsystem must index and search OmniLang source files.

**Success Criteria:**
- [ ] SearchIndex: Builds searchable index from omni files (component names, signal names, tag names)
- [ ] SearchModal: Renders search dialog with keyboard shortcut (Ctrl+P / Cmd+P)
- [ ] IndexLoader: Loads index data from the server or pre-built index
- [ ] Results display: Shows results grouped by file/component with match preview
- [ ] Navigation: Selecting a result navigates to the file/component
- [ ] Error handling: Gracefully handles missing index or server errors
- [ ] Empty state: Shows "No results" message when query matches nothing

**Expected Output:** Fast code search across the project; no crash on empty index.

---

### E10: Server Backend (Python) — Compilation & File Management

**Priority:** P0 (Core)

The Python backend must compile OmniLang, manage files, and serve the editor.

**Success Criteria:**
- [ ] Compile endpoint: POST `/api/compile` compiles .omni source via WASM compiler; returns files + errors
- [ ] File operations: listDir, readFile, writeFile endpoints with path validation (no path traversal)
- [ ] Project info: GET `/api/project` returns project directory, target, main file
- [ ] OmniJson export: Converts .omni source → .omni.json document with sections (meta, design, code, config)
- [ ] OmniJson load: Converts .omni.json → in-memory representation for editor
- [ ] OmniJson save: Writes .omni.json to disk with atomic write (temp file + rename)
- [ ] Hand-written detection: Heuristic to distinguish AI-generated vs hand-written .omni source
- [ ] Watch: WebSocket push on file changes with debounced recompile
- [ ] Run bridge: Start/stop/manage Vite dev server for live preview
- [ ] Annotation endpoints: annotatedSave, annotatedLoad for annotation persistence
- [ ] Debug WebSocket: Bidirectional communication for runtime debugger
- [ ] Error handling: All endpoints return structured `{ error: string }` on failure
- [ ] Path safety: All file operations sanitize paths (no directory traversal, no absolute paths outside project)
- [ ] Performance: Compilation completes within 500ms for typical components; file ops within 100ms

**Expected Output:** Reliable backend with full CRUD for files, compilation, project persistence, and live preview.

---

### E11: Server Backend (Python) — OmniJson Subsystem

**Priority:** P0 (Core)

OmniJson must provide reliable project serialization/deserialization with merge support.

**Success Criteria:**
- [ ] Export: Serializes full document with meta, design (elements, wires), code (tabs), config, canvasLayout sections
- [ ] Load: Deserializes .omni.json into editor-compatible state (elements, wires, tabs, config)
- [ ] Import: Converts existing .omni.json into .omni source files (reverse of export)
- [ ] Merge: Three-way merge between auto-generated and hand-written .omni source
- [ ] Schema validation: Validates document structure against schema on load
- [ ] Wire validation: Validates wire connections (port existence, type compatibility)
- [ ] Round-trip: export → load → re-export produces equivalent document (ignoring timestamps)
- [ ] Error handling: Malformed documents produce clear error messages with line numbers
- [ ] Performance: Export/load completes within 200ms for typical project (100+ elements)
- [ ] Edge cases: Handles empty projects, projects with no wires, projects with only stores

**Expected Output:** .omni.json files that survive repeated export-load cycles without data loss or corruption.
