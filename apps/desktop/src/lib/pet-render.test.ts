import { describe, expect, it, vi } from "vitest";
import {
  containPetFrame,
  LatestPetRequest,
  PetRequestCoordinator,
  petCanvasMetrics,
  replacePetBitmap,
} from "./pet-render";

describe("pet frame layout", () => {
  it("contains a non-square frame within the safe stage without stretching it", () => {
    expect(containPetFrame(256, 128, 180, 180, 8)).toEqual({ width: 164, height: 82 });
  });

  it("uses the maximum safe height for a tall frame", () => {
    expect(containPetFrame(120, 240, 200, 140, 8)).toEqual({ width: 62, height: 124 });
  });

  it("applies horizontal correction without stretching the canvas or leaving the safe stage", () => {
    expect(containPetFrame(120, 240, 200, 140, 8, 1.25)).toEqual({ width: 77, height: 124 });
  });

  it("keeps CSS and device-pixel canvas sizes derived from one geometry", () => {
    expect(petCanvasMetrics(256, 128, 180, 180, 8, 2, 1)).toEqual({
      cssWidth: 164,
      cssHeight: 82,
      backingWidth: 328,
      backingHeight: 164,
      devicePixelRatio: 2,
    });
  });

  it("keeps the calibrated subject contained in all four supported pet window ratios", () => {
    for (const [width, height] of [[192, 208], [192, 416], [384, 208], [384, 416]]) {
      const frame = containPetFrame(187, 234, width, height, 8);
      expect(frame.width).toBeLessThanOrEqual(width - 16);
      expect(frame.height).toBeLessThanOrEqual(height - 16);
      expect(frame.width / frame.height).toBeCloseTo(187 / 234, 2);
    }
  });

  it("rejects stale async pet loads after a newer request starts", () => {
    const requests = new LatestPetRequest();
    const first = requests.issue();
    const second = requests.issue();
    expect(requests.isCurrent(first)).toBe(false);
    expect(requests.isCurrent(second)).toBe(true);
  });

  it("keeps package and animation request generations independent", () => {
    const requests = new PetRequestCoordinator();
    const packageRequest = requests.beginPackage("agent-a");

    expect(requests.commitPackage(packageRequest, "pack-a")).toBe(true);
    const firstAnimation = requests.beginAnimation("agent-a");
    const secondAnimation = requests.beginAnimation("agent-a");

    expect(firstAnimation).not.toBeNull();
    expect(secondAnimation).not.toBeNull();
    expect(requests.isCurrentPackage(packageRequest)).toBe(true);
    expect(requests.isCurrentAnimation(firstAnimation!)).toBe(false);
    expect(requests.isCurrentAnimation(secondAnimation!)).toBe(true);
  });

  it("invalidates old animation work when a package refresh starts", () => {
    const requests = new PetRequestCoordinator();
    const firstPackage = requests.beginPackage("agent-a");
    expect(requests.commitPackage(firstPackage, "pack-a")).toBe(true);
    const oldAnimation = requests.beginAnimation("agent-a");

    const nextPackage = requests.beginPackage("agent-a");

    expect(oldAnimation).not.toBeNull();
    expect(requests.isCurrentAnimation(oldAnimation!)).toBe(false);
    expect(requests.beginAnimation("agent-a")).toBeNull();
    expect(requests.commitPackage(nextPackage, "pack-b")).toBe(true);
    expect(requests.beginAnimation("agent-a")).not.toBeNull();
  });

  it("refuses a stale package commit after the current Agent changes", () => {
    const requests = new PetRequestCoordinator();
    const oldPackage = requests.beginPackage("agent-a");
    const currentPackage = requests.beginPackage("agent-b");

    expect(requests.commitPackage(oldPackage, "pack-a")).toBe(false);
    expect(requests.commitPackage(currentPackage, "pack-b")).toBe(true);
    expect(requests.beginAnimation("agent-a")).toBeNull();
    expect(requests.beginAnimation("agent-b")).not.toBeNull();
  });

  it("closes the previous decoded atlas when replacing or clearing it", () => {
    const oldBitmap = { close: vi.fn() };
    const nextBitmap = { close: vi.fn() };
    expect(replacePetBitmap(oldBitmap, nextBitmap)).toBe(nextBitmap);
    expect(oldBitmap.close).toHaveBeenCalledOnce();
    expect(nextBitmap.close).not.toHaveBeenCalled();
    expect(replacePetBitmap(nextBitmap, null)).toBeNull();
    expect(nextBitmap.close).toHaveBeenCalledOnce();
  });
});
