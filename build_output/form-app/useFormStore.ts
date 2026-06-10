import { create } from "zustand";

// .omni:5 -- store FormStore

interface FormStoreState {
  submitted: boolean;
  submit_count: number;
  submit: () => void;
  reset: () => void;
}

export const useFormStore = create<FormStoreState>()((set, get) => ({
  submitted: false,
  submit_count: 0,
  submit: () => {
    set(state => ({ submitted: true }));
    set(state => ({ submit_count: (state.submit_count + 1) }));
  },
  reset: () => {
    set(state => ({ submitted: false }));
  },
}));