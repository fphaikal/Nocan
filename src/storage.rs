use serde::{de::DeserializeOwned, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static SERIAL: AtomicU64 = AtomicU64::new(0);
static SAVE_LOCK: Mutex<()> = Mutex::new(());

pub fn data_dir() -> PathBuf {
    #[cfg(test)]
    let root = std::env::temp_dir().join(format!("nocan-unit-{}", std::process::id()));
    #[cfg(not(test))]
    let root = resolve_root(
        std::env::var_os("NOCAN_DATA_DIR").filter(|p| !p.is_empty()).map(PathBuf::from),
        known_local_appdata,
        || std::env::var_os("LOCALAPPDATA").filter(|p| !p.is_empty()).map(PathBuf::from),
        || std::env::var_os("USERPROFILE").filter(|p| !p.is_empty()).map(PathBuf::from),
    );
    let _ = fs::create_dir_all(&root);
    root
}

fn resolve_root(
    explicit: Option<PathBuf>,
    known: impl FnOnce() -> Option<PathBuf>,
    local: impl FnOnce() -> Option<PathBuf>,
    profile: impl FnOnce() -> Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = explicit { return path; }
    known().or_else(local).or_else(|| profile().map(|p| p.join("AppData").join("Local")))
        .unwrap_or_else(std::env::temp_dir).join("Nocan")
}

#[cfg(not(test))]
fn known_local_appdata() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_LocalAppData, KF_FLAG_DEFAULT};
    unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None).ok()?;
        let result = PathBuf::from(std::ffi::OsString::from_wide(path.as_wide()));
        CoTaskMemFree(Some(path.0.cast()));
        Some(result)
    }
}

pub fn backup_path(path: &Path) -> PathBuf { append_suffix(path, ".bak") }

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

enum Stored<T> { Missing, Valid(T, Vec<u8>), Invalid(Option<Vec<u8>>) }

fn read<T: DeserializeOwned>(path: &Path) -> Stored<T> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(value) => Stored::Valid(value, bytes),
            Err(_) => Stored::Invalid(Some(bytes)),
        },
        Err(e) if e.kind() == ErrorKind::NotFound => Stored::Missing,
        Err(_) => Stored::Invalid(None),
    }
}

pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let missing = match read(path) {
        Stored::Valid(value, _) => return Ok(Some(value)),
        Stored::Missing => true,
        Stored::Invalid(_) => false,
    };
    let backup_missing = match read(&backup_path(path)) {
        Stored::Valid(value, _) => return Ok(Some(value)),
        Stored::Missing => true,
        Stored::Invalid(_) => false,
    };
    // Old versions used a fixed temporary name. Only recover it if the original is absent.
    if missing && backup_missing {
        match read(&append_suffix(path, ".tmp")) {
            Stored::Valid(value, _) => return Ok(Some(value)),
            Stored::Missing => return Ok(None),
            Stored::Invalid(_) => (),
        }
    }
    Err("Unable to read saved data. The original file has been preserved.".into())
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) { if !self.0.as_os_str().is_empty() { let _ = fs::remove_file(&self.0); } }
}
impl Temporary {
    fn keep(mut self) { self.0 = PathBuf::new(); }
}

fn unique_file(path: &Path, kind: &str) -> Result<(Temporary, fs::File), String> {
    for _ in 0..32 {
        let suffix = format!(".{}-{}-{}", kind, std::process::id(), SERIAL.fetch_add(1, Ordering::Relaxed));
        let candidate = append_suffix(path, &suffix);
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => return Ok((Temporary(candidate), file)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(_) => return Err("Unable to create the storage file.".into()),
        }
    }
    Err("Unable to create the storage file.".into())
}

fn staged_bytes(path: &Path, bytes: &[u8]) -> Result<Temporary, String> {
    let (temporary, mut file) = unique_file(path, "tmp")?;
    file.write_all(bytes).and_then(|_| file.sync_all())
        .map_err(|_| "Unable to write data. Previous data has been preserved.".to_string())?;
    drop(file);
    Ok(temporary)
}

fn promote(temporary: Temporary, path: &Path) -> Result<(), String> {
    fs::rename(&temporary.0, path)
        .map_err(|_| "Unable to update data. Previous data has been preserved.".to_string())
}

