# Proposal: cairn-sentinel — Autonomous System Operations for Sid

## Summary

Give the Sid AI assistant the ability to monitor, diagnose, and fix NixOS hosts across a Tailscale network. Three components share a core library: a per-host agent daemon with a fixed command vocabulary, an MCP server for rich queries through the existing mcp-gateway, and a CLI for emergency operations when the gateway is down.

## Motivation

### The Problem

On March 20, 2026, the `edge` desktop suffered a GPU hang that killed the Wayland compositor and network services. The system was unresponsive for nearly two hours until a manual hard reboot. Sid (the AI assistant running on `mini`) had no way to detect the problem, diagnose it, or take corrective action. All of Sid's MCP tools route through edge's mcp-gateway — so when edge went down, Sid lost access to everything.

This is a recurring problem. The system has logged 63+ unclean crashes over 8 months. Each one requires manual intervention: noticing the system is down, physically accessing it (or SSHing in if the network stack survived), and rebooting.

### The Vision

Sid should be able to:
1. **Detect** — notice when a host is unhealthy (via scheduled health checks)
2. **Diagnose** — query system state (temps, memory, disk, services, GPU, logs)
3. **Act** — restart failed services, reset the GPU, or reboot the host
4. **Report** — notify the user what happened and what was done about it

When the user is asleep at 3am, Sid should handle tier 1/2 issues autonomously and leave a summary for morning. When the user is awake, Sid should notify and let them decide on anything destructive.

### Why Two Paths

Sid's current tool access goes through `mcp-gw` CLI → edge's mcp-gateway HTTP API. If edge is the host that's down, this path is broken. The architecture needs a fallback:

- **Normal path**: sentinel-mcp registered on edge's mcp-gateway. Rich queries, structured responses, full monitoring. Works when everything is healthy.
- **Emergency path**: sentinel-cli invoked via Sid's native shell tool. Direct HTTP call to the target host's agent over Tailscale. No middleware, no gateway dependency. Works when the gateway is down.

The failure mode uses the simple path, not the complex one. That's deliberate.

## Architecture

```
┌─ Normal Operations ──────────────────────────────────┐
│                                                       │
│  Sid (mini)                                           │
│    └── mcp-gw → edge mcp-gateway                     │
│                   └── sentinel-mcp (stdio server)     │
│                         ├── GET edge/status            │
│                         ├── GET mini/status            │
│                         └── GET pangolin/status        │
│                              │                        │
│                    ┌─────────┼─────────┐              │
│                    ▼         ▼         ▼              │
│              edge:agent  mini:agent  pangolin:agent   │
│                                                       │
└───────────────────────────────────────────────────────┘

┌─ Emergency Operations ───────────────────────────────┐
│                                                       │
│  Sid (mini)                                           │
│    └── shell: sentinel-cli reboot edge                │
│                    │                                  │
│                    ▼ (direct HTTP over Tailscale)     │
│              edge:agent → systemctl reboot            │
│                                                       │
└───────────────────────────────────────────────────────┘
```

### Components

**sentinel-agent** (runs on every host)
- Lightweight HTTP server bound to Tailscale interface only
- Fixed command vocabulary — no arbitrary command execution
- Tiered operations with per-host configuration
- Authentication via Tailscale node identity (peer IP verification)
- Responds to health checks, returns structured JSON

**sentinel-mcp** (runs on edge, registered with mcp-gateway)
- MCP server exposing monitoring and operations tools
- Aggregates data from all sentinel-agents across the fleet
- Tools: `query_host`, `list_hosts`, `check_health`, `restart_service`, `reboot_host`, `view_logs`, `system_status`, etc.
- Shares core library with sentinel-cli

