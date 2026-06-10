import { create } from "zustand";

// .omni:27 -- store UIStore

interface UIStoreState {
  sidebar_open: boolean;
  theme: string;
  selected_page: string;
  is_dark: () => any;
  toggle_sidebar: () => void;
  set_page: (page: string) => void;
}

export const useUIStore = create<UIStoreState>()((set, get) => ({
  sidebar_open: true,
  theme: "light",
  selected_page: "dashboard",
  is_dark: () => {
    const state = get();
    return (state.theme === "dark");
  },
  toggle_sidebar: () => {
    set(state => ({ sidebar_open: !state.sidebar_open }));
  },
  set_page: (page) => {
    set(state => ({ selected_page: page }));
  },
}));