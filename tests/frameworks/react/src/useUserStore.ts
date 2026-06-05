import { create } from "zustand";

// .omni:3 -- store UserStore

interface UserStoreState {
  name: string;
  role: string;
  logged_in: boolean;
  login: (user: string) => void;
  logout: () => void;
}

export const useUserStore = create<UserStoreState>()((set, get) => ({
  name: "Guest",
  role: "user",
  logged_in: false,
  login: (user) => {
    set(state => ({ name: user }));
    set(state => ({ logged_in: true }));
  },
  logout: () => {
    set(state => ({ name: "Guest" }));
    set(state => ({ logged_in: false }));
  },
}));