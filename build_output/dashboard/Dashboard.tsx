import React, { useCallback, useState } from "react";
import styles from "./Dashboard.module.css";
import { useStatsStore } from "./useStatsStore";
import { useUIStore } from "./useUIStore";

export function Dashboard() {
    // .omni:45 -- component Dashboard

    const active_pct = useStatsStore(s => s.active_pct());
    const active_users = useStatsStore(s => s.active_users);
    const avg_order_value = useStatsStore(s => s.avg_order_value());
    const conversion_rate = useStatsStore(s => s.conversion_rate());
    const growth_rate = useStatsStore(s => s.growth_rate);
    const orders = useStatsStore(s => s.orders);
    const revenue = useStatsStore(s => s.revenue);
    const total_users = useStatsStore(s => s.total_users);
    const sidebar_open = useUIStore(s => s.sidebar_open);
    const [period, setPeriod] = useState<string>("today");
    const handle_period = useCallback((p) => {
        setPeriod(p);
    }, []);
    const format_number = useCallback((n) => {
        setN(n);
    }, []);

    return (
        <div data-testid="dashboard">
        <div style={{flexDirection: 'row', gap: '16px', alignItems: 'center', display: 'flex'}}>
        <h1 className={styles.heading} data-testid="dash-title">Dashboard</h1>
        <div style={{flex: 1}}></div>
        <button onClick={useUIStore.getState().toggle_sidebar} data-testid="btn-toggle-sidebar">☰</button>
        </div>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}} data-testid="period-bar">
        <button onClick={() => {
            handle_period("today");
        }} data-testid="period-today">Today</button>
        <button onClick={() => {
            handle_period("week");
        }} data-testid="period-week">Week</button>
        <button onClick={() => {
            handle_period("month");
        }} data-testid="period-month">Month</button>
        </div>
        <div style={{display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '16px'}} data-testid="kpi-grid">
        <div className={styles.card} data-testid="kpi-users">
        <h3 className={styles.heading}>Total Users</h3>
        <span data-testid="kpi-users-value">{total_users}</span>
        <span data-testid="kpi-users-growth">
        Growth: 
        {growth_rate}
        %
        </span>
        </div>
        <div className={styles.card} data-testid="kpi-active">
        <h3 className={styles.heading}>Active Users</h3>
        <span data-testid="kpi-active-value">{active_users}</span>
        <span data-testid="kpi-active-pct">
        {active_pct}
        % active
        </span>
        </div>
        <div className={styles.card} data-testid="kpi-revenue">
        <h3 className={styles.heading}>Revenue</h3>
        <span data-testid="kpi-revenue-value">
        $
        {revenue}
        </span>
        <span data-testid="kpi-aov">
        AOV: $
        {avg_order_value}
        </span>
        </div>
        <div className={styles.card} data-testid="kpi-orders">
        <h3 className={styles.heading}>Orders</h3>
        <span data-testid="kpi-orders-value">{orders}</span>
        <span data-testid="kpi-conversion">
        Conversion: 
        {conversion_rate}
        %
        </span>
        </div>
        </div>
        {sidebar_open ? <>
        <div data-testid="sidebar">
        <div style={{flexDirection: 'column', gap: '8px', display: 'flex'}}>
        <button onClick={() => {
            useUIStore.getState().set_page("dashboard");
        }} data-testid="nav-dashboard">📊 Dashboard</button>
        <button onClick={() => {
            useUIStore.getState().set_page("analytics");
        }} data-testid="nav-analytics">📈 Analytics</button>
        <button onClick={() => {
            useUIStore.getState().set_page("settings");
        }} data-testid="nav-settings">⚙ Settings</button>
        </div>
        </div>
        </> : (null)}
        {(total_users === 0) ? <>
        <div data-testid="empty-state">
        <h2 className={styles.heading}>Welcome!</h2>
        <span>No data yet. Use the buttons above to load sample data.</span>
        <button onClick={() => {
            useStatsStore.getState().update_stats(1250, 843, 45600.0, 312, 12.5);
        }} data-testid="btn-load-data">Load Sample Data</button>
        </div>
        </> : (null)}
        </div>
    );
}