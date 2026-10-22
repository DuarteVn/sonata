// Markdown formatting actions for the note toolbar (FR-017). Pure text transforms on
// {text, start, end} so they are easy to test (research R10).

export type Edit = { text: string; start: number; end: number };
export type FormatAction = "bold" | "italic" | "strike" | "code" | "heading" | "checklist" | "bullet" | "codeBlock";

const MARKERS: Record<"bold" | "italic" | "strike" | "code", string> = {
  bold: "**",
  italic: "*",
  strike: "~~",
  code: "`",
};

function wrap({ text, start, end }: Edit, marker: string): Edit {
  const m = marker.length;
  const before = text.slice(Math.max(0, start - m), start);
  const after = text.slice(end, end + m);
  if (start !== end && before === marker && after === marker) {
    return { text: text.slice(0, start - m) + text.slice(start, end) + text.slice(end + m), start: start - m, end: end - m };
  }
  const inner = text.slice(start, end);
  return { text: text.slice(0, start) + marker + inner + marker + text.slice(end), start: start + m, end: end + m };
}

function lineBounds(text: string, pos: number) {
  const from = text.lastIndexOf("\n", pos - 1) + 1;
  const nl = text.indexOf("\n", pos);
  return { from, to: nl === -1 ? text.length : nl };
}

function replaceLine(e: Edit, change: (line: string) => string): Edit {
  const { from, to } = lineBounds(e.text, e.start);
  const line = e.text.slice(from, to);
  const next = change(line);
  const delta = next.length - line.length;
  return { text: e.text.slice(0, from) + next + e.text.slice(to), start: Math.max(from, e.start + delta), end: Math.max(from, e.end + delta) };
}

function cycleHeading(line: string) {
  const m = /^(#{1,3}) /.exec(line);
  if (!m) return `# ${line}`;
  const level = m[1].length;
  const rest = line.slice(m[0].length);
  return level >= 3 ? rest : `${"#".repeat(level + 1)} ${rest}`;
}

function togglePrefix(line: string, prefix: string, other: RegExp) {
  if (line.startsWith(prefix)) return line.slice(prefix.length);
  return prefix + line.replace(other, "");
}

export function applyAction(action: FormatAction, e: Edit): Edit {
  switch (action) {
    case "bold":
    case "italic":
    case "strike":
    case "code":
      return wrap(e, MARKERS[action]);
    case "heading":
      return replaceLine(e, cycleHeading);
    case "checklist":
      return replaceLine(e, (l) => togglePrefix(l, "- [ ] ", /^(- \[[ xX]\] |- )/));
    case "bullet":
      return replaceLine(e, (l) => (/^- \[[ xX]\] /.test(l) ? l : togglePrefix(l, "- ", /^$/)));
    case "codeBlock": {
      const lead = e.start > 0 && e.text[e.start - 1] !== "\n" ? "\n" : "";
      const inner = e.text.slice(e.start, e.end);
      const opening = `${lead}\`\`\`\n`;
      const block = `${opening}${inner}\n\`\`\`\n`;
      const pos = e.start + opening.length;
      return { text: e.text.slice(0, e.start) + block + e.text.slice(e.end), start: pos, end: pos + inner.length };
    }
  }
}

/** Flips `[ ]` / `[x]` on the 1-based source `line`, if it is a task item. */
export function toggleCheckbox(text: string, line: number): string {
  const lines = text.split("\n");
  const i = line - 1;
  const l = lines[i];
  if (l === undefined) return text;
  const m = /^(\s*[-*+] )\[([ xX])\]/.exec(l);
  if (!m) return text;
  lines[i] = l.slice(0, m[1].length) + (m[2] === " " ? "[x]" : "[ ]") + l.slice(m[0].length);
  return lines.join("\n");
}
