use std::fs;
use std::io;
use std::path::Path;

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut staging = path.as_os_str().to_owned();
    staging.push(".new");
    fs::write(&staging, bytes)?;
    fs::rename(&staging, path)
}

#[must_use]
pub fn read_text(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{read_text, write_atomic};

    #[test]
    fn replaces_files_in_place_and_creates_folders() {
        let root = std::env::temp_dir().join(format!("tgow-storage-{}", std::process::id()));
        let file = root.join("nested").join("settings.json");
        write_atomic(&file, b"one").unwrap();
        write_atomic(&file, b"two").unwrap();
        assert_eq!(read_text(&file).as_deref(), Some("two"));
        assert!(!file.with_extension("json.new").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
