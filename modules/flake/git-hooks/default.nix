{ inputs, ... }:

{
  imports = [ inputs.git-hooks.flakeModule ];

  perSystem = {
    pre-commit = {
      check.enable = false;

      settings = {
        default_stages = [ "pre-push" ];

        hooks = {
          treefmt.enable = true;

          flake-eval = {
            enable = true;
            name = "nix flake check --no-build";
            entry = "nix flake check --no-build";
            pass_filenames = false;
            always_run = true;
          };

          actionlint.enable = true;
        };
      };
    };
  };
}
