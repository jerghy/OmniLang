import { create } from "zustand";

// .omni:3 -- store UserStore

interface UserStoreState {
  username: string;
  login_count: number;
  display_name: () => string;
  login: (name: string) => void;
  logout: () => void;
}

export const useUserStore = create<UserStoreState>()((set, get) => ({
  username: "Guest",
  login_count: 0,
  display_name: () => {
    const state = get();
    return state.username;
  },
  login: (name) => {
    set(state => ({ username: name }));
    set(state => ({ login_count: (state.login_count + 1) }));
  },
  logout: () => {
    set(state => ({ username: "Guest" }));
  },
}));