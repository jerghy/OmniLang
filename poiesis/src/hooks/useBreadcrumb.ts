// hooks/useBreadcrumb.ts
// Hook managing breadcrumb state synced with the store.
// Provides helpers to push, pop, and reset breadcrumbs based on navigation.

import { useCallback } from "react";
import { usePoiesisStore } from "../store";
import type { Breadcrumb } from "../core/types/design-page";

export function useBreadcrumb() {
  const breadcrumbs = usePoiesisStore((s) => s.designPage.breadcrumbs);
  const setBreadcrumbs = usePoiesisStore((s) => s.setBreadcrumbs);
  const navigateTo = usePoiesisStore((s) => s.navigateTo);

  const pushCrumb = useCallback(
    (crumb: Breadcrumb) => {
      setBreadcrumbs([...breadcrumbs, crumb]);
    },
    [breadcrumbs, setBreadcrumbs]
  );

  const popTo = useCallback(
    (index: number) => {
      const trimmed = breadcrumbs.slice(0, index + 1);
      setBreadcrumbs(trimmed);
      const last = trimmed[trimmed.length - 1];
      if (last?.type === "architecture") {
        navigateTo("architecture");
      }
    },
    [breadcrumbs, setBreadcrumbs, navigateTo]
  );

  const resetToArchitecture = useCallback(() => {
    setBreadcrumbs([
      { type: "architecture", label: "架构", id: "arch" },
    ]);
    navigateTo("architecture");
  }, [setBreadcrumbs, navigateTo]);

  const currentCrumb = breadcrumbs[breadcrumbs.length - 1] ?? null;

  return {
    breadcrumbs,
    pushCrumb,
    popTo,
    resetToArchitecture,
    currentCrumb,
  };
}
