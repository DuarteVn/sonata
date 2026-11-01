//! Agent helpers for PowerShell terminals (feature 012, research R8): `claude` and `codex` typed
//! by hand start through `sonata run`, as in zsh. The helpers are passed with `-Command`, so the
//! user's profile still loads, no script file runs (the execution policy does not apply and is
//! never changed), and nothing is written to the user's files.

/// Defines the helpers only when `sonata` is on `PATH`. The text has no quotes, so it passes
/// through Windows command-line quoting unchanged.
pub const HELPERS: &str = "if (Get-Command sonata -CommandType Application -ErrorAction SilentlyContinue) { function global:claude { sonata run claude @args }; function global:codex { sonata run codex @args } }";

/// Arguments after the PowerShell executable.
pub fn args(helpers: bool) -> Vec<String> {
    let mut out = vec!["-NoLogo".to_string()];
    if helpers {
        out.extend(["-NoExit".into(), "-Command".into(), HELPERS.into()]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers_define_both_agents_behind_a_sonata_check() {
        let a = args(true);
        assert_eq!(a[..3], ["-NoLogo", "-NoExit", "-Command"]);
        let script = &a[3];
        assert!(script.starts_with("if (Get-Command sonata "));
        assert!(script.contains("function global:claude { sonata run claude @args }"));
        assert!(script.contains("function global:codex { sonata run codex @args }"));
        assert!(!script.contains('"') && !script.contains('\''));
    }

    #[test]
    fn profile_and_policy_are_left_alone() {
        for a in [args(true), args(false)] {
            assert!(!a.iter().any(|x| x.eq_ignore_ascii_case("-NoProfile")));
            assert!(!a.iter().any(|x| x.eq_ignore_ascii_case("-ExecutionPolicy")));
        }
        assert_eq!(args(false), ["-NoLogo"]);
    }
}
