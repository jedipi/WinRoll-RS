use std::{io, path::Path, ptr::null_mut};
use windows_sys::{
    Win32::{Foundation::*, System::Registry::*},
    core::w,
};

pub(super) fn installed() -> Result<bool, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let location = install_location().map_err(|e| e.to_string())?;
    matches_location(&executable, location.as_deref().map(Path::new)).map_err(|e| e.to_string())
}

fn matches_location(executable: &Path, location: Option<&Path>) -> io::Result<bool> {
    let Some(location) = location else {
        return Ok(false);
    };
    let directory = executable
        .parent()
        .ok_or_else(|| io::Error::other("Cannot locate the executable directory."))?;
    Ok(directory.canonicalize()? == location.canonicalize()?)
}

fn install_location() -> io::Result<Option<String>> {
    let mut bytes = 0;
    // SAFETY: registry strings are terminated and output buffers are valid for each call.
    unsafe {
        let key = w!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{4A9106DA-A87A-4CD2-8BD8-2177786DA580}_is1"
        );
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            w!("InstallLocation"),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            null_mut(),
            null_mut(),
            &mut bytes,
        );
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let mut value = vec![0u16; (bytes as usize).div_ceil(2)];
        let status = RegGetValueW(
            HKEY_CURRENT_USER,
            key,
            w!("InstallLocation"),
            RRF_RT_REG_SZ | RRF_SUBKEY_WOW6464KEY,
            null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        );
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32));
        }
        let length = value
            .iter()
            .position(|unit| *unit == 0)
            .unwrap_or(value.len());
        if length == 0 {
            return Err(io::Error::other("The installed location is empty."));
        }
        String::from_utf16(&value[..length])
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
#[test]
fn registered_directory_distinguishes_installed_and_portable_copies() {
    let installed = tempfile::tempdir().unwrap();
    let portable = tempfile::tempdir().unwrap();
    let executable = installed.path().join("winroll.exe");
    assert!(matches_location(&executable, Some(installed.path())).unwrap());
    assert!(
        !matches_location(&portable.path().join("winroll.exe"), Some(installed.path())).unwrap()
    );
    assert!(!matches_location(&executable, None).unwrap());
    assert!(matches_location(&executable, Some(&installed.path().join("missing"))).is_err());
}
