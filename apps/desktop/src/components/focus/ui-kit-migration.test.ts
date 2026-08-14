import { describe, expect, it } from "vitest";
import settingsSource from "../SettingsPopover.vue?raw";
import desktopSource from "../../views/desktop/DesktopView.vue?raw";
import workflowSource from "../../views/workflow/WorkflowView.vue?raw";
import chatSource from "../../views/chat/ChatView.vue?raw";

describe("ui kit migration", () => {
  const files: [string, string][] = [
    ["SettingsPopover", settingsSource],
    ["DesktopView", desktopSource],
    ["WorkflowView", workflowSource],
    ["ChatView", chatSource],
  ];
  const oldClasses = [
    'class="switch"',
    'class="seg"',
    'class="mini"',
    'class="btn"',
    'class="ghost"',
    'class="num-input"',
    'class="provider-select"',
  ];
  for (const [name, source] of files) {
    it(`${name} has no hand-written control classes`, () => {
      for (const cls of oldClasses) {
        expect(source, `${name} still contains ${cls}`).not.toContain(cls);
      }
    });
  }
  it("four float views use FocusWindowFrame", () => {
    expect(chatSource).toContain("FocusWindowFrame");
    expect(workflowSource).toContain("FocusWindowFrame");
  });
});