mod config;
pub mod git;
pub mod interrupt;
mod invocation;
mod machine;
pub mod os;
mod pattern;
mod ports;
pub mod process;
mod terminal;

pub use config::{ConfigError, Settings, load as load_settings};
pub use git::Git;
pub use invocation::{Invocation, Output};
pub use machine::{Machine, remove_if_exists};
pub use os::boot::{BootEntry, boot_entries, generation_went_back};
pub use os::executable::{find_in_path, is_executable};
pub use os::mount::Mounts;
pub use os::space::{Space, space};
#[cfg(any(test, feature = "testing"))]
pub use pattern::statics_declared_under;
pub use pattern::{Pattern, PatternInvalid};
pub use ports::accounts::{Accounts, Directory, Group, Owner, User};
pub use ports::clock::Clock;
pub use ports::commands::{Answer, Commands};
pub use ports::files::Files;
pub use ports::sizes::Sizes;
pub use ports::storage::Storage;
pub use ports::streams::{Streamed, Streams};
pub use ports::terminal::Terminal;
#[cfg(any(test, feature = "testing"))]
pub use ports::terminal::Typed;
pub use terminal::console::Console;
