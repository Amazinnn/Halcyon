<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ name: string }>();

const paths = computed<{ d: string; filled?: boolean }[]>(() => {
  switch (props.name) {
    case "play": return [{ d: "M8 5v14l11-7z" }];
    case "pause": return [{ d: "M7 5h4v14H7zM13 5h4v14h-4z" }];
    case "prev": return [{ d: "M7 6h2v12H7zM19 6l-9 6 9 6z" }];
    case "next": return [{ d: "M15 6h2v12h-2zM5 6l9 6-9 6z" }];
    case "pin": return [{ d: "M16 3l5 5-4 1-4 4 4 4-2 2-4-4-5 5-2-2 5-5-4-4 2-2 4 4 4-4z" }];
    case "pin-off": return [{ d: "M9 3l12 12-2 2-2-2-4 4-2-2 2-2-4-4-2 2-2-2zM4 4l2-2 14 14-2 2z" }];
    case "collapse": return [{ d: "M12 5v14M5 12h14" }];
    case "chat": return [{ d: "M4 5h16v11H9l-5 4z" }];
    case "stats": return [{ d: "M5 20V10M12 20V4M19 20v-7" }];
    case "music": return [{ d: "M9 18V5l12-2v13" }, { d: "M7 20a3 3 0 1 0 2-5.83M19 16a3 3 0 1 0 2-5.83" }];
    case "image": return [{ d: "M4 5h16v14H4zM4 15l5-5 4 4 3-3 4 4" }];
    case "close": return [{ d: "M6 6l12 12M18 6L6 18" }];
    case "leaf": return [{ d: "M12 20C12 10 16 6 22 4c0 10-4 14-10 16zM12 20C12 10 8 6 2 4c0 10 4 14 10 16z" }];
    case "file": return [{ d: "M7 3h7l6 6v12H7z" }, { d: "M14 3v6h6" }];
    case "folder": return [{ d: "M3 7h7l2 2h9v10H3z" }];
    case "app": return [{ d: "M5 5h5v5H5zM14 5h5v5h-5zM5 14h5v5H5zM14 14h5v5h-5z" }];
    case "plus": return [{ d: "M12 5v14M5 12h14" }];
    case "settings": return [{ d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" }, { d: "M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9L17 7M7 17l-2.1 2.1" }];
    case "power": return [{ d: "M12 3v9" }, { d: "M6.3 6.3a8 8 0 1 0 11.4 0" }];
    default: return [{ d: "" }];
  }
});
</script>

<template>
  <svg
    viewBox="0 0 24 24"
    width="16"
    height="16"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path v-for="(p, i) in paths" :key="i" :d="p.d" />
  </svg>
</template>