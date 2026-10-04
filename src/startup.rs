use std::{io, os::windows::ffi::OsStrExt, ptr::null_mut};
use windows_sys::{
    Win32::{Foundation::*, System::Registry::*},
    core::w,
};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub fn enabled() -> io::Result<bool> {
    registered(RUN_KEY)
}

pub fn set_enabled(enabled: bool) -> io::Result<()> {
    set_registered(RUN_KEY, enabled)
}

fn registered(key: &str) -> io::Result<bool> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let mut bytes = 0;
    // SAFETY: strings are terminated, and bytes is a writable size output.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            w!("WinRoll RS"),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut bytes,
        )
    };
    match status {
        ERROR_FILE_NOT_FOUND => Ok(false),
        ERROR_SUCCESS => Ok(bytes > 2),
        _ => Err(io::Error::from_raw_os_error(status as i32)),
    }
}

fn startup_command() -> io::Result<Vec<u16>> {
    let executable = std::env::current_exe()?;
    let command: Vec<u16> = std::iter::once('"' as u16)
        .chain(executable.as_os_str().encode_wide())
        .chain(['"' as u16, 0])
        .collect();
    // Windows limits Run commands to 260 characters (excluding the terminator).
    if command.len() > 261 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The executable path is too long for Windows startup. Move WinRoll RS to a shorter path.",
        ));
    }
    Ok(command)
}

fn set_registered(key: &str, enabled: bool) -> io::Result<()> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    // SAFETY: strings and data are terminated and valid for each synchronous call;
    // the created registry handle is closed after writing.
    let status = unsafe {
        if enabled {
            let command = startup_command()?;
            let mut handle = null_mut();
            let status = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                0,
                null_mut(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                std::ptr::null(),
                &mut handle,
                null_mut(),
            );
            if status != ERROR_SUCCESS {
                return Err(io::Error::from_raw_os_error(status as i32));
            }
            let status = RegSetValueExW(
                handle,
                w!("WinRoll RS"),
                0,
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            );
            RegCloseKey(handle);
            status
        } else {
            RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), w!("WinRoll RS"))
        }
    };
    if status == ERROR_SUCCESS || (!enabled && status == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

#[cfg(test)]
#[test]
fn startup_registration_round_trip() {
    // Never change the user's real startup setting during tests.
    let key = format!("Software\\WinRoll RS Startup Test {}", std::process::id());
    let wide_key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let result = (|| -> io::Result<()> {
        assert!(!registered(&key)?);
        set_registered(&key, false)?;
        set_registered(&key, true)?;
        assert!(registered(&key)?);
        let expected = startup_command()?;
        let mut actual = vec![0u16; expected.len()];
        let mut bytes = (actual.len() * 2) as u32;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                wide_key.as_ptr(),
                w!("WinRoll RS"),
                RRF_RT_REG_SZ,
                null_mut(),
                actual.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        assert_eq!(status, ERROR_SUCCESS);
        assert_eq!(actual, expected);
        assert_eq!(actual[0], '"' as u16);
        assert_eq!(actual[actual.len() - 2], '"' as u16);
        set_registered(&key, true)?;
        set_registered(&key, false)?;
        assert!(!registered(&key)?);
        set_registered(&key, false)
    })();
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide_key.as_ptr()) };
    result.unwrap();
}
