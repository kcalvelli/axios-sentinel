# sentinel-mcp Specification

## Purpose
TBD - created by archiving change initial-implementation. Update Purpose after archive.
## Requirements
### Requirement: MCP server exposes monitoring tools
The sentinel-mcp SHALL expose MCP tools that aggregate data from sentinel-agents across the fleet. It runs as a stdio MCP server registered on the mcp-gateway.

#### Scenario: List available tools
- **WHEN** an MCP client sends a `tools/list` request
- **THEN** sentinel-mcp returns tools including: `query_host`, `list_hosts`, `check_fleet_health`, `restart_service`, `reboot_host`, `view_logs`, `system_status`, `host_temperatures`, `host_disk`, `host_gpu`

### Requirement: query_host tool returns comprehensive host status
The `query_host` tool SHALL accept a hostname parameter and return a comprehensive status object combining status, services, temperatures, and disk information.

#### Scenario: Query a healthy host
- **WHEN** `query_host` is called with `{"host": "edge"}`
- **THEN** the tool returns a combined JSON object with system status, failed services (if any), current temperatures, and disk usage

#### Scenario: Query an unreachable host
- **WHEN** `query_host` is called with `{"host": "edge"}` and edge is unreachable
- **THEN** the tool returns an error indicating the host is unreachable, with the last-seen timestamp if available

### Requirement: check_fleet_health tool returns health of all hosts
The `check_fleet_health` tool SHALL query the health endpoint on all configured hosts and return an aggregate status.

#### Scenario: All hosts healthy
- **WHEN** `check_fleet_health` is called and all hosts respond with `pass`
- **THEN** the tool returns `{"overall": "pass", "hosts": {"edge": "pass", "mini": "pass", "pangolin": "pass"}}`

#### Scenario: One host unhealthy
- **WHEN** `check_fleet_health` is called and edge returns `fail`
- **THEN** the tool returns `{"overall": "fail", "hosts": {"edge": {"status": "fail", "reasons": [...]}, "mini": "pass", "pangolin": "pass"}}`

#### Scenario: One host unreachable
- **WHEN** `check_fleet_health` is called and pangolin is unreachable
- **THEN** the tool returns `{"overall": "fail", "hosts": {"edge": "pass", "mini": "pass", "pangolin": {"status": "unreachable"}}}`

### Requirement: restart_service tool restarts a service on a host
The `restart_service` tool SHALL accept host and unit parameters and restart the specified service via the target host's agent.

#### Scenario: Successful restart
- **WHEN** `restart_service` is called with `{"host": "edge", "unit": "niri"}`
- **THEN** the tool calls `POST /restart-service` on edge's agent and returns the result

#### Scenario: Service not in allowlist
- **WHEN** `restart_service` is called with `{"host": "edge", "unit": "sshd"}` and sshd is not in edge's allowlist
- **THEN** the tool returns an error indicating the service is not in the allowlist

### Requirement: reboot_host tool reboots a host
The `reboot_host` tool SHALL accept a host parameter, reboot the host via the agent, and indicate that a notification should be sent.

#### Scenario: Successful reboot
- **WHEN** `reboot_host` is called with `{"host": "edge"}`
- **THEN** the tool calls `POST /reboot` on edge's agent, returns a success message, and includes a note that Pushover notification is recommended

### Requirement: view_logs tool returns journal output
The `view_logs` tool SHALL accept host, unit, and optional lines parameters.

#### Scenario: View recent logs
- **WHEN** `view_logs` is called with `{"host": "edge", "unit": "ollama", "lines": 50}`
- **THEN** the tool returns the last 50 journal lines for the ollama unit on edge

### Requirement: MCP server reads host configuration from environment
The sentinel-mcp SHALL read its host list, tailnet domain, and agent port from environment variables set by the NixOS module / mcp-gateway server registration.

#### Scenario: Configuration from environment
- **WHEN** `SENTINEL_HOSTS=edge,mini,pangolin`, `SENTINEL_DOMAIN=taile0fb4.ts.net`, and `SENTINEL_PORT=9256` are set
- **THEN** sentinel-mcp uses these to construct agent URLs for each host

