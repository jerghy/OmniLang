// ── Theme slice for Poiesis store ──

import type { PoiesisState } from "../types";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export interface ThemeSlice {
  theme: "light" | "dark" | "system";
  resolvedTheme: "light" | "dark";
  setTheme: (theme: "light" | "dark" | "system") => void;
}

export function createThemeSlice(set: SetFn, _get?: GetFn): ThemeSlice {
  return {
    theme: (localStorage.getItem("poiesis-theme") as "light" | "dark" | "system") || "system",
    resolvedTheme: "light",
    setTheme: (theme) => {
      localStorage.setItem("poiesis-theme", theme);
      const resolved = theme === "system"
        ? (window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light")
        : theme;
      document.documentElement.setAttribute("data-theme", resolved);
      set({ theme, resolvedTheme: resolved } as any);
    },
  };
}
