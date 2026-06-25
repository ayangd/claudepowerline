//! Git branch resolution (impure — shells out to `git`).

use crate::text::truncate_branch;

/// Current git branch for `raw_cwd`; detached HEAD shows `:<short-hash>`.
/// Any failure (not a repo, git missing) yields an empty string.
pub(crate) fn git_branch(raw_cwd: &str) -> String {
    let Ok(out) = std::process::Command::new("git")
        .args(["-C", raw_cwd, "branch", "--show-current"])
        .output()
    else {
        return String::new();
    };
    if !out.status.success() {
        return String::new();
    }

    let mut branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if branch.is_empty() {
        // Detached HEAD — fall back to the short commit hash.
        if let Ok(ha) = std::process::Command::new("git")
            .args(["-C", raw_cwd, "rev-parse", "--short", "HEAD"])
            .output()
            && ha.status.success()
        {
            branch = format!(":{}", String::from_utf8_lossy(&ha.stdout).trim());
        }
    }
    truncate_branch(branch)
}
