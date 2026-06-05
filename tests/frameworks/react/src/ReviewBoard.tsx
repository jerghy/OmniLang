import React, { useCallback, useEffect, useState } from "react";
import styles from "./ReviewBoard.module.css";
import { useReviewStore } from "./useReviewStore";

export function ReviewBoard() {
    // .omni:24 -- component ReviewBoard

    const pending_count = useReviewStore(s => s.pending_count());
    const total_count = useReviewStore(s => s.total_count());
    const [new_title, setNew_title] = useState<string>("");
    const [status_filter, setStatus_filter] = useState<string>("all");
    useEffect(() => {
        setNew_title("");
    }, []); // @mount
    const handle_add = useCallback(() => {
        useReviewStore.getState().add_review(new_title);
        setNew_title("");
    }, [new_title]);
    const filter_all = useCallback(() => {
        useReviewStore.getState().set_filter("all");
        setStatus_filter("all");
    }, [status_filter]);
    const filter_pending = useCallback(() => {
        useReviewStore.getState().set_filter("pending");
        setStatus_filter("pending");
    }, [status_filter]);
    const filter_done = useCallback(() => {
        useReviewStore.getState().set_filter("approved");
        setStatus_filter("approved");
    }, [status_filter]);
    const handle_clear = useCallback(() => {
        useReviewStore.getState().clear_all();
    }, []);

    return (
        <div className={styles.container}>
        <div style={{flexDirection: 'row', gap: '16px', alignItems: 'center', display: 'flex'}}>
        <h2 className={styles.heading}>Review Board</h2>
        <div style={{flex: 1}}></div>
        <span data-testid="stats-total">
        Total: 
        {total_count}
        </span>
        <span data-testid="stats-pending">
        Pending: 
        {pending_count}
        </span>
        </div>
        <div style={{flexDirection: 'row', gap: '12px', display: 'flex'}} data-testid="add-section">
        <input placeholder="Enter review title..." value={new_title} onChange={e => setNew_title(e.target.value)} data-testid="input-title" />
        <button onClick={handle_add} data-testid="btn-add">Add Review</button>
        </div>
        <div style={{flexDirection: 'row', gap: '8px', display: 'flex'}} data-testid="filter-bar">
        <button onClick={filter_all} data-testid="filter-all">All</button>
        <button onClick={filter_pending} data-testid="filter-pending">Pending</button>
        <button onClick={filter_done} data-testid="filter-done">Approved</button>
        <div style={{flex: 1}}></div>
        <button onClick={handle_clear} data-testid="btn-clear">Clear All</button>
        </div>
        {(status_filter === "all") ? (<div style={{display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(250px, 1fr))', gap: '16px'}} data-testid="review-list">
        <div className={styles.card} data-testid="empty-card">
        <h2 className={styles.heading}>No Reviews Yet</h2>
        <span>Add your first review above</span>
        </div>
        </div>) : (null)}
        {(status_filter === "pending") ? (<div style={{flexDirection: 'column', gap: '8px', display: 'flex'}} data-testid="pending-list"><div data-testid="alert-pending">Showing pending reviews</div></div>) : (null)}
        {(status_filter === "approved") ? (<div style={{flexDirection: 'column', gap: '8px', display: 'flex'}} data-testid="approved-list"><div data-testid="alert-approved">Showing approved reviews</div></div>) : (null)}
        <div style={{flexDirection: 'row', gap: '16px', display: 'flex'}} className={styles.footer} data-testid="footer">
        <span data-testid="footer-text">ReviewBoard v1.0</span>
        <div style={{flex: 1}}></div>
        <span data-testid="footer-status">
        Filter: 
        {status_filter}
        </span>
        </div>
        </div>
    );
}