export function composeSkillMessage(skill: string | null, text: string): string {
  const body = text.trim();
  return skill ? `$${skill}  ${body}` : body;
}

export function shouldRemoveSelectedSkill(
  key: string,
  text: string,
  selectionStart: number,
  selectionEnd: number,
): boolean {
  if (selectionStart !== selectionEnd) return false;
  return (
    (key === "Backspace" || key === "Delete") &&
    (text.length === 0 || selectionStart === 0)
  );
}
