<script setup lang="ts">
/** FocusSlider (ADR-0038): range slider with the accent thumb. */
withDefaults(defineProps<{
  modelValue: number;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
}>(), { min: 0, max: 100, step: 1, disabled: false });
const emit = defineEmits<{ (e: "update:modelValue", v: number): void }>();
</script>

<template>
  <input
    class="focus-slider"
    type="range"
    :min="min"
    :max="max"
    :step="step"
    :value="modelValue"
    :disabled="disabled"
    @input="emit('update:modelValue', Number(($event.target as HTMLInputElement).value))"
  />
</template>

<style scoped>
.focus-slider { accent-color: var(--accent); }
</style>
