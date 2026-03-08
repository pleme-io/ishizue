//! Path manipulation utilities for Neovim plugins.
//!
//! Pure Rust path helpers: normalize, join, expand `~`, compute relative paths,
//! and test absolute-ness. All functions operate on string slices and return
//! owned [`String`]s so they are trivially callable from nvim-oxi bindings.

use std::path::{Component, Path, PathBuf};

/// Normalize a path by resolving `.` and `..` components without touching the
/// filesystem. Does NOT follow symlinks — this is purely lexical.
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::path::normalize("/foo/bar/../baz"), "/foo/baz");
/// assert_eq!(ishizue::path::normalize("/foo/./bar"), "/foo/bar");
/// ```
#[must_use]
pub fn normalize(path: &str) -> String {
    let p = Path::new(path);
    let mut components: Vec<Component<'_>> = Vec::new();

    for component in p.components() {
        match component {
            Component::CurDir => {} // skip `.`
            Component::ParentDir => {
                match components.last() {
                    // Pop the last normal component if one exists.
                    Some(Component::Normal(_)) => {
                        components.pop();
                    }
                    // `..` past root is absorbed (e.g., `/..` → `/`).
                    Some(Component::RootDir) => {}
                    _ => {
                        components.push(component);
                    }
                }
            }
            other => components.push(other),
        }
    }

    if components.is_empty() {
        return ".".to_owned();
    }

    let result: PathBuf = components.iter().collect();
    result.to_string_lossy().into_owned()
}

/// Join two path segments. If `child` is absolute it replaces `parent` entirely,
/// matching [`std::path::Path::join`] semantics. An empty `child` returns
/// `parent` unchanged (no trailing separator).
///
/// # Examples
///
/// ```
/// assert_eq!(ishizue::path::join("/foo", "bar"), "/foo/bar");
/// assert_eq!(ishizue::path::join("/foo", "/bar"), "/bar");
/// assert_eq!(ishizue::path::join("/foo", ""), "/foo");
/// ```
#[must_use]
pub fn join(parent: &str, child: &str) -> String {
    if child.is_empty() {
        return parent.to_owned();
    }
    Path::new(parent)
        .join(child)
        .to_string_lossy()
        .into_owned()
}

/// Compute a relative path from `base` to `target`. Both paths are normalized
/// first. Returns `None` when a relative path cannot be computed (e.g., one
/// path is relative and the other absolute).
///
/// # Examples
///
/// ```
/// assert_eq!(
///     ishizue::path::relative_to("/home/user/projects/foo", "/home/user"),
///     Some("projects/foo".to_owned()),
/// );
/// ```
#[must_use]
pub fn relative_to(target: &str, base: &str) -> Option<String> {
    let target_norm = normalize(target);
    let base_norm = normalize(base);

    let target_path = Path::new(&target_norm);
    let base_path = Path::new(&base_norm);

    // Both must be either absolute or both relative — mixing is ambiguous.
    if target_path.is_absolute() != base_path.is_absolute() {
        return None;
    }

    let target_components: Vec<_> = target_path.components().collect();
    let base_components: Vec<_> = base_path.components().collect();

    // Find length of common prefix.
    let common_len = target_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let ups = base_components.len() - common_len;
    let mut result = PathBuf::new();

    for _ in 0..ups {
        result.push("..");
    }

    for component in &target_components[common_len..] {
        result.push(component.as_os_str());
    }

    if result.as_os_str().is_empty() {
        Some(".".to_owned())
    } else {
        Some(result.to_string_lossy().into_owned())
    }
}

/// Expand a leading `~` to the value of `$HOME`. If the path does not start
/// with `~` it is returned unchanged.
///
/// # Examples
///
/// ```
/// // When HOME=/home/user:
/// // expand_home("~/projects") => "/home/user/projects"
/// let expanded = ishizue::path::expand_home("~/projects");
/// assert!(expanded.ends_with("/projects"));
/// ```
#[must_use]
pub fn expand_home(path: &str) -> String {
    if path == "~" {
        return std::env::var("HOME").unwrap_or_else(|_| "~".to_owned());
    }

    if let Some(rest) = path.strip_prefix("~/") {
        match std::env::var("HOME") {
            Ok(home) => format!("{home}/{rest}"),
            Err(_) => path.to_owned(),
        }
    } else {
        path.to_owned()
    }
}

