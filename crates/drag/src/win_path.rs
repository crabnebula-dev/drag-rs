// Copyright 2023-2023 CrabNebula Ltd.
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! Converting verbatim Windows paths for the shell namespace parsers.
//!
//! `ILCreateFromPathW` parses through the shell namespace, which does not
//! accept the verbatim `\\?\` prefix and answers with a null PIDL for any
//! path carrying one.
//!
//! `start_drag` does not produce such paths itself, but a caller can: an
//! application that canonicalizes before handing paths over gets the verbatim
//! form from `std::fs::canonicalize` for network paths and for anything over
//! `MAX_PATH`, and passing that straight through would fail for a reason that
//! is not its fault.
//!
//! The conversion works on UTF-16 code units so that it is lossless for paths
//! that are not valid Unicode, and so that it can be tested on any platform.

// Compiled on other platforms only so that the tests below can run there too.
#![cfg_attr(not(windows), allow(dead_code))]

/// `\\?\`
const VERBATIM: [u16; 4] = [b'\\' as u16, b'\\' as u16, b'?' as u16, b'\\' as u16];
/// `UNC\`
const UNC: [u16; 4] = [b'U' as u16, b'N' as u16, b'C' as u16, b'\\' as u16];

/// Rewrites a verbatim Windows path into the legacy form that the shell's
/// namespace parsers understand. Anything that is not verbatim is returned
/// unchanged.
///
/// - `\\?\C:\dir\file` becomes `C:\dir\file`
/// - `\\?\UNC\server\share\file` becomes `\\server\share\file`
pub(crate) fn strip_verbatim_prefix(wide: &[u16]) -> Vec<u16> {
    let Some(rest) = wide.strip_prefix(&VERBATIM) else {
        return wide.to_vec();
    };

    let is_unc = rest.get(..UNC.len()).is_some_and(|head| {
        head.iter()
            .zip(UNC)
            .all(|(a, b)| eq_ignore_ascii_case(*a, b))
    });

    if is_unc {
        // `UNC\server\share` describes `\\server\share`.
        let mut out = Vec::with_capacity(rest.len() - UNC.len() + 2);
        out.extend_from_slice(&VERBATIM[..2]);
        out.extend_from_slice(&rest[UNC.len()..]);
        out
    } else {
        rest.to_vec()
    }
}

fn eq_ignore_ascii_case(a: u16, b: u16) -> bool {
    let fold = |c: u16| {
        if (b'a' as u16..=b'z' as u16).contains(&c) {
            c - 32
        } else {
            c
        }
    };
    fold(a) == fold(b)
}

#[cfg(test)]
mod tests {
    use super::strip_verbatim_prefix;

    fn strip(path: &str) -> String {
        let wide: Vec<u16> = path.encode_utf16().collect();
        String::from_utf16(&strip_verbatim_prefix(&wide)).unwrap()
    }

    #[test]
    fn leaves_legacy_paths_alone() {
        assert_eq!(strip(r"C:\dir\file.png"), r"C:\dir\file.png");
        assert_eq!(
            strip(r"\\server\share\file.png"),
            r"\\server\share\file.png"
        );
        assert_eq!(strip(""), "");
    }

    #[test]
    fn strips_the_verbatim_disk_prefix() {
        assert_eq!(strip(r"\\?\C:\dir\file.png"), r"C:\dir\file.png");
        assert_eq!(strip(r"\\?\Z:\file.png"), r"Z:\file.png");
    }

    #[test]
    fn rewrites_verbatim_unc_as_a_network_path() {
        assert_eq!(
            strip(r"\\?\UNC\server\share\dir\file.png"),
            r"\\server\share\dir\file.png"
        );
        // `fs::canonicalize` emits uppercase, but do not depend on it.
        assert_eq!(
            strip(r"\\?\unc\server\share\file.png"),
            r"\\server\share\file.png"
        );
    }

    #[test]
    fn handles_a_path_longer_than_max_path() {
        let long = "a".repeat(300);
        assert_eq!(
            strip(&format!(r"\\?\C:\{long}\file.png")),
            format!(r"C:\{long}\file.png")
        );
    }

    #[test]
    fn does_not_mistake_a_unc_prefixed_name_for_the_unc_marker() {
        assert_eq!(strip(r"\\?\C:\UNCLE\file.png"), r"C:\UNCLE\file.png");
    }

    #[test]
    fn is_lossless_for_unpaired_surrogates() {
        let mut wide: Vec<u16> = r"\\?\C:\".encode_utf16().collect();
        wide.push(0xD800); // lone high surrogate
        let out = strip_verbatim_prefix(&wide);
        assert_eq!(out, [b'C' as u16, b':' as u16, b'\\' as u16, 0xD800]);
    }
}
