{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell rec {
    buildInputs = with pkgs; [
        # general dev
        curl
        git

        # rust stuff
        cargo
        clippy
        rust-analyzer
        rustc
        rustfmt

        # deployment stuff
        flyctl
    ];
}