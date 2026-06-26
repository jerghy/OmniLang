import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Toaster } from "sonner";
import { ErrorBoundary } from "./ui/shared/ErrorBoundary";
import App from "./App";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <Toaster position="bottom-right" richColors closeButton />
      <App />
    </ErrorBoundary>
  </StrictMode>
);
