import { describe, expect, it } from "vitest";
import {
  VIEW_REGISTRY,
  viewForLabel,
  floatViews,
  isTransparentLabel,
} from "./view-registry";
import DesktopView from "../views/desktop/DesktopView.vue";
import ChatView from "../views/chat/ChatView.vue";
import StatsView from "../views/stats/StatsView.vue";
import MusicView from "../views/music/MusicView.vue";
import WorkflowView from "../views/workflow/WorkflowView.vue";
import PetView from "../views/pet/PetView.vue";
import PetBubbleView from "../views/pet/PetBubbleView.vue";
import TopbarView from "../views/topbar/TopbarView.vue";
import GridOverlayView from "../views/overlay/GridOverlayView.vue";

describe("view registry", () => {
  it("maps every window label to its component", () => {
    expect(viewForLabel("desktop")).toBe(DesktopView);
    expect(viewForLabel("chat")).toBe(ChatView);
    expect(viewForLabel("stats")).toBe(StatsView);
    expect(viewForLabel("music")).toBe(MusicView);
    expect(viewForLabel("workflow")).toBe(WorkflowView);
    expect(viewForLabel("pet")).toBe(PetView);
    expect(viewForLabel("pet-bubble")).toBe(PetBubbleView);
    expect(viewForLabel("topbar")).toBe(TopbarView);
    expect(viewForLabel("grid-overlay")).toBe(GridOverlayView);
  });

  it("falls back to the desktop view for unknown labels", () => {
    expect(viewForLabel("nope")).toBe(DesktopView);
    expect(viewForLabel("")).toBe(DesktopView);
  });

  it("flags every window except desktop as transparent", () => {
    for (const v of VIEW_REGISTRY) {
      expect(isTransparentLabel(v.label)).toBe(v.transparent);
    }
    expect(isTransparentLabel("desktop")).toBe(false);
    expect(isTransparentLabel("unknown")).toBe(false);
  });

  it("lists exactly the tray floats in registry order", () => {
    const views = floatViews();
    expect(views.map((v) => v.label)).toEqual(["chat", "stats", "music", "workflow"]);
    expect(views.map((v) => v.title)).toEqual(["对话", "统计", "音乐", "工作流"]);
    expect(views.map((v) => v.icon)).toEqual(["chat", "stats", "music", "panel"]);
  });

  it("keeps every label unique", () => {
    const labels = VIEW_REGISTRY.map((v) => v.label);
    expect(new Set(labels).size).toBe(labels.length);
  });
});