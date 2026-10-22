import { describe, expect, it } from "vitest";
import { applyAction, toggleCheckbox } from "../src/notes/markdownEdit";

const at = (text: string, start: number, end = start) => ({ text, start, end });

describe("markdown edit actions", () => {
  it("wraps and unwraps a selection", () => {
    const r = applyAction("bold", at("say hello now", 4, 9));
    expect(r).toEqual({ text: "say **hello** now", start: 6, end: 11 });
    expect(applyAction("bold", r)).toEqual({ text: "say hello now", start: 4, end: 9 });
    expect(applyAction("italic", at("a b", 2, 3)).text).toBe("a *b*");
    expect(applyAction("strike", at("a b", 2, 3)).text).toBe("a ~~b~~");
    expect(applyAction("code", at("a b", 2, 3)).text).toBe("a `b`");
  });

  it("inserts empty markers with the cursor between them", () => {
    expect(applyAction("bold", at("ab", 1))).toEqual({ text: "a****b", start: 3, end: 3 });
    const block = applyAction("codeBlock", at("x", 1));
    expect(block.text).toBe("x\n```\n\n```\n");
    expect(block.text.slice(0, block.start)).toBe("x\n```\n");
  });

  it("cycles headings on the current line", () => {
    let r = applyAction("heading", at("one\ntitle\nthree", 6));
    expect(r.text).toBe("one\n# title\nthree");
    r = applyAction("heading", r);
    expect(r.text).toBe("one\n## title\nthree");
    r = applyAction("heading", r);
    expect(r.text).toBe("one\n### title\nthree");
    r = applyAction("heading", r);
    expect(r.text).toBe("one\ntitle\nthree");
  });

  it("toggles list prefixes", () => {
    expect(applyAction("checklist", at("task", 2)).text).toBe("- [ ] task");
    expect(applyAction("checklist", at("- [ ] task", 7)).text).toBe("task");
    expect(applyAction("bullet", at("item", 0)).text).toBe("- item");
    expect(applyAction("bullet", at("- item", 3)).text).toBe("item");
  });

  it("toggles a checkbox by source line", () => {
    const text = "# T\n- [ ] a\n- [x] b";
    expect(toggleCheckbox(text, 2)).toBe("# T\n- [x] a\n- [x] b");
    expect(toggleCheckbox(text, 3)).toBe("# T\n- [ ] a\n- [ ] b");
    expect(toggleCheckbox(text, 1)).toBe(text);
  });
});
