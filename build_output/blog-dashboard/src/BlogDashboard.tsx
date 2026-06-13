import React, { useCallback, useState } from "react";

export function BlogDashboard() {
    const [activeMenu, setActiveMenu] = useState<string>("articles");
    const [totalArticles, setTotalArticles] = useState<number>(128);
    const [totalViews, setTotalViews] = useState<number>(45678);
    const [totalComments, setTotalComments] = useState<number>(1234);
    const [totalLikes, setTotalLikes] = useState<number>(9876);
    const [searchQuery, setSearchQuery] = useState<string>("");
    const [activeCategory, setActiveCategory] = useState<string>("all");
    const selectDashboard = useCallback(() => { setActiveMenu("dashboard"); }, []);
    const selectArticles = useCallback(() => { setActiveMenu("articles"); }, []);
    const selectTheme = useCallback(() => { setActiveMenu("theme"); }, []);
    const selectStats = useCallback(() => { setActiveMenu("stats"); }, []);
    const filterAll = useCallback(() => { setActiveCategory("all"); }, []);
    const filterTech = useCallback(() => { setActiveCategory("tech"); }, []);
    const filterDesign = useCallback(() => { setActiveCategory("design"); }, []);
    const filterBusiness = useCallback(() => { setActiveCategory("business"); }, []);

    return (
        <div style={{padding: '24px', background: '#FFFFFF', borderRadius: '8px', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="dashboard-root">
        <h1 data-testid="app-title">Blog Dashboard</h1>
        <div style={{flexDirection: 'row', gap: '24px', display: 'flex'}} data-testid="main-layout">
        <div style={{flexDirection: 'column', gap: '4px', display: 'flex'}} data-testid="sidebar">
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectDashboard} data-testid="nav-dashboard">Dashboard</button>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectArticles} data-testid="nav-articles">Articles</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectTheme} data-testid="nav-theme">Theme Settings</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={selectStats} data-testid="nav-stats">Statistics</button>
        </div>
        <div style={{padding: '0px', flex: 1, background: '#FFFFFF', borderRadius: '8px', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="main-content">
        <div style={{flexDirection: 'row', gap: '16px', padding: '24px 24px 0 24px', display: 'flex'}} data-testid="kpi-row">
        <div style={{padding: '20px', flex: 1, background: '#EEF2FF', borderRadius: 12, boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="kpi-articles">
        <span style={{fontSize: 28, fontWeight: 700, color: '#5252E0'}} data-testid="kpi-articles-value">{totalArticles}</span>
        <span style={{fontSize: 13, fontWeight: 500, color: '#6B7280'}} data-testid="kpi-articles-label">Total Articles</span>
        </div>
        <div style={{padding: '20px', flex: 1, background: '#FFF7ED', borderRadius: 12, boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="kpi-views">
        <span style={{fontSize: 28, fontWeight: 700, color: '#EA580C'}} data-testid="kpi-views-value">{totalViews}</span>
        <span style={{fontSize: 13, fontWeight: 500, color: '#6B7280'}} data-testid="kpi-views-label">Total Views</span>
        </div>
        <div style={{padding: '20px', flex: 1, background: '#F0FDF4', borderRadius: 12, boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="kpi-comments">
        <span style={{fontSize: 28, fontWeight: 700, color: '#16A34A'}} data-testid="kpi-comments-value">{totalComments}</span>
        <span style={{fontSize: 13, fontWeight: 500, color: '#6B7280'}} data-testid="kpi-comments-label">Comments</span>
        </div>
        <div style={{padding: '20px', flex: 1, background: '#FDF2F8', borderRadius: 12, boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="kpi-likes">
        <span style={{fontSize: 28, fontWeight: 700, color: '#DB2777'}} data-testid="kpi-likes-value">{totalLikes}</span>
        <span style={{fontSize: 13, fontWeight: 500, color: '#6B7280'}} data-testid="kpi-likes-label">Likes</span>
        </div>
        </div>
        <div style={{flexDirection: 'row', gap: '12px', padding: '24px', display: 'flex'}} data-testid="search-row">
        <input placeholder="Search articles..." data-testid="search-input" />
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} data-testid="btn-search">Search</button>
        </div>
        <div style={{flexDirection: 'row', gap: '8px', padding: '0 24px 16px 24px', display: 'flex'}} data-testid="category-filters">
        <span style={{fontSize: 14, fontWeight: 500, color: '#6B7280'}}>Category:</span>
        <button style={{backgroundColor: '#5252E0', color: '#FFFFFF', border: 'none', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={filterAll} data-testid="cat-all">All</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={filterTech} data-testid="cat-tech">Tech</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={filterDesign} data-testid="cat-design">Design</button>
        <button style={{backgroundColor: 'transparent', color: '#1F2937', border: '1px solid #E5E7EB', borderRadius: '6px', padding: '8px 16px', cursor: 'pointer', fontSize: '14px', fontWeight: 500}} onClick={filterBusiness} data-testid="cat-business">Business</button>
        </div>
        <div style={{flexDirection: 'column', gap: '16px', padding: '0 24px 24px 24px', display: 'flex'}} data-testid="article-list">
        <div style={{padding: '0px', borderRadius: 12, background: '#FFFFFF', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="article-1">
        <div style={{flexDirection: 'row', gap: '0px', display: 'flex'}}>
        <div style={{width: 200, height: 160, background: '#DBEAFE', borderRadius: '12px 0 0 12'}} data-testid="article-1-cover"></div>
        <div style={{flexDirection: 'column', gap: '8px', padding: '20px', flex: 1, display: 'flex'}}>
        <span style={{fontSize: 12, fontWeight: 600, color: '#5252E0', background: '#EEF2FF', padding: '2px 10px', borderRadius: 20}} data-testid="article-1-category">Technology</span>
        <h3 data-testid="article-1-title">Building Scalable Microservices with Rust</h3>
        <span style={{fontSize: 14, color: '#6B7280'}} data-testid="article-1-excerpt">Learn how to design and deploy high-performance microservices using Rust's async runtime and zero-cost abstractions.</span>
        <div style={{flexDirection: 'row', gap: '10px', alignItems: 'center', display: 'flex'}}>
        <div style={{width: 32, height: 32, background: '#5252E0', borderRadius: '50%'}} data-testid="article-1-avatar"></div>
        <span style={{fontSize: 13, fontWeight: 600, color: '#1F2937'}} data-testid="article-1-author">Alice Chen</span>
        <span style={{fontSize: 12, color: '#9CA3AF'}} data-testid="article-1-date">Jun 12, 2026</span>
        </div>
        </div>
        </div>
        </div>
        <div style={{padding: '0px', borderRadius: 12, background: '#FFFFFF', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="article-2">
        <div style={{flexDirection: 'row', gap: '0px', display: 'flex'}}>
        <div style={{width: 200, height: 160, background: '#FEF3C7', borderRadius: '12px 0 0 12'}} data-testid="article-2-cover"></div>
        <div style={{flexDirection: 'column', gap: '8px', padding: '20px', flex: 1, display: 'flex'}}>
        <span style={{fontSize: 12, fontWeight: 600, color: '#D97706', background: '#FFFBEB', padding: '2px 10px', borderRadius: 20}} data-testid="article-2-category">Design</span>
        <h3 data-testid="article-2-title">The Future of UI: Beyond Flat Design</h3>
        <span style={{fontSize: 14, color: '#6B7280'}} data-testid="article-2-excerpt">Exploring neumorphism, glassmorphism, and the next generation of user interface design patterns.</span>
        <div style={{flexDirection: 'row', gap: '10px', alignItems: 'center', display: 'flex'}}>
        <div style={{width: 32, height: 32, background: '#D97706', borderRadius: '50%'}} data-testid="article-2-avatar"></div>
        <span style={{fontSize: 13, fontWeight: 600, color: '#1F2937'}} data-testid="article-2-author">Bob Wang</span>
        <span style={{fontSize: 12, color: '#9CA3AF'}} data-testid="article-2-date">Jun 10, 2026</span>
        </div>
        </div>
        </div>
        </div>
        <div style={{padding: '0px', borderRadius: 12, background: '#FFFFFF', boxShadow: '0 2px 8px rgba(0,0,0,0.08)'}} data-testid="article-3">
        <div style={{flexDirection: 'row', gap: '0px', display: 'flex'}}>
        <div style={{width: 200, height: 160, background: '#D1FAE5', borderRadius: '12px 0 0 12'}} data-testid="article-3-cover"></div>
        <div style={{flexDirection: 'column', gap: '8px', padding: '20px', flex: 1, display: 'flex'}}>
        <span style={{fontSize: 12, fontWeight: 600, color: '#059669', background: '#ECFDF5', padding: '2px 10px', borderRadius: 20}} data-testid="article-3-category">Business</span>
        <h3 data-testid="article-3-title">Monetization Strategies for Open Source</h3>
        <span style={{fontSize: 14, color: '#6B7280'}} data-testid="article-3-excerpt">A comprehensive guide to sustainable business models for open-source projects in 2026.</span>
        <div style={{flexDirection: 'row', gap: '10px', alignItems: 'center', display: 'flex'}}>
        <div style={{width: 32, height: 32, background: '#059669', borderRadius: '50%'}} data-testid="article-3-avatar"></div>
        <span style={{fontSize: 13, fontWeight: 600, color: '#1F2937'}} data-testid="article-3-author">Carol Liu</span>
        <span style={{fontSize: 12, color: '#9CA3AF'}} data-testid="article-3-date">Jun 8, 2026</span>
        </div>
        </div>
        </div>
        </div>
        </div>
        </div>
        </div>
        </div>
    );
}
