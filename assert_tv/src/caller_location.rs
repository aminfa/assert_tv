use std::borrow::Cow;

/// Format a host-path string from `Location::file()` into a concise, portable display form.
///
/// Pure string processing (no `Path`, no filesystem access), so it remains valid even when
/// the compiled code runs on a different target than the build host.
///
/// Heuristic:
/// 1) Normalize `\` to `/`.
/// 2) If the path contains `.../<crate>/src/...`, return `"<crate>/src/..."`.
/// 3) Otherwise, return the last two components (e.g., `"dir/file.rs"`).
pub(crate) fn pretty_location_file(file: &str) -> Cow<'_, str> {
    // 1) Normalize separators.
    let s: Cow<'_, str> = if file.contains('\\') {
        Cow::Owned(file.replace('\\', "/"))
    } else {
        Cow::Borrowed(file)
    };

    // 2) Prefer `<crate>/src/...` if we can recognize it.
    if let Some(src_idx) = s.rfind("/src/") {
        // Find the slash before the crate directory name
        let crate_name_start = s[..src_idx].rfind('/').map(|p| p + 1).unwrap_or(0);
        let crate_name = &s[crate_name_start..src_idx];
        let suffix = &s[src_idx + 1..]; // keep `src/...`
        if !crate_name.is_empty() {
            return Cow::Owned(format!("{}/{}", crate_name, suffix));
        }
        // If crate name is empty (edge case), fall through to fallback.
    }

    // 3) Fallback: last two components (e.g., `dir/file.rs`), or just the filename.
    let mut parts = s.rsplit('/');
    let last = parts.next().unwrap_or(&s);
    let prev = parts.next();
    match prev {
        Some(p) if !p.is_empty() => Cow::Owned(format!("{}/{}", p, last)),
        _ => Cow::Owned(last.to_string()),
    }
}
