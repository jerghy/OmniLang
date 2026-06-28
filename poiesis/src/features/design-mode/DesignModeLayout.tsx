import { useState } from "react";
import { useEditorStore } from "../../store/editor";
import { ComponentPalette } from "../../components/panels/ComponentPalette";
import { ComponentTree } from "../../components/panels/ComponentTree";
import { CanvasPanel } from "../../components/panels/CanvasPanel";
import { TokenPanel } from "../../components/panels/TokenPanel";
import { PropertyPanel } from "../../components/panels/PropertyPanel";
import { ActivityBar, PanelContainer, DESIGN_PANELS, type PanelId } from "../../components/panels/ActivityBar";
import { Toolbar } from "../../components/shell/Toolbar";
import { DesignToolbar } from "../../components/shell/DesignToolbar";
import { StyleDesignModal } from "../../components/panels/StyleDesignModal";
import { StatusBar } from "../../components/shell/StatusBar";
import { S } from "../../components/shell/theme";
import { LayoutView } from "../../components/canvas/design/LayoutView";
import { CanvasTabBar } from "../../components/canvas/design/CanvasTabBar";
import { StateHUD } from "../../components/canvas/StateHUD";
import { collectFocusOrder, focusOverlayStyles } from "../../a11y/FocusOrder";
import type { CommandContext } from "../../core/commands/types";

export interface DesignModeLayoutProps {
  ctx: CommandContext;
  mode: "design" | "code";
  toolbarProps: {
    isDirty: boolean;
    projectDir: string | null;
    runStatus: { running: boolean; url: string | null };
    runLoading: boolean;
    status: { text: string; isError: boolean };
    showPerfBadge?: boolean;
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
  breakpoint: "desktop" | "tablet" | "mobile";
  previewState: string;
  wireMode: boolean;
  showTokens: boolean;
  showHUD: boolean;
  showFocusOrder: boolean;
  showSnippets: boolean;
  showConnector: boolean;
  activePanel: PanelId;
  onBreakpointChange: (b: "desktop" | "tablet" | "mobile") => void;
  onPreviewStateChange: (s: string) => void;
  onToggleTokens: () => void;
  onToggleWireMode: () => void;
  onToggleHUD: () => void;
  onToggleFocusOrder: () => void;
  onOpenProject: () => void;
  onActivePanelChange: (id: PanelId) => void;
  selectedId: string | null;
  handleNavigateToCode: (wireId: string) => void;
}

export function DesignModeLayout({
  ctx,
  mode,
  toolbarProps,
  breakpoint,
  previewState,
  wireMode,
  showTokens,
  showHUD,
  showFocusOrder,
  activePanel,
  onBreakpointChange,
  onPreviewStateChange,
  onToggleTokens,
  onToggleWireMode,
  onToggleHUD,
  onToggleFocusOrder,
  onOpenProject,
  onActivePanelChange,
  selectedId,
  handleNavigateToCode,
}: DesignModeLayoutProps) {
  const [showStyleModal, setShowStyleModal] = useState(false);
  return (
    <>
      <ActivityBar
        panels={DESIGN_PANELS}
        activePanel={activePanel}
        onToggle={(id) => onActivePanelChange(activePanel === id ? null : id)}
      />
      <PanelContainer visible={activePanel === "components" || activePanel === "tree" || activePanel === "canvas"}>
        {activePanel === "components" && <ComponentPalette />}
        {activePanel === "tree" && <ComponentTree ctx={ctx} />}
        {activePanel === "canvas" && <CanvasPanel />}
        {showTokens && activePanel !== null && (
          <div style={{ borderTop: "1px solid #e5e7eb" }}>
            <TokenPanel />
          </div>
        )}
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
          onOpenCssStyles={() => setShowStyleModal(true)}
        >
          <DesignToolbar
            breakpoint={breakpoint}
            previewState={previewState}
            showTokens={showTokens}
            wireMode={wireMode}
            showHUD={showHUD}
            showFocusOrder={showFocusOrder}
            onBreakpointChange={onBreakpointChange}
            onPreviewStateChange={onPreviewStateChange}
            onToggleTokens={onToggleTokens}
            onToggleWireMode={onToggleWireMode}
            onToggleHUD={onToggleHUD}
            onToggleFocusOrder={onToggleFocusOrder}
            onOpenProject={onOpenProject}
          />
        </Toolbar>
        <CanvasTabBar />
        <div style={{ display: "flex", justifyContent: "center", background: "#f0f0f0", flex: 1, overflow: "auto" }}>
          <div
            style={{
              width: breakpoint === "desktop" ? "100%" : breakpoint === "tablet" ? 768 : 375,
              maxWidth: "100%",
              background: "#fff",
              boxShadow: "0 0 20px rgba(0,0,0,0.1)",
              position: "relative",
              transition: "width 0.3s ease",
              display: "flex",
              flexDirection: "column",
            }}
          >
            <LayoutView ctx={ctx} hideCodeBlocks />
            {selectedId && (
              <StateHUD
                elementId={selectedId}
                visible={showHUD}
                onClose={onToggleHUD}
              />
            )}
            {showFocusOrder && <FocusOrderOverlay />}
          </div>
        </div>
        <StatusBar
          status={toolbarProps.status}
          isDirty={toolbarProps.isDirty}
          projectName={toolbarProps.projectName}
          projectDir={toolbarProps.projectDir}
          showPerfBadge
        />
      </main>
      <aside style={S.inspector}>
        <PropertyPanel
          ctx={ctx}
          onNavigateToCode={(wireId) => handleNavigateToCode(wireId)}
        />
      </aside>
      {showStyleModal && (
        <StyleDesignModal
          componentName="App"
          ctx={ctx}
          onClose={() => setShowStyleModal(false)}
        />
      )}
    </>
  );
}

// ── M5.4 Focus Order Overlay ──────────────────

function FocusOrderOverlay() {
  const elements = useEditorStore((s) => s.elements);
  const entries = collectFocusOrder(elements);

  return (
    <div style={{ position: "absolute", top: 0, left: 0, zIndex: 50, pointerEvents: "none" }}>
      {entries.map((entry) => {
        const styles = focusOverlayStyles(entry);
        return (
          <div key={entry.id} style={styles.container}>
            <div style={styles.box}>
              <span style={styles.label}>Tab {entry.index}</span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
