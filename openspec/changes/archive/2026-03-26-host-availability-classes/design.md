## Context

Sentinel treats all configured hosts identically. When `check_fleet_health` runs and any host is unreachable, the overall status is `fail`. This creates two problems: pangolin (a laptop) triggers false-positive failures when it's simply turned off, and edge crash detection gets lost in the noise of expected-offline hosts.

The fleet needs host-level metadata so the system can distinguish between "server down" (a problem) and "laptop off" (normal).

## Goals / Non-Goals

**Goals:**
- Fleet health reporting correctly handles hosts that are expected to be offline sometimes
- Overall fleet health status reflects only always-on hosts
- Transient hosts still appear in reports but are clearly labeled and don't trigger failure
- Backward-compatible configuration — existing deployments work without changes

**Non-Goals:**
- No criticality tiers (high/medium/low) — availability class is sufficient for now
- No changes to sentinel-agent — agents don't know about fleet topology
- No changes to how individual host health checks work (thresholds, checks, etc.)
- No proactive alerting changes — Sid's cron is out of scope

## Decisions

### Decision 1: Two availability classes, not three

**Choice:** `always-on` and `transient`. No intermediate "best-effort" or "business-hours" class.

**Alternatives considered:**
- *Three-tier model (always-on / business-hours / transient)*: Could handle hosts that should be on during work hours. But this requires time-of-day logic and timezone handling — unnecessary complexity for a 3-host homelab.
- *Freeform labels*: Maximum flexibility, but then every consumer needs to decide what each label means.

**Rationale:** The problem is binary: either we expect the host to be reachable (and should alert when it's not) or we don't. Two classes maps directly to this.

### Decision 2: Parse availability from SENTINEL_HOSTS with colon suffix

**Choice:** `SENTINEL_HOSTS=edge,mini,pangolin:transient` — the availability class is appended to the hostname with a colon delimiter.

**Alternatives considered:**
- *Separate environment variable (`SENTINEL_HOST_CLASSES=pangolin:transient`)*: Must be kept in sync with SENTINEL_HOSTS. Easy to add a host to one and forget the other.
- *JSON format (`SENTINEL_HOSTS=[{"name":"edge"},{"name":"pangolin","availability":"transient"}]`)*: Verbose and ugly in NixOS environment blocks.
- *Config file only*: Would require implementing the `~/.config/sentinel/config.toml` fallback that doesn't exist yet.

**Rationale:** Colon suffix keeps host + metadata together in one place. It's compact, backward-compatible (no colon = `always-on`), and trivial to parse. Colons can't appear in hostnames, so there's no ambiguity.

### Decision 3: `HostEntry` struct in FleetConfig

**Choice:** Replace `hosts: Vec<String>` with `hosts: Vec<HostEntry>` where `HostEntry` has `name: String` and `availability: Availability`. The `Availability` enum has variants `AlwaysOn` and `Transient`.

**Rationale:** Typed representation prevents stringly-typed bugs. The `HostEntry` struct is extensible if we later add criticality or other per-host metadata. The `FleetConfig::from_env` parser handles both the old format (bare hostnames) and the new format (with colon suffix).

### Decision 4: "offline" vs "unreachable" status label

**Choice:** Unreachable transient hosts are labeled `"offline"`. Unreachable always-on hosts are labeled `"unreachable"`. Both mean "couldn't reach the agent" — the label conveys intent.

**Rationale:** When Sid reads the fleet health output, the label immediately signals whether action is needed. `"unreachable"` implies a problem; `"offline"` implies expected state. This is more useful than a single `"unreachable"` label with a separate availability field that Sid would need to cross-reference.

### Decision 5: NixOS module supports both string and attrset host entries

**Choice:** `fleet.hosts` accepts a mixed list:
```nix
fleet.hosts = [
  "edge"                                      # string → always-on
  "mini"                                       # string → always-on
  { name = "pangolin"; availability = "transient"; }  # attrset → transient
];
```

The module converts both forms to the `name:availability` environment variable format.

**Rationale:** Plain strings keep simple configs simple. Attribute sets are available when you need per-host metadata. NixOS's type system (`either str attrs`) handles validation.

## Implementation Notes

### Core library changes (`sentinel-core`)

**`config.rs`:**
- Add `Availability` enum (`AlwaysOn`, `Transient`) with `Default` impl returning `AlwaysOn`
- Add `HostEntry` struct with `name: String` and `availability: Availability`
- Change `FleetConfig.hosts` from `Vec<String>` to `Vec<HostEntry>`
- Update `FleetConfig::from_env` to parse `host:class` format
- Add `FleetConfig::host_names()` convenience method returning `Vec<&str>`

**`client.rs`:**
- `SentinelClient::hosts()` returns `&[HostEntry]` instead of `&[String]`
- `SentinelClient::check_fleet_health()` returns `Vec<(HostEntry, Result<...>)>` instead of `Vec<(String, Result<...>)>`

### MCP changes (`sentinel-mcp`)

**`mcp.rs` — `check_fleet_health`:**
- Include `"availability"` field in per-host output
- Use `"offline"` status for unreachable transient hosts
- Use `"unreachable"` status for unreachable always-on hosts
- Only always-on host statuses affect the `"overall"` field

**`mcp.rs` — `list_hosts`:**
- Include `"availability"` field in each host entry

### CLI changes (`sentinel-cli`)

**`main.rs` — `run_all` for `Health`:**
- Print `"offline"` with note `(transient)` for unreachable transient hosts
- Print `"ERROR"` for unreachable always-on hosts (existing behavior)

### NixOS module changes

**`default.nix`:**
- `fleet.hosts` type changes to `listOf (either str (submodule { name, availability }))`
- Serialization to `SENTINEL_HOSTS` env var handles both forms
- Add `fleet.hosts.*.availability` option with default `"always-on"` and enum validation

## Risks / Trade-offs

**[Risk] Changing `FleetConfig.hosts` type is a breaking API change** — Any code that indexes into `hosts` expecting strings will need updating. Mitigation: only three consumers (agent doesn't use fleet config, CLI and MCP do). The change is mechanical and caught at compile time.

**[Risk] NixOS module type change could break existing configs** — Mitigation: plain strings are still accepted via `either str attrs`, so `fleet.hosts = [ "edge" "mini" ];` continues to work unchanged.

## Open Questions

1. **Should `query_host` for a transient host that's offline return an error or a structured "offline" response?** Currently it returns a connection error. It could instead check the host's availability class first and return `{"status": "offline", "availability": "transient"}` — more informative for Sid.
