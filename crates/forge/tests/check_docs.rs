use std::fs;
use std::path::Path;

use forge::check_docs;
use insta::assert_snapshot;
use tempfile::TempDir;

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn clean() -> Self {
        let fixture = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fixture.write(
            "README.md",
            "# Readme\n\nRead [the docs](docs/README.md).\n",
        );
        fixture.write(
            "docs/README.md",
            "# Docs\n\n- [Guide](how-to/guide.md)\n- [Apps](reference/apps.md)\n",
        );
        fixture.write(
            "docs/how-to/guide.md",
            "# Guide\n\n## `world` (opt-in)\n\nSee [the apps](../reference/apps.md#apps).\n",
        );
        fixture.write(
            "docs/reference/apps.md",
            "# Apps\n\n| App | What |\n|---|---|\n| `hello` | says hello |\n",
        );
        fixture.write("modules/flake/docs/pending.txt", "");
        for dir in [
            "modules/apps/hello",
            "modules/features/world",
            "modules/features/base/nix",
            "modules/features/profiles",
            "modules/features/hardware",
        ] {
            fs::create_dir_all(fixture.path(dir)).unwrap();
        }
        fixture
    }

    fn path(&self, relative: &str) -> std::path::PathBuf {
        self.dir.path().join(relative)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap_or(Path::new("."))).unwrap();
        fs::write(path, text).unwrap();
    }

    fn append(&self, relative: &str, text: &str) {
        let mut current = fs::read_to_string(self.path(relative)).unwrap();
        current.push_str(text);
        self.write(relative, &current);
    }

    fn module(&self, dir: &str) {
        fs::create_dir_all(self.path(dir)).unwrap();
    }

    fn report(&self) -> String {
        check_docs::run(self.dir.path()).unwrap().report()
    }
}

#[test]
fn clean_tree_passes() {
    let fixture = Fixture::clean();

    let outcome = check_docs::run(fixture.dir.path()).unwrap();

    assert!(outcome.passed());
    assert_snapshot!(outcome.report());
}

#[test]
fn broken_file_link_is_reported() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\nSee [nothing](missing.md), [a folder](../reference/) and ![a picture](../assets/gone.svg).\n",
    );

    assert_snapshot!(fixture.report());
}

#[test]
fn broken_anchor_is_reported() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\nSee [no section](../reference/apps.md#nowhere), [this page](#gone) and [by reference][ref].\n\nFolders: [no index](../reference#x), [index](../#docs), [index without it](../#nope), [here](./#guide).\n\n[ref]: ../reference/apps.md#also-gone\n",
    );

    assert_snapshot!(fixture.report());
}

#[test]
fn duplicate_headings_get_numbered_anchors() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\n## Same\n\n## Same\n\n<a id=\"custom\"></a>\n\n[first](#same) [second](#same-1) [third](#same-2) [custom](#custom) [upper](#SAME-1)\n\n<div data-id=\"hidden\"></div>\n\n[data](#hidden) <img data-src=\"nothing.png\">\n",
    );

    assert_snapshot!(fixture.report());
}

#[test]
fn links_with_a_scheme_are_skipped() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\n[web](https://example.org/x.md) [mail](mailto:a@b.c) <https://example.org> <a@b.c>\n",
    );

    assert!(check_docs::run(fixture.dir.path()).unwrap().passed());
}

#[test]
fn page_missing_from_the_index_is_an_orphan() {
    let fixture = Fixture::clean();
    fixture.write("docs/how-to/orphan.md", "# Orphan\n");

    assert_snapshot!(fixture.report());
}

#[test]
fn each_forbidden_name_is_reported() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\nalice\nexample-server\nexample-pi\naxelhamil\nnixos-forge\nnixos-starter\nthe `example` host and example hosts, example-disk, deploy/example\nThe Starter\n",
    );

    assert_snapshot!(fixture.report());
}

#[test]
fn allow_marker_fido2_origin_and_maintenance_pages_are_not_reported() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\nalice signs here <!-- docs-check: allow alice -->\nthe origin pam://nixos-forge stays\nexample.org and my-example hosts are fine\n",
    );
    fixture.write(
        "docs/maintenance/notes.md",
        "# Notes\n\nalice and nixos-forge\n",
    );
    fixture.append("docs/README.md", "- [Notes](maintenance/notes.md)\n");

    assert_snapshot!(fixture.report());
}

#[test]
fn allow_marker_only_covers_its_own_name() {
    let fixture = Fixture::clean();
    fixture.append(
        "docs/how-to/guide.md",
        "\nalice and nixos-forge <!-- docs-check: allow alice -->\n",
    );

    assert_snapshot!(fixture.report());
}

#[test]
fn module_without_a_section_is_reported() {
    let fixture = Fixture::clean();
    fixture.module("modules/apps/quiet");
    fixture.module("modules/features/hidden");
    fixture.module("modules/features/hardware/board");
    fixture.module("modules/features/profiles/kiosk");

    assert_snapshot!(fixture.report());
}

#[test]
fn heading_or_first_table_cell_covers_a_module() {
    let fixture = Fixture::clean();
    fixture.module("modules/features/hardware/board");
    fixture.module("modules/features/profiles/kiosk");
    fixture.module("modules/apps/later");
    fixture.append(
        "docs/reference/apps.md",
        "| `board` and more | x |\n| other | `later` only in the second cell |\n\n### The `kiosk` profile\n",
    );

    assert_snapshot!(fixture.report());
}

#[test]
fn pending_entry_hides_its_violation() {
    let fixture = Fixture::clean();
    fixture.write("docs/how-to/orphan.md", "# Orphan\n");
    fixture.write(
        "modules/flake/docs/pending.txt",
        "orphan docs/how-to/orphan.md\n",
    );

    let outcome = check_docs::run(fixture.dir.path()).unwrap();

    assert!(outcome.passed());
    assert_snapshot!(outcome.report());
}

#[test]
fn stale_pending_entry_fails() {
    let fixture = Fixture::clean();
    fixture.write(
        "modules/flake/docs/pending.txt",
        "orphan docs/how-to/fixed.md\n",
    );

    assert_snapshot!(fixture.report());
}
