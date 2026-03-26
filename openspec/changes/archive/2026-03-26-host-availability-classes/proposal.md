# Proposal: Host Availability Classes

## Summary

Add host availability classification (`always-on` vs `transient`) to the fleet configuration so that sentinel correctly distinguishes between a server being down (a problem) and a laptop being off (normal). Update fleet health reporting to account for host class when determining overall fleet health and per-host status.

## Motivation

### The Problem

Two defects exposed on the same day:

1. **False positive**: Pangolin (a laptop) appeared in sentinel reports as unreachable/failed. Laptops are off most of the time — this is expected behavior, not a failure. Reporting it as a fleet health failure creates noise that erodes trust in the monitoring system.

2. **Missed failure**: Edge suffered a crash around noon. Nothing was reported. The sentinel system is entirely on-demand — it only detects problems when Sid actively polls via the cron health check. But the fleet health output doesn't distinguish between "this host is critical and being down is an emergency" vs "this host is a laptop and being offline is fine." Even when the cron fires, the signal (edge down) gets lost in the noise (pangolin also "down").

### Root Cause

The fleet configuration treats all hosts identically: `hosts = ["edge", "mini", "pangolin"]`. There is no metadata about whether a host is expected to be always reachable or only sometimes reachable. When `check_fleet_health` runs:

- An unreachable always-on server (edge, mini) should be `fail` — something is wrong.
- An unreachable transient host (pangolin) should be `offline` — this is expected and should not affect the overall fleet health status.

Without this distinction, Sid cannot tell which unreachable hosts actually need attention.

### Why This Matters for Edge Detection

The original March 20 incident showed that edge going down is critical — it takes out Sid's entire MCP tool chain. The 5-minute cron health check (`sentinel-cli all health`) is the detection mechanism for this exact scenario. But if the health check reports `overall: fail` every time pangolin's lid is closed, Sid either:
- Ignores the output (desensitized to false positives) and misses real edge failures
- Investigates every time (wastes cycles on a laptop being a laptop)

Neither outcome is acceptable.

## Design

### Host Availability Classes

Two classes, configured per-host in the fleet config:

| Class | Meaning | When Unreachable |
|-------|---------|------------------|
| `always-on` | Expected to be reachable 24/7 (servers, desktops) | Report as `unreachable` (fail), affects overall fleet health |
| `transient` | May be offline at any time (laptops, mobile devices) | Report as `offline` (informational), does NOT affect overall fleet health |

Default: `always-on` (safe default — if you add a host and forget to classify it, you'll get alerted if it goes down).

### Configuration Changes

**Environment variable** — `SENTINEL_HOSTS` gains an optional per-host suffix:

```
# Current format (all always-on, backward compatible):
SENTINEL_HOSTS=edge,mini

# New format with availability class:
SENTINEL_HOSTS=edge,mini,pangolin:transient
```

Hosts without a suffix default to `always-on`. This is backward-compatible: existing configs that don't use the suffix continue to work identically.

**NixOS module** — `fleet.hosts` changes from a list of strings to a list of attribute sets:

```nix
fleet.hosts = [
  { name = "edge"; }           # always-on (default)
  { name = "mini"; }           # always-on (default)
  { name = "pangolin"; availability = "transient"; }
];
```

For backward compatibility, plain strings are also accepted and treated as `always-on`:

```nix
fleet.hosts = [ "edge" "mini" ];  # still works, all always-on
```

### Reporting Changes

**`check_fleet_health` (MCP + CLI)** — The overall status only considers `always-on` hosts. Transient hosts are reported with their status but don't contribute to `overall`:

```json
{
  "overall": "pass",
  "hosts": {
    "edge": {"status": "pass", "availability": "always-on", "checks": [...]},
    "mini": {"status": "pass", "availability": "always-on", "checks": [...]},
    "pangolin": {"status": "offline", "availability": "transient"}
  }
}
```

If edge goes down:
```json
{
  "overall": "fail",
  "hosts": {
    "edge": {"status": "unreachable", "availability": "always-on", "error": "..."},
    "mini": {"status": "pass", "availability": "always-on", "checks": [...]},
    "pangolin": {"status": "offline", "availability": "transient"}
  }
}
```

Key distinction: unreachable always-on hosts report `"status": "unreachable"` (failure). Unreachable transient hosts report `"status": "offline"` (informational).

**`list_hosts` (MCP)** — Includes availability class in output:

```json
[
  {"host": "edge", "reachable": true, "availability": "always-on"},
  {"host": "mini", "reachable": true, "availability": "always-on"},
  {"host": "pangolin", "reachable": false, "availability": "transient"}
]
```

**`sentinel-cli all health`** — Human-readable output distinguishes availability:

```
HOST            STATUS   DETAILS
------------------------------------------------------------
edge            pass     disk: ok; services: ok; memory: ok
mini            pass     disk: ok; services: ok; memory: ok
pangolin        offline  (transient — not expected to be always reachable)
```

## Scope

### In Scope
- `FleetConfig` changes: host availability class parsing from env var and struct
- `SentinelClient::check_fleet_health` returns availability metadata
- MCP `check_fleet_health` and `list_hosts` output include availability, overall status respects class
- CLI `all health` output distinguishes transient offline vs always-on unreachable
- NixOS module `fleet.hosts` option supports availability class
- Update specs for sentinel-mcp and nixos-module

### Out of Scope
- Proactive alerting or push notifications (Sid's cron is the alerting mechanism)
- Per-host health check thresholds
- Scheduled wake-on-LAN for transient hosts
- Changes to sentinel-agent (agent doesn't know about fleet topology)

## Risks / Trade-offs

**[Trade-off] Environment variable syntax is slightly more complex** — The `host:class` suffix format is a minor increase in complexity. But it's backward-compatible (no suffix = always-on) and avoids needing a separate `SENTINEL_HOST_CLASSES` variable that would be error-prone to keep in sync.

**[Risk] Transient hosts mask real problems** — If a host is miscategorized as `transient`, its failures will be suppressed. Mitigation: default is `always-on`, so you have to explicitly opt in to suppressed reporting. The host still appears in output — just doesn't affect `overall`.

**[Trade-off] No separate config for "critical" hosts** — Edge is more critical than mini (it hosts the mcp-gateway), but this proposal doesn't add a criticality tier. The availability class solves the immediate problem; criticality can be layered on later if needed.
