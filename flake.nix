{
  inputs = {
    utils.url = "github:numtide/flake-utils";
  };
  outputs =
    { self
    , nixpkgs
    , utils
    ,
    }:
    utils.lib.eachDefaultSystem (system:
    let
      pkgs = nixpkgs.legacyPackages.${system};
      nodejs = pkgs.nodejs_20;
    in
    rec {
      defaultPackage = pkgs.rustPlatform.buildRustPackage {
        pname = "http-cache";
        version = "0.1.0";

        src = pkgs.lib.sourceByRegex ./. [
          "^src.*$"
          "^Cargo\.toml$"
          "^Cargo\.lock$"
        ];

        cargoLock = {
          lockFile = ./Cargo.lock;
        };
      };

      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          kubernetes-helm
          nodejs
          rustup
        ];
      };
    }
    );
}
