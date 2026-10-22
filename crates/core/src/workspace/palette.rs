//! Icons and colors given to new workspaces.

use rand::seq::IndexedRandom;

/// Icon ids, rendered by the frontend with matching `lucide` icons.
pub const ICONS: [&str; 24] = [
    "terminal",
    "code",
    "rocket",
    "flask-conical",
    "bot",
    "brain",
    "cpu",
    "database",
    "globe",
    "layers",
    "package",
    "puzzle",
    "wrench",
    "hammer",
    "leaf",
    "flame",
    "zap",
    "star",
    "music",
    "compass",
    "anchor",
    "feather",
    "gem",
    "shield",
];

pub const COLORS: [&str; 8] = [
    "#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899", "#14B8A6", "#F97316",
];

pub fn random_icon() -> String {
    ICONS
        .choose(&mut rand::rng())
        .copied()
        .unwrap_or(ICONS[0])
        .to_string()
}

/// A random color that no existing workspace uses, or any color when all are taken.
pub fn random_color<'a>(used: impl IntoIterator<Item = &'a str>) -> String {
    let used: Vec<&str> = used.into_iter().collect();
    let unused: Vec<&str> = COLORS
        .iter()
        .copied()
        .filter(|c| !used.contains(c))
        .collect();
    let pool: &[&str] = if unused.is_empty() { &COLORS } else { &unused };
    pool.choose(&mut rand::rng())
        .copied()
        .unwrap_or(COLORS[0])
        .to_string()
}
