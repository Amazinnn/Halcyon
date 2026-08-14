// Declarative frontend window registry (ADR-0037, OpenSpec `window-registry`).
// Mirrors the Rust `WINDOW_SPECS` labels; adding a window means one entry here
// plus one Rust `WindowSpec` entry plus a capability list sync.
import type { Component } from "vue";
import DesktopView from "../views/desktop/DesktopView.vue";
import ChatView from "../views/chat/ChatView.vue";
import StatsView from "../views/stats/StatsView.vue";
import MusicView from "../views/music/MusicView.vue";
import WorkflowView from "../views/workflow/WorkflowView.vue";
import PetView from "../views/pet/PetView.vue";
import PetBubbleView from "../views/pet/PetBubbleView.vue";
import TopbarView from "../views/topbar/TopbarView.vue";
import GridOverlayView from "../views/overlay/GridOverlayView.vue";

export type ViewKind = "desktop" | "float" | "bubble" | "overlay" | "topbar";

export interface ViewSpec {
  label: string;
  kind: ViewKind;
  /** Tray entry title (float views). */
  title: string;
  /** AppIcon name for the tray entry (float views). */
  icon: string;
  /** Whether the desktop view tray shows an entry for this window. */
  inTray: boolean;
  component: Component;
  transparent: boolean;
}

export const VIEW_REGISTRY: readonly ViewSpec[] = [
  { label: "desktop", kind: "desktop", title: "Focus Desktop", icon: "panel", component: DesktopView, transparent: false, inTray: false },
  { label: "chat", kind: "float", title: "对话", icon: "chat", component: ChatView, transparent: true, inTray: true },
  { label: "stats", kind: "float", title: "统计", icon: "stats", component: StatsView, transparent: true, inTray: true },
  { label: "music", kind: "float", title: "音乐", icon: "music", component: MusicView, transparent: true, inTray: true },
  { label: "pet", kind: "float", title: "桌宠", icon: "panel", component: PetView, transparent: true, inTray: false },
  { label: "pet-bubble", kind: "bubble", title: "气泡", icon: "panel", component: PetBubbleView, transparent: true, inTray: false },
  { label: "workflow", kind: "float", title: "工作流", icon: "panel", component: WorkflowView, transparent: true, inTray: true },
  { label: "grid-overlay", kind: "overlay", title: "Grid Overlay", icon: "panel", component: GridOverlayView, transparent: true, inTray: false },
  { label: "topbar", kind: "topbar", title: "状态", icon: "panel", component: TopbarView, transparent: true, inTray: false },
];

/** Component for a window label; unknown labels fall back to the desktop view. */
export function viewForLabel(label: string): Component {
  return VIEW_REGISTRY.find((v) => v.label === label)?.component ?? DesktopView;
}

/** Tray entries: the float views flagged for the tray, in registry order. */
export function floatViews(): ViewSpec[] {
  return VIEW_REGISTRY.filter((v) => v.kind === "float" && v.inTray);
}

export function isTransparentLabel(label: string): boolean {
  return VIEW_REGISTRY.find((v) => v.label === label)?.transparent ?? false;
}
