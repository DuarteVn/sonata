# Contract: `sonata device` Commands

All require the device node to be linked to the calling terminal (exit 3 otherwise, same "not
linked" error as other nodes). `<device>`: node name or short id from `sonata list` (`d-xxxx`).
Commands that change the screen return after the screen is still for 500 ms (at most 5 s).

```text
sonata device list                                   # linked devices: name, platform, state
sonata device boot <device> [--timeout 3m]           # returns when ready
sonata device stop <device>
sonata device read <device>
sonata device tap <device> <number | x y>
sonata device swipe <device> <up|down|left|right> | <x1 y1 x2 y2> [--duration 300ms]
sonata device type <device> "<text>" [--into <number>]
sonata device key <device> <back|home|recents|enter|delete>
sonata device launch <device> <app id>
sonata device apps <device>
sonata device install <device> <file.apk | file.app> [--timeout 3m]   # prints installed app id
sonata device screenshot <device> [--out file.png]   # prints the PNG path
```

`read` output:

```text
App: com.android.settings
Screen: 1080x2400

  [1] input  "Search settings" (editable)
  [2] text   "Network & internet"
  [3] button "Network & internet  Mobile, Wi‑Fi, hotspot" (tappable)
  [4] list   "" (scrollable)
```

Errors (exit 1 unless noted): `device not running`, `device is booting`, `element 7 not found; run
read again`, `the screen changed; run read again`, `not supported on iOS simulator`, `not installed
or not launchable`, `screen never settled; try again or use coordinates`, `timed out after 3m`.
