// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

//! At-rest API-key sealing.
//!
//! Windows uses DPAPI (`CryptProtectData`) bound to the current user; the ciphertext is
//! stored base64-encoded in `ai-settings.json`. Other platforms fall back to an
//! obfuscated (base64) value with a `plain:` prefix, because the desktop product targets
//! Windows; callers should treat that as convenience, not strong encryption.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use recraft_common::{Error, Result};

const DPAPI_PREFIX: &str = "dpapi:";
const PLAIN_PREFIX: &str = "plain:";

/// Seal a key for persistence.
pub fn seal(plain: &str) -> Result<String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    #[cfg(windows)]
    {
        let cipher = dpapi_protect(plain.as_bytes())
            .ok_or_else(|| Error::internal("DPAPI CryptProtectData failed"))?;
        Ok(format!("{DPAPI_PREFIX}{}", STANDARD.encode(cipher)))
    }
    #[cfg(not(windows))]
    {
        Ok(format!("{PLAIN_PREFIX}{}", STANDARD.encode(plain)))
    }
}

/// Open a sealed key. Returns `None` for empty/corrupt values.
pub fn open(sealed: &str) -> Option<String> {
    if let Some(value) = sealed.strip_prefix(DPAPI_PREFIX) {
        let cipher = STANDARD.decode(value).ok()?;
        #[cfg(windows)]
        {
            return dpapi_unprotect(&cipher);
        }
        #[cfg(not(windows))]
        {
            let _ = cipher;
            return None;
        }
    }
    if let Some(value) = sealed.strip_prefix(PLAIN_PREFIX) {
        let bytes = STANDARD.decode(value).ok()?;
        return String::from_utf8(bytes).ok();
    }
    None
}

#[cfg(windows)]
fn dpapi_protect(data: &[u8]) -> Option<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptProtectData};

    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptProtectData(
            &input,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return None;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData as *mut core::ffi::c_void);
    }
    Some(bytes)
}

#[cfg(windows)]
fn dpapi_unprotect(cipher: &[u8]) -> Option<String> {
    use std::ptr;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptUnprotectData};

    let input = CRYPT_INTEGER_BLOB {
        cbData: cipher.len() as u32,
        pbData: cipher.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return None;
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe {
        LocalFree(output.pbData as *mut core::ffi::c_void);
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_key_roundtrips_and_does_not_contain_plaintext() {
        let key = "sk-test-secret-123";
        let sealed = seal(key).unwrap();
        assert!(!sealed.contains(key));
        assert_eq!(open(&sealed).as_deref(), Some(key));
        assert!(open("garbage").is_none());
    }
}
