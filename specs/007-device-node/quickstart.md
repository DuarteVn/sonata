# Quickstart: Validate Feature 007

Run the app with throwaway data: `SONATA_HOME="$(mktemp -d)/sonata" npm run tauri dev`.

## Without the Android SDK (works on any Mac)

1. Press D, click the canvas → node says "Android SDK not found", lists the paths checked, opens
   the official install page in one click, offers "Choose SDK folder…". Nothing on the system
   changes (compare `ls ~/Library/Android ~/.android` before and after).
2. Choose an empty folder → still "not found"; iOS section shows "Xcode not found".

## With Android Studio and one AVD

3. "Check again" → the AVD appears with its Android version. Choose it → booting progress, then the
   live screen (cold boot < 90 s; stop and boot again < 15 s). No emulator window appears.
4. Click Settings, type in its search field, scroll, press Back/Home/Recents → each reacts in under
   300 ms; resize the node and zoom the canvas → taps still land where clicked.
5. Pan the canvas → the node shows its last image during the gesture; three device nodes on screen
   → at most two live.
6. Link a shell: `sonata list` shows the device; run `read`, `tap <n>`, `type --into <n> "hi"`,
   `swipe up`, `key back`, `apps`, `launch com.android.settings`, `screenshot` → each works and the
   node shows "controlled by <terminal>". An unlinked shell → exit 3.
7. `sonata device install Pixel app-debug.apk` → prints the app id; `launch` it.
8. Start the AVD from Android Studio first, then choose it in Sonata → Sonata attaches; deleting
   the node does not stop it.
9. Quit Sonata with a booted device → the confirmation lists it; the emulator process exits.
10. Record measured fps and click-to-screen latency in `quickstart-results.md`.
