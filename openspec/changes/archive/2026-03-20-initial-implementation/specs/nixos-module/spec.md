## ADDED Requirements

### Requirement: NixOS module configures the sentinel-agent daemon
The `services.sentinel.agent` NixOS option SHALL configure and run the sentinel-agent as a systemd service.

#### Scenario: Enable agent with defaults
- **WHEN** `services.sentinel.agent.enable = true` is set in NixOS config
- **THEN** a systemd service `sentinel-agent` is created, bound to the Tailscale interface on port 9256, with tier 1 and tier 2 enabled by default and an empty `restartableServices` allowlist

#### Scenario: Disable tier 2 on a host
- **WHEN** `services.sentinel.agent = { enable = true; tier2 = false; }` is set
- **THEN** the agent starts but rejects all tier 2 commands (reboot, kill-process)

#### Scenario: Configure restartable services
- **WHEN** `services.sentinel.agent.restartableServices = [ "niri" "ollama" ]` is set
- **THEN** the agent allows restarting only `niri` and `ollama` units, rejecting all others

#### Scenario: Custom port
- **WHEN** `services.sentinel.agent.port = 9300` is set
- **THEN** the agent listens on port 9300 instead of the default 9256

### Requirement: Agent systemd service has appropriate security hardening
The sentinel-agent systemd service SHALL run with minimal privileges using systemd security features.

#### Scenario: Service runs as dedicated user
- **WHEN** the sentinel-agent service starts
- **THEN** it runs as a dedicated `sentinel` system user, not root

#### Scenario: Service has required capabilities
- **WHEN** the sentinel-agent service starts with tier 2 enabled
- **THEN** the systemd unit has `AmbientCapabilities` for `CAP_SYS_BOOT` (reboot) and the `sentinel` user has polkit rules allowing `systemctl restart` for allowlisted services

#### Scenario: Service has filesystem restrictions
- **WHEN** the sentinel-agent service starts
- **THEN** the systemd unit uses `ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, and only has write access to its own state directory

### Requirement: NixOS module opens firewall on Tailscale interface
The NixOS module SHALL open the agent's port on the Tailscale interface only.

#### Scenario: Firewall configured
- **WHEN** `services.sentinel.agent.enable = true` with default port 9256
- **THEN** `networking.firewall.interfaces."tailscale0".allowedTCPPorts` includes 9256

### Requirement: Flake exports packages and NixOS module
The cairn-sentinel flake SHALL export `nixosModules.default`, `packages.x86_64-linux.sentinel-agent`, `packages.x86_64-linux.sentinel-cli`, and `packages.x86_64-linux.sentinel-mcp`.

#### Scenario: Import as flake input
- **WHEN** `inputs.sentinel.url = "github:kcalvelli/cairn-sentinel"` is added to nixos_config's flake.nix
- **THEN** `inputs.sentinel.nixosModules.default` can be imported, and `inputs.sentinel.packages.x86_64-linux.sentinel-cli` is available for adding to Sid's PATH

#### Scenario: Register sentinel-mcp on mcp-gateway
- **WHEN** the sentinel-mcp package is referenced in edge's mcp-gateway server config
- **THEN** `sentinel-mcp` runs as a stdio MCP server under the gateway, with host/domain/port configured via environment variables
