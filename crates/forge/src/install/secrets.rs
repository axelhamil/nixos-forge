use std::path::Path;

use super::{Session, Stop};
use crate::system::Invocation;

const PASSWORDS: &str = "/mnt/persist/passwords";

pub fn read_file(session: &Session, path: &Path, holding: &str) -> Result<Vec<u8>, Stop> {
    let Ok(mut content) = session.ports.files.read(path) else {
        return Err(Stop::Refused(format!("cannot read {}.", path.display())));
    };
    while content.last() == Some(&b'\n') {
        content.pop();
    }
    if content.is_empty() {
        return Err(Stop::Refused(format!(
            "{} is empty: it must hold {holding}.",
            path.display()
        )));
    }
    Ok(content)
}

pub fn ask_twice(session: &mut Session, prompt: &str) -> Result<Vec<u8>, Stop> {
    loop {
        let first = session.secret(prompt)?;
        let second = session.secret("Same again")?;
        if first.is_empty() {
            session.print("It cannot be empty.\n");
        } else if first != second {
            session.print("They differ, try again.\n");
        } else {
            return Ok(first.into_bytes());
        }
    }
}

pub fn hash(session: &mut Session, password: &[u8]) -> Result<String, Stop> {
    let hashed = session
        .run(&Invocation::new("mkpasswd", &["--method=yescrypt", "--stdin"]).input(password))?;
    if !hashed.success {
        session.print(&hashed.stderr);
        return Err(Stop::failed(&hashed));
    }
    Ok(hashed.stdout.trim_end_matches('\n').to_owned())
}

pub fn write(session: &Session, hashes: &[(String, String)]) -> Result<(), Stop> {
    let storage = session.ports.storage;
    let directory = Path::new(PASSWORDS);
    storage
        .install_dir(directory, 0o700)
        .map_err(|error| Stop::Refused(format!("cannot create {PASSWORDS}: {error}")))?;
    for (user, hash) in hashes {
        let file = directory.join(user);
        storage
            .install_file(&file, format!("{hash}\n").as_bytes(), 0o600)
            .map_err(|error| Stop::Refused(format!("cannot write {}: {error}", file.display())))?;
    }
    Ok(())
}
