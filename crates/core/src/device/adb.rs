//! Parsing what adb and the SDK tools print (research R4, R8). Pure functions.

/// `wm size`: the override size when set, else the physical size.
pub fn parse_wm_size(out: &str) -> Option<(u32, u32)> {
    let size = |prefix: &str| {
        out.lines()
            .find_map(|l| l.trim().strip_prefix(prefix))
            .and_then(|v| v.trim().split_once('x'))
            .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))
    };
    size("Override size:").or_else(|| size("Physical size:"))
}

/// `cmd package query-activities --brief …`: unique package names of `package/activity` lines.
pub fn parse_launchable(out: &str) -> Vec<String> {
    let mut apps: Vec<String> = out
        .lines()
        .map(str::trim)
        .filter_map(|l| l.split_once('/'))
        .filter(|(pkg, act)| {
            !pkg.is_empty()
                && !act.is_empty()
                && pkg
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
                && !act.contains(char::is_whitespace)
        })
        .map(|(pkg, _)| pkg.to_string())
        .collect();
    apps.sort();
    apps.dedup();
    apps
}

/// `pm list packages`: `package:<name>` lines.
pub fn parse_packages(out: &str) -> Vec<String> {
    let mut pkgs: Vec<String> = out
        .lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    pkgs.sort();
    pkgs
}

/// `aapt2 dump packagename <apk>` prints the package name alone.
pub fn parse_aapt_package(out: &str) -> Option<String> {
    let name = out.lines().map(str::trim).find(|l| !l.is_empty())?;
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
        .then(|| name.to_string())
}

/// The XML of `uiautomator dump /dev/tty`, without the status line it prints after it.
pub fn strip_dump_status(out: &str) -> &str {
    match out.rfind("</hierarchy>") {
        Some(end) => &out[..end + "</hierarchy>".len()],
        None => out,
    }
}
