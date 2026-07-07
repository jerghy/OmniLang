import { useEffect, useRef } from "react";
import { usePoiesisStore } from "../../store";
import { parseAndPopulate } from "../../services/parser/source-parser";

/** Simple non-cryptographic string hash for content change detection. */
function simpleHash(s: string): string {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    hash = ((hash << 5) - hash) + s.charCodeAt(i);
    hash |= 0;
  }
  return hash.toString(36);
}

export function useAutoCompile(
  handleCompile: () => void,
  setCompileErrors: (errors: unknown[]) => void,
  setStatus: (s: { text: string; isError: boolean }) => void,
) {
  const activeFilePath = usePoiesisStore((s) => s.activeFilePath);
  const activeFileContent = usePoiesisStore((s) => s.activeFileContent);
  const prevFilePath = useRef<string | null>(null);
  const prevContentHashRef = useRef<string>("");
  const pendingTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const currentPath = usePoiesisStore.getState().activeFilePath;
    const currentContent = usePoiesisStore.getState().activeFileContent;
    const currentHash = simpleHash(currentContent || "");

    if (
      currentPath &&
      currentContent &&
      (currentPath !== prevFilePath.current || currentHash !== prevContentHashRef.current) &&
      !pendingTimerRef.current
    ) {
      prevFilePath.current = currentPath;
      prevContentHashRef.current = currentHash;
      pendingTimerRef.current = setTimeout(() => {
        pendingTimerRef.current = null;
        if (currentPath.endsWith(".omni")) {
          setStatus({ text: "解析中...", isError: false });
          const result = parseAndPopulate(currentContent, setCompileErrors, setStatus);
          if (result) handleCompile();
        }
      }, 150);
    }

    return () => {
      if (pendingTimerRef.current) {
        clearTimeout(pendingTimerRef.current);
        pendingTimerRef.current = null;
      }
    };
  }, [activeFilePath, activeFileContent, handleCompile, setCompileErrors, setStatus]);
}
