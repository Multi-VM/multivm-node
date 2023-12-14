{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; config.allowUnfree = true; };
      in
      {
        devShell = pkgs.mkShell
          rec {
            buildInputs =
              [
                pkgs.rustup
                pkgs.protobuf
                pkgs.openssl
                pkgs.pkg-config
                pkgs.linuxPackages.nvidia_x11
                pkgs.cudatoolkit
                pkgs.nodejs_20
              ];
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
            CUDA_LIBRARY_PATH = pkgs.cudatoolkit;
            shellHook = ''
              export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
              export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/stable-x86_64-unknown-linux-gnu/bin/
            '';
          };
      });
}
