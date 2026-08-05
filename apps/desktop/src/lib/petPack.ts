// Pet-pack manifest validation (design doc §5.4) + the built-in placeholder.
// V1 uses a sprite-sheet format with a validated manifest; no Live2D and no
// third-party assets are imported (ADR-0004).

export interface PetAnimation {
  frames: number[];
  fps: number;
  loop: boolean;
}

export interface PetManifest {
  schemaVersion: number;
  id: string;
  name: string;
  author: string;
  license: string;
  bubbleAnchor?: { x: number; y: number };
  animations: Record<string, PetAnimation>;
}

export function validatePetManifest(raw: unknown): PetManifest {
  const m = raw as PetManifest;
  if (!m || typeof m !== "object") throw new Error("pet-pack: manifest is not an object");
  if (m.schemaVersion !== 1) throw new Error(`pet-pack: unsupported schemaVersion ${m.schemaVersion}`);
  if (!m.id || typeof m.id !== "string") throw new Error("pet-pack: missing id");
  if (!m.name || typeof m.name !== "string") throw new Error("pet-pack: missing name");
  if (!m.author || typeof m.author !== "string") throw new Error("pet-pack: missing author");
  if (!m.license || typeof m.license !== "string") throw new Error("pet-pack: missing license");
  if (!m.animations || typeof m.animations !== "object" || Object.keys(m.animations).length === 0) {
    throw new Error("pet-pack: animations must be a non-empty object");
  }
  for (const [name, anim] of Object.entries(m.animations)) {
    if (!anim || !Array.isArray(anim.frames) || anim.frames.length === 0) {
      throw new Error(`pet-pack: animation "${name}" frames must be non-empty`);
    }
    if (typeof anim.fps !== "number" || anim.fps <= 0) {
      throw new Error(`pet-pack: animation "${name}" fps must be > 0`);
    }
    if (typeof anim.loop !== "boolean") {
      throw new Error(`pet-pack: animation "${name}" loop must be boolean`);
    }
  }
  return m;
}

export const BUILTIN_PET_MANIFEST: PetManifest = {
  schemaVersion: 1,
  id: "focus.builtin.placeholder",
  name: "Placeholder Pet",
  author: "Focus Desktop Contributors",
  license: "MIT",
  bubbleAnchor: { x: 0.5, y: 0.05 },
  animations: {
    idle: { frames: [0], fps: 1, loop: true },
    thinking: { frames: [0, 1, 2, 3], fps: 8, loop: true },
    editing: { frames: [2, 3, 4], fps: 6, loop: true },
    waiting: { frames: [4, 5, 4], fps: 4, loop: true },
    success: { frames: [0, 5], fps: 10, loop: false },
    error: { frames: [5], fps: 1, loop: true },
  },
};