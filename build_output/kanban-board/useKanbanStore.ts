import { create } from "zustand";

// .omni:1 -- store KanbanStore

interface KanbanStoreState {
  todoTasks: string[];
  inProgressTasks: string[];
  doneTasks: string[];
  todoCount: () => any;
  inProgressCount: () => any;
  doneCount: () => any;
  totalCount: () => any;
  addTask: (title: string) => void;
  moveToProgress: (title: string) => void;
  moveToDone: (title: string) => void;
  moveToTodo: (title: string) => void;
  moveBackToProgress: (title: string) => void;
  clearDone: () => void;
  resetAll: () => void;
}

export const useKanbanStore = create<KanbanStoreState>()((set, get) => ({
  todoTasks: ["Buy groceries", "Call dentist", "Review PR"],
  inProgressTasks: ["Write report", "Fix login bug"],
  doneTasks: ["Setup CI/CD", "Update docs"],
  todoCount: () => {
    const state = get();
    return state.todoTasks.length;
  },
  inProgressCount: () => {
    const state = get();
    return state.inProgressTasks.length;
  },
  doneCount: () => {
    const state = get();
    return state.doneTasks.length;
  },
  totalCount: () => {
    const state = get();
    return ((state.todoCount() + state.inProgressCount()) + state.doneCount());
  },
  addTask: (title) => {
    set(state => ({ todoTasks: [...state.todoTasks, title] }));
  },
  moveToProgress: (title) => {
    set(state => ({ inProgressTasks: [...state.inProgressTasks, title] }));
  },
  moveToDone: (title) => {
    set(state => ({ doneTasks: [...state.doneTasks, title] }));
  },
  moveToTodo: (title) => {
    set(state => ({ todoTasks: [...state.todoTasks, title] }));
  },
  moveBackToProgress: (title) => {
    set(state => ({ inProgressTasks: [...state.inProgressTasks, title] }));
  },
  clearDone: () => {
    set(state => ({ doneTasks: [] }));
  },
  resetAll: () => {
    set(state => ({ todoTasks: [] }));
    set(state => ({ inProgressTasks: [] }));
    set(state => ({ doneTasks: [] }));
  },
}));