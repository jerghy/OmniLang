import { create } from "zustand";

// .omni:1 -- store TodoStore

interface TodoStoreState {
  task_count: number;
  display_count: () => number;
  add_task: () => void;
}

export const useTodoStore = create<TodoStoreState>()((set, get) => ({
  task_count: 0,
  display_count: () => {
    const state = get();
    return state.task_count;
  },
  add_task: () => {
    set(state => ({ task_count: (state.task_count + 1) }));
  },
}));