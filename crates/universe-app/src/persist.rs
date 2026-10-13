//! Settings file load and save (#473, T2).
//!
//! The window build reads the player's settings at start-up and writes them
//! back on every change. The file is plain `key = value` text (see
//! `universe_core::settings`); this module only resolves where it lives and
//! moves bytes. `--verify` and `--capture` never call into this module, so
//! their output and the golden files stay untouched (`E-GOLDEN`).

use bevy::ecs::change_detection::DetectChanges;
use std::path::{Path, PathBuf};
use universe_core::settings::Settings;
use universe_render::SettingsStore;

/// File name of the settings file.
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.toml";

/// Folder name holding the settings file (`Astrolith` on Windows and macOS,
/// lowercase `astrolith` under XDG on other platforms).
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub(crate) const SETTINGS_FOLDER_NAME: &str = "Astrolith";
/// Folder name holding the settings file (`Astrolith` on Windows and macOS,
/// lowercase `astrolith` under XDG on other platforms).
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(crate) const SETTINGS_FOLDER_NAME: &str = "astrolith";

/// Environment variable naming the settings file to use instead of the
/// platform one (#613): a game started from an agent's worktree gets it from
/// that worktree's cargo config, so agents never touch the owner's settings.
pub(crate) const SETTINGS_ENV: &str = "ASTROLITH_SETTINGS";

/// Where the settings file lives: [`SETTINGS_ENV`] when set, else the
/// platform folder ([`platform_settings_path`]).
#[must_use]
pub(crate) fn settings_path() -> PathBuf {
    settings_path_from(std::env::var_os(SETTINGS_ENV))
}

/// The settings path for an [`SETTINGS_ENV`] value: that path when it is
/// set and not empty, else the platform folder.
#[must_use]
pub(crate) fn settings_path_from(value: Option<std::ffi::OsString>) -> PathBuf {
    match value {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ => platform_settings_path(),
    }
}

/// The platform settings file.
///
/// Windows reads `%APPDATA%\Astrolith\settings.toml`; other platforms use the
/// platform config folder (`$XDG_CONFIG_HOME/astrolith` or
/// `$HOME/.config/astrolith`, `$HOME/Library/Application Support/Astrolith`
/// on macOS). When no folder can be found, the system temp folder is the
/// fallback, so a path is always returned and the game still starts.
#[must_use]
pub(crate) fn platform_settings_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA")
            && !appdata.is_empty()
        {
            return Path::new(&appdata)
                .join(SETTINGS_FOLDER_NAME)
                .join(SETTINGS_FILE_NAME);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            return Path::new(&home)
                .join("Library")
                .join("Application Support")
                .join(SETTINGS_FOLDER_NAME)
                .join(SETTINGS_FILE_NAME);
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            return Path::new(&xdg)
                .join(SETTINGS_FOLDER_NAME)
                .join(SETTINGS_FILE_NAME);
        }
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            return Path::new(&home)
                .join(".config")
                .join(SETTINGS_FOLDER_NAME)
                .join(SETTINGS_FILE_NAME);
        }
    }
    std::env::temp_dir().join(SETTINGS_FILE_NAME)
}

/// Settings read from `path`, or the defaults when the file is missing or
/// damaged.
///
/// A missing or unreadable file is normal on first launch; damaged text
/// falls back line by line through `Settings::parse`, so this never fails.
/// Logs one neutral line when the file could not be used.
#[must_use]
pub(crate) fn load_or_default(path: &Path) -> Settings {
    match std::fs::read_to_string(path) {
        Ok(text) => Settings::parse(&text),
        Err(_) => {
            bevy::log::info!("Settings file not present; starting with defaults");
            Settings::defaults()
        }
    }
}

/// Writes `settings` to `path`, creating parent folders as needed.
///
/// # Errors
///
/// Returns the I/O failure when the folders or the file cannot be written.
pub(crate) fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, settings.format())?;
    Ok(())
}

/// Bevy system: persists the synced store whenever it changes.
///
/// Keys and widgets both write the store; this covers every change.
/// Failures are logged once with a neutral line; the game keeps running.
pub(crate) fn save_on_change(
    settings: bevy::prelude::Res<SettingsStore>,
    path: bevy::prelude::Res<SettingsPath>,
) {
    if !settings.is_changed() {
        return;
    }
    if save(&path.0, &settings.0).is_err() {
        bevy::log::info!("Settings file not stored; keeping current settings");
    }
}

/// Resource holding the resolved settings path.
#[derive(bevy::prelude::Resource, Debug, Clone, PartialEq, Eq)]
pub(crate) struct SettingsPath(pub PathBuf);

#[cfg(test)]
mod tests {
    use super::*;

    /// Scratch folder for one test (hermetic: nothing outside it is touched).
    fn scratch(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("astrolith-persist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_settings_env_overrides_the_platform_file_when_set() {
        let worktree = PathBuf::from("work/613-x/.agent-local/settings.toml");
        assert_eq!(
            settings_path_from(Some(worktree.clone().into_os_string())),
            worktree,
            "a worktree's game uses its own file"
        );
        assert_eq!(
            settings_path_from(Some(std::ffi::OsString::new())),
            platform_settings_path(),
            "an empty value is ignored"
        );
        assert_eq!(
            settings_path_from(None),
            platform_settings_path(),
            "unset: the platform file, as before"
        );
    }

    #[test]
    fn missing_file_gives_defaults() {
        let path = scratch("missing").join(SETTINGS_FILE_NAME);
        assert_eq!(load_or_default(&path), Settings::defaults());
    }

    #[test]
    fn damaged_file_gives_defaults_for_bad_lines() {
        let dir = scratch("damaged");
        let path = dir.join(SETTINGS_FILE_NAME);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        std::fs::write(&path, "exposure = 99\nlabels = maybe\nvsync = off\n")
            .expect("scratch file");
        let mut want = Settings::defaults();
        want.vsync = false;
        assert_eq!(load_or_default(&path), want);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = scratch("roundtrip");
        let path = dir.join(SETTINGS_FILE_NAME);
        let mut settings = Settings::defaults();
        settings.fullscreen = true;
        settings.speed = universe_core::settings::FlightSpeed::new(9).unwrap_or_default();
        save(&path, &settings).expect("save works");
        assert_eq!(load_or_default(&path), settings);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_and_capture_drivers_do_not_reference_persist() {
        let verify = include_str!("verify.rs");
        let capture = include_str!("capture.rs");
        for source in [verify, capture] {
            assert!(
                !source.contains("persist"),
                "headless drivers must not touch the settings file"
            );
            assert!(
                !source.contains("settings"),
                "headless drivers must not read settings"
            );
        }
    }
}
