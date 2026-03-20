## Context

The axios homelab runs three NixOS hosts (edge, mini, pangolin) connected via Tailscale. The Sid AI assistant runs on mini as a ZeroClaw daemon. Today, Sid has no visibility into host health and no ability to take corrective action when systems fail.

The March 20, 2026 incident demonstrated the gap: edge's GPU hung, killing the compositor and network services. The system sat unresponsive for ~2 hours until manual intervention. Sid had no way to detect, diagnose, or fix the problem — and couldn't even reach its own tools since they route through edge's mcp-gateway.

**Current tool architecture:**
- Sid's native built-in tools: shell, file ops, cron, memory, XMPP, pushover (compiled into ZeroClaw)
- External MCP tools: `mcp-gw` CLI → edge's mcp-gateway HTTP API → stdio MCP servers (github, ai-mail, dav, brave, HA, zeroclaw-mcp)
- ZeroClaw has no native MCP client — all external tools go through `mcp-gw`

**Constraints:**
- Agent must be a standalone flake (like sid, mcp-gateway, axios-dav)
- Imported into nixos_config alongside axios
- No changes to ZeroClaw's MCP handling (Sid uses shell for the emergency path)
- Tailscale provides authentication — no application-level auth

## Goals / Non-Goals

**Goals:**
- Sid can autonomously detect and remediate common system failures (service crashes, GPU hangs, unresponsive hosts)
- Emergency operations work even when edge's mcp-gateway is down
- Fixed command vocabulary — the agent never executes arbitrary commands
- Per-host configuration of which operations are permitted
- NixOS module with clean declarative API, consistent with axios patterns

**Non-Goals:**
- No Prometheus or time-series metrics in v1 (point-in-time queries only)
- No web dashboard or UI
- No tier 3 (approval-gated) operations in v1
- No changes to ZeroClaw's core MCP handling
- Not a general-purpose remote execution framework
- No multi-tailnet or cross-network support

## Decisions

### Decision 1: Two access paths — MCP for comfort, CLI for emergencies

**Choice:** sentinel-mcp (registered on edge's mcp-gateway) for rich monitoring queries, plus sentinel-cli (in Sid's PATH) for direct agent access when the gateway is down.

**Alternatives considered:**
- *Second mcp-gateway on mini*: Would give Sid a local tool path, but adds operational complexity (two gateways to maintain) for a problem the CLI solves more simply.
- *Native MCP client in ZeroClaw*: Correct long-term but requires implementing the MCP stdio transport in ZeroClaw — significant effort for marginal benefit.
- *Everything through edge's gateway*: Single point of failure. When edge is down, Sid loses all ops capability — exactly when it's needed most.

**Rationale:** The failure mode should use the simpler path. When things go wrong, Sid drops to a direct CLI call over Tailscale — no middleware, no schema negotiation. The MCP path is for the comfortable case where everything is healthy and Sid wants rich, structured queries.

### Decision 2: Rust for all components

**Choice:** Rust for sentinel-agent, sentinel-cli, and sentinel-mcp, with a shared `sentinel-core` library.

**Alternatives considered:**
- *Go*: Simpler for HTTP servers, but adds a second language to the ecosystem (sid/zeroclaw are Rust).
- *Python*: Easy to prototype but wrong choice for a security-sensitive daemon that needs to be fast and reliable.
- *Shell scripts behind a thin HTTP wrapper*: Tempting for simplicity but harder to test, audit, and enforce the command vocabulary.

**Rationale:** The Rust ecosystem is already established (ZeroClaw, zeroclaw-mcp). Shared Cargo workspace means one build system. The agent is small enough that Rust's complexity overhead is minimal. The type system helps enforce the fixed command vocabulary at compile time.

### Decision 3: HTTP API on Tailscale interface only

**Choice:** sentinel-agent exposes a JSON HTTP API bound exclusively to the Tailscale interface (`tailscale0`). Authentication is implicit via Tailscale node identity.

**Alternatives considered:**
- *gRPC*: More structured but adds protobuf toolchain complexity for a simple request/response API.
- *SSH with restricted commands*: No daemon needed, but harder to constrain (ForceCommand can be bypassed) and slower for repeated queries.
- *Unix socket with SSH tunneling*: Secure but operationally complex.

**Rationale:** Tailscale already provides mutual authentication (every node has a verified identity), end-to-end encryption (WireGuard), and ACL support. Binding to `tailscale0` means the agent is unreachable from the LAN or internet. HTTP is simple to implement, debug, and test. The agent can optionally verify the caller's Tailscale IP against an allowlist.

