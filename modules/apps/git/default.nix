{ config, ... }:

let
  inherit (config.helpers) mkDefaults;
in
{
  apps.git.homeManager.programs = mkDefaults {
    git = {
      enable = true;
      settings = {
        init.defaultBranch = "main";
        column.ui = "auto";
        branch.sort = "-committerdate";
        tag.sort = "version:refname";
        fetch.prune = true;
        push = {
          autoSetupRemote = true;
          followTags = true;
        };
        diff = {
          algorithm = "histogram";
          colorMoved = "plain";
        };
        commit.verbose = true;
        merge.conflictStyle = "zdiff3";
        rebase = {
          autoSquash = true;
          autoStash = true;
          updateRefs = true;
        };
        rerere.enabled = true;
        help.autocorrect = "prompt";
      };
    };

    delta = {
      enable = true;
      enableGitIntegration = true;
      options.navigate = true;
    };
  };
}
