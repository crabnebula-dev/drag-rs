// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use std::path::{Path, PathBuf};

/// Using std::fs::canonicalize on Windows will retuen a UNC path ("\\?\C:\\path\to\file.txt")
/// Some applications do not support this for dropping as URI.
pub(crate) fn adjust_canonicalization<P: AsRef<Path>>(p: P) -> PathBuf {
    let p = p.as_ref().display().to_string();
    if let Some(stripped) = p.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(p)
    }
}
