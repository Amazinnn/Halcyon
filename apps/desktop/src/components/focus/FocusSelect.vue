<script setup lang="ts">
/** FocusSelect (ADR-0038): native select with the shared glass style. */
withDefaults(defineProps<{ modelValue: string; options: { label: string; value: string }[]; disabled?: boolean }>(), { disabled: false });
const emit = defineEmits<{ (e: "update:modelValue", v: string): void }>();
</script>

<template>
  <select
    class="focus-select"
    :value="modelValue"
    :disabled="disabled"
    @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
  >
    <option v-for="opt in options" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
  </select>
</template>

<style scoped>
.focus-select {
  border: 1px solid var(--glass-border);
  background: var(--glass-strong);
  color: var(--text-hi);
  border-radius: var(--r-sm);
  padding: 2px 5px;
  font-size: var(--fs-sm);
  cursor: pointer;
  font: inherit;
  min-width: 0;
}
</style>
