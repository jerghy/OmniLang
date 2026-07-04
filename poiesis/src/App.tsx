import { usePoiesisStore } from "./store";
import { ArchitecturePage } from "./pages/ArchitecturePage";
import { DesignPage } from "./pages/DesignPage";
import { StatusBar } from "./components/shell/StatusBar";

export default function App() {
  const currentPage = usePoiesisStore((s) => s.currentPage);
  const isDirty = usePoiesisStore((s) => s.isDirty);
  const projectName = usePoiesisStore((s) => s.projectName);
  const projectDir = usePoiesisStore((s) => s.projectDir);
  const compileErrors = usePoiesisStore((s) => s.compileErrors);

  const statusText =
    compileErrors.length > 0
      ? `${compileErrors.length} 个错误`
      : "就绪";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        overflow: "hidden",
      }}
    >
      {/* Page content */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        {currentPage === "architecture" ? <ArchitecturePage /> : <DesignPage />}
      </div>

      {/* Status bar at bottom */}
      <StatusBar
        status={{ text: statusText, isError: compileErrors.length > 0 }}
        isDirty={isDirty}
        projectName={projectName || "未命名项目"}
        projectDir={projectDir}
        compileErrors={compileErrors}
      />
    </div>
  );
}
