use std::time::Duration;

use super::{Answer, Commands, Invocation, Owner};

#[derive(Clone)]
pub struct Git<'a> {
    commands: &'a dyn Commands,
    repo: String,
    env: Vec<(String, String)>,
    run_as: Option<Owner>,
}

impl<'a> Git<'a> {
    #[must_use]
    pub fn new(commands: &'a dyn Commands, repo: &str) -> Self {
        Self {
            commands,
            repo: repo.to_owned(),
            env: Vec::new(),
            run_as: None,
        }
    }

    #[must_use]
    pub fn repo(&self) -> &str {
        &self.repo
    }

    #[must_use]
    pub fn run_as(mut self, owner: Owner) -> Self {
        self.run_as = Some(owner);
        self
    }

    #[must_use]
    pub fn with_env(&self, name: &str, value: &str) -> Self {
        let mut git = self.clone();
        git.env.push((name.to_owned(), value.to_owned()));
        git
    }

    #[must_use]
    pub fn invocation(&self, args: &[&str]) -> Invocation {
        let mut invocation = Invocation::new("git", &["-C", &self.repo]);
        invocation
            .args
            .extend(args.iter().map(|&arg| arg.to_owned()));
        invocation.env.clone_from(&self.env);
        invocation.run_as = self.run_as;
        invocation
    }

    #[must_use]
    pub fn run(&self, invocation: &Invocation) -> Answer {
        self.commands.run(invocation)
    }

    #[must_use]
    pub fn quiet(&self, args: &[&str]) -> Answer {
        self.run(&self.invocation(args))
    }

    #[must_use]
    pub fn loud(&self, args: &[&str]) -> Answer {
        self.run(&self.invocation(args).loud())
    }

    #[must_use]
    pub fn attached(&self, args: &[&str]) -> Answer {
        self.run(&self.invocation(args).attached())
    }

    #[must_use]
    pub fn fetch(&self, args: &[&str], timeout: Duration) -> Answer {
        let args: Vec<&str> = std::iter::once("fetch")
            .chain(args.iter().copied())
            .collect();
        self.run(&self.invocation(&args).timeout(timeout).loud())
    }

    #[must_use]
    pub fn remote(&self, args: &[&str], timeout: Duration) -> Answer {
        self.run(&Invocation::new("git", args).timeout(timeout).loud())
    }
}
