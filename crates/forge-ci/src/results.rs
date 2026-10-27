use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::model::{BootTime, CheckResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded<T> {
    pub values: Vec<T>,
    pub unreadable: Vec<Unreadable>,
}

impl<T> Default for Loaded<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            unreadable: Vec::new(),
        }
    }
}

#[must_use]
pub fn json_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .filter(|path| path.is_file())
        .collect();
    files.sort();
    files
}

#[must_use]
pub fn load<T: DeserializeOwned>(dir: &Path) -> Loaded<T> {
    let mut loaded = Loaded::default();
    for path in json_files(dir) {
        match read_json(&path) {
            Ok(value) => loaded.values.push(value),
            Err(reason) => loaded.unreadable.push(Unreadable { path, reason }),
        }
    }
    loaded
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

#[must_use]
pub fn check_results(dir: &Path) -> Loaded<CheckResult> {
    load(dir)
}

#[must_use]
pub fn perf(dir: &Path) -> BTreeMap<String, Map<String, Value>> {
    json_files(&dir.join("perf"))
        .into_iter()
        .filter_map(|path| {
            let system = path.file_stem()?.to_string_lossy().into_owned();
            let measures = read_json::<Map<String, Value>>(&path).ok()?;
            Some((system, measures))
        })
        .collect()
}

#[must_use]
pub fn boot_times(file: &Path) -> BTreeMap<String, BootTime> {
    let Ok(hosts) = read_json::<Map<String, Value>>(file) else {
        return BTreeMap::new();
    };

    hosts
        .into_iter()
        .filter_map(|(host, time)| Some((host, serde_json::from_value(time).ok()?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str) {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn results_load_in_name_order_and_report_unreadable_files() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "x86_64-linux-docs.json",
            r#"{"check":"docs","system":"x86_64-linux","outcome":"passed"}"#,
        );
        write(
            dir.path(),
            "aarch64-linux-example-pi-sd-image.json",
            r#"{"check":"example-pi-sd-image","system":"aarch64-linux","outcome":"failed","excerpt":["error: x"],"bootTime":null,"screenshots":[]}"#,
        );
        write(dir.path(), "broken.json", "{");
        write(dir.path(), "notes.md", "not a result");
        write(dir.path(), "perf/x86_64-linux.json", "{}");

        let loaded = check_results(dir.path());

        let checks: Vec<&str> = loaded
            .values
            .iter()
            .map(|result| result.check.as_str())
            .collect();
        assert_eq!(checks, ["example-pi-sd-image", "docs"]);
        assert_eq!(loaded.values[0].excerpt, ["error: x"]);
        assert_eq!(loaded.unreadable.len(), 1);
        assert_eq!(loaded.unreadable[0].path, dir.path().join("broken.json"));
    }

    #[test]
    fn a_missing_results_dir_loads_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            check_results(&dir.path().join("results")),
            Loaded::default()
        );
    }

    #[test]
    fn perf_keeps_the_objects_named_by_their_system() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "perf/x86_64-linux.json",
            r#"{"example":{"boot":1}}"#,
        );
        write(dir.path(), "perf/aarch64-linux.json", "[1]");
        write(dir.path(), "perf/riscv64-linux.json", "{");

        let perf = perf(dir.path());

        assert_eq!(perf.keys().collect::<Vec<_>>(), ["x86_64-linux"]);
        assert_eq!(perf["x86_64-linux"]["example"]["boot"], 1);
    }

    #[test]
    fn boot_times_keep_the_hosts_with_a_boot_time_object() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "current/boot-time.json",
            r#"{"example":{"total":6.84,"userspace":2.1},"broken":"x","server":{"total":12}}"#,
        );

        let times = boot_times(&dir.path().join("current/boot-time.json"));

        assert_eq!(times.keys().collect::<Vec<_>>(), ["example", "server"]);
        assert_eq!(times["example"].total, Some(6.84));
        assert_eq!(times["server"].userspace, None);
    }

    #[test]
    fn a_missing_or_malformed_boot_time_file_holds_no_host() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "base/boot-time.json", "[]");
        assert!(boot_times(&dir.path().join("base/boot-time.json")).is_empty());
        assert!(boot_times(&dir.path().join("current/boot-time.json")).is_empty());
    }
}
