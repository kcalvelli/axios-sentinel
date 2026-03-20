## 1. Project Setup

- [ ] 1.1 Set up Cargo workspace with four crates: `sentinel-core`, `sentinel-agent`, `sentinel-cli`, `sentinel-mcp`
- [ ] 1.2 Add shared dependencies: `tokio`, `serde`, `serde_json`, `reqwest`, `axum` (agent), `clap` (cli)
- [ ] 1.3 Update `flake.nix` to build all three binaries from the Cargo workspace

## 2. Core Library (`sentinel-core`)

- [ ] 2.1 Define command vocabulary types: `ReadCommand`, `Tier1Command`, `Tier2Command` enums
- [ ] 2.2 Define response types: `AgentResponse<T>` envelope with `ok`, `data`, `error`, `hostname`, `notify` fields
- [ ] 2.3 Define data types for each read command: `SystemStatus`, `ServiceInfo`, `TemperatureReading`, `DiskUsage`, `GpuStatus`, `HealthCheck`
- [ ] 2.4 Implement HTTP client for talking to sentinel-agents: `SentinelClient` with methods for each command
- [ ] 2.5 Implement host resolution: resolve `<host>.<tailnet>.ts.net` to construct agent URLs
- [ ] 2.6 Implement tier enforcement types: `TierConfig` with `tier1`, `tier2`, `restartableServices`

## 3. Sentinel Agent (`sentinel-agent`)

- [ ] 3.1 Set up axum HTTP server that binds to the Tailscale interface IP on configured port
- [ ] 3.2 Implement read endpoints: `/health`, `/status`, `/services`, `/failed`, `/temperatures`, `/disk`, `/gpu`, `/logs/{unit}`
- [ ] 3.3 Implement `/status` handler: parse `uptime`, `free -b`, `df`, load averages into `SystemStatus`
- [ ] 3.4 Implement `/temperatures` handler: parse `sensors -j` (JSON output) into `TemperatureReading`
- [ ] 3.5 Implement `/services` and `/failed` handlers: parse `systemctl list-units --output=json`
- [ ] 3.6 Implement `/gpu` handler: read from sysfs amdgpu paths (`/sys/class/drm/card*/device/`)
- [ ] 3.7 Implement `/disk` handler: parse `df --output` and `smartctl --json` for health
- [ ] 3.8 Implement `/logs/{unit}` handler: read from `journalctl -u <unit> -n <lines> --output=json`
- [ ] 3.9 Implement `/health` handler: aggregate checks (disk space, failed services, temperatures) into pass/warn/fail
- [ ] 3.10 Implement tier 1 endpoints: `/restart-service`, `/gpu-reset`, `/journal-vacuum` with allowlist enforcement
- [ ] 3.11 Implement tier 2 endpoints: `/reboot`, `/kill-process` with `notify: true` in response
- [ ] 3.12 Add tier enforcement middleware: reject commands above configured tier with HTTP 403
- [ ] 3.13 Load configuration from environment variables or config file: `SENTINEL_PORT`, `SENTINEL_TIER1`, `SENTINEL_TIER2`, `SENTINEL_RESTARTABLE`

## 4. Sentinel CLI (`sentinel-cli`)

- [ ] 4.1 Set up clap CLI with subcommands matching agent command vocabulary: `status`, `health`, `services`, `failed`, `temperatures`, `disk`, `gpu`, `logs`, `restart-service`, `gpu-reset`, `reboot`, `kill-process`, `journal-vacuum`
- [ ] 4.2 Implement `<host> <command>` dispatch using `SentinelClient` from core
- [ ] 4.3 Implement `all <command>` that queries all configured hosts in parallel
- [ ] 4.4 Implement human-readable table output (default) and `--json` flag for raw JSON
- [ ] 4.5 Implement configuration loading: `SENTINEL_DOMAIN`, `SENTINEL_HOSTS`, `SENTINEL_PORT` env vars with `~/.config/sentinel/config.toml` fallback

## 5. Sentinel MCP Server (`sentinel-mcp`)

- [ ] 5.1 Set up MCP stdio server using `mcp-server` Rust crate (or manual JSON-RPC over stdin/stdout)
- [ ] 5.2 Implement `tools/list` response with all sentinel tools: `query_host`, `list_hosts`, `check_fleet_health`, `restart_service`, `reboot_host`, `view_logs`, `system_status`, `host_temperatures`, `host_disk`, `host_gpu`
- [ ] 5.3 Implement `query_host` tool: aggregate `/status`, `/failed`, `/temperatures`, `/disk` from a single host
- [ ] 5.4 Implement `check_fleet_health` tool: parallel `/health` queries across all hosts
- [ ] 5.5 Implement `restart_service` and `reboot_host` tools: proxy to agent with error handling
- [ ] 5.6 Implement `view_logs`, `system_status`, `host_temperatures`, `host_disk`, `host_gpu` tools: proxy to individual agent endpoints
- [ ] 5.7 Implement `list_hosts` tool: return configured hosts with connectivity status
- [ ] 5.8 Load configuration from environment: `SENTINEL_HOSTS`, `SENTINEL_DOMAIN`, `SENTINEL_PORT`

## 6. NixOS Module

- [ ] 6.1 Implement `services.sentinel.agent` options: `enable`, `port`, `tier1`, `tier2`, `restartableServices`
- [ ] 6.2 Create systemd service unit for sentinel-agent with security hardening (`ProtectSystem`, `ProtectHome`, `PrivateTmp`, dedicated user)
- [ ] 6.3 Configure polkit rules for the `sentinel` user to restart allowlisted services
- [ ] 6.4 Open firewall port on `tailscale0` interface when agent is enabled
- [ ] 6.5 Create `sentinel` system user and group via `users.users.sentinel`

## 7. Integration (nixos_config)

- [ ] 7.1 Add `inputs.sentinel` to `nixos_config/flake.nix`
- [ ] 7.2 Enable `services.sentinel.agent` on edge, mini, and pangolin with appropriate `restartableServices` lists
- [ ] 7.3 Register `sentinel-mcp` on edge's mcp-gateway server config with host/domain/port env vars
- [ ] 7.4 Add `sentinel-cli` to Sid's PATH via `services.sid.extraPackages` on mini
- [ ] 7.5 Configure Sid cron health check: `sentinel-cli all health` every 5 minutes via zeroclaw cron
