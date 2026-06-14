import { bindTextContent, bindProperty, bindAttribute } from "./omniview-runtime";
import { dispose } from "./omniview-runtime";
import { h } from "./omniview-runtime";
import { onCleanup } from "./omniview-runtime";
import { render } from "./omniview-runtime";
import { signal } from "./omniview-runtime";
import { traceExecution, withTrace } from "./omniview-runtime";

// Component: Counter

/** Mount Counter into a container element. */
export function Counter(mountEl) {
    const count = signal(0, { id: "Counter.count", name: "count", type: "number" });

    const increment = () => {
    traceExecution("Counter.increment");
        traceExecution("Counter.increment.stmt0");
        count.set((count.get() + 1));
    };

    // Render view tree
    const __root = h("div", {}, h("h1", {}, "Counter"), h("span", { "data-testid": "counter" }, "Count: ", count), h("button", { onClick: increment, "data-testid": "btn-inc" }, "+1"));
    render(__root, mountEl);

    // Inject styles
    const __style = document.createElement("style");
    __style.textContent = ".container_Counter {\n  display: flex;\n  flex-direction: column;\n  gap: 16px;\n  padding: 32px;\n  align-items: center;\n  justify-content: center;\n  max-width: 400px;\n}\n.heading_Counter {\n  font-size: 28px;\n  font-weight: 700;\n}";
    document.head.appendChild(__style);
    onCleanup(() => __style.remove());

    onCleanup(() => dispose());
    return { count, increment };
}