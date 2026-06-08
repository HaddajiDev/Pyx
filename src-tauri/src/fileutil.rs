use std::path::{Path, PathBuf};

pub fn sanitize_filename(raw: &str) -> String {
    let last = raw
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or("");
    let cleaned: String = last
        .chars()
        .filter(|c| !matches!(c, '\0'..='\u{1f}'))
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "received_file".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn sanitize_rel_components(raw: &str) -> Vec<String> {
    let mut out: Vec<String> = raw
        .split(|c| c == '/' || c == '\\')
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(sanitize_filename)
        .collect();
    if out.is_empty() {
        out.push("received_file".to_string());
    }
    out
}

pub fn safe_dest_path(dest_dir: &Path, rel_path: &str) -> PathBuf {
    let comps = sanitize_rel_components(rel_path);
    let (file, dirs) = comps.split_last().expect("at least one component");
    let mut parent = dest_dir.to_path_buf();
    for d in dirs {
        parent.push(d);
    }
    unique_destination(&parent, file)
}

pub fn unique_destination(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name);
    let ext = path.extension().and_then(|s| s.to_str());
    for n in 1.. {
        let new_name = match ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(&new_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_directory_components() {
        assert_eq!(sanitize_filename("foo/bar/baz.txt"), "baz.txt");
        assert_eq!(sanitize_filename("foo\\bar\\baz.txt"), "baz.txt");
    }

    #[test]
    fn blocks_traversal() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("..\\..\\windows\\system32"), "system32");
        assert_eq!(sanitize_filename("../.."), "received_file");
    }

    #[test]
    fn handles_empty_and_dot_names() {
        assert_eq!(sanitize_filename(""), "received_file");
        assert_eq!(sanitize_filename("   "), "received_file");
        assert_eq!(sanitize_filename("..."), "received_file");
    }

    #[test]
    fn rel_components_keep_structure_and_strip_traversal() {
        assert_eq!(
            sanitize_rel_components("build/assets/logo.png"),
            vec!["build", "assets", "logo.png"]
        );
        assert_eq!(
            sanitize_rel_components("build\\sub\\a.bin"),
            vec!["build", "sub", "a.bin"]
        );
        assert_eq!(
            sanitize_rel_components("../../etc/passwd"),
            vec!["etc", "passwd"]
        );
        assert_eq!(sanitize_rel_components("a/./b/../c.txt"), vec!["a", "b", "c.txt"]);
        assert_eq!(sanitize_rel_components("../.."), vec!["received_file"]);
    }

    #[test]
    fn safe_dest_path_stays_inside_dest_dir() {
        let dir = tempfile::tempdir().unwrap();
        let dest = safe_dest_path(dir.path(), "../../escape.txt");
        assert!(dest.starts_with(dir.path()));
        assert_eq!(dest, dir.path().join("escape.txt"));

        let nested = safe_dest_path(dir.path(), "proj/src/main.rs");
        assert_eq!(nested, dir.path().join("proj").join("src").join("main.rs"));
    }

    #[test]
    fn unique_destination_suffixes_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let p0 = unique_destination(dir.path(), "a.txt");
        assert_eq!(p0, dir.path().join("a.txt"));
        std::fs::write(&p0, b"x").unwrap();
        let p1 = unique_destination(dir.path(), "a.txt");
        assert_eq!(p1, dir.path().join("a (1).txt"));
        std::fs::write(&p1, b"x").unwrap();
        let p2 = unique_destination(dir.path(), "a.txt");
        assert_eq!(p2, dir.path().join("a (2).txt"));
    }
}
