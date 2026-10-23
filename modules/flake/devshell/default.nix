{
  perSystem =
    { config, pkgs, ... }:
    {
      devShells.default = pkgs.mkShell {
        inputsFrom = [ config.pre-commit.devShell ];
        packages = [
          pkgs.sops
          pkgs.age
          pkgs.cargo
          pkgs.rustc
          pkgs.clippy
          pkgs.rustfmt
          pkgs.rust-analyzer
          pkgs.cargo-nextest
        ];
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
    };
}
