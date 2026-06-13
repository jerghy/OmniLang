import { ref, computed } from "vue";
import { defineStore } from "pinia";

// .omni:3 -- store UserStore
interface UserStoreState {
  username: string;
  login_count: number;
}

export const useUserStore = defineStore('UserStore', () => {
  const username = ref<string>("Guest");
  const login_count = ref<number>(0);
  const display_name = computed(() => username.value);
  function login(name: string) {
    username.value = name;
    login_count.value = (login_count.value + 1);
  }
  function logout() {
    username.value = "Guest";
  }
  return { username, login_count, display_name, login, logout };
});