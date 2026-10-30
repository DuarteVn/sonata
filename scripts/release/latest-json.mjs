#!/usr/bin/env node
// Builds latest.json, the manifest the in-app updater reads from the newest release.
// Usage: node scripts/release/latest-json.mjs --version v0.2.0 --notes-file notes.md \
//          --sig-file Sonata.app.tar.gz.sig --url https://.../Sonata.app.tar.gz \
//          [--linux-sig-file Sonata_0.2.0_amd64.AppImage.sig --linux-url https://.../Sonata_0.2.0_amd64.AppImage] \
//          [--windows-sig-file Sonata_0.2.0_x64-setup.exe.sig --windows-url https://.../Sonata_0.2.0_x64-setup.exe] \
//          [--date ISO]
// One universal archive serves both Mac architectures; the Linux AppImage updates itself
// (feature 011); the Windows installer is also the Windows update (feature 012).
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

function entry(sig, url) {
  const signature = sig.trim();
  if (!signature) throw new Error("empty update signature");
  if (!url.startsWith("https://")) throw new Error(`update URL must use https: ${url}`);
  return { signature, url };
}

export function buildLatestJson({ version, notes, sig, url, date, linux, windows }) {
  const mac = entry(sig, url);
  const platforms = { "darwin-aarch64": mac, "darwin-x86_64": mac };
  if (linux) platforms["linux-x86_64"] = entry(linux.sig, linux.url);
  if (windows) platforms["windows-x86_64"] = entry(windows.sig, windows.url);
  return {
    version: version.replace(/^v/, ""),
    notes,
    pub_date: date.toISOString().replace(/\.\d{3}Z$/, "Z"),
    platforms,
  };
}

export function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 2) {
    if (!argv[i].startsWith("--") || argv[i + 1] === undefined) throw new Error(`bad argument: ${argv[i]}`);
    args[argv[i].slice(2)] = argv[i + 1];
  }
  for (const name of ["version", "notes-file", "sig-file", "url"]) {
    if (!args[name]) throw new Error(`missing --${name}`);
  }
  if (args["linux-sig-file"] && !args["linux-url"]) throw new Error("--linux-sig-file needs --linux-url");
  if (args["linux-url"] && !args["linux-sig-file"]) throw new Error("--linux-url needs --linux-sig-file");
  if (args["windows-sig-file"] && !args["windows-url"]) throw new Error("--windows-sig-file needs --windows-url");
  if (args["windows-url"] && !args["windows-sig-file"]) throw new Error("--windows-url needs --windows-sig-file");
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
      linux: args["linux-url"]
        ? { sig: readFileSync(args["linux-sig-file"], "utf8"), url: args["linux-url"] }
        : undefined,
      windows: args["windows-url"]
        ? { sig: readFileSync(args["windows-sig-file"], "utf8"), url: args["windows-url"] }
        : undefined,
    });
    process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
  } catch (err) {
    process.stderr.write(`latest-json: ${err.message}\n`);
    process.exit(1);
  }
}
