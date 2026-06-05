import { create } from "zustand";

// .omni:4 -- store ReviewStore

interface ReviewStoreState {
  reviews: any[];
  filter_status: string;
  next_id: number;
  filtered_reviews: () => any[];
  pending_count: () => number;
  total_count: () => number;
  add_review: (title: string) => void;
  set_filter: (status: string) => void;
  clear_all: () => void;
}

export const useReviewStore = create<ReviewStoreState>()((set, get) => ({
  reviews: [],
  filter_status: "all",
  next_id: 1,
  filtered_reviews: () => {
    const state = get();
    return state.reviews;
  },
  pending_count: () => {
    const state = get();
    return state.next_id;
  },
  total_count: () => {
    const state = get();
    return state.next_id;
  },
  add_review: (title) => {
    set(state => ({ reviews: [...state.reviews, title] }));
    set(state => ({ next_id: (state.next_id + 1) }));
  },
  set_filter: (status) => {
    set(state => ({ filter_status: status }));
  },
  clear_all: () => {
    set(state => ({ reviews: [] }));
    set(state => ({ next_id: 1 }));
  },
}));