pub fn save_json<T: Serialize + DeserializeOwned>(path: &Path, value: &T) -> Result<(), String> {
    let _guard = SAVE_LOCK.lock().map_err(|_| "Data storage is unavailable.".to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| "Unable to save data.".to_string())?;
    serde_json::from_slice::<T>(&bytes).map_err(|_| "Unable to save data.".to_string())?;
    let primary = read::<T>(path);
    let backup = read::<T>(&backup_path(path));
    match &primary {
        Stored::Invalid(_) if !matches!(backup, Stored::Valid(..)) =>
            return Err("The data file is invalid and has no valid backup. The original file has been preserved.".into()),
        Stored::Missing if matches!(backup, Stored::Invalid(_)) =>
            return Err("Unable to read the data backup. The file has been preserved.".into()),
        Stored::Invalid(None) =>
            return Err("Unable to access the data file. Please try again.".into()),
        _ => (),
    }
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|_| "Unable to create the storage folder.".to_string())?;
    }
    let temporary = staged_bytes(path, &bytes)?;
    match primary {
        Stored::Valid(_, previous) => {
            let backup = backup_path(path);
            promote(staged_bytes(&backup, &previous)?, &backup)?;
        }
        Stored::Invalid(Some(original)) => {
            let (preserved, mut file) = unique_file(path, "corrupt")?;
            file.write_all(&original).and_then(|_| file.sync_all())
                .map_err(|_| "Unable to preserve the original file. Save canceled.".to_string())?;
            drop(file);
            preserved.keep();
        }
        _ => (),
    }
    promote(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Data { title: String, count: u32 }
    fn data(count: u32) -> Data { Data { title: "Catatan 日本語 💧".into(), count } }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = data_dir().join(format!("storage-{}", SERIAL.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn file(&self) -> PathBuf { self.0.join("data.json") }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }

    #[test]
    fn writes_atomically_and_recovers_previous_valid_version_without_rewriting_primary() {
        let f = Fixture::new(); let p = f.file();
        save_json(&p, &data(1)).unwrap();
        save_json(&p, &data(2)).unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(2)));
        assert_eq!(load_json::<Data>(&backup_path(&p)).unwrap(), Some(data(1)));
        fs::write(&p, b"{interrupted").unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(1)));
        assert_eq!(fs::read(&p).unwrap(), b"{interrupted");
        save_json(&p, &data(3)).unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(3)));
        assert_eq!(load_json::<Data>(&backup_path(&p)).unwrap(), Some(data(1)));
        assert!(fs::read_dir(&f.0).unwrap().any(|e| {
            let p = e.unwrap().path();
            p.to_string_lossy().contains(".corrupt-") && fs::read(p).unwrap() == b"{interrupted"
        }));
    }

    #[test]
    fn malformed_data_without_valid_backup_is_never_overwritten() {
        let f = Fixture::new(); let p = f.file();
        fs::write(&p, b"not-json-private-content").unwrap();
        let error = load_json::<Data>(&p).unwrap_err();
        assert!(!error.contains("private-content"));
        assert!(save_json(&p, &data(0)).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"not-json-private-content");
        fs::write(backup_path(&p), b"also broken").unwrap();
        assert!(save_json(&p, &data(0)).is_err());
        assert_eq!(fs::read(backup_path(&p)).unwrap(), b"also broken");
    }

    #[test]
    fn missing_files_do_not_create_default_data_and_legacy_tmp_is_only_used_when_missing() {
        let f = Fixture::new(); let p = f.file();
        assert_eq!(load_json::<Data>(&p).unwrap(), None);
        assert!(!p.exists()); assert!(!backup_path(&p).exists());
        fs::write(append_suffix(&p, ".tmp"), serde_json::to_vec(&data(4)).unwrap()).unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(4)));
        assert!(!p.exists());
        fs::write(&p, b"broken").unwrap();
        assert!(load_json::<Data>(&p).is_err());
        assert_eq!(fs::read(&p).unwrap(), b"broken");
    }

    #[test]
    fn failed_backup_update_leaves_original_bytes_and_removes_staged_files() {
        let f = Fixture::new(); let p = f.file();
        save_json(&p, &data(1)).unwrap();
        let previous = fs::read(&p).unwrap();
        fs::create_dir(backup_path(&p)).unwrap();
        assert!(save_json(&p, &data(2)).is_err());
        assert_eq!(fs::read(&p).unwrap(), previous);
        assert_eq!(fs::read_dir(&f.0).unwrap().count(), 2);
        let blocker = f.0.join("blocker");
        fs::write(&blocker, b"keep").unwrap();
        assert!(save_json(&blocker.join("data.json"), &data(3)).is_err());
        assert_eq!(fs::read(blocker).unwrap(), b"keep");
    }

    #[test]
    fn new_storage_directories_are_created_on_save_only() {
        let f = Fixture::new(); let p = f.0.join("new").join("nested").join("data.json");
        assert_eq!(load_json::<Data>(&p).unwrap(), None);
        assert!(!p.parent().unwrap().exists());
        save_json(&p, &data(7)).unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(7)));
    }

    #[test]
    fn missing_primary_recovers_backup_and_unreadable_primary_is_preserved() {
        let f = Fixture::new(); let p = f.file();
        fs::write(backup_path(&p), serde_json::to_vec(&data(5)).unwrap()).unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(5)));
        fs::create_dir(&p).unwrap();
        assert_eq!(load_json::<Data>(&p).unwrap(), Some(data(5)));
        assert!(save_json(&p, &data(6)).is_err()); assert!(p.is_dir());
    }

    #[test]
    fn root_is_stable_and_known_folder_wins_over_redirected_environment() {
        let known = PathBuf::from("C:/Users/test/AppData/Local");
        assert_eq!(resolve_root(None, || Some(known.clone()), || panic!("must use known folder"), || None), known.join("Nocan"));
        let portable = PathBuf::from("H:/portable-data");
        assert_eq!(resolve_root(Some(portable.clone()), || panic!("explicit override"), || None, || None), portable);
        assert_eq!(resolve_root(None, || None, || Some(PathBuf::from("local")), || None), PathBuf::from("local/Nocan"));
        assert_eq!(resolve_root(None, || None, || None, || Some(PathBuf::from("profile"))), PathBuf::from("profile/AppData/Local/Nocan"));
        assert_eq!(data_dir(), data_dir());
        assert_eq!(data_dir(), std::env::temp_dir().join(format!("nocan-unit-{}", std::process::id())));
    }
}
