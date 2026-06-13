<script setup lang="ts">
import { ref } from "vue";
import { useUserStore } from "./stores/useUserStore";
// .omni:18 -- component UserProfile
const store = useUserStore();

const input_name = ref<string>("");
function handle_login() {
    store.login(input_name);
    input_name.value = "";
}
function handle_logout() {
    store.logout();
}
</script>

<template>
<div>
<h2 data-testid="welcome">
Welcome, 
{{ store.display_name }}
!
</h2>
<span data-testid="login-count">
Logins: 
{{ store.login_count }}
</span>
<div style="flex-direction: row; gap: 8px; display: flex">
<input placeholder="Enter name" v-model="input_name" data-testid="input-name" />
<button @click="handle_login" data-testid="btn-login">Login</button>
<button @click="handle_logout" data-testid="btn-logout">Logout</button>
</div>
</div>
</template>