export interface PetFrameSize {
  width: number;
  height: number;
}

export interface PetCanvasMetrics {
  cssWidth: number;
  cssHeight: number;
  backingWidth: number;
  backingHeight: number;
  devicePixelRatio: number;
}

export class LatestPetRequest {
  private generation = 0;

  issue(): number {
    this.generation += 1;
    return this.generation;
  }

  isCurrent(generation: number): boolean {
    return generation === this.generation;
  }

  invalidate(): void {
    this.generation += 1;
  }
}

export interface PetPackageRequest {
  generation: number;
  characterId: string;
}

export interface PetAnimationRequest {
  generation: number;
  packageGeneration: number;
  characterId: string;
  petId: string;
}

export class PetRequestCoordinator {
  private packageGeneration = 0;
  private animationGeneration = 0;
  private loadedPackage: {
    generation: number;
    characterId: string;
    petId: string;
  } | null = null;
  private packagePending = false;

  beginPackage(characterId: string): PetPackageRequest {
    this.packageGeneration += 1;
    this.animationGeneration += 1;
    this.packagePending = true;
    return { generation: this.packageGeneration, characterId };
  }

  isCurrentPackage(request: PetPackageRequest): boolean {
    return request.generation === this.packageGeneration;
  }

  commitPackage(request: PetPackageRequest, petId: string): boolean {
    if (!this.isCurrentPackage(request)) return false;
    this.packagePending = false;
    this.loadedPackage = {
      generation: request.generation,
      characterId: request.characterId,
      petId,
    };
    return true;
  }

  clearPackage(request: PetPackageRequest): boolean {
    if (!this.isCurrentPackage(request)) return false;
    this.packagePending = false;
    this.loadedPackage = null;
    return true;
  }

  beginAnimation(characterId: string): PetAnimationRequest | null {
    const loaded = this.loadedPackage;
    if (this.packagePending || !loaded || loaded.characterId !== characterId) return null;
    this.animationGeneration += 1;
    return {
      generation: this.animationGeneration,
      packageGeneration: loaded.generation,
      characterId,
      petId: loaded.petId,
    };
  }

  isCurrentAnimation(request: PetAnimationRequest): boolean {
    const loaded = this.loadedPackage;
    return request.generation === this.animationGeneration
      && request.packageGeneration === this.packageGeneration
      && loaded?.generation === request.packageGeneration
      && loaded.characterId === request.characterId
      && loaded.petId === request.petId;
  }

  invalidate(): void {
    this.packageGeneration += 1;
    this.animationGeneration += 1;
    this.packagePending = false;
    this.loadedPackage = null;
  }
}

export function replacePetBitmap<T extends { close(): void }>(current: T | null, next: T | null): T | null {
  if (current && current !== next) current.close();
  return next;
}

export function containPetFrame(
  sourceWidth: number,
  sourceHeight: number,
  stageWidth: number,
  stageHeight: number,
  inset: number,
  horizontalCorrection = 1,
): PetFrameSize {
  const correctedWidth = sourceWidth * horizontalCorrection;
  const safeWidth = Math.max(1, stageWidth - inset * 2);
  const safeHeight = Math.max(1, stageHeight - inset * 2);
  const scale = Math.min(safeWidth / correctedWidth, safeHeight / sourceHeight);
  return {
    width: Math.max(1, Math.floor(correctedWidth * scale)),
    height: Math.max(1, Math.floor(sourceHeight * scale)),
  };
}

export function petCanvasMetrics(
  sourceWidth: number,
  sourceHeight: number,
  stageWidth: number,
  stageHeight: number,
  inset: number,
  devicePixelRatio: number,
  horizontalCorrection = 1,
): PetCanvasMetrics {
  const frame = containPetFrame(
    sourceWidth,
    sourceHeight,
    stageWidth,
    stageHeight,
    inset,
    horizontalCorrection,
  );
  const dpr = Number.isFinite(devicePixelRatio) ? Math.max(1, devicePixelRatio) : 1;
  return {
    cssWidth: frame.width,
    cssHeight: frame.height,
    backingWidth: Math.max(1, Math.round(frame.width * dpr)),
    backingHeight: Math.max(1, Math.round(frame.height * dpr)),
    devicePixelRatio: dpr,
  };
}
