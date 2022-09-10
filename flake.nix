{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs = {
      self,
      nixpkgs,
      utils,
    }:
    utils.lib.eachDefaultSystem (system:
      let 
        pkgs = nixpkgs.legacyPackages.${system};
      in rec {
        defaultPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "http-cache";
          version = "0.1.0";

          src = pkgs.lib.sourceByRegex ./. [
            "^src.*$"
            "^Cargo\.toml$"
            "^Cargo\.lock$"
            "^build\.rs$"
            "^queries\.graphql$"
          ];

          cargoLock = {
            lockFile = ./Cargo.lock;
          };
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustup
          ];
        };
        packages.docker = pkgs.dockerTools.buildImage {
          name = "gcr.io/wecare-190609/http-cache";
          contents = defaultPackage;
          config = {
            Cmd = [ "${defaultPackage}/bin/http-cache" ];
          };
        };
      }
    );
}
