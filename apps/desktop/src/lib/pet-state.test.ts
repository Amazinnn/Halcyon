import { describe, expect, it } from "vitest";
import { nextFocusPetState, type FocusPetState } from "./pet-state";

describe("Focus pet state adapter", () => {
  it("uses the focus timer only as the idle base state", () => {
    expect(nextFocusPetState("focusing", "idle")).toBe("focusing");
    expect(nextFocusPetState("resting", "idle")).toBe("resting");
  });

  it("maps active Agent work and waits to continuous Focus states", () => {
    expect(nextFocusPetState("resting", "thinking")).toBe("working");
    expect(nextFocusPetState("focusing", "waiting_permission")).toBe("waiting");
  });

  it("keeps success and failure as Focus-owned persistent states", () => {
    expect(nextFocusPetState("resting", "success")).toBe("happy");
    expect(nextFocusPetState("focusing", "error")).toBe("troubled");
  });

  it("does not invent a transient pet state for cancellation", () => {
    const previous: FocusPetState = "troubled";
    expect(nextFocusPetState("resting", "cancelled", previous)).toBe("resting");
  });

  it("clears a troubled state only when the next Agent task begins", () => {
    expect(nextFocusPetState("resting", "error")).toBe("troubled");
    expect(nextFocusPetState("resting", "idle", "troubled")).toBe("resting");
    expect(nextFocusPetState("resting", "thinking", "troubled")).toBe("working");
  });
});
