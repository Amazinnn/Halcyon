<script setup lang="ts">
defineProps<{
  data: { detail: string };
}>();
</script>

<template>
  <div class="trigger-node">
    <span class="title">触发</span>
    <span class="detail">{{ data.detail }}</span>
  </div>
</template>

<style scoped>
.trigger-node {
  width: 116px;
  padding: 8px 10px;
  border: 1px dashed rgba(163, 230, 53, 0.65);
  border-radius: var(--r-md, 8px);
  background: rgba(16, 26, 21, 0.88);
  color: var(--text-mid);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.title { color: var(--accent-bright); font-size: 11px; font-weight: 700; }
.detail { font-size: 10px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
