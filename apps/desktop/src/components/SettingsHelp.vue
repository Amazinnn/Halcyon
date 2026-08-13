<script setup lang="ts">
defineProps<{ summary: string; detail: string }>();
</script>

<template>
  <div class="settings-help">
    <p class="summary">{{ summary }}</p>
    <div class="detail" role="tooltip">{{ detail }}</div>
  </div>
</template>

<style scoped>
.settings-help { position: relative; }
.summary { margin: -2px 0 0; color: var(--text-low); font-size: 11px; line-height: 1.45; }
.detail {
  position: absolute; z-index: 40; left: 0; bottom: calc(100% + 6px);
  width: min(268px, 76vw); padding: 8px 10px; border-radius: 6px;
  background: rgba(10, 12, 11, 0.96); border: 1px solid rgba(255, 255, 255, 0.13);
  color: rgba(255, 255, 255, 0.9); font-size: 11px; line-height: 1.45;
  box-shadow: 0 8px 18px rgba(0, 0, 0, 0.32); pointer-events: none;
  opacity: 0; transform: translateY(3px); transition: opacity 120ms ease, transform 120ms ease;
}
.settings-help:hover .detail, .settings-help:focus-within .detail { opacity: 1; transform: translateY(0); }
</style>
