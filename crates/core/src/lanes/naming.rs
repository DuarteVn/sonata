//! Lane folder and branch names (research R3).

pub const SLUG_MAX_CHARS: usize = 40;
pub const BRANCH_PREFIX: &str = "lane/";

/// A folder- and branch-safe name from a terminal name.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.truncate(SLUG_MAX_CHARS);
    let out = out.trim_end_matches('-');
    if out.is_empty() {
        "lane".into()
    } else {
        out.into()
    }
}

/// The first of `slug`, `slug-2`, `slug-3`, … for which `taken` is false.
pub fn unique(slug: &str, taken: impl Fn(&str) -> bool) -> String {
    let mut candidate = slug.to_string();
    let mut i = 2;
    while taken(&candidate) {
        candidate = format!("{slug}-{i}");
        i += 1;
    }
    candidate
}

pub fn branch(slug: &str) -> String {
    format!("{BRANCH_PREFIX}{slug}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_folds_to_lowercase_ascii_and_dashes() {
        assert_eq!(slug("Login Form"), "login-form");
        assert_eq!(slug("  Fix: #42 / bug  "), "fix-42-bug");
        assert_eq!(slug("Ação Rápida"), "a-o-r-pida");
        assert_eq!(slug("a--b__c"), "a-b-c");
    }

    #[test]
    fn slug_never_empty_and_capped() {
        assert_eq!(slug(""), "lane");
        assert_eq!(slug("!!!"), "lane");
        let long = slug(&"x".repeat(100));
        assert_eq!(long.len(), SLUG_MAX_CHARS);
        let cut = slug(&format!("{}-y", "x".repeat(39)));
        assert!(!cut.ends_with('-'), "{cut}");
    }

    #[test]
    fn slug_rejects_git_special_forms() {
        assert_eq!(slug(".hidden.lock"), "hidden-lock");
        assert_eq!(slug("a..b"), "a-b");
    }

    #[test]
    fn unique_appends_counter() {
        assert_eq!(unique("api", |_| false), "api");
        let taken = ["api", "api-2"];
        assert_eq!(unique("api", |s| taken.contains(&s)), "api-3");
    }

    #[test]
    fn branch_has_prefix() {
        assert_eq!(branch("api"), "lane/api");
    }
}
