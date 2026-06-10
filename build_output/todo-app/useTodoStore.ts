import { create } from "zustand";

// .omni:5 -- store TodoStore

interface TodoStoreState {
  tasks: string[];
  completed: string[];
  next_id: number;
  active_count: () => any;
  done_count: () => any;
  total_count: () => any;
  add_task: (text: string) => void;
  complete_task: (text: string) => void;
  uncomplete_task: (text: string) => void;
  remove_task: (text: string) => void;
  clear_completed: () => void;
}

export const useTodoStore = create<TodoStoreState>()((set, get) => ({
  tasks: [],
  completed: [],
  next_id: 1,
  active_count: () => {
    const state = get();
    return state.tasks.length;
  },
  done_count: () => {
    const state = get();
    return state.completed.length;
  },
  total_count: () => {
    const state = get();
    return (state.tasks.length + state.completed.length);
  },
  add_task: (text) => {
    set(state => ({ tasks: [...state.tasks, text] }));
  },
  complete_task: (text) => {
    set(state => ({ completed: [...state.completed, text] }));
  },
  uncomplete_task: (text) => {
    set(state => ({ tasks: [...state.tasks, text] }));
  },
  remove_task: (text) => {
    console.warn('Action "remove_task" is not implemented');
  },
  clear_completed: () => {
    set(state => ({ completed: [] }));
  },
}));