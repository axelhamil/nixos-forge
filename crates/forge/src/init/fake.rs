use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io;
use std::path::{Path, PathBuf};

use super::{Environment, Ports};
use crate::system::interrupt::Flag;
use crate::system::prompt::Typed;
use crate::system::{Answer, Commands, Files, Invocation, Owner, Storage};

pub const ROOT: &str = "/fork";

pub const HOST: &str = include_str!("../../tests/fixtures/init/modules/hosts/example/default.nix");
pub const SERVER: &str =
    include_str!("../../tests/fixtures/init/modules/hosts/example-server/default.nix");
pub const PI: &str = include_str!("../../tests/fixtures/init/modules/hosts/example-pi/default.nix");
pub const PERSON: &str = include_str!("../../tests/fixtures/init/modules/users/alice/default.nix");
pub const README: &str = include_str!("../../tests/fixtures/init/README.md");

pub const PUBLIC_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFake deploy\n";

pub struct Machine {
    pub files: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
    pub dirs: RefCell<BTreeSet<PathBuf>>,
    pub answers: HashMap<String, Answer>,
    pub log: RefCell<Vec<String>>,
    pub interrupt_on: Option<(String, Flag, i32)>,
}

impl Machine {
    pub fn fork() -> Self {
        let machine = Self {
            files: RefCell::default(),
            dirs: RefCell::default(),
            answers: HashMap::new(),
            log: RefCell::default(),
            interrupt_on: None,
        };
        for (path, text) in [
            ("modules/hosts/example/default.nix", HOST),
            ("modules/hosts/example-server/default.nix", SERVER),
            ("modules/hosts/example-pi/default.nix", PI),
            ("modules/users/alice/default.nix", PERSON),
            ("README.md", README),
        ] {
            let path = Path::new(ROOT).join(path);
            if let Some(dir) = path.parent() {
                machine.dirs.borrow_mut().insert(dir.to_path_buf());
            }
            machine
                .files
                .borrow_mut()
                .insert(path, text.as_bytes().to_vec());
        }
        machine
            .dirs
            .borrow_mut()
            .insert(PathBuf::from("/run/user/1000"));
        machine
    }

    pub fn answer(mut self, command: &str, answer: Answer) -> Self {
        self.answers.insert(command.to_owned(), answer);
        self
    }

    pub fn ran(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    pub fn text(&self, relative: &str) -> Option<String> {
        self.files
            .borrow()
            .get(&Path::new(ROOT).join(relative))
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    }
}

impl Commands for Machine {
    fn run(&self, invocation: &Invocation) -> Answer {
        let line = invocation.command_line();
        self.log.borrow_mut().push(line.clone());
        if let Some((command, flag, signal)) = &self.interrupt_on
            && line.starts_with(command.as_str())
        {
            flag.raise(*signal);
            return Answer::exited(128 + signal);
        }
        if let Some(answer) = self.answers.get(&line) {
            return answer.clone();
        }
        if line == "git rev-parse --is-inside-work-tree --show-cdup" {
            return Answer::ok("true\n\n");
        }
        if line == "git -C . remote get-url origin" {
            return Answer::exited(2);
        }
        if invocation.program == "ssh-keygen"
            && let Some(file) = invocation.args.last()
        {
            let mut files = self.files.borrow_mut();
            files.insert(PathBuf::from(file), b"PRIVATE".to_vec());
            files.insert(
                PathBuf::from(format!("{file}.pub")),
                PUBLIC_KEY.as_bytes().to_vec(),
            );
        }
        Answer::ok("")
    }
}

impl Files for Machine {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }

    fn read_link(&self, _: &Path) -> Option<PathBuf> {
        None
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.dirs.borrow().contains(path)
    }

    fn list(&self, _: &Path) -> Vec<String> {
        Vec::new()
    }

    fn canonicalize(&self, path: &Path) -> PathBuf {
        if path == Path::new("/etc/localtime") {
            return PathBuf::from("/nix/store/tzdata/share/zoneinfo/Europe/Berlin");
        }
        path.to_path_buf()
    }

    fn owner(&self, _: &Path) -> Option<Owner> {
        None
    }
}

impl Storage for Machine {
    fn is_block_device(&self, _: &Path) -> bool {
        false
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.borrow().contains_key(path) || self.dirs.borrow().contains(path)
    }

    fn link(&self, _: &Path, _: &Path) -> io::Result<()> {
        Ok(())
    }

    fn create_new_dir(&self, path: &Path, _: u32) -> io::Result<()> {
        if !self.dirs.borrow_mut().insert(path.to_path_buf()) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        Ok(())
    }

    fn install_dir(&self, path: &Path, _: u32) -> io::Result<()> {
        self.dirs.borrow_mut().insert(path.to_path_buf());
        Ok(())
    }

    fn install_file(&self, path: &Path, bytes: &[u8], _: u32) -> io::Result<()> {
        self.write(path, bytes)
    }

    fn create_dirs(&self, path: &Path) -> io::Result<()> {
        self.dirs.borrow_mut().insert(path.to_path_buf());
        Ok(())
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), bytes.to_vec());
        Ok(())
    }

    fn rename(&self, _: &Path, _: &Path) -> io::Result<()> {
        Ok(())
    }

    fn remove(&self, path: &Path) {
        self.files.borrow_mut().remove(path);
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.dirs.borrow_mut().remove(path);
        Ok(())
    }
}

pub fn environment() -> Environment {
    Environment {
        upstream: "axelhamil/nixos-forge".to_owned(),
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        temp_dir: PathBuf::from("/tmp"),
        language: "de_DE.utf8".to_owned(),
        nix_on_path: false,
        unique: "42-7".to_owned(),
    }
}

pub struct Outcome {
    pub code: u8,
    pub out: String,
    pub err: String,
    pub prompts: Vec<String>,
}

pub fn run_init(
    machine: &Machine,
    terminal: &mut Typed,
    flag: &Flag,
    env: &Environment,
    args: &[&str],
) -> Outcome {
    let mut out = Vec::new();
    let outcome = run_init_into(machine, terminal, flag, env, args, &mut out);
    Outcome {
        out: String::from_utf8_lossy(&out).into_owned(),
        ..outcome
    }
}

pub fn run_init_into(
    machine: &Machine,
    terminal: &mut Typed,
    flag: &Flag,
    env: &Environment,
    args: &[&str],
    out: &mut dyn io::Write,
) -> Outcome {
    let args: Vec<_> = args.iter().map(std::ffi::OsString::from).collect();
    let Ok(super::Parsed::Run(flags)) = super::parse(&args) else {
        panic!("the arguments were refused");
    };
    let mut err = Vec::new();
    let enter = |_: &Path| Ok(PathBuf::from(ROOT));
    let code = {
        let mut ports = Ports {
            commands: machine,
            files: machine,
            storage: machine,
            terminal,
            interrupt: flag,
            enter: &enter,
            out,
            err: &mut err,
        };
        super::steps::init(&flags, env, &mut ports)
    };
    Outcome {
        code,
        out: String::new(),
        err: String::from_utf8_lossy(&err).into_owned(),
        prompts: terminal.prompts.clone(),
    }
}
