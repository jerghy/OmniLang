import { create } from "zustand";

// .omni:5 -- store StatsStore

interface StatsStoreState {
  total_users: number;
  active_users: number;
  revenue: number;
  orders: number;
  growth_rate: number;
  conversion_rate: () => any;
  avg_order_value: () => any;
  active_pct: () => any;
  update_stats: (users: number, active: number, rev: number, ord: number, growth: number) => void;
}

export const useStatsStore = create<StatsStoreState>()((set, get) => ({
  total_users: 0,
  active_users: 0,
  revenue: 0.0,
  orders: 0,
  growth_rate: 0.0,
  conversion_rate: () => {
    const state = get();
    return (state.revenue / state.total_users);
  },
  avg_order_value: () => {
    const state = get();
    return (state.revenue / state.orders);
  },
  active_pct: () => {
    const state = get();
    return ((state.active_users / state.total_users) * 100.0);
  },
  update_stats: (users, active, rev, ord, growth) => {
    set(state => ({ total_users: users }));
    set(state => ({ active_users: active }));
    set(state => ({ revenue: rev }));
    set(state => ({ orders: ord }));
    set(state => ({ growth_rate: growth }));
  },
}));