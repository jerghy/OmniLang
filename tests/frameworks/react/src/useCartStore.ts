import { create } from "zustand";

// .omni:1 -- store CartStore

interface CartStoreState {
  item_count: number;
  discount: number;
  cart_total: () => number;
  cart_count: () => number;
  add_item: () => void;
  clear_cart: () => void;
}

export const useCartStore = create<CartStoreState>()((set, get) => ({
  item_count: 0,
  discount: 0,
  cart_total: () => {
    const state = get();
    return state.discount;
  },
  cart_count: () => {
    const state = get();
    return state.item_count;
  },
  add_item: () => {
    set(state => ({ item_count: (state.item_count + 1) }));
  },
  clear_cart: () => {
    set(state => ({ item_count: 0 }));
  },
}));