{
  description = "Autonomous system operations and monitoring for NixOS hosts over Tailscale";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          buildSentinelBin =
            name:
            pkgs.rustPlatform.buildRustPackage {
              pname = name;
              version = "0.1.0";
              src = ./.;

              cargoLock.lockFile = ./Cargo.lock;

              cargoBuildFlags = [
                "--package"
                name
              ];

              nativeBuildInputs = with pkgs; [ pkg-config ];
              buildInputs = with pkgs; [ openssl ];

              doCheck = false;

              meta = {
                description = "cairn-sentinel ${name}";
                license = pkgs.lib.licenses.mit;
                mainProgram = name;
              };
            };
        in
        {
          sentinel-agent = buildSentinelBin "sentinel-agent";
          sentinel-cli = buildSentinelBin "sentinel-cli";
          sentinel-mcp = buildSentinelBin "sentinel-mcp";
          default = self.packages.${system}.sentinel-cli;
        }
      );

      nixosModules.default = import ./modules/nixos;
    };
}
