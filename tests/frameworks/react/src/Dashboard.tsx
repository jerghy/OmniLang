import React, { useCallback, useEffect, useState } from "react";
import styles from "./Dashboard.module.css";
import { useUserStore } from "./useUserStore";

export function Dashboard() {
    // .omni:12 -- component Dashboard

    const logged_in = useUserStore(s => s.logged_in);
    const name = useUserStore(s => s.name);
    const role = useUserStore(s => s.role);
    const [sidebar_open, setSidebar_open] = useState<boolean>(true);
    useEffect(() => {
        setSidebar_open(true);
    }, []); // @mount
    const handle_login = useCallback(() => {
        useUserStore.getState().login("Admin");
    }, []);
    const handle_logout = useCallback(() => {
        useUserStore.getState().logout();
    }, []);
    const close_sidebar = useCallback(() => {
        setSidebar_open(false);
    }, [sidebar_open]);
    const open_sidebar = useCallback(() => {
        setSidebar_open(true);
    }, [sidebar_open]);

    return (
        <div className={styles.container}>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} className={styles.header} data-testid="header">
        <h2>Dashboard</h2>
        <div style={{flex: 1}}></div>
        <span data-testid="user-name">{name}</span>
        <span data-testid="user-role">{role}</span>
        {logged_in ? (<button onClick={handle_logout} data-testid="btn-logout">Logout</button>) : (<button onClick={handle_login} data-testid="btn-login">Login</button>)}
        </div>
        <div style={{flexDirection: 'row', gap: '24px', display: 'flex'}}>
        {sidebar_open ? (<div className={styles.sidebar} data-testid="sidebar"><button onClick={close_sidebar} data-testid="nav-close">Close Sidebar</button></div>) : (null)}
        <div className={styles.container} data-testid="main-content">
        <div style={{display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '16px'}} data-testid="stats-grid">
        <div data-testid="stat-visitors">
        <span>Welcome</span>
        <h2>{name}</h2>
        </div>
        </div>
        {!sidebar_open ? (<button onClick={open_sidebar} data-testid="btn-open-sidebar">Open Sidebar</button>) : (null)}
        </div>
        </div>
        </div>
    );
}