//! The missing-git message names the install route of the running OS (feature 011).

use sonata_core::platform::git_install_hint;

#[cfg(target_os = "macos")]
#[test]
fn macos_points_to_the_command_line_tools() {
    assert_eq!(
        git_install_hint(),
        "Git is not installed. Install the Command Line Tools with: xcode-select --install"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn linux_points_to_the_package_manager() {
    assert_eq!(
        git_install_hint(),
        "Git is not installed. Install it with your package manager, for example: sudo apt install git"
    );
}
