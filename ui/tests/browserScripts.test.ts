// The page scripts run inside browser nodes (src-tauri/src/browser/scripts). Here they run in
// jsdom against a small test page.
import { beforeEach, describe, expect, it } from "vitest";
import clickJs from "../../src-tauri/src/browser/scripts/click.js?raw";
import fillJs from "../../src-tauri/src/browser/scripts/fill.js?raw";
import pressJs from "../../src-tauri/src/browser/scripts/press.js?raw";
import readJs from "../../src-tauri/src/browser/scripts/read.js?raw";

function run(script: string, args: unknown = null) {
  const out = new Function(`const __SONATA_ARGS = ${JSON.stringify(args)}; return ${script}`)();
  return JSON.parse(out as string);
}

beforeEach(() => {
  document.title = "Login · Test";
  document.body.innerHTML = `
    <h1>Welcome back</h1>
    <form id="f">
      <label for="email">Email</label><input id="email" name="email">
      <input id="pw" type="password" placeholder="Password">
      <button type="submit">Sign in</button>
    </form>
    <a href="/reset">Forgot password?</a>
    <button hidden>Hidden</button>
    <div style="display:none"><button>Also hidden</button></div>`;
});

describe("page scripts", () => {
  it("read lists text and numbered visible interactive elements", () => {
    const r = run(readJs);
    expect(r.title).toBe("Login · Test");
    expect(r.text).toContain("Welcome back");
    expect(r.elements.map((e: { label: string }) => e.label)).toEqual(["Email", "Password", "Sign in", "Forgot password?"]);
    expect(r.elements[1]).toMatchObject({ n: 2, kind: "input", extra: "password" });
    expect(r.elements[3]).toMatchObject({ kind: "link", extra: "/reset" });
    expect(document.querySelector("#pw")!.getAttribute("data-sonata-id")).toBe("2");
  });

  it("fill sets the value with input and change events", () => {
    run(readJs);
    const events: string[] = [];
    const email = document.querySelector<HTMLInputElement>("#email")!;
    email.addEventListener("input", () => events.push("input"));
    email.addEventListener("change", () => events.push("change"));
    expect(run(fillJs, { target: "1", text: "a@b.c" })).toEqual({ ok: true });
    expect(email.value).toBe("a@b.c");
    expect(events).toEqual(["input", "change"]);
    expect(run(fillJs, { target: "#pw", text: "secret" })).toEqual({ ok: true });
    expect(document.querySelector<HTMLInputElement>("#pw")!.value).toBe("secret");
  });

  it("click by number or selector, and reports stale numbers", () => {
    run(readJs);
    let submitted = 0;
    document.querySelector("#f")!.addEventListener("submit", (e) => {
      e.preventDefault();
      submitted++;
    });
    expect(run(clickJs, { target: "3" })).toEqual({ ok: true });
    expect(run(clickJs, { target: "button[type=submit]" })).toEqual({ ok: true });
    expect(submitted).toBe(2);
    expect(run(clickJs, { target: "99" }).error).toMatch(/read/);
  });

  it("press Enter submits the focused field's form", () => {
    let submitted = 0;
    document.querySelector("#f")!.addEventListener("submit", (e) => {
      e.preventDefault();
      submitted++;
    });
    document.querySelector<HTMLInputElement>("#email")!.focus();
    expect(run(pressJs, { key: "Enter" })).toEqual({ ok: true });
    expect(submitted).toBe(1);
  });
});
