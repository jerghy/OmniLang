# Phase 0 Task 3: Delete Deprecated Panels and Services

## Status: COMPLETE

## File Count Deleted: 81

### Breakdown by group

| Group | Count | Files |
|-------|-------|-------|
| Panel components | 22 | BlockPalette, CodePropertyPanel, StatePanel, FunctionLibrary, ConnectorPanel, SnippetPanel, TokenPanel, PropertyPanel, StyleDesignModal, StylePropertyEditor, StyleValueEditor, CssVariableEditor, ClassCard, ClassPreview, tree-utils, TreeNodeRenderer, ConflictPanel, EventCreationPanel, ReadOnlyBanner, AnimationImport, CanvasPanel, ProjectWelcome |
| Inspector sections | 5 | ComponentMetaSection, DataSection, EventSection, LayoutSection, StyleSection |
| Canvas shared/code/design | 22 | fnPortGenerator, socketTypes, connectionValidators, nodeRegistry, elementsToNodes, wiresToEdges, FlowEngine, layoutUtils, CodeBlockNode, BaseNode, CodeTabBar, FlowCanvas, CanvasTabBar, LayoutView, CanvasPreview, ComponentCanvasHeader, NodePalette, DebugPanel, PerfBadge, PerfMonitor, StateHUD, VariablePanel |
| Codegraph services | 4 | store, types, context, migration |
| Feature layouts | 4 | DesignModeLayout, useDesignKeyboard, CodeModeLayout, useCodeKeyboard |
| Old stores | 16 | undo, undoPersistence, componentRegistry (store/), elements, wires, selection, project, tabs, undo, componentRegistry (core/state/), codegraph, codegraph-context, codegraph-migration, styleRegistry |
| Misc | 10 | debug-service, omni-runtime, omniview-runtime, run-service, ContrastChecker, FocusOrder, ErrorBoundary, SearchModal, CodeToolbar, DesignToolbar |

### Files that couldn't be deleted: None

All 81 files were successfully removed.

### Verification

- `tsc --noEmit` reports **179 errors** (all expected -- references from remaining files to deleted modules)
- **Zero errors reference deleted files themselves** -- all errors originate in remaining files (App.tsx, ComponentPalette.tsx, ComponentTree.tsx, StatusBar.tsx, etc.) that imported from the now-deleted modules
- Empty directories were pruned with `find . -type d -empty -delete`
