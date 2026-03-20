{
  description = "Autonomous system operations and monitoring for NixOS hosts over Tailscale";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    {
      nixosModules.default = import ./modules/nixos;

      # TODO: packages for sentinel-agent, sentinel-cli, sentinel-mcp
      # packages.x86_64-linux = { ... };
    };
}
