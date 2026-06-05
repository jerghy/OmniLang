import React, { useCallback, useEffect, useState } from "react";
import styles from "./TodoApp.module.css";
import { useTodoStore } from "./useTodoStore";

export function TodoApp() {
    // .omni:7 -- component TodoApp

    const display_count = useTodoStore(s => s.display_count());
    const [input_text, setInput_text] = useState<string>("");
    const [filter, setFilter] = useState<string>("all");
    useEffect(() => {
        setInput_text("");
    }, []); // @mount
    const handle_add = useCallback(() => {
        setInput_text("");
    }, [input_text]);
    const show_all = useCallback(() => {
        setFilter("all");
    }, [filter]);
    const show_active = useCallback(() => {
        setFilter("active");
    }, [filter]);
    const show_done = useCallback(() => {
        setFilter("done");
    }, [filter]);

    return (
        <div className={styles.container}>
        <h2 className={styles.heading}>Todos</h2>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}}>
        <input placeholder="What needs to be done?" value={input_text} onChange={e => setInput_text(e.target.value)} data-testid="todo-input" />
        <button onClick={handle_add} data-testid="btn-add">Add</button>
        </div>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="filter-bar">
        <button onClick={show_all} data-testid="filter-all">All</button>
        <button onClick={show_active} data-testid="filter-active">Active</button>
        <button onClick={show_done} data-testid="filter-done">Done</button>
        </div>
        {(filter === "all") ? (<span data-testid="content-all">Showing ALL tasks</span>) : (null)}
        {(filter === "active") ? (<span data-testid="content-active">Showing ACTIVE tasks</span>) : (null)}
        {(filter === "done") ? (<span data-testid="content-done">Showing DONE tasks</span>) : (null)}
        <ul data-testid="todo-list"><li data-testid="list-item"><span>Sample Task</span></li></ul>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}}>
        <span data-testid="counter-label">
        Total: 
        {display_count}
         items
        </span>
        </div>
        </div>
    );
}