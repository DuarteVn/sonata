# Quickstart: Verify Linux Support

The maintainer's Mac cannot run Linux. Verification uses GitHub Actions; manual steps are for a
friend with a Linux desktop.

## 1. CI on the feature branch

```sh
git push origin 011-linux-support
gh run list --branch 011-linux-support --limit 3
gh run view <run-id> --log | less          # linux-app job
gh run download <run-id> -n linux-smoke     # screenshot.png from the Xvfb smoke run
```

Expected: `core-and-ui` and `linux-app` pass. `linux-app` log shows clippy and tests for the
whole workspace (including `usage_bwrap`, `bash_helpers`, `platform_linux`), an AppImage and a
`.deb` built, and the smoke script lines `socket ready`, `cli copied`, `deb has /usr/bin/sonata`.
The screenshot shows the Sonata window.

## 2. macOS does not regress

```sh
scripts/check.sh
npm run tauri build -- --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

Expected: all checks pass; the app builds and behaves as before (menus, Cmd shortcuts, Finder
reveal, sandbox-exec providers, iOS section).

## 3. Release dry run (needs the maintainer's OK to tag)

Push a test tag on a fork or run `release` manually with an existing tag. Expected: draft
release with the macOS files and `Sonata_<v>_amd64.AppImage`, `.AppImage.sig`, `.deb`;
`latest.json` with `darwin-aarch64`, `darwin-x86_64`, and `linux-x86_64`
([contracts/release.md](contracts/release.md)).

## 4. Friend on Linux (manual)

```sh
chmod +x Sonata_<v>_amd64.AppImage && ./Sonata_<v>_amd64.AppImage
# or
sudo apt install ./Sonata_<v>_amd64.deb
```

In Sonata: create a workspace, two terminals, a note; link them. In a bash terminal type
`claude` (starts through Sonata), run `sonata list`, `sonata ask`, `sonata note write`. Open a
browser node and confirm the card picture. Settings → Usage with a custom provider: works with
`bubblewrap` installed, shows the install message without it. Check shortcuts from
[contracts/platform.md](contracts/platform.md): Ctrl+C and Ctrl+W reach the terminal;
Ctrl+Shift+W closes the item.