**sentinel-cli** (available in Sid's PATH on mini)
- Direct command-line interface to sentinel-agents
- No gateway dependency — talks directly to agents over Tailscale
- Same core library as sentinel-mcp
- Usage: `sentinel-cli <host> <command> [args]`
- Examples:
  - `sentinel-cli edge status`
  - `sentinel-cli edge restart-service niri`
  - `sentinel-cli edge reboot`
  - `sentinel-cli all health` (check all hosts)

**Shared core library** (`src/core/`)
- HTTP client for talking to sentinel-agents
- Command vocabulary definitions
- Tier enforcement logic
- Host discovery (Tailscale DNS resolution)
- Response types and serialization

## Command Vocabulary

### Read Operations (always allowed)

| Command | Description | Returns |
|---------|-------------|---------|
| `status` | System overview | uptime, load, memory, swap, disk |
| `services` | Service status | list of running/failed/inactive units |
| `failed` | Failed services | systemctl --failed output |
| `temperatures` | Hardware temps | CPU, GPU, NVMe, board sensors |
| `disk` | Disk usage | df + SMART health summary |
| `gpu` | GPU status | amdgpu info, VRAM usage, clocks |
| `logs <unit> [lines]` | Journal tail | last N lines of a systemd unit |
| `health` | Health check | pass/warn/fail with reasons |

### Tier 1 — Autonomous (safe, reversible)

| Command | Description | Safeguards |
|---------|-------------|------------|
| `restart-service <unit>` | Restart a systemd service | Only user-defined allowlist |
| `gpu-reset` | Trigger amdgpu recovery | Only on hosts with AMD GPU |
| `journal-vacuum <size>` | Reclaim journal disk space | Minimum retention enforced |

### Tier 2 — Autonomous + Notify (disruptive but not destructive)

| Command | Description | Safeguards |
|---------|-------------|------------|
| `reboot` | Reboot the host | Notification sent via Pushover |
| `kill-process <pid>` | Kill a runaway process | Must specify PID, no patterns |

### Tier 3 — Future (approval required, not in v1)

Reserved for data-affecting operations: ZFS operations, nix-collect-garbage, multi-host coordinated actions. Not implemented in initial version.

## NixOS Module API

```nix
# Every host — enable the agent:
services.sentinel.agent = {
  enable = true;

  # Which tiers this host allows (read is always on)
  tier1 = true;   # restart-service, gpu-reset, journal-vacuum
  tier2 = true;   # reboot, kill-process

  # Service allowlist for restart-service (tier 1)
  # Only these units can be restarted remotely
  restartableServices = [
    "niri"
    "greetd"
    "wyoming-satellite"
    "wyoming-openwakeword"
    "immich"
    "ollama"
  ];

  # Port for the agent HTTP API (bound to tailscale0 only)
  port = 9256;
};
```

```nix
# nixos_config edge.nix — register sentinel-mcp on the gateway:
services.mcp-gateway.servers.sentinel = {
  enable = true;
  command = "${inputs.sentinel.packages.x86_64-linux.sentinel-mcp}/bin/sentinel-mcp";
  env.SENTINEL_HOSTS = "edge,mini,pangolin";
  env.SENTINEL_DOMAIN = "taile0fb4.ts.net";
  env.SENTINEL_PORT = "9256";
};
```

```nix
# nixos_config mini.nix — add CLI to Sid's PATH:
services.sid = {
  # ... existing config ...
  extraPackages = [
    inputs.sentinel.packages.x86_64-linux.sentinel-cli
  ];
};
```

## Integration with Sid's Existing Capabilities

**Scheduled health checks** — Sid uses zeroclaw's built-in cron to run `sentinel-cli all health` every 5 minutes. If any host returns warn/fail, Sid investigates using the MCP tools (if available) or CLI (if not).

**Pushover notifications** — Tier 2 actions trigger Pushover notifications via Sid's existing pushover tool. "Edge was unresponsive, rebooted at 07:10, services recovered by 07:11."

**Home Assistant correlation** — Sid can cross-reference sentinel data with HA data. "Edge GPU temp spiked to 95°C, and the office AC was off" (via existing ha-mcp).

**Morning reports** — Sid can summarize overnight events: "Edge had 2 service restarts and 1 reboot overnight. All hosts currently healthy."

## Scope

### In Scope (v1)
- sentinel-agent daemon with full read + tier 1 + tier 2 command vocabulary
- sentinel-cli for direct agent access
- sentinel-mcp for mcp-gateway integration
- NixOS module for agent configuration
- Nix flake packaging for all three binaries
- Tailscale-only binding and peer identity verification

### Out of Scope (v1)
- Prometheus / time-series metrics (future: optional `services.sentinel.prometheus.enable`)
- Tier 3 approval-gated operations
- Web dashboard / UI
- Alerting rules engine (Sid IS the alerting engine)
- Windows/macOS host support
- Multi-tailnet support

## Non-goals

- **Not a general-purpose RPC system** — fixed vocabulary, not arbitrary execution
- **Not a replacement for SSH** — SSH is for humans, sentinel is for Sid
- **Not a monitoring SaaS** — no external dependencies, runs entirely on-tailnet
- **Not a config management tool** — doesn't deploy, doesn't modify NixOS config

## Success Criteria

1. `sentinel-cli edge status` returns structured system info over Tailscale
2. `sentinel-cli edge reboot` reboots edge and sends Pushover notification
3. sentinel-mcp registered on mcp-gateway, Sid can query host status via MCP tools
4. Sid's cron health check detects a downed service and autonomously restarts it
5. When edge's mcp-gateway is unreachable, Sid falls back to sentinel-cli and can still operate
6. Agent rejects commands not in the configured tier/allowlist
