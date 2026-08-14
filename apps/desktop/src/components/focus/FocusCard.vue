<script setup lang="ts">
/** FocusCard (ADR-0038): glass card with optional header/note. */
defineProps<{ title?: string; note?: string }>();
</script>

<template>
  <div class="focus-card glass">
    <h4 v-if="title" class="fc-title">{{ title }}</h4>
    <slot />
    <p v-if="note" class="fc-note">{{ note }}</p>
  </div>
</template>

<style scoped>
.focus-card { display: flex; flex-direction: column; gap: 8px; padding-top: 8px; border-top: 1px solid var(--glass-border); }
.fc-title { margin: 0; font-size: var(--fs-sm); color: var(--text-low); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
.fc-note { margin: -2px 0 0; color: var(--text-low); font-size: var(--fs-sm); line-height: 1.45; }
</style>
