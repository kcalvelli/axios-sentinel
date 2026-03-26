## CHANGED Requirements

### Requirement: check_fleet_health tool returns health of all hosts
The `check_fleet_health` tool SHALL query the health endpoint on all configured hosts and return an aggregate status. The overall status SHALL only reflect `always-on` hosts. Transient hosts are reported but do not affect the overall status.

#### Scenario: All hosts healthy
- **WHEN** `check_fleet_health` is called and all hosts respond with `pass`
- **THEN** the tool returns `{"overall": "pass", "hosts": {"edge": {"status": "pass", "availability": "always-on", ...}, "mini": {"status": "pass", "availability": "always-on", ...}}}`

#### Scenario: One always-on host unhealthy
- **WHEN** `check_fleet_health` is called and edge returns `fail`
- **THEN** the tool returns `{"overall": "fail", "hosts": {"edge": {"status": "fail", "availability": "always-on", "reasons": [...]}, "mini": {"status": "pass", "availability": "always-on", ...}}}`

#### Scenario: One always-on host unreachable
- **WHEN** `check_fleet_health` is called and edge is unreachable
- **THEN** the tool returns `{"overall": "fail", "hosts": {"edge": {"status": "unreachable", "availability": "always-on", "error": "..."}, ...}}`

#### Scenario: Transient host offline
- **WHEN** `check_fleet_health` is called and pangolin (availability: transient) is unreachable
- **THEN** the tool returns `{"overall": "pass", "hosts": {..., "pangolin": {"status": "offline", "availability": "transient"}}}` — overall is NOT degraded by the transient host being offline

#### Scenario: Transient host online and unhealthy
- **WHEN** `check_fleet_health` is called and pangolin (availability: transient) is reachable but returns `warn`
- **THEN** the tool returns the warning status for pangolin with `"availability": "transient"` — a reachable transient host's health IS reported normally but still does not affect overall fleet status

### Requirement: list_hosts tool includes availability class
The `list_hosts` tool SHALL include each host's availability class in the output.

#### Scenario: List hosts with mixed availability
- **WHEN** `list_hosts` is called with hosts configured as edge (always-on), mini (always-on), pangolin (transient)
- **THEN** the tool returns `[{"host": "edge", "reachable": true, "availability": "always-on"}, {"host": "mini", "reachable": true, "availability": "always-on"}, {"host": "pangolin", "reachable": false, "availability": "transient"}]`

### Requirement: MCP server reads host configuration from environment
The sentinel-mcp SHALL read its host list, tailnet domain, and agent port from environment variables set by the NixOS module / mcp-gateway server registration. The host list MAY include per-host availability classes.

#### Scenario: Configuration with availability classes
- **WHEN** `SENTINEL_HOSTS=edge,mini,pangolin:transient`, `SENTINEL_DOMAIN=taile0fb4.ts.net`, and `SENTINEL_PORT=9256` are set
- **THEN** sentinel-mcp configures edge and mini as `always-on` (default) and pangolin as `transient`

#### Scenario: Configuration without availability classes (backward compatible)
- **WHEN** `SENTINEL_HOSTS=edge,mini` is set without any colon suffixes
- **THEN** sentinel-mcp treats all hosts as `always-on` (identical to current behavior)
