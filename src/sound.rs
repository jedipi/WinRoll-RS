use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};

pub static ENABLED: AtomicBool = AtomicBool::new(true);

pub const ROLL_UP: &[u8] = include_bytes!("../assets/sounds/roll-up.wav");
pub const ROLL_DOWN: &[u8] = include_bytes!("../assets/sounds/roll-down.wav");

pub fn play(wave: &'static [u8]) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    // Embedded bytes outlive asynchronous playback; unavailable audio must not affect restoration.
    unsafe {
        PlaySoundW(
            wave.as_ptr().cast(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_sounds_are_distinct_short_pcm_waves() {
        assert_ne!(ROLL_UP, ROLL_DOWN);
        for wave in [ROLL_UP, ROLL_DOWN] {
            let word = |offset| u16::from_le_bytes(wave[offset..offset + 2].try_into().unwrap());
            let dword = |offset| u32::from_le_bytes(wave[offset..offset + 4].try_into().unwrap());
            assert_eq!(&wave[..4], b"RIFF");
            assert_eq!(dword(4) as usize + 8, wave.len());
            assert_eq!(&wave[8..16], b"WAVEfmt ");
            assert_eq!(dword(16), 16);
            assert_eq!(word(20), 1); // PCM
            assert_eq!(word(22), 1); // Mono
            assert_eq!(dword(24), 44100);
            assert_eq!(dword(28), 88200);
            assert_eq!(word(32), 2);
            assert_eq!(word(34), 16);
            assert_eq!(&wave[36..40], b"data");
            assert_eq!(dword(40) as usize + 44, wave.len());
            let duration = f64::from(dword(40)) / f64::from(dword(28));
            assert!((0.49..=0.51).contains(&duration));
            assert!(wave[44..].iter().any(|&byte| byte != 0));
        }
    }
}
