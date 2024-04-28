{ pkgs, lib, config, inputs, ... }:

{
  packages = [ pkgs.git ];

  scripts.hello.exec = "echo hello from $GREET";

  enterShell = ''
  '';

  enterTest = ''
  '';

  # https://devenv.sh/languages/
  languages.nix.enable = true;
  languages.rust.enable = true;
  languages.rust.channel = "stable";
  languages.rust.targets = [ "wasm32-unknown-unknown" ];

  languages.python.enable = true;
  languages.python.directory = "./server";

  # https://devenv.sh/pre-commit-hooks/
  # pre-commit.hooks.shellcheck.enable = true;

  # https://devenv.sh/processes/
  # processes.ping.exec = "ping example.com";

  # See full reference at https://devenv.sh/reference/options/
  cachix.enable = false;
}