/// Returns `true` when the path is absolute (starts with `/` on Unix).
///
/// # Examples
///
/// ```
/// assert!(ishizue::path::is_absolute("/usr/bin"));
/// assert!(!ishizue::path::is_absolute("relative/path"));
/// ```
#[must_use]
pub fn is_absolute(path: &str) -> bool {
    Path::new(path).is_absolute()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- normalize ----------------------------------------------------------

    #[test]
    fn normalize_removes_dot() {
        assert_eq!(normalize("/foo/./bar"), "/foo/bar");
    }

    #[test]
    fn normalize_resolves_dotdot() {
        assert_eq!(normalize("/foo/bar/../baz"), "/foo/baz");
    }

    #[test]
    fn normalize_multiple_dotdot() {
        assert_eq!(normalize("/a/b/c/../../d"), "/a/d");
    }

    #[test]
    fn normalize_trailing_slash_dot() {
        assert_eq!(normalize("/foo/bar/."), "/foo/bar");
    }

    #[test]
    fn normalize_root() {
        assert_eq!(normalize("/"), "/");
    }

    #[test]
    fn normalize_empty_becomes_dot() {
        assert_eq!(normalize(""), ".");
    }

    #[test]
    fn normalize_relative() {
        assert_eq!(normalize("a/b/../c"), "a/c");
    }

    #[test]
    fn normalize_dotdot_past_root() {
        // `..` past root is absorbed by the root component
        assert_eq!(normalize("/.."), "/");
    }

    #[test]
    fn normalize_relative_leading_dotdot() {
        assert_eq!(normalize("../a/b"), "../a/b");
    }

    // -- join ---------------------------------------------------------------

    #[test]
    fn join_basic() {
        assert_eq!(join("/foo", "bar"), "/foo/bar");
    }

    #[test]
    fn join_absolute_child_replaces() {
        assert_eq!(join("/foo", "/bar"), "/bar");
    }

    #[test]
    fn join_empty_child() {
        assert_eq!(join("/foo", ""), "/foo");
    }

    #[test]
    fn join_empty_parent() {
        assert_eq!(join("", "bar"), "bar");
    }

    // -- relative_to --------------------------------------------------------

    #[test]
    fn relative_to_subdirectory() {
        assert_eq!(
            relative_to("/home/user/projects/foo", "/home/user"),
            Some("projects/foo".to_owned()),
        );
    }

    #[test]
    fn relative_to_same_path() {
        assert_eq!(relative_to("/foo/bar", "/foo/bar"), Some(".".to_owned()));
    }

    #[test]
    fn relative_to_sibling() {
        assert_eq!(
            relative_to("/a/b", "/a/c"),
            Some("../b".to_owned()),
        );
    }

    #[test]
    fn relative_to_deeper() {
        assert_eq!(
            relative_to("/a/b/c/d", "/a/b"),
            Some("c/d".to_owned()),
        );
    }

    #[test]
    fn relative_to_mixed_absolute_relative_returns_none() {
        assert_eq!(relative_to("/absolute", "relative"), None);
    }

    #[test]
    fn relative_to_both_relative() {
        assert_eq!(
            relative_to("a/b/c", "a/d"),
            Some("../b/c".to_owned()),
        );
    }

    // -- expand_home --------------------------------------------------------

    #[test]
    fn expand_home_tilde_prefix() {
        let result = expand_home("~/projects");
        assert!(
            !result.starts_with('~') || std::env::var("HOME").is_err(),
            "expected tilde to be expanded, got: {result}",
        );
        assert!(result.ends_with("/projects"));
    }

    #[test]
    fn expand_home_bare_tilde() {
        let result = expand_home("~");
        assert!(
            !result.starts_with('~') || std::env::var("HOME").is_err(),
            "expected bare tilde to expand, got: {result}",
        );
    }

    #[test]
    fn expand_home_no_tilde() {
        assert_eq!(expand_home("/usr/bin"), "/usr/bin");
    }

    #[test]
    fn expand_home_tilde_not_at_start() {
        assert_eq!(expand_home("/foo/~bar"), "/foo/~bar");
    }

    // -- is_absolute --------------------------------------------------------

    #[test]
    fn is_absolute_true() {
        assert!(is_absolute("/usr/bin"));
    }

    #[test]
    fn is_absolute_false() {
        assert!(!is_absolute("relative/path"));
    }

    #[test]
    fn is_absolute_empty() {
        assert!(!is_absolute(""));
    }
}
