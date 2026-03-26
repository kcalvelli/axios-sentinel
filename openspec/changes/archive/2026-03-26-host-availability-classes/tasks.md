## 1. Core Library (`sentinel-core`)

- [ ] 1.1 Add `Availability` enum (`AlwaysOn`, `Transient`) with `Default` returning `AlwaysOn`, serde rename to `"always-on"` / `"transient"`
- [ ] 1.2 Add `HostEntry` struct with `name: String` and `availability: Availability`
- [ ] 1.3 Change `FleetConfig.hosts` from `Vec<String>` to `Vec<HostEntry>`
- [ ] 1.4 Update `FleetConfig::from_env` to parse `host:class` suffix format (bare hostname = `AlwaysOn`)
- [ ] 1.5 Add `FleetConfig::host_names()` returning `Vec<&str>` for callers that just need names
- [ ] 1.6 Update `SentinelClient::hosts()` to return `&[HostEntry]`
- [ ] 1.7 Update `SentinelClient::check_fleet_health()` to return `Vec<(HostEntry, Result<...>)>`

## 2. MCP Server (`sentinel-mcp`)

- [ ] 2.1 Update `check_fleet_health` handler: include `"availability"` in per-host output, use `"offline"` for unreachable transient hosts, `"unreachable"` for unreachable always-on hosts
- [ ] 2.2 Update `check_fleet_health` handler: only always-on hosts affect `"overall"` status
- [ ] 2.3 Update `list_hosts` handler: include `"availability"` field in each host entry

## 3. CLI (`sentinel-cli`)

- [ ] 3.1 Update `run_all` Health output: show `"offline"` with `(transient)` note for unreachable transient hosts
- [ ] 3.2 Update `run_all` Health output: show `"ERROR"` for unreachable always-on hosts (preserve existing behavior)
- [ ] 3.3 Update any other `run_all` commands that iterate over hosts to use `HostEntry`

## 4. NixOS Module

- [ ] 4.1 Change `fleet.hosts` type to `listOf (either str (submodule { name, availability }))` with `availability` defaulting to `"always-on"` and enum validation
- [ ] 4.2 Update `SENTINEL_HOSTS` serialization to emit `name:class` for transient hosts and bare names for always-on hosts

## 5. Spec Updates

- [ ] 5.1 Update `openspec/specs/sentinel-mcp/spec.md` with new fleet health scenarios
- [ ] 5.2 Update `openspec/specs/nixos-module/spec.md` with new fleet.hosts type

## 6. Integration (nixos_config — out of tree)

- [ ] 6.1 Update edge's mcp-gateway sentinel server config: `SENTINEL_HOSTS = "edge,mini,pangolin:transient"`
- [ ] 6.2 Update fleet.hosts on all hosts to include pangolin with `availability = "transient"`
- [ ] 6.3 Verify Sid's cron health check correctly ignores transient-offline hosts and escalates always-on failures
