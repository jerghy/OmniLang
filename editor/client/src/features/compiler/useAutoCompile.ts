import { useEffect, useRef } from "react";
import { useEditorStore } from "../../store/editor";
import { parseAndPopulate } from "../../services/parser/source-parser";

export function useAutoCompile(
  handleCompile: () => void,
  setCompileErrors: (errors: unknown[]) => void,
  setStatus: (s: { text: string; isError: boolean }) => void,
) {
  const activeFilePath = useEditorStore((s) => s.activeFilePath);
  const activeFileContent = useEditorStore((s) => s.activeFileContent);
  const prevFilePath = useRef<string | null>(null);

  useEffect(() => {
    const currentPath = useEditorStore.getState().activeFilePath;
    const currentContent = useEditorStore.getState().activeFileContent;
    if (currentPath && currentContent && currentPath !== prevFilePath.current) {
      prevFilePath.current = currentPath;
      const timer = setTimeout(() => {
        if (currentPath.endsWith(".omni")) {
          setStatus({ text: "解析中...", isError: false });
          const result = parseAndPopulate(currentContent, setCompileErrors, setStatus);
          if (result) handleCompile();
        }
      }, 150);
      return () => clearTimeout(timer);
    }
  }, [activeFilePath, activeFileContent, handleCompile, setCompileErrors, setStatus]);
}
