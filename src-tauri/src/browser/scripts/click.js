(() => {
  const target = String(__SONATA_ARGS.target);
  const el = /^\d+$/.test(target) ? document.querySelector(`[data-sonata-id="${target}"]`) : (() => { try { return document.querySelector(target); } catch { return null; } })();
  if (!el) return JSON.stringify({ error: `No element ${target}. The page may have changed; run \`sonata browser read\` again.` });
  if (el.scrollIntoView) el.scrollIntoView({ block: "center" });
  if (el.focus) el.focus();
  el.click();
  return JSON.stringify({ ok: true });
})()
