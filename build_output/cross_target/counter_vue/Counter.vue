<script setup lang="ts">
import { ref } from "vue";
// .omni:3 -- component Counter

const count = ref<number>(0);
function increment() {
    count.value = (count + 1);
}
function decrement() {
    count.value = (count - 1);
}
function reset() {
    count.value = 0;
}
</script>

<template>
<div>
<h1 data-testid="counter-label">
Count: 
{{ count }}
</h1>
<div style="flex-direction: row; gap: 8px; display: flex">
<button @click="decrement" data-testid="btn-decrement">-</button>
<button @click="increment" data-testid="btn-increment">+</button>
<button @click="reset" data-testid="btn-reset">Reset</button>
</div>
</div>
</template>