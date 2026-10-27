use std::io;
use std::path::{Path, PathBuf};

use super::{Environment, Ports, Stop};
use crate::system::Invocation;

const PRIVATE: u32 = 0o700;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployKey {
    pub file: PathBuf,
    pub signer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyFailure {
    pub stop: Stop,
    pub file: Option<PathBuf>,
}

impl KeyFailure {
    fn before_key(message: String) -> Self {
        Self {
            stop: Stop::Refused(message),
            file: None,
        }
    }
}

#[must_use]
pub fn key_dir(ports: &Ports, env: &Environment) -> PathBuf {
    let base = env
        .runtime_dir
        .as_ref()
        .filter(|dir| ports.files.is_dir(dir))
        .unwrap_or(&env.temp_dir);
    base.join(format!("deploy-key.{}", env.unique))
}

pub fn create(ports: &Ports, env: &Environment) -> Result<DeployKey, KeyFailure> {
    let dir = key_dir(ports, env);
    ports
        .storage
        .create_new_dir(&dir, PRIVATE)
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => KeyFailure::before_key(format!(
                "cannot create {}: it already exists",
                dir.display()
            )),
            _ => KeyFailure::before_key(format!("cannot create {}: {error}", dir.display())),
        })?;

    let file = dir.join("deploy");
    let public = dir.join("deploy.pub");
    let generated = generate(ports, &file);
    let signer = generated.and_then(|()| read_signer(ports, &public));
    ports.storage.remove(&public);
    match signer {
        Ok(signer) => Ok(DeployKey { file, signer }),
        Err(stop) => Err(KeyFailure {
            stop,
            file: ports.storage.exists(&file).then_some(file),
        }),
    }
}

fn generate(ports: &Ports, file: &Path) -> Result<(), Stop> {
    let path = file.display().to_string();
    let invocation = Invocation::new(
        "ssh-keygen",
        &["-q", "-t", "ed25519", "-N", "", "-C", "deploy", "-f", &path],
    );
    let answer = ports.run(&invocation)?;
    if answer.success {
        return Ok(());
    }
    Err(Stop::Refused(format!(
        "ssh-keygen cannot write {path}: {}",
        answer.stderr.trim_end()
    )))
}

fn read_signer(ports: &Ports, public: &Path) -> Result<String, Stop> {
    let unreadable = || Stop::Refused(format!("cannot read the public key {}", public.display()));
    let bytes = ports.files.read(public).map_err(|_| unreadable())?;
    let text = String::from_utf8_lossy(&bytes);
    let mut fields = text.split_whitespace();
    match (fields.next(), fields.next()) {
        (Some(kind), Some(body)) => Ok(format!("{kind} {body} deploy")),
        _ => Err(unreadable()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::init::fake::{Machine, environment};
    use crate::system::Storage;
    use crate::system::interrupt::Flag;
    use crate::system::prompt::Typed;

    fn created(machine: &Machine, env: &Environment) -> Result<DeployKey, KeyFailure> {
        let mut terminal = Typed::new(&[]);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let enter = |_: &Path| Ok(PathBuf::from(crate::init::fake::ROOT));
        let flag = Flag::default();
        let ports = Ports {
            commands: machine,
            files: machine,
            storage: machine,
            terminal: &mut terminal,
            interrupt: &flag,
            enter: &enter,
            out: &mut out,
            err: &mut err,
        };
        create(&ports, env)
    }

    #[test]
    fn the_key_goes_in_a_private_folder_of_the_runtime_dir_and_its_public_half_is_removed() {
        let machine = Machine::fork();

        let key = created(&machine, &environment()).unwrap();

        assert_eq!(
            key.file,
            PathBuf::from("/run/user/1000/deploy-key.42-7/deploy")
        );
        assert_eq!(
            key.signer,
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFake deploy"
        );
        assert!(!machine.exists(Path::new("/run/user/1000/deploy-key.42-7/deploy.pub")));
    }

    #[test]
    fn without_a_runtime_dir_the_key_goes_in_the_temp_dir() {
        let machine = Machine::fork();
        let env = Environment {
            runtime_dir: Some(PathBuf::from("/run/user/missing")),
            ..environment()
        };

        let key = created(&machine, &env).unwrap();

        assert_eq!(key.file, PathBuf::from("/tmp/deploy-key.42-7/deploy"));
    }

    #[test]
    fn a_folder_already_there_is_never_reused() {
        let machine = Machine::fork();
        machine
            .dirs
            .borrow_mut()
            .insert(PathBuf::from("/run/user/1000/deploy-key.42-7"));

        let failure = created(&machine, &environment()).unwrap_err();

        assert_eq!(failure.file, None);
        assert!(machine.ran().is_empty());
    }
}
