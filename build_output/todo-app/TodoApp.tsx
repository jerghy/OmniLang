import React, { useCallback, useState } from "react";
import styles from "./TodoApp.module.css";
import { useTodoStore } from "./useTodoStore";

export function TodoApp() {
    // .omni:37 -- component TodoApp

    const active_count = useTodoStore(s => s.active_count());
    const completed = useTodoStore(s => s.completed);
    const done_count = useTodoStore(s => s.done_count());
    const tasks = useTodoStore(s => s.tasks);
    const total_count = useTodoStore(s => s.total_count());
    const [input_text, setInput_text] = useState<string>("");
    const [filter, setFilter] = useState<string>("all");
    const handle_add = useCallback(() => {
        if ((input_text !== "")) {
            useTodoStore.getState().add_task(input_text);
            setInput_text("");
        }
    }, [input_text]);
    const handle_complete = useCallback((text) => {
        useTodoStore.getState().complete_task(text);
    }, []);
    const handle_restore = useCallback((text) => {
        useTodoStore.getState().uncomplete_task(text);
    }, []);
    const handle_clear_done = useCallback(() => {
        useTodoStore.getState().clear_completed();
    }, []);
    const show_all = useCallback(() => {
        setFilter("all");
    }, []);
    const show_active = useCallback(() => {
        setFilter("active");
    }, []);
    const show_done = useCallback(() => {
        setFilter("done");
    }, []);

    return (
        <div>
        <h1 className={styles.heading} data-testid="app-title">OmniLang Todo</h1>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}}>
        <input placeholder="What needs to be done?" value={input_text} onChange={e => setInput_text(e.target.value)} data-testid="todo-input" />
        <button onClick={handle_add} data-testid="btn-add">Add</button>
        </div>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}} data-testid="filter-bar">
        <button onClick={show_all} data-testid="filter-all">
        All (
        {total_count}
        )
        </button>
        <button onClick={show_active} data-testid="filter-active">
        Active (
        {active_count}
        )
        </button>
        <button onClick={show_done} data-testid="filter-done">
        Done (
        {done_count}
        )
        </button>
        </div>
        <ul data-testid="task-list">
        {((filter === "all") || (filter === "active")) ? <>
        {tasks.map(task => (
        <li key={task} data-testid="task-item">
        <div style={{flexDirection: 'row', gap: '8px', alignItems: 'center', display: 'flex'}}>
        <button onClick={() => {
            handle_complete(task);
        }} data-testid="btn-complete">✓</button>
        <span className={styles.task_text} data-testid="task-text">{task}</span>
        </div>
        </li>
        ))}
        </> : (null)}
        {((filter === "all") || (filter === "done")) ? <>
        {completed.map(task => (
        <li key={task} data-testid="done-item">
        <div style={{flexDirection: 'row', gap: '8px', alignItems: 'center', display: 'flex'}}>
        <button onClick={() => {
            handle_restore(task);
        }} data-testid="btn-restore">↩</button>
        <span className={styles.done_text} data-testid="done-text">{task}</span>
        </div>
        </li>
        ))}
        </> : (null)}
        {(total_count === 0) ? <span data-testid="empty-message">No tasks yet. Add one above!</span> : (null)}
        </ul>
        {(done_count > 0) ? <>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="footer">
        <span data-testid="summary">
        {active_count}
         active, 
        {done_count}
         done
        </span>
        <button onClick={handle_clear_done} data-testid="btn-clear-done">Clear Done</button>
        </div>
        </> : (null)}
        </div>
    );
}