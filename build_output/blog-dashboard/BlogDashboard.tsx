import React, { useCallback, useState } from "react";

export function BlogDashboard() {
    // .omni:1 -- component BlogDashboard

    const [activeMenu, setActiveMenu] = useState<string>("dashboard");
    const selectDashboard = useCallback(() => {
        setActiveMenu("dashboard");
    }, []);
    const selectArticles = useCallback(() => {
        setActiveMenu("articles");
    }, []);
    const selectTheme = useCallback(() => {
        setActiveMenu("theme");
    }, []);
    const selectStats = useCallback(() => {
        setActiveMenu("stats");
    }, []);

    return (
        <div style={{padding: '24px', backgroundColor: '#FFFFFF', borderRadius: '8px', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="dashboard-root">
        <h1 data-testid="app-title">Blog Dashboard</h1>
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} data-testid="main-layout">
        <div style={{flexDirection: 'column', gap: '8px', display: 'flex'}} data-testid="sidebar">
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectDashboard} data-testid="nav-dashboard">Dashboard</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectArticles} data-testid="nav-articles">Articles</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectTheme} data-testid="nav-theme">Theme Settings</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectStats} data-testid="nav-stats">Statistics</button>
        </div>
        <div style={{padding: '24px', backgroundColor: '#FFFFFF', borderRadius: '8px', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="main-content">
        <h2 data-testid="content-title">Welcome to Dashboard</h2>
        <span data-testid="active-menu-display">
        Active menu: 
        {activeMenu}
        </span>
        </div>
        </div>
        </div>
    );
}