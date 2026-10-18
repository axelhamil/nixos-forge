assert "delta" in as_tester("git config --global --get pager.diff"), "git does not page through delta"
assert as_tester("git config --global --get delta.navigate").strip() == "true"
