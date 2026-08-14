<script setup lang="ts">
/**
 * FocusButton (ADR-0038): one button for every existing variant.
 * size preserves the historical paddings exactly:
 * tight = 1px 5px/10px, xs = 2px 8px/10px, sm = 2px 8px/11px,
 * md = 4px 10px/12px, lg = 5px 10px/12px, icon = 3px.
 */
const props = withDefaults(defineProps<{
  variant?: "default" | "glass" | "ghost" | "accent" | "danger";
  size?: "tight" | "xs" | "sm" | "md" | "lg" | "icon";
  off?: boolean;
}>(), { variant: "default", size: "md", off: false });
import { computed } from "vue";
const cls = computed(() => [
  `v-${props.variant}`,
  `s-${props.size}`,
  { off: props.off },
]);
</script>

<template>
  <button class="focus-btn" :class="cls">
    <slot />
  </button>
</template>

<style scoped>
.focus-btn {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  cursor: pointer;
  font: inherit;
  line-height: 1.4;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
}
.focus-btn:disabled { opacity: 0.45; cursor: default; }
.off { opacity: 0.5; }
/* sizes (historical paddings, one source) */
.s-tight { padding: 1px 5px; font-size: var(--fs-xs); }
.s-xs { padding: 2px 8px; font-size: var(--fs-xs); }
.s-sm { padding: 2px 8px; font-size: var(--fs-sm); }
.s-md { padding: 4px 10px; font-size: var(--fs-md); }
.s-lg { padding: 5px 10px; font-size: var(--fs-md); }
.s-icon { padding: 3px; }
/* variants */
.v-default:hover { color: var(--text-hi); border-color: var(--accent); }
.v-glass { background: var(--glass-strong); color: var(--text-hi); }
.v-glass:hover { border-color: var(--accent); color: var(--accent-bright); }
.v-ghost { border-color: transparent; }
.v-ghost:hover { color: var(--accent); background: var(--accent-wash); }
.v-accent { background: var(--accent); color: #0a110e; border-color: var(--accent); font-weight: 600; }
.v-danger { background: #b23c3c; border-color: #b23c3c; color: #fff; }
.v-danger:hover { background: #c94f4f; }
.v-ghost.v-danger { border-color: transparent; background: transparent; color: var(--text-mid); }
.v-ghost.v-danger:hover { color: #ff7b72; border-color: #ff7b72; background: transparent; }
</style>