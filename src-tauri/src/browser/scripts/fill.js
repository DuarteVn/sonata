(() => {
  const target = String(__SONATA_ARGS.target);
  const text = String(__SONATA_ARGS.text);
  const el = /^\d+$/.test(target) ? document.querySelector(`[data-sonata-id="${target}"]`) : (() => { try { return document.querySelector(target); } catch { return null; } })();
  if (!el) return JSON.stringify({ error: `No element ${target}. The page may have changed; run \`sonata browser read\` again.` });
  if (el.focus) el.focus();
  if (el.isContentEditable) {
    el.textContent = text;
  } else {
    // Use the native setter so frameworks that track values see the change.
    const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : el instanceof HTMLSelectElement ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(proto, "value")?.set;
    if (setter) setter.call(el, text);
    else el.value = text;
  }
  el.dispatchEvent(new Event("input", { bubbles: true }));
  el.dispatchEvent(new Event("change", { bubbles: true }));
  return JSON.stringify({ ok: true });
})()
