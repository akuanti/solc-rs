//! Utility functions

use std::fmt::Debug;
use std::path::{Component, Path, PathBuf};
use std::path;
use std::ffi::OsString;

// ported from Python
/// Normalize a path, eliminating double slashes, etc.
/// Does not handle symbolic links!
pub fn norm_path<P>(path: P) -> PathBuf
where
    P: AsRef<Path> + Debug,
{
    println!("orig: {:?}", path);

    let mut sep = String::new();
    sep.push(path::MAIN_SEPARATOR);
    let sep = sep.as_str();
    let has_initial_slashes = path.as_ref().starts_with(sep);

    // let mut initial_slashes = 1;

    // POSIX allows one or two initial slashes, but treats three or more as a single slash
    // if has_initial_slashes {
    //     if path.as_ref().starts_with(sep.repeat(2)) && !path.as_ref().starts_with(sep.repeat(3)) {
    //         initial_slashes = 2;
    //     }
    // }

    let mut new_comps: Vec<Component> = Vec::new();
    let empty: OsString = OsString::new();

    for component in path.as_ref().components() {
        // println!("{:?}", component);

        // skip empty or .
        if component == Component::Normal(empty.as_os_str()) || component == Component::CurDir {
            continue;
        }

        if (component != Component::ParentDir || (!has_initial_slashes && new_comps.is_empty()))
            || (new_comps.last() == Some(&Component::ParentDir))
        {
            new_comps.push(component);
        } else if !new_comps.is_empty() {
            // If we have a ParentDir, remove the last component
            new_comps.pop();
        } else {
            continue;
        }
    }

    // build up the result
    let normalized: PathBuf = new_comps.iter().map(|comp| comp.as_os_str()).collect();

    // if has_initial_slashes {
    //     let prefix = PathBuf::from(sep.repeat(initial_slashes));
    //     normalized = prefix.join(normalized);
    // }
    println!("norm: {:?}", normalized);
    normalized
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_leave_root() {
        let p = norm_path("/");
        assert_eq!(p, PathBuf::from("/"));
    }

    #[test]
    fn should_remove_double_slash() {
        let p = norm_path("A//B");
        assert_eq!(p, PathBuf::from("A/B"));
    }

    #[test]
    fn should_remove_dot() {
        let p = norm_path("A/./B");
        assert_eq!(p, PathBuf::from("A/B"));
    }

    #[test]
    fn should_remove_double_dot() {
        let p = norm_path("A/foo/../B");
        assert_eq!(p, PathBuf::from("A/B"));
    }

    #[test]
    fn should_reduce_multiple_initial_slashes() {
        let p = norm_path("///A/foo/../B");
        assert_eq!(p, PathBuf::from("/A/B"));
    }
}
