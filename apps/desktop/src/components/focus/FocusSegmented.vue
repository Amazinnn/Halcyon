<script setup lang="ts">
/**
 * FocusSegmented (ADR-0038): option row. variant keeps the historical
 * on-state styles: soft = accent-wash, solid = accent fill, pill =
 * capsule container with bright fill.
 */
defineProps<{
  options: { label: string; value: string; icon?: string }[];
  modelValue: string;
  variant?: "soft" | "solid" | "pill";
}>();
const emit = defineEmits<{ (e: "update:modelValue", v: string): void }>();
</script>

<template>
  <div class="focus-seg" :class="variant ?? 'soft'" role="group">
    <button
      v-for="opt in options"
      :key="opt.value"
      type="button"
      :class="{ on: modelValue === opt.value }"
      :aria-pressed="modelValue === opt.value"
      @click="emit('update:modelValue', opt.value)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>

<style scoped>
.focus-seg { display: flex; gap: 4px; }
.focus-seg button {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  padding: 3px 10px;
  font-size: var(--fs-md);
  cursor: pointer;
  font: inherit;
}
.focus-seg.soft button.on { background: var(--accent-wash); color: var(--accent-bright); border-color: var(--accent); }
.focus-seg.solid button { background: var(--glass-strong); }
.focus-seg.solid button.on { background: var(--accent); color: #0a110e; border-color: var(--accent); }
.focus-seg.pill {
  display: inline-flex;
  padding: 3px;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-pill);
  background: rgba(0, 0, 0, 0.14);
}
.focus-seg.pill button { border: 0; border-radius: var(--r-pill); padding: 5px 8px; }
.focus-seg.pill button.on { color: var(--bg-0); background: var(--accent-bright); }
</style>
