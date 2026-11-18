#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub gid: u32,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Owner {
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Directory {
    pub users: Vec<User>,
    pub groups: Vec<Group>,
}

pub trait Accounts {
    fn directory(&self) -> Directory;
}

impl Directory {
    #[must_use]
    pub fn parse(passwd: &str, group: &str) -> Self {
        Self {
            users: passwd.lines().filter_map(user).collect(),
            groups: group.lines().filter_map(group_entry).collect(),
        }
    }

    #[must_use]
    pub fn user_name(&self, uid: u32) -> Option<&str> {
        self.users
            .iter()
            .find(|user| user.uid == uid)
            .map(|user| user.name.as_str())
    }

    #[must_use]
    pub fn group_name(&self, gid: u32) -> Option<&str> {
        self.groups
            .iter()
            .find(|group| group.gid == gid)
            .map(|group| group.name.as_str())
    }

    #[must_use]
    pub fn primary_group(&self, uid: u32) -> Option<u32> {
        self.users
            .iter()
            .find(|user| user.uid == uid)
            .map(|user| user.gid)
    }

    #[must_use]
    pub fn groups_of(&self, name: &str) -> Option<Vec<&str>> {
        let user = self.users.iter().find(|user| user.name == name)?;
        let primary = self.group_name(user.gid);
        let supplementary = self
            .groups
            .iter()
            .filter(|group| group.members.iter().any(|member| member == name))
            .map(|group| group.name.as_str());
        Some(primary.into_iter().chain(supplementary).collect())
    }
}

fn user(entry: &str) -> Option<User> {
    let fields: Vec<&str> = entry.split(':').collect();
    Some(User {
        name: (*fields.first()?).to_owned(),
        uid: fields.get(2)?.parse().ok()?,
        gid: fields.get(3)?.parse().ok()?,
    })
}

fn group_entry(entry: &str) -> Option<Group> {
    let fields: Vec<&str> = entry.split(':').collect();
    let members = fields.get(3).copied().unwrap_or_default();
    Some(Group {
        name: (*fields.first()?).to_owned(),
        gid: fields.get(2)?.parse().ok()?,
        members: members
            .split(',')
            .filter(|member| !member.is_empty())
            .map(str::to_owned)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_user_belongs_to_its_primary_group_and_the_groups_listing_it() {
        let directory = Directory::parse(
            "root:x:0:0::/root:/bin/sh\nalice:x:1000:100::/home/alice:/bin/sh\n",
            "root:x:0:\nwheel:x:1:alice,bob\nusers:x:100:\n",
        );

        assert_eq!(directory.groups_of("alice"), Some(vec!["users", "wheel"]));
        assert_eq!(directory.groups_of("bob"), None);
        assert_eq!(directory.user_name(1000), Some("alice"));
        assert_eq!(directory.group_name(1), Some("wheel"));
        assert_eq!(directory.primary_group(1000), Some(100));
    }
}
