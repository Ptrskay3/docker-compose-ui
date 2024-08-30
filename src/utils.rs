use std::{
    iter::once,
    path::{Component, Path, PathBuf},
};

use crate::MAX_PATH_CHARS;

/// Shortens a path by replacing all components up to the last two with the single starting character and a dot.
/// Leaves length 2 or shorter path components unchanged.
/// Has no effect for paths with less than MAX_PATH_CHARS characters, or for paths that have 2 or less components.
pub fn shorten_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let components = path.components();
    let count = components.clone().count();
    if path.to_string_lossy().len() <= *MAX_PATH_CHARS.get().unwrap() || count <= 2 {
        return path.to_path_buf();
    }
    components
        .enumerate()
        .fold(PathBuf::new(), |mut acc, (i, component)| {
            // Preserve the last two components, and the at most 2 char long components.
            if count - i <= 2 || component.as_os_str().len() <= 2 {
                acc = acc.join(component);
            } else if let Component::Normal(component) = component {
                let component = component.to_string_lossy();
                acc = acc.join(
                    component
                        .chars()
                        .take(1)
                        .chain(once('.'))
                        .collect::<String>(),
                );
            }

            acc
        })
}
