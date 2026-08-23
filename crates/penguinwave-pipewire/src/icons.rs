//! Application icons.
//!
//! Resolving an icon walks the filesystem for `.desktop` entries, so listings
//! carry a key and the icon itself is fetched separately.

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::parse::RawStream;

const ICON_DIRS: &[&str] = &[
    "/usr/share/icons/hicolor",
    "/usr/share/pixmaps",
    "/usr/local/share/icons/hicolor",
];

const SIZES: &[&str] = &["256x256", "128x128", "64x64", "48x48", "scalable"];

/// Key identifying this stream's icon, stable for the life of the stream.
pub fn icon_key(stream: &RawStream) -> Option<String> {
    stream
        .icon_name
        .clone()
        .or_else(|| stream.binary.clone())
        .filter(|k| !k.is_empty())
}

/// Locate an icon file for a key.
pub fn icon_path(key: &str) -> Option<PathBuf> {
    if key.contains('/') || key.contains("..") {
        return None;
    }

    for dir in ICON_DIRS {
        let base = Path::new(dir);
        if !base.exists() {
            continue;
        }

        for size in SIZES {
            for ext in ["png", "svg"] {
                let candidate = base.join(size).join("apps").join(format!("{key}.{ext}"));
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }

        for ext in ["png", "svg"] {
            let flat = base.join(format!("{key}.{ext}"));
            if flat.exists() {
                return Some(flat);
            }
        }
    }

    None
}

/// Read an icon and base64-encode it.
pub fn icon_base64(key: &str) -> Option<String> {
    let bytes = fs::read(icon_path(key)?).ok()?;
    Some(base64(&bytes))
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        let indices = [n >> 18 & 63, n >> 12 & 63, n >> 6 & 63, n & 63];

        for (i, idx) in indices.iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[*idx as usize] as char);
            } else {
                out.push('=');
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    /// The key reaches the filesystem, so it must not be able to escape the
    /// icon directories.
    #[test]
    fn traversal_keys_are_rejected() {
        assert!(icon_path("../../etc/passwd").is_none());
        assert!(icon_path("/etc/passwd").is_none());
        assert!(icon_path("a/b").is_none());
    }
}
