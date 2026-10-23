#!/usr/bin/env node
// Builds latest.json, the manifest the in-app updater reads from the newest release.
// Usage: node scripts/release/latest-json.mjs --version v0.2.0 --notes-file notes.md \
//          --sig-file Sonata.app.tar.gz.sig --url https://.../Sonata.app.tar.gz [--date ISO]
// One universal archive serves both Mac architectures.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export function buildLatestJson({ version, notes, sig, url, date }) {
  const signature = sig.trim();
  if (!signature) throw new Error("empty update signature");
  if (!url.startsWith("https://")) throw new Error(`update URL must use https: ${url}`);
  const entry = { signature, url };
  return {
    version: version.replace(/^v/, ""),
    notes,
    pub_date: date.toISOString().replace(/\.\d{3}Z$/, "Z"),
    platforms: { "darwin-aarch64": entry, "darwin-x86_64": entry },
  };
}

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 2) {
    if (!argv[i].startsWith("--") || argv[i + 1] === undefined) throw new Error(`bad argument: ${argv[i]}`);
    args[argv[i].slice(2)] = argv[i + 1];
  }
  for (const name of ["version", "notes-file", "sig-file", "url"]) {
    if (!args[name]) throw new Error(`missing --${name}`);
  }
  return args;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const args = parseArgs(process.argv.slice(2));
    const manifest = buildLatestJson({
      version: args.version,
      notes: readFileSync(args["notes-file"], "utf8").trim(),
      sig: readFileSync(args["sig-file"], "utf8"),
      url: args.url,
      date: args.date ? new Date(args.date) : new Date(),
    });
    process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
  } catch (err) {
    process.stderr.write(`latest-json: ${err.message}\n`);
    process.exit(1);
  }
}
