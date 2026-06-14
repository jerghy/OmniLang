import React, { useCallback, useState } from "react";

export function SmartTaskBoard() {
    // .omni:1 -- component SmartTaskBoard

    const [totalTasks, setTotalTasks] = useState<number>(12);
    const [todoCount, setTodoCount] = useState<number>(5);
    const [inProgressCount, setInProgressCount] = useState<number>(3);
    const [doneCount, setDoneCount] = useState<number>(4);
    const [searchQuery, setSearchQuery] = useState<string>("");
    const [selectedCategory, setSelectedCategory] = useState<string>("all");
    const handleSearch = useCallback(() => {
        setSearchQuery(prev => prev);
    }, [searchQuery]);
    const handleFilterAll = useCallback(() => {
        setSelectedCategory("all");
    }, [selectedCategory]);
    const handleFilterBug = useCallback(() => {
        setSelectedCategory("bug");
    }, [selectedCategory]);
    const handleFilterFeature = useCallback(() => {
        setSelectedCategory("feature");
    }, [selectedCategory]);
    const handleFilterDocs = useCallback(() => {
        setSelectedCategory("docs");
    }, [selectedCategory]);
    const handleRefresh = useCallback(() => {
        setTotalTasks(12);
        setTodoCount(5);
        setInProgressCount(3);
        setDoneCount(4);
        setSearchQuery("");
        setSelectedCategory("all");
    }, [doneCount, inProgressCount, searchQuery, selectedCategory, todoCount, totalTasks]);
    const handleAddTask = useCallback(() => {
        if ((selectedCategory === "all")) {
            setTodoCount(prev => (prev + 1));
            setTotalTasks(prev => (prev + 1));
        } else {
            setTodoCount(prev => (prev + 1));
            setTotalTasks(prev => (prev + 1));
        }
    }, [selectedCategory, todoCount, totalTasks]);
    const handleQuickAdd = useCallback(() => {
        if ((selectedCategory === "bug")) {
            setTodoCount(prev => (prev + 1));
            setTotalTasks(prev => (prev + 1));
        } else {
            setTodoCount(prev => (prev + 1));
            setTotalTasks(prev => (prev + 1));
        }
    }, [selectedCategory, todoCount, totalTasks]);

    return (
        <div style={{padding: '0px', background: '#f5f5f5', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="root">
        <div style={{flexDirection: 'row', gap: '16px', padding: '16px 32px', background: '#1e293b', width: '100%', color: '#E5E7EB', display: 'flex'}} data-testid="header">
        <h1 data-testid="app-title">Smart Task Board</h1>
        <div style={{flex: 1}}></div>
        <button data-testid="btn-settings">Settings</button>
        <button style={{backgroundColor: 'transparent', color: '#E5E7EB', border: '1px solid #E5E7EB', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleRefresh} data-testid="btn-refresh">Refresh</button>
        </div>
        <div style={{flexDirection: 'row', gap: '16px', padding: '24px 32px', width: '100%', display: 'flex'}} data-testid="stats-row">
        <div style={{padding: '20px', background: '#ffffff', flex: 1, borderRadius: 12, boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="stat-total">
        <span style={{color: '#64748b', fontSize: 14, fontWeight: 500}}>Total Tasks</span>
        <h2 data-testid="stat-total-value">{String(totalTasks)}</h2>
        </div>
        <div style={{padding: '20px', background: '#eff6ff', flex: 1, borderRadius: 12, boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="stat-todo">
        <span style={{color: '#3b82f6', fontSize: 14, fontWeight: 500}}>Todo</span>
        <h2 data-testid="stat-todo-value">{String(todoCount)}</h2>
        </div>
        <div style={{padding: '20px', background: '#fef3c7', flex: 1, borderRadius: 12, boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="stat-progress">
        <span style={{color: '#f59e0b', fontSize: 14, fontWeight: 500}}>In Progress</span>
        <h2 data-testid="stat-progress-value">{String(inProgressCount)}</h2>
        </div>
        <div style={{padding: '20px', background: '#f0fdf4', flex: 1, borderRadius: 12, boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="stat-done">
        <span style={{color: '#22c55e', fontSize: 14, fontWeight: 500}}>Done</span>
        <h2 data-testid="stat-done-value">{String(doneCount)}</h2>
        </div>
        </div>
        <div style={{flexDirection: 'row', gap: '12px', padding: '16px 32px', width: '100%', background: '#ffffff', borderBottom: '1px solid #e2e8f0', display: 'flex'}} data-testid="filter-bar">
        <input placeholder="Search tasks..." style={{flex: 2}} data-testid="search-input" />
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleFilterAll} data-testid="filter-all">All</button>
        <button style={{backgroundColor: 'transparent', color: '#E5E7EB', border: '1px solid #E5E7EB', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleFilterBug} data-testid="filter-bug">Bug</button>
        <button style={{backgroundColor: 'transparent', color: '#E5E7EB', border: '1px solid #E5E7EB', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleFilterFeature} data-testid="filter-feature">Feature</button>
        <button style={{backgroundColor: 'transparent', color: '#E5E7EB', border: '1px solid #E5E7EB', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleFilterDocs} data-testid="filter-docs">Docs</button>
        <div style={{flex: 1}}></div>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleAddTask} data-testid="btn-new-task">+ New Task</button>
        </div>
        <div style={{flexDirection: 'row', gap: '16px', padding: '24px 32px', width: '100%', flex: 1, display: 'flex'}} data-testid="kanban-row">
        <div style={{padding: '16px', background: '#f8fafc', flex: 1, borderRadius: 12, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="column-todo">
        <div style={{flexDirection: 'row', gap: '8px', padding: '0 0 12px 0px', display: 'flex'}}>
        <span style={{color: '#3b82f6', fontSize: 16, fontWeight: 600}}>📋 TODO</span>
        <span style={{background: '#3b82f6', color: '#ffffff', fontSize: 12, fontWeight: 600, padding: '2px 8px', borderRadius: 12}}>5</span>
        </div>
        <div style={{padding: '12px', background: '#ffffff', borderRadius: 8, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="task-1">
        <span style={{fontSize: 14, fontWeight: 500, color: '#1e293b'}}>Fix login page bug</span>
        <div style={{flexDirection: 'row', gap: '8px', padding: '8px 0 0 0px', display: 'flex'}}>
        <span style={{background: '#fee2e2', color: '#ef4444', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>bug</span>
        <span style={{background: '#fef3c7', color: '#f59e0b', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>P0</span>
        <div style={{flex: 1}}></div>
        <button>→</button>
        </div>
        </div>
        <div style={{padding: '12px', background: '#ffffff', borderRadius: 8, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="task-2">
        <span style={{fontSize: 14, fontWeight: 500, color: '#1e293b'}}>Update API docs</span>
        <div style={{flexDirection: 'row', gap: '8px', padding: '8px 0 0 0px', display: 'flex'}}>
        <span style={{background: '#dbeafe', color: '#3b82f6', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>docs</span>
        <span style={{background: '#fef3c7', color: '#f59e0b', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>P1</span>
        <div style={{flex: 1}}></div>
        <button>→</button>
        </div>
        </div>
        </div>
        <div style={{padding: '16px', background: '#f8fafc', flex: 1, borderRadius: 12, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="column-progress">
        <div style={{flexDirection: 'row', gap: '8px', padding: '0 0 12px 0px', display: 'flex'}}>
        <span style={{color: '#f59e0b', fontSize: 16, fontWeight: 600}}>🚀 IN PROGRESS</span>
        <span style={{background: '#f59e0b', color: '#ffffff', fontSize: 12, fontWeight: 600, padding: '2px 8px', borderRadius: 12}}>3</span>
        </div>
        <div style={{padding: '12px', background: '#ffffff', borderRadius: 8, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="task-3">
        <span style={{fontSize: 14, fontWeight: 500, color: '#1e293b'}}>Implement dark mode</span>
        <div style={{flexDirection: 'row', gap: '8px', padding: '8px 0 0 0px', display: 'flex'}}>
        <span style={{background: '#dcfce7', color: '#22c55e', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>feature</span>
        <span style={{background: '#fef3c7', color: '#f59e0b', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>P1</span>
        <div style={{flex: 1}}></div>
        <button>→</button>
        </div>
        </div>
        <div style={{padding: '12px', background: '#ffffff', borderRadius: 8, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="task-4">
        <span style={{fontSize: 14, fontWeight: 500, color: '#1e293b'}}>Add export CSV feature</span>
        <div style={{flexDirection: 'row', gap: '8px', padding: '8px 0 0 0px', display: 'flex'}}>
        <span style={{background: '#dcfce7', color: '#22c55e', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>feature</span>
        <span style={{background: '#e2e8f0', color: '#64748b', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>P2</span>
        <div style={{flex: 1}}></div>
        <button>→</button>
        </div>
        </div>
        </div>
        <div style={{padding: '16px', background: '#f8fafc', flex: 1, borderRadius: 12, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="column-done">
        <div style={{flexDirection: 'row', gap: '8px', padding: '0 0 12px 0px', display: 'flex'}}>
        <span style={{color: '#22c55e', fontSize: 16, fontWeight: 600}}>✅ DONE</span>
        <span style={{background: '#22c55e', color: '#ffffff', fontSize: 12, fontWeight: 600, padding: '2px 8px', borderRadius: 12}}>4</span>
        </div>
        <div style={{padding: '12px', background: '#ffffff', borderRadius: 8, border: '1px solid #e2e8f0', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="task-5">
        <span style={{fontSize: 14, fontWeight: 500, color: '#1e293b'}}>Setup CI/CD pipeline</span>
        <div style={{flexDirection: 'row', gap: '8px', padding: '8px 0 0 0px', display: 'flex'}}>
        <span style={{background: '#f3e8ff', color: '#a855f7', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>infra</span>
        <span style={{background: '#fef3c7', color: '#f59e0b', fontSize: 11, fontWeight: 600, padding: '2px 8px', borderRadius: 4}}>P0</span>
        <div style={{flex: 1}}></div>
        <button>✕</button>
        </div>
        </div>
        </div>
        </div>
        <div style={{flexDirection: 'row', gap: '12px', padding: '20px 32px', width: '100%', background: '#ffffff', borderTop: '1px solid #e2e8f0', display: 'flex'}} data-testid="quick-add">
        <span style={{fontSize: 14, fontWeight: 600, color: '#475569'}}>Quick Add:</span>
        <input placeholder="New task title..." style={{flex: 3}} data-testid="quick-add-title" />
        <input placeholder="Category..." style={{flex: 1}} data-testid="quick-add-category" />
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={handleQuickAdd} data-testid="btn-quick-add">Add Task</button>
        </div>
        </div>
    );
}