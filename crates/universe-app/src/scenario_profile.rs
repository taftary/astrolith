//! Safety boundary for explicitly marked production-profile scenarios (#1238).
//! The public Python runner additionally excludes the real configuration root.

use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use universe_render::ScenarioProfileStart;

const MARKER: &str = "disposable-v1\n";
const USED: &str = "input-profile-v1\n";
const USED_FILE: &str = ".astrolith-input-used";

#[derive(Debug, Clone)]
pub(crate) struct Profile {
    root: PathBuf,
    pub settings: PathBuf,
    pub start: ScenarioProfileStart,
}

fn plain_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("disposable profile paths must be absolute".into());
    }
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                let linked = metadata.file_type().is_symlink();
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    // FILE_ATTRIBUTE_REPARSE_POINT includes junctions, not only symlinks.
                    linked || metadata.file_attributes() & 0x400 != 0
                };
                if linked {
                    return Err(
                        "disposable profile paths cannot contain links or reparse points".into(),
                    );
                }
            }
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(e) => return Err(format!("cannot inspect disposable profile path: {e}")),
        }
    }
    Ok(())
}

fn exact_marker(path: &Path, expected: &str) -> Result<(), String> {
    plain_path(path)?;
    let mut text = String::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(64).read_to_string(&mut text))
        .map_err(|e| format!("cannot read disposable profile marker: {e}"))?;
    if text != expected {
        return Err("disposable profile marker is missing or invalid".into());
    }
    Ok(())
}

impl Profile {
    pub(crate) fn check(root: &Path, configured_settings: &Path) -> Result<Self, String> {
        plain_path(root)?;
        plain_path(configured_settings)?;
        let root = root
            .canonicalize()
            .map_err(|e| format!("cannot resolve disposable profile: {e}"))?;
        exact_marker(&root.join(".astrolith-profile"), MARKER)?;
        let configured_root = configured_settings
            .parent()
            .and_then(Path::parent)
            .and_then(|p| p.canonicalize().ok());
        if configured_root.as_ref() != Some(&root)
            || configured_settings.file_name()
                != Some(std::ffi::OsStr::new(crate::persist::SETTINGS_FILE_NAME))
            || configured_settings.parent().and_then(Path::file_name)
                != Some(std::ffi::OsStr::new(crate::persist::SETTINGS_FOLDER_NAME))
        {
            return Err(
                "production settings path does not match the marked disposable profile".into(),
            );
        }
        let settings = root
            .join(crate::persist::SETTINGS_FOLDER_NAME)
            .join(crate::persist::SETTINGS_FILE_NAME);
        plain_path(&settings)?;
        let used = root.join(USED_FILE);
        let start = if used.try_exists().map_err(|e| e.to_string())? {
            exact_marker(&used, USED)?;
            ScenarioProfileStart::Restart
        } else {
            ScenarioProfileStart::Initial
        };
        Ok(Self {
            root,
            settings,
            start,
        })
    }

    /// Atomic first-use claim, only after parsing and restart checks succeed.
    pub(crate) fn claim(&self) -> Result<(), String> {
        if self.start == ScenarioProfileStart::Initial {
            std::fs::File::create_new(self.root.join(USED_FILE))
                .and_then(|mut file| file.write_all(USED.as_bytes()))
                .map_err(|e| format!("cannot claim initial disposable profile: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_must_be_marked_match_production_path_and_never_repeat_initial_claim() {
        let root =
            std::env::temp_dir().join(format!("astrolith-input-profile-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let settings = root
            .join(crate::persist::SETTINGS_FOLDER_NAME)
            .join(crate::persist::SETTINGS_FILE_NAME);
        assert!(Profile::check(&root, &settings).is_err());
        std::fs::write(root.join(".astrolith-profile"), MARKER).unwrap();
        assert!(Profile::check(&root, &root.join("not-settings.toml")).is_err());
        let profile = Profile::check(&root, &settings).unwrap();
        assert_eq!(profile.start, ScenarioProfileStart::Initial);
        profile.claim().unwrap();
        assert!(profile.claim().is_err());
        assert_eq!(
            Profile::check(&root, &settings).unwrap().start,
            ScenarioProfileStart::Restart
        );
        assert!(
            !settings.exists(),
            "marking a profile must not seed settings"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checked_in_compatibility_bytes_use_the_unmodified_production_loader() {
        use universe_core::settings::ViewMode;
        let root = std::env::temp_dir().join(format!(
            "astrolith-profile-compatibility-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        let file = root.join("settings.toml");
        for (text, mode, has_player) in [
            (
                include_str!("../../../scripts/validation/profiles/old-planet.toml"),
                Some(ViewMode::Dive),
                true,
            ),
            (
                include_str!("../../../scripts/validation/profiles/old-space.toml"),
                Some(ViewMode::Free),
                true,
            ),
            (
                include_str!("../../../scripts/validation/profiles/missing-records.toml"),
                None,
                false,
            ),
            (
                include_str!("../../../scripts/validation/profiles/damaged-records.toml"),
                None,
                false,
            ),
            (
                include_str!("../../../scripts/validation/profiles/unresolvable-records.toml"),
                Some(ViewMode::Dive),
                true,
            ),
        ] {
            std::fs::write(&file, text).unwrap();
            let loaded = crate::persist::load_or_default(&file);
            assert!(!loaded.labels && loaded.invert_look);
            assert!(
                loaded.shadows && loaded.bloom,
                "existing-file legacy defaults"
            );
            assert_eq!(loaded.views[0].map(|pose| pose.mode), mode);
            assert_eq!(loaded.player.is_some(), has_player);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
