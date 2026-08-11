import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const sourceRoot = new URL("../", import.meta.url);

function source(relativePath: string): string {
  return readFileSync(new URL(relativePath, `${sourceRoot}/`), "utf8");
}

describe("float host corner alignment", () => {
  it("uses the pet-aligned radius for every floating WebView edge", () => {
    expect(source("styles.css")).toContain("--window-host-radius: 10px");
    expect(source("styles.css")).toContain("border-radius: var(--window-host-radius)");

    for (const path of [
      "views/chat/ChatView.vue",
      "views/stats/StatsView.vue",
      "views/music/MusicView.vue",
      "views/workflow/WorkflowView.vue",
      "views/pet/PetView.vue",
    ]) {
      expect(source(path)).toContain("border-radius: var(--window-host-radius)");
    }
  });
});
