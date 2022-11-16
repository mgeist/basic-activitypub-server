{ pkgs ? import <nixpkgs> {} }:
with pkgs; mkShell rec {
    buildInputs = with pkgs; [
        # general dev
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