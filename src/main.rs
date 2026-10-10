#![windows_subsystem = "windows"]

#[cfg(not(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc")))]
compile_error!("WinRoll RS requires x86_64-pc-windows-msvc.");

mod geometry;
mod localization;
mod native;
mod sound;
mod startup;
mod transparency_settings;

fn main() {
    if let Err(error) = native::run() {
        native::report_error(&error);
        std::process::exit(1);
    }
}