### Decision 4: Fixed command vocabulary with tier enforcement in the agent

**Choice:** The agent defines a fixed set of commands at compile time. Each command belongs to a tier. The agent refuses commands above its configured tier, regardless of who calls it.

**Alternatives considered:**
- *Caller-side enforcement only*: Relies on sentinel-mcp/cli to check tiers. If someone curls the agent directly, no enforcement.
- *Capability-based tokens*: More flexible but adds token management complexity.

**Rationale:** Defense in depth. Even if a caller is compromised or misconfigured, the agent itself enforces what operations are permitted on each host. The configuration is declarative in NixOS — the host owner decides what's allowed.

### Decision 5: Notifications via Sid's existing Pushover tool

**Choice:** The agent itself does not send notifications. Tier 2 actions return a response indicating "notification recommended," and the caller (sentinel-mcp or sentinel-cli) delegates notification to Sid's existing Pushover capability.

**Alternatives considered:**
- *Agent sends Pushover directly*: Requires Pushover credentials on every host.
- *Dedicated notification service*: Over-engineered for 3 hosts.

**Rationale:** Sid already has Pushover integration via zeroclaw. Notifications are a caller concern, not an agent concern. This keeps the agent simple and credential-free.

### Decision 6: Nix flake structure

**Choice:** Standalone flake (`axios-sentinel`) with:
- `nixosModules.default` — the agent NixOS module
- `packages.x86_64-linux.sentinel-agent` — the agent daemon
- `packages.x86_64-linux.sentinel-cli` — the emergency CLI
- `packages.x86_64-linux.sentinel-mcp` — the MCP server

Imported into `nixos_config/flake.nix` as an input. sentinel-mcp is registered on edge's mcp-gateway in `edge.nix` home-manager config (same pattern as zeroclaw-mcp). sentinel-cli is added to Sid's PATH via `services.sid.extraPackages` in `mini.nix`.

**Rationale:** Follows the established pattern (sid, mcp-gateway, axios-dav are all standalone flakes imported into nixos_config). Keeps axios itself focused on framework concerns.

## Risks / Trade-offs

**[Risk] Agent daemon is a privilege escalation surface** → Mitigation: Fixed command vocabulary (no arbitrary execution), bind to Tailscale only (unreachable from LAN/internet), per-host tier configuration, agent runs as a dedicated system user with only the specific capabilities it needs (e.g., `CAP_SYS_BOOT` for reboot, polkit rules for service restart).

**[Risk] Sid takes incorrect autonomous action (false positive)** → Mitigation: Tier 2 actions (reboot, kill) always send a Pushover notification. The action is disruptive but not destructive — reboots are safe on NixOS (services auto-restart, filesystems journal). Worst case is a few minutes of unnecessary downtime from a false positive.

**[Risk] sentinel-mcp on edge's gateway is unavailable when edge is down** → Mitigation: This is the entire reason for the dual-path design. The CLI path is independent and works over Tailscale as long as the target host's agent is responsive.

**[Risk] Command vocabulary is too restrictive** → Mitigation: Start minimal and add commands based on real incidents. The vocabulary is defined in code, so adding a new command is a PR, not a config change. Better to be too restrictive and add than too permissive and regret.

**[Trade-off] No historical metrics in v1** → Sid can only ask "what's the CPU temp now?" not "what was it trending toward?" This is acceptable for v1 — Prometheus can be added later as `services.sentinel.prometheus.enable` without changing the agent architecture.

**[Trade-off] Rust adds compile-time overhead** → Building the agent from source on each `nixos-rebuild` adds time. Mitigation: The agent is small (~few thousand lines), and Nix caching means it only rebuilds on code changes.

## Open Questions

1. **Should the agent run as root or a dedicated user?** Running as root simplifies implementation (can restart any service, read any log, reboot) but violates least-privilege. A dedicated user with targeted capabilities (polkit for systemctl, CAP_SYS_BOOT for reboot) is more secure but more complex to configure in NixOS.

2. **Health check semantics:** What constitutes "healthy" vs "warning" vs "failed"? Need to define thresholds (e.g., disk > 90% = warning, any failed systemd unit = warning, host unreachable = failed). These could be configurable or hardcoded for v1.

3. **Should the agent auto-register with Tailscale Services?** Like Ollama does (`axios-sentinel.tailnet.ts.net`). This would enable service discovery instead of hardcoding host lists. Trade-off: adds Tailscale API dependency to the agent.
