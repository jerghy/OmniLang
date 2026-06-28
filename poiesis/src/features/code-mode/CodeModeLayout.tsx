import { FileExplorer } from "../../components/panels/FileExplorer";
import { StatePanel } from "../../components/panels/StatePanel";
import { FunctionLibrary } from "../../components/panels/FunctionLibrary";
import { BlockPalette } from "../../components/panels/BlockPalette";
import { CodePropertyPanel } from "../../components/panels/CodePropertyPanel";
import { ConnectorPanel } from "../../components/panels/ConnectorPanel";
import { ActivityBar, PanelContainer, type PanelId } from "../../components/panels/ActivityBar";
import { Toolbar } from "../../components/shell/Toolbar";
import { CodeToolbar } from "../../components/shell/CodeToolbar";
import { StatusBar } from "../../components/shell/StatusBar";
import { S } from "../../components/shell/theme";
import { FlowCanvas } from "../../components/canvas/code/FlowCanvas";
import { CodeTabBar } from "../../components/canvas/code/CodeTabBar";
import { DebugPanel } from "../../components/canvas/DebugPanel";
import type { CommandContext } from "../../core/commands/types";
import type { DebugState } from "../compiler/useRun";

export interface CodeModeLayoutProps {
  ctx: CommandContext;
  mode: "design" | "code";
  toolbarProps: {
    isDirty: boolean;
    projectDir: string | null;
    runStatus: { running: boolean; url: string | null };
    runLoading: boolean;
    status: { text: string; isError: boolean };
    compileErrors: Array<{ code: string; message: string; line: number }>;
    projectName: string;
    onModeChange: (m: "design" | "code") => void;
    onUndo: () => void;
    onRedo: () => void;
    onSave: () => void;
    onRun: () => void;
    onStop: () => void;
    onCompile: () => void;
  };
  activePanel: PanelId;
  showConnector: boolean;
  target: string;
  debugState: DebugState;
  fnToNodeMap: Record<string, string>;
  highlightedWireId: string | null;
  layoutTrigger: number;
  codeEditorRef: React.RefObject<{ insertSnippet: (code: string) => void } | null>;
  onActivePanelChange: (id: PanelId) => void;
  onTargetChange: (t: string) => void;
  onToggleConnector: () => void;
  onAutoLayout: () => void;
  onNodeDoubleClick: (id: string, el: import("../../store/editor").ElementData) => void;
  onClearDebug: () => void;
  onToggleBreakpoint: (signalId: string) => void;
  onStep: () => void;
  onResume: () => void;
  onFnDoubleClick: (fnId: string, fnData: import("../../components/panels/FunctionLibrary").FnEntry) => void;
}

export function CodeModeLayout({
  ctx,
  mode,
  toolbarProps,
  activePanel,
  showConnector,
  target,
  debugState,
  highlightedWireId,
  layoutTrigger,
  codeEditorRef,
  onActivePanelChange,
  onTargetChange,
  onToggleConnector,
  onAutoLayout,
  onNodeDoubleClick,
  onClearDebug,
  onToggleBreakpoint,
  onStep,
  onResume,
  onFnDoubleClick,
}: CodeModeLayoutProps) {
  return (
    <>
      <ActivityBar
        activePanel={activePanel}
        onToggle={(id) => onActivePanelChange(activePanel === id ? null : id)}
      />
      <PanelContainer visible={activePanel === "files"}>
        <FileExplorer />
      </PanelContainer>
      <PanelContainer visible={activePanel === "state"}>
        <StatePanel ctx={ctx} />
      </PanelContainer>
      <PanelContainer visible={activePanel === "functions"}>
        <FunctionLibrary ctx={ctx} onFnDoubleClick={onFnDoubleClick} />
        <BlockPalette ctx={ctx} />
      </PanelContainer>
      <main style={S.main}>
        <Toolbar
          mode={mode}
          isDirty={toolbarProps.isDirty}
          projectDir={toolbarProps.projectDir}
          runStatus={toolbarProps.runStatus}
          runLoading={toolbarProps.runLoading}
          onModeChange={toolbarProps.onModeChange}
          onUndo={toolbarProps.onUndo}
          onRedo={toolbarProps.onRedo}
          onSave={toolbarProps.onSave}
          onRun={toolbarProps.onRun}
          onStop={toolbarProps.onStop}
          onCompile={toolbarProps.onCompile}
          compileLabel="编译 (Ctrl+Enter)"
        >
          <CodeToolbar
            target={target}
            showConnector={showConnector}
            projectDir={toolbarProps.projectDir}
            onTargetChange={onTargetChange}
            onToggleConnector={onToggleConnector}
            onAutoLayout={onAutoLayout}
          />
        </Toolbar>
        <CodeTabBar />
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          <div style={S.codeArea}>
            <FlowCanvas
              highlightedWireId={highlightedWireId}
              layoutTrigger={layoutTrigger}
              activeNodeIds={debugState.activeNodeIds}
              activeEdgeIds={debugState.activeEdgeIds}
              onNodeDoubleClick={onNodeDoubleClick}
            />
            {toolbarProps.runStatus.running && (
              <DebugPanel
                connected={debugState.debugConnected}
                stateChanges={debugState.debugStateChanges}
                onClear={onClearDebug}
                onToggleBreakpoint={onToggleBreakpoint}
                breakpoints={debugState.debugBreakpoints}
                paused={debugState.debugPaused}
                onStep={onStep}
                onResume={onResume}
              />
            )}
          </div>
          <aside
            style={{
              width: 260,
              minWidth: 220,
              flexShrink: 0,
              borderLeft: "1px solid #e5e7eb",
              background: "#f9fafb",
              overflow: "auto",
            }}
          >
            <div
              style={{
                padding: "4px 8px",
                fontSize: 10,
                fontWeight: 600,
                color: "#9CA3AF",
                borderBottom: "1px solid #e5e7eb",
              }}
            >
              节点属性
            </div>
            <CodePropertyPanel />
          </aside>
          {showConnector && (
            <aside
              style={{
                width: 280,
                minWidth: 240,
                flexShrink: 0,
                borderLeft: "1px solid #e5e7eb",
                background: "#f9fafb",
                overflow: "hidden",
              }}
            >
              <ConnectorPanel
                onInsertCode={(code) => codeEditorRef.current?.insertSnippet(code)}
              />
            </aside>
          )}
        </div>
        <StatusBar
          status={toolbarProps.status}
          isDirty={toolbarProps.isDirty}
          projectName={toolbarProps.projectName}
          projectDir={toolbarProps.projectDir}
          compileErrors={toolbarProps.compileErrors}
        />
      </main>
    </>
  );
}
