import React, { useCallback, useState } from "react";
import { useKanbanStore } from "./useKanbanStore";

export function KanbanBoard() {
    // .omni:1 -- component KanbanBoard

    const doneCount = useKanbanStore(s => s.doneCount());
    const doneTasks = useKanbanStore(s => s.doneTasks);
    const inProgressCount = useKanbanStore(s => s.inProgressCount());
    const inProgressTasks = useKanbanStore(s => s.inProgressTasks);
    const todoCount = useKanbanStore(s => s.todoCount());
    const todoTasks = useKanbanStore(s => s.todoTasks);
    const totalCount = useKanbanStore(s => s.totalCount());
    const [searchQuery, setSearchQuery] = useState<string>("");
    const [newTaskTitle, setNewTaskTitle] = useState<string>("");
    const [showForm, setShowForm] = useState<boolean>(false);
    const handleShowForm = useCallback(() => {
        setShowForm(true);
    }, []);
    const handleCancelForm = useCallback(() => {
        setShowForm(false);
        setNewTaskTitle("");
    }, []);
    const handleAddTask = useCallback(() => {
        if ((newTaskTitle !== "")) {
            useKanbanStore.getState().addTask(newTaskTitle);
            setNewTaskTitle("");
            setShowForm(false);
        }
    }, [newTaskTitle]);
    const handleMoveToProgress = useCallback((title) => {
        useKanbanStore.getState().moveToProgress(title);
    }, []);
    const handleMoveToDone = useCallback((title) => {
        useKanbanStore.getState().moveToDone(title);
    }, []);
    const handleMoveToTodo = useCallback((title) => {
        useKanbanStore.getState().moveToTodo(title);
    }, []);
    const handleMoveBackToProgress = useCallback((title) => {
        useKanbanStore.getState().moveBackToProgress(title);
    }, []);
    const handleClearDone = useCallback(() => {
        useKanbanStore.getState().clearDone();
    }, []);
    const handleResetAll = useCallback(() => {
        useKanbanStore.getState().resetAll();
    }, []);

    return (
        <div>
        <h1 data-testid="app-title">Kanban Board</h1>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="toolbar">
        <input placeholder="Search tasks..." value={searchQuery} onChange={e => setSearchQuery(e.target.value)} data-testid="search-input" />
        <button onClick={handleShowForm} data-testid="btn-show-form">+ New Task</button>
        <button onClick={handleResetAll} data-testid="btn-reset">Reset</button>
        </div>
        {showForm ? <>
        <div data-testid="add-task-form">
        <h3>New Task</h3>
        <div style={{flexDirection: 'row', gap: '12px', display: 'flex'}}>
        <input placeholder="Task title..." value={newTaskTitle} onChange={e => setNewTaskTitle(e.target.value)} data-testid="input-new-title" />
        <button onClick={handleAddTask} data-testid="btn-add-task">Add</button>
        <button onClick={handleCancelForm} data-testid="btn-cancel-form">Cancel</button>
        </div>
        </div>
        </> : (null)}
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="stats-bar">
        <div>
        <span data-testid="stat-total">
        Total: 
        {totalCount}
        </span>
        </div>
        <div>
        <span data-testid="stat-todo">
        Todo: 
        {todoCount}
        </span>
        </div>
        <div>
        <span data-testid="stat-progress">
        In Progress: 
        {inProgressCount}
        </span>
        </div>
        <div>
        <span data-testid="stat-done">
        Done: 
        {doneCount}
        </span>
        </div>
        </div>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="kanban-columns">
        <div data-testid="column-todo">
        <h3 data-testid="col-todo-title">
        {todoCount}
         To Do
        </h3>
        {todoTasks.length === 0 ? <div data-testid="empty-todo"><span>No tasks yet. Create one above!</span></div> : todoTasks.map(task => (
        <div key={task} data-testid="task-card">
        <span data-testid="task-title">{task}</span>
        <div style={{flexDirection: 'row', gap: '4px', display: 'flex'}}><button onClick={() => handleMoveToProgress(task)} data-testid="btn-move-progress">Start</button></div>
        </div>
        ))}
        </div>
        <div data-testid="column-progress">
        <h3 data-testid="col-progress-title">
        {inProgressCount}
         In Progress
        </h3>
        {inProgressTasks.length === 0 ? <div data-testid="empty-progress"><span>Nothing in progress. Start a task!</span></div> : inProgressTasks.map(task => (
        <div key={task} data-testid="task-card">
        <span data-testid="task-title">{task}</span>
        <div style={{flexDirection: 'row', gap: '4px', display: 'flex'}}>
        <button onClick={() => handleMoveToTodo(task)} data-testid="btn-move-todo">Back</button>
        <button onClick={() => handleMoveToDone(task)} data-testid="btn-move-done">Done</button>
        </div>
        </div>
        ))}
        </div>
        <div data-testid="column-done">
        <h3 data-testid="col-done-title">
        {doneCount}
         Done
        </h3>
        {doneTasks.length === 0 ? <div data-testid="empty-done"><span>Nothing completed yet. Keep working!</span></div> : doneTasks.map(task => (
        <div key={task} data-testid="task-card">
        <span data-testid="task-title">{task}</span>
        <div style={{flexDirection: 'row', gap: '4px', display: 'flex'}}><button onClick={() => handleMoveBackToProgress(task)} data-testid="btn-move-progress">Reopen</button></div>
        </div>
        ))}
        </div>
        </div>
        {(doneCount > 0) ? <>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}} data-testid="footer">
        <span data-testid="summary">
        {todoCount}
         active, 
        {doneCount}
         done
        </span>
        <button onClick={handleClearDone} data-testid="btn-clear-done">Clear All Done</button>
        </div>
        </> : (null)}
        </div>
    );
}