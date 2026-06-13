import React, { useCallback, useState } from "react";

export function TaskBoard() {
    // .omni:1 -- component TaskBoard

    const [aliceTasks, setAliceTasks] = useState<number>(0);
    const [bobTasks, setBobTasks] = useState<number>(0);
    const [charlieTasks, setCharlieTasks] = useState<number>(1);
    const [task1Assignee, setTask1Assignee] = useState<string>("");
    const [task2Assignee, setTask2Assignee] = useState<string>("");
    const [filterPriority, setFilterPriority] = useState<string>("all");
    const [isSaving, setIsSaving] = useState<boolean>(false);
    const [saveError, setSaveError] = useState<string>("");
    const [errorMessage, setErrorMessage] = useState<string>("");
    const [maxCapacity, setMaxCapacity] = useState<number>(3);
    const assignTask1ToAlice = useCallback(() => {
        if ((aliceTasks < maxCapacity)) {
            setTask1Assignee("Alice");
            setAliceTasks(prev => (prev + 1));
            setErrorMessage("");
        } else {
            setErrorMessage("Alice is at capacity!");
        }
    }, [aliceTasks, errorMessage, maxCapacity, task1Assignee]);
    const assignTask2ToBob = useCallback(() => {
        if ((bobTasks < maxCapacity)) {
            setTask2Assignee("Bob");
            setBobTasks(prev => (prev + 1));
            setErrorMessage("");
        } else {
            setErrorMessage("Bob is at capacity!");
        }
    }, [bobTasks, errorMessage, maxCapacity, task2Assignee]);
    const filterAll = useCallback(() => {
        setFilterPriority("all");
    }, [filterPriority]);
    const filterHigh = useCallback(() => {
        setFilterPriority("high");
    }, [filterPriority]);
    const syncToBackend = useCallback(() => {
        if (isSaving) {
            setIsSaving(false);
            setSaveError("Sync failed: network error");
        } else {
            setIsSaving(true);
            setSaveError("");
        }
    }, [isSaving, saveError]);

    return (
        <div style={{padding: '24px', background: '#1a1a2e', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="task-board-root">
        <h2 data-testid="app-title">Task Assignment Dashboard</h2>
        <div style={{flexDirection: 'row', gap: '16px', flex: 1, display: 'flex'}} data-testid="main-layout">
        <div style={{flexDirection: 'column', gap: '12px', width: 240, padding: '16px', background: '#16213e', color: '#E5E7EB', display: 'flex'}} data-testid="members-panel">
        <h3>Team Members</h3>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="member-alice">
        <h4>Alice Chen</h4>
        <span data-testid="alice-tasks">
        Tasks:
        {String(aliceTasks)}
         | Done: 0
        </span>
        </div>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="member-bob">
        <h4>Bob Wang</h4>
        <span data-testid="bob-tasks">
        Tasks:
        {String(bobTasks)}
         | Done: 0
        </span>
        </div>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="member-charlie">
        <h4>Charlie Li</h4>
        <span data-testid="charlie-tasks">
        Tasks:
        {String(charlieTasks)}
         | Done: 0
        </span>
        </div>
        </div>
        <div style={{flexDirection: 'column', gap: '12px', flex: 1, padding: '16px', background: '#0f3460', color: '#E5E7EB', display: 'flex'}} data-testid="task-panel">
        <h3 data-testid="task-board-title">Task Board</h3>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="task-1">
        <h4>T1: Design API</h4>
        <span>Status: todo | Priority: high</span>
        <span data-testid="task1-assignee">
        Assignee:
        {String(task1Assignee)}
        </span>
        </div>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="task-2">
        <h4>T2: Build Auth</h4>
        <span>Status: todo | Priority: medium</span>
        <span data-testid="task2-assignee">
        Assignee:
        {String(task2Assignee)}
        </span>
        </div>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="task-3">
        <h4>T3: Write Tests</h4>
        <span>Status: todo | Priority: low</span>
        <span>Assignee: —</span>
        </div>
        <div style={{padding: '12px', background: '#1e3a5f', borderRadius: '8px', boxShadow: '0 2px 8px rgba(255,255,255,0.06)', color: '#E5E7EB'}} data-testid="task-4">
        <h4>T4: Deploy CI</h4>
        <span>Status: todo | Priority: high</span>
        <span>Assignee: —</span>
        </div>
        <div style={{flexDirection: 'column', gap: '8px', padding: '8px 0px', display: 'flex'}} data-testid="assign-buttons">
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={assignTask1ToAlice} data-testid="btn-assign-t1-alice">Assign T1 to Alice</button>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={assignTask2ToBob} data-testid="btn-assign-t2-bob">Assign T2 to Bob</button>
        </div>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}} data-testid="filter-buttons">
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={filterAll} data-testid="btn-filter-all">All</button>
        <button style={{backgroundColor: 'transparent', color: '#E5E7EB', border: '1px solid #E5E7EB', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={filterHigh} data-testid="btn-filter-high">High Priority</button>
        </div>
        </div>
        <div style={{flexDirection: 'column', gap: '12px', width: 280, padding: '16px', background: '#16213e', color: '#E5E7EB', display: 'flex'}} data-testid="stats-panel">
        <h3>Statistics</h3>
        <span data-testid="stat-total">Total tasks: 4</span>
        <span data-testid="stat-filter">
        Filter:
        {String(filterPriority)}
        </span>
        <span data-testid="stat-error">{String(errorMessage)}</span>
        <span data-testid="stat-saving">
        Saving:
        {String(isSaving)}
        </span>
        <span data-testid="stat-save-error">{String(saveError)}</span>
        <button style={{backgroundColor: 'transparent', color: '#E5E7EB', border: '1px solid #E5E7EB', borderRadius: '8px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={syncToBackend} data-testid="btn-sync">Sync to Backend</button>
        </div>
        </div>
        </div>
    );
}
