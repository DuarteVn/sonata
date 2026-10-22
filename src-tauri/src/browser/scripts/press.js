(() => {
  const key = String(__SONATA_ARGS.key);
  const el = document.activeElement || document.body;
  const init = { key, code: key.length === 1 ? `Key${key.toUpperCase()}` : key, bubbles: true, cancelable: true };
  const down = el.dispatchEvent(new KeyboardEvent("keydown", init));
  if (key.length === 1 || key === "Enter") el.dispatchEvent(new KeyboardEvent("keypress", init));
  el.dispatchEvent(new KeyboardEvent("keyup", init));
  // Synthetic events do not trigger default actions; perform the common ones.
  if (down && key === "Enter" && el.form && el.tagName !== "TEXTAREA") {
    if (el.form.requestSubmit) el.form.requestSubmit();
    else el.form.submit();
  } else if (down && key === "Tab") {
    const all = [...document.querySelectorAll("a[href], button, input, textarea, select, [tabindex]")].filter((e) => !e.disabled && e.tabIndex >= 0);
    const next = all[(all.indexOf(el) + 1) % all.length];
    if (next) next.focus();
  } else if (down && key.length === 1 && "value" in el) {
    el.value += key;
    el.dispatchEvent(new Event("input", { bubbles: true }));
  }
  return JSON.stringify({ ok: true });
})()
