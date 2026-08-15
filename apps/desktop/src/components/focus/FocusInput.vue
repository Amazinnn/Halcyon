<script setup lang="ts">
/** FocusInput (ADR-0038): text/number input with the shared glass style. */
withDefaults(defineProps<{
  modelValue: string | number;
  type?: "text" | "number";
  placeholder?: string;
  min?: number;
  max?: number;
}>(), { type: "text" });
const emit = defineEmits<{ (e: "update:modelValue", v: string | number): void }>();
</script>

<template>
  <input
    class="focus-input"
    :type="type"
    :placeholder="placeholder"
    :min="min"
    :max="max"
    :value="modelValue"
    @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>

<style scoped>
.focus-input {
  border: 1px solid var(--glass-border);
  background: var(--glass-strong);
  color: var(--text-hi);
  border-radius: var(--r-sm);
  padding: 4px 8px;
  font-size: var(--fs-md);
  font-family: inherit;
  min-width: var(--ctrl-min-input);
}
.focus-input:focus { outline: none; border-color: var(--accent); }
</style>