export type InlineComposerPart =
  | { kind: "skill"; name: string }
  | { kind: "text"; text: string };

export function serializeInlineComposer(parts: InlineComposerPart[]): string {
  return parts.map((part) => part.kind === "skill" ? `$${part.name}` : part.text).join("");
}

export function insertSkillToken(
  parts: InlineComposerPart[],
  index: number,
  name: string,
): InlineComposerPart[] {
  const at = Math.max(0, Math.min(index, parts.length));
  return [
    ...parts.slice(0, at),
    { kind: "skill", name },
    { kind: "text", text: "  " },
    ...parts.slice(at),
  ];
}

export function removeAdjacentSkillToken(
  parts: InlineComposerPart[],
  index: number,
  direction: "backward" | "forward",
): InlineComposerPart[] {
  const target = direction === "backward" ? index - 1 : index;
  if (parts[target]?.kind !== "skill") return parts;
  const after = parts[target + 1];
  const removeAfter = after?.kind === "text" && /^\s+$/.test(after.text) ? 1 : 0;
  return [...parts.slice(0, target), ...parts.slice(target + 1 + removeAfter)];
}

export function hasInlineComposerBody(parts: InlineComposerPart[]): boolean {
  return parts.some((part) => part.kind === "text" && part.text.trim().length > 0);
}
