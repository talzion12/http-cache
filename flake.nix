{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-22.05";
    utils.url = "github:numtide/flake-utils";
    # naersk.url = "github:nmattia/naersk";
  };
  outputs = {
      self,
      nixpkgs,
      utils,
      # naersk,
    }:
    utils.lib.eachDefaultSystem (system:
      let 
        pkgs = nixpkgs.legacyPackages.${system};
        # naersk-lib = naersk.lib."${system}";
      in rec {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "scraper";
          version = "0.1.0";

          src = pkgs.stdenv.mkDerivation {
            name = "scraper-source";
            src = pkgs.lib.sourceByRegex ./. [
              "^src.*$"
              "^Cargo\.toml$"
              "^Cargo\.lock$"
              "^build\.rs$"
              "^queries\.graphql$"
            ];

            installPhase = ''
              mkdir $out
              cp -r $src/* $out
              cp -r ${../graphql-schema} $out/graphql-schema
              ls -l $out/src
            '';
          };
          doCheck = false;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };

        # defaultPackage = naersk-lib.buildPackage {
        #   pname = "scraper";
        #   version = "0.1.0";
        #   root = src-with-graphql-schema;
        # };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustup
            htmlq
            html-tidy
            protobuf
            pkg-config
            openssl
            nushell
            # rust-analyzer
            #  cargo
            #  cargo-outdated
            #  rustc
          ];
        };
        packages.docker = pkgs.dockerTools.buildImage {
          name = "gcr.io/wecare-190609/scraper";
          contents = defaultPackage;
          config = {
            Cmd = [ "${defaultPackage}/bin/wecare_scraper" ];
          };
        };
      }
    );
}
