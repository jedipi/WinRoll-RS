use std::{io, ptr::null_mut};
use windows_sys::Win32::{Foundation::*, System::Registry::*};

const SETTINGS_KEY: &str = "Software\\WinRoll RS";

pub fn load() -> io::Result<u32> {
    load_from(SETTINGS_KEY)
}

pub fn save(percent: u32) -> io::Result<()> {
    save_to(SETTINGS_KEY, percent)
}

pub fn load_ignore_middle() -> io::Result<bool> {
    load_ignore_middle_from(SETTINGS_KEY)
}

pub fn save_ignore_middle(ignore: bool) -> io::Result<()> {
    save_dword(SETTINGS_KEY, "IgnoreMiddleMouseButton", u32::from(ignore))
}

pub fn load_minimize_as_menu() -> io::Result<bool> {
    load_minimize_as_menu_from(SETTINGS_KEY)
}

pub fn save_minimize_as_menu(as_menu: bool) -> io::Result<()> {
    save_dword(SETTINGS_KEY, "MinimizeAsMenu", u32::from(as_menu))
}

fn load_minimize_as_menu_from(key: &str) -> io::Result<bool> {
    match load_dword(key, "MinimizeAsMenu", 0)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Saved minimize as menu setting must be 0 or 1.",
        )),
    }
}

fn load_ignore_middle_from(key: &str) -> io::Result<bool> {
    match load_dword(key, "IgnoreMiddleMouseButton", 0)? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Saved ignore middle mouse button setting must be 0 or 1.",
        )),
    }
}

fn load_from(key: &str) -> io::Result<u32> {
    let percent = load_dword(key, "Transparency", 50)?;
    if percent <= 100 && percent.is_multiple_of(10) {
        Ok(percent)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Saved transparency must be between 0 and 100 in steps of 10.",
        ))
    }
}

fn load_dword(key: &str, name: &str, default: u32) -> io::Result<u32> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut value = 0u32;
    let mut bytes = size_of::<u32>() as u32;
    // SAFETY: strings are terminated and the DWORD buffer and size are writable.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            (&mut value as *mut u32).cast(),
            &mut bytes,
        )
    };
    match status {
        ERROR_FILE_NOT_FOUND => Ok(default),
        ERROR_SUCCESS => Ok(value),
        _ => Err(io::Error::from_raw_os_error(status as i32)),
    }
}

fn save_to(key: &str, percent: u32) -> io::Result<()> {
    if percent > 100 || !percent.is_multiple_of(10) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Transparency must be between 0 and 100 in steps of 10.",
        ));
    }
    save_dword(key, "Transparency", percent)
}

fn save_dword(key: &str, name: &str, value: u32) -> io::Result<()> {
    let key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let mut handle = null_mut();
    // SAFETY: strings and DWORD data are valid for these synchronous calls;
    // the created registry handle is closed after writing.
    let status = unsafe {
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
            name.as_ptr(),
            0,
            REG_DWORD,
            (&value as *const u32).cast(),
            size_of::<u32>() as u32,
        );
        RegCloseKey(handle);
        status
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

#[cfg(test)]
#[test]
fn transparency_setting_round_trip() {
    // Never change the user's real settings during tests.
    let key = format!(
        "Software\\WinRoll RS Transparency Test {}",
        std::process::id()
    );
    let wide_key: Vec<u16> = key.encode_utf16().chain(Some(0)).collect();
    let result = (|| -> io::Result<()> {
        assert_eq!(load_from(&key)?, 50);
        assert!(!load_minimize_as_menu_from(&key)?);
        for as_menu in [true, false] {
            save_dword(&key, "MinimizeAsMenu", u32::from(as_menu))?;
            assert_eq!(load_minimize_as_menu_from(&key)?, as_menu);
        }
        save_dword(&key, "MinimizeAsMenu", 2)?;
        assert_eq!(
            load_minimize_as_menu_from(&key).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(!load_ignore_middle_from(&key)?);
        for ignore in [true, false] {
            save_dword(&key, "IgnoreMiddleMouseButton", u32::from(ignore))?;
            assert_eq!(load_ignore_middle_from(&key)?, ignore);
        }
        for invalid in [2, u32::MAX] {
            save_dword(&key, "IgnoreMiddleMouseButton", invalid)?;
            assert_eq!(
                load_ignore_middle_from(&key).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
        for percent in (0..=100).step_by(10) {
            save_to(&key, percent)?;
            assert_eq!(load_from(&key)?, percent);
        }
        for percent in [1, 99, 101, u32::MAX] {
            assert_eq!(
                save_to(&key, percent).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        assert_eq!(load_from(&key)?, 100);
        Ok(())
    })();
    unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide_key.as_ptr()) };
    result.unwrap();
}
