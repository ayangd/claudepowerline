//! Thin OS adapter for the few things that differ between *nix and Windows:
//! home-directory resolution and path-separator handling.

/// The user's home directory — `$HOME` on *nix, `%USERPROFILE%` on Windows
/// (falling back to the other variable if the native one is unset, and ignoring
/// an empty value). `None` → callers degrade gracefully: no `~`-shortening and
/// no usage window.
pub(crate) fn home_dir() -> Option<String> {
    let (primary, secondary) = if cfg!(windows) {
        ("USERPROFILE", "HOME")
    } else {
        ("HOME", "USERPROFILE")
    };
    std::env::var(primary)
        .or_else(|_| std::env::var(secondary))
        .ok()
        .filter(|s| !s.is_empty())
}

/// Path-component separators accepted when shortening a cwd for display. Both
/// are recognized on every platform, so a posix `/…` or a Windows `C:\…` splits.
pub(crate) const PATH_SEPARATORS: [char; 2] = ['/', '\\'];
