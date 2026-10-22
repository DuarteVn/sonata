(() => {
  const visible = (el) => {
    if (el.hidden || el.closest("[hidden]")) return false;
    for (let e = el; e && e.nodeType === 1; e = e.parentElement) {
      const st = getComputedStyle(e);
      if (st.display === "none" || st.visibility === "hidden") return false;
    }
    return true;
  };
  const clean = (s) => (s || "").replace(/\s+/g, " ").trim();
  const labelOf = (el) => {
    const aria = el.getAttribute("aria-label");
    if (aria) return clean(aria);
    if (el.id) {
      const l = document.querySelector(`label[for="${CSS.escape(el.id)}"]`);
      if (l) return clean(l.textContent);
    }
    const wrap = el.closest("label");
    if (wrap && wrap !== el) return clean(wrap.textContent);
    return clean(el.innerText ?? el.textContent) || clean(el.getAttribute("placeholder")) || clean(el.getAttribute("title")) || clean(el.getAttribute("name")) || clean(el.value) || clean(el.querySelector("img")?.getAttribute("alt"));
  };
  const kindOf = (el) => {
    const tag = el.tagName.toLowerCase();
    if (tag === "a") return "link";
    if (tag === "button" || el.getAttribute("role") === "button") return "button";
    if (tag === "select") return "select";
    if (tag === "textarea") return "textarea";
    if (el.isContentEditable || el.getAttribute("contenteditable") === "true") return "editable";
    return "input";
  };
  document.querySelectorAll("[data-sonata-id]").forEach((e) => e.removeAttribute("data-sonata-id"));
  const selector = "a[href], button, input:not([type=hidden]), textarea, select, [role=button], [role=link], [contenteditable=true]";
  const elements = [];
  let n = 0;
  for (const el of document.querySelectorAll(selector)) {
    if (!visible(el) || el.disabled) continue;
    n += 1;
    el.setAttribute("data-sonata-id", String(n));
    const kind = kindOf(el);
    let extra = null;
    if (kind === "link") extra = el.getAttribute("href");
    else if (kind === "input" && el.type && el.type !== "text") extra = el.type;
    elements.push({ n, kind, label: labelOf(el).slice(0, 120), extra });
  }
  const text = clean(document.body ? (document.body.innerText ?? document.body.textContent) : "").slice(0, 100000);
  return JSON.stringify({ title: document.title, url: location.href, text, elements });
})()
