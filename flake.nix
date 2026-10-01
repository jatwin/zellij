{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-sqSWJDUxc+zaz1nBWMAJKTAGBuGWP25GCftIOlCEAtA=";
        };
        zellij = pkgs.zellij.unwrapped.overrideAttrs {
          version = "0.46.0";
          src = ./.;
          cargoDeps = pkgs.rustPlatform.fetchCargoVendor {
            src = ./.;
            hash = "sha256-lOwmzZZPjit1Hh7/TFZEuUcqBKdZGtvoKT6u6nNxm+Y=";
          };
          postInstall = ''
            installShellCompletion --cmd zellij \
              --bash <($out/bin/zellij setup --generate-completion bash) \
              --fish <($out/bin/zellij setup --generate-completion fish) \
              --zsh <($out/bin/zellij setup --generate-completion zsh)
          '';
        };
      in
      {
        packages.default = zellij;

        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.gcc
            pkgs.openssl
            pkgs.pkg-config
            pkgs.protobuf
          ];
        };
      }
    );
}
