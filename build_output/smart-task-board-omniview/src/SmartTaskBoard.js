import { bindTextContent, bindProperty, bindAttribute } from "./omniview-runtime";
import { dispose } from "./omniview-runtime";
import { h } from "./omniview-runtime";
import { onCleanup } from "./omniview-runtime";
import { render } from "./omniview-runtime";
import { signal } from "./omniview-runtime";
import { traceExecution, withTrace } from "./omniview-runtime";

// Component: SmartTaskBoard

/** Mount SmartTaskBoard into a container element. */
export function SmartTaskBoard(mountEl) {
    const totalTasks = signal(12, { id: "SmartTaskBoard.totalTasks", name: "totalTasks", type: "number" });
    const todoCount = signal(5, { id: "SmartTaskBoard.todoCount", name: "todoCount", type: "number" });
    const inProgressCount = signal(3, { id: "SmartTaskBoard.inProgressCount", name: "inProgressCount", type: "number" });
    const doneCount = signal(4, { id: "SmartTaskBoard.doneCount", name: "doneCount", type: "number" });
    const searchQuery = signal("", { id: "SmartTaskBoard.searchQuery", name: "searchQuery", type: "string" });
    const selectedCategory = signal("all", { id: "SmartTaskBoard.selectedCategory", name: "selectedCategory", type: "string" });

    const handleSearch = () => {
    traceExecution("SmartTaskBoard.handleSearch");
        traceExecution("SmartTaskBoard.handleSearch.stmt0");
        searchQuery.set(searchQuery.get());
    };
    const handleFilterAll = () => {
    traceExecution("SmartTaskBoard.handleFilterAll");
        traceExecution("SmartTaskBoard.handleFilterAll.stmt0");
        selectedCategory.set("all");
    };
    const handleFilterBug = () => {
    traceExecution("SmartTaskBoard.handleFilterBug");
        traceExecution("SmartTaskBoard.handleFilterBug.stmt0");
        selectedCategory.set("bug");
    };
    const handleFilterFeature = () => {
    traceExecution("SmartTaskBoard.handleFilterFeature");
        traceExecution("SmartTaskBoard.handleFilterFeature.stmt0");
        selectedCategory.set("feature");
    };
    const handleFilterDocs = () => {
    traceExecution("SmartTaskBoard.handleFilterDocs");
        traceExecution("SmartTaskBoard.handleFilterDocs.stmt0");
        selectedCategory.set("docs");
    };
    const handleRefresh = () => {
    traceExecution("SmartTaskBoard.handleRefresh");
        traceExecution("SmartTaskBoard.handleRefresh.stmt0");
        totalTasks.set(12);
        traceExecution("SmartTaskBoard.handleRefresh.stmt1");
        todoCount.set(5);
        traceExecution("SmartTaskBoard.handleRefresh.stmt2");
        inProgressCount.set(3);
        traceExecution("SmartTaskBoard.handleRefresh.stmt3");
        doneCount.set(4);
        traceExecution("SmartTaskBoard.handleRefresh.stmt4");
        searchQuery.set("");
        traceExecution("SmartTaskBoard.handleRefresh.stmt5");
        selectedCategory.set("all");
    };
    const handleAddTask = () => {
    traceExecution("SmartTaskBoard.handleAddTask");
        traceExecution("SmartTaskBoard.handleAddTask.stmt0");
        if ((selectedCategory.get() == "all")) {
        traceExecution("SmartTaskBoard.handleAddTask.stmt0");
        todoCount.set((todoCount.get() + 1));
        traceExecution("SmartTaskBoard.handleAddTask.stmt1");
        totalTasks.set((totalTasks.get() + 1));
        } else {
        traceExecution("SmartTaskBoard.handleAddTask.stmt0");
        todoCount.set((todoCount.get() + 1));
        traceExecution("SmartTaskBoard.handleAddTask.stmt1");
        totalTasks.set((totalTasks.get() + 1));
        }
    };
    const handleQuickAdd = () => {
    traceExecution("SmartTaskBoard.handleQuickAdd");
        traceExecution("SmartTaskBoard.handleQuickAdd.stmt0");
        if ((selectedCategory.get() == "bug")) {
        traceExecution("SmartTaskBoard.handleQuickAdd.stmt0");
        todoCount.set((todoCount.get() + 1));
        traceExecution("SmartTaskBoard.handleQuickAdd.stmt1");
        totalTasks.set((totalTasks.get() + 1));
        } else {
        traceExecution("SmartTaskBoard.handleQuickAdd.stmt0");
        todoCount.set((todoCount.get() + 1));
        traceExecution("SmartTaskBoard.handleQuickAdd.stmt1");
        totalTasks.set((totalTasks.get() + 1));
        }
    };

    // Render view tree
    const __root = h("div", { padding: "0", background: "#f5f5f5", "data-testid": "root" }, h("div", { direction: "row", gap: "16px", padding: "16px 32px", background: "#1e293b", width: "100%", "data-testid": "header" }, h("h1", { "data-testid": "app-title" }, "Smart Task Board"), h("div", { flex: "1" }), h("button", { variant: "ghost", "data-testid": "btn-settings" }, "Settings"), h("button", { variant: "outline", onClick: handleRefresh, "data-testid": "btn-refresh" }, "Refresh")), h("div", { direction: "row", gap: "16px", padding: "24px 32px", width: "100%", "data-testid": "stats-row" }, h("div", { padding: "20px", background: "#ffffff", flex: "1", borderRadius: "12px", "data-testid": "stat-total" }, h("span", { color: "#64748b", fontSize: "14px", fontWeight: "500" }, "Total Tasks"), h("h1", { "data-testid": "stat-total-value" }, totalTasks)), h("div", { padding: "20px", background: "#eff6ff", flex: "1", borderRadius: "12px", "data-testid": "stat-todo" }, h("span", { color: "#3b82f6", fontSize: "14px", fontWeight: "500" }, "Todo"), h("h1", { "data-testid": "stat-todo-value" }, todoCount)), h("div", { padding: "20px", background: "#fef3c7", flex: "1", borderRadius: "12px", "data-testid": "stat-progress" }, h("span", { color: "#f59e0b", fontSize: "14px", fontWeight: "500" }, "In Progress"), h("h1", { "data-testid": "stat-progress-value" }, inProgressCount)), h("div", { padding: "20px", background: "#f0fdf4", flex: "1", borderRadius: "12px", "data-testid": "stat-done" }, h("span", { color: "#22c55e", fontSize: "14px", fontWeight: "500" }, "Done"), h("h1", { "data-testid": "stat-done-value" }, doneCount))), h("div", { direction: "row", gap: "12px", padding: "16px 32px", width: "100%", background: "#ffffff", borderBottom: "1px solid #e2e8f0", "data-testid": "filter-bar" }, h("input", { placeholder: "Search tasks...", type: "search", flex: "2", "data-testid": "search-input" }), h("button", { variant: "primary", onClick: handleFilterAll, "data-testid": "filter-all" }, "All"), h("button", { variant: "outline", onClick: handleFilterBug, "data-testid": "filter-bug" }, "Bug"), h("button", { variant: "outline", onClick: handleFilterFeature, "data-testid": "filter-feature" }, "Feature"), h("button", { variant: "outline", onClick: handleFilterDocs, "data-testid": "filter-docs" }, "Docs"), h("div", { flex: "1" }), h("button", { variant: "primary", onClick: handleAddTask, "data-testid": "btn-new-task" }, "+ New Task")), h("div", { direction: "row", gap: "16px", padding: "24px 32px", width: "100%", flex: "1", "data-testid": "kanban-row" }, h("div", { padding: "16px", background: "#f8fafc", flex: "1", borderRadius: "12px", border: "1px solid #e2e8f0", "data-testid": "column-todo" }, h("div", { direction: "row", gap: "8px", padding: "0 0 12px 0" }, h("span", { color: "#3b82f6", fontSize: "16px", fontWeight: "600" }, "📋 TODO"), h("span", { background: "#3b82f6", color: "#ffffff", fontSize: "12px", fontWeight: "600", padding: "2px 8px", borderRadius: "12px" }, "5")), h("div", { padding: "12px", background: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0", "data-testid": "task-1" }, h("span", { fontSize: "14px", fontWeight: "500", color: "#1e293b" }, "Fix login page bug"), h("div", { direction: "row", gap: "8px", padding: "8px 0 0 0" }, h("span", { background: "#fee2e2", color: "#ef4444", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "bug"), h("span", { background: "#fef3c7", color: "#f59e0b", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "P0"), h("div", { flex: "1" }), h("button", { variant: "ghost" }, "→"))), h("div", { padding: "12px", background: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0", "data-testid": "task-2" }, h("span", { fontSize: "14px", fontWeight: "500", color: "#1e293b" }, "Update API docs"), h("div", { direction: "row", gap: "8px", padding: "8px 0 0 0" }, h("span", { background: "#dbeafe", color: "#3b82f6", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "docs"), h("span", { background: "#fef3c7", color: "#f59e0b", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "P1"), h("div", { flex: "1" }), h("button", { variant: "ghost" }, "→")))), h("div", { padding: "16px", background: "#f8fafc", flex: "1", borderRadius: "12px", border: "1px solid #e2e8f0", "data-testid": "column-progress" }, h("div", { direction: "row", gap: "8px", padding: "0 0 12px 0" }, h("span", { color: "#f59e0b", fontSize: "16px", fontWeight: "600" }, "🚀 IN PROGRESS"), h("span", { background: "#f59e0b", color: "#ffffff", fontSize: "12px", fontWeight: "600", padding: "2px 8px", borderRadius: "12px" }, "3")), h("div", { padding: "12px", background: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0", "data-testid": "task-3" }, h("span", { fontSize: "14px", fontWeight: "500", color: "#1e293b" }, "Implement dark mode"), h("div", { direction: "row", gap: "8px", padding: "8px 0 0 0" }, h("span", { background: "#dcfce7", color: "#22c55e", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "feature"), h("span", { background: "#fef3c7", color: "#f59e0b", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "P1"), h("div", { flex: "1" }), h("button", { variant: "ghost" }, "→"))), h("div", { padding: "12px", background: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0", "data-testid": "task-4" }, h("span", { fontSize: "14px", fontWeight: "500", color: "#1e293b" }, "Add export CSV feature"), h("div", { direction: "row", gap: "8px", padding: "8px 0 0 0" }, h("span", { background: "#dcfce7", color: "#22c55e", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "feature"), h("span", { background: "#e2e8f0", color: "#64748b", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "P2"), h("div", { flex: "1" }), h("button", { variant: "ghost" }, "→")))), h("div", { padding: "16px", background: "#f8fafc", flex: "1", borderRadius: "12px", border: "1px solid #e2e8f0", "data-testid": "column-done" }, h("div", { direction: "row", gap: "8px", padding: "0 0 12px 0" }, h("span", { color: "#22c55e", fontSize: "16px", fontWeight: "600" }, "✅ DONE"), h("span", { background: "#22c55e", color: "#ffffff", fontSize: "12px", fontWeight: "600", padding: "2px 8px", borderRadius: "12px" }, "4")), h("div", { padding: "12px", background: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0", "data-testid": "task-5" }, h("span", { fontSize: "14px", fontWeight: "500", color: "#1e293b" }, "Setup CI/CD pipeline"), h("div", { direction: "row", gap: "8px", padding: "8px 0 0 0" }, h("span", { background: "#f3e8ff", color: "#a855f7", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "infra"), h("span", { background: "#fef3c7", color: "#f59e0b", fontSize: "11px", fontWeight: "600", padding: "2px 8px", borderRadius: "4px" }, "P0"), h("div", { flex: "1" }), h("button", { variant: "ghost" }, "✕"))))), h("div", { direction: "row", gap: "12px", padding: "20px 32px", width: "100%", background: "#ffffff", borderTop: "1px solid #e2e8f0", "data-testid": "quick-add" }, h("span", { fontSize: "14px", fontWeight: "600", color: "#475569" }, "Quick Add:"), h("input", { placeholder: "New task title...", flex: "3", "data-testid": "quick-add-title" }), h("input", { placeholder: "Category...", flex: "1", "data-testid": "quick-add-category" }), h("button", { variant: "primary", onClick: handleQuickAdd, "data-testid": "btn-quick-add" }, "Add Task")));
    render(__root, mountEl);

    onCleanup(() => dispose());
    return { totalTasks, todoCount, inProgressCount, doneCount, searchQuery, selectedCategory, handleSearch, handleFilterAll, handleFilterBug, handleFilterFeature, handleFilterDocs, handleRefresh, handleAddTask, handleQuickAdd };
}