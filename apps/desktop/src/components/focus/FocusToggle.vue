<script setup lang="ts">
/** FocusToggle (ADR-0038): pill switch with accent on-state. */
defineProps<{ modelValue: boolean; label?: string }>();
const emit = defineEmits<{ (e: "update:modelValue", v: boolean): void }>();
</script>

<template>
  <button
    type="button"
    class="focus-toggle"
    :class="{ on: modelValue }"
    :aria-pressed="modelValue"
    @click="emit('update:modelValue', !modelValue)"
  >
    {{ label }}
  </button>
</template>

<style scoped>
.focus-toggle {
  border: 1px solid var(--glass-border);
  background: var(--glass-strong);
  color: var(--text-low);
  border-radius: var(--r-pill);
  padding: 4px 14px;
  font-size: var(--fs-md);
  cursor: pointer;
  font: inherit;
}
.focus-toggle.on { background: var(--accent); color: #0a110e; border-color: var(--accent); font-weight: 600; }
</style>
