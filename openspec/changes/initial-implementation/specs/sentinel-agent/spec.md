## ADDED Requirements

### Requirement: Agent binds to Tailscale interface only
The sentinel-agent SHALL bind its HTTP listener exclusively to the Tailscale interface (`tailscale0`). It MUST NOT be reachable from the LAN, WAN, or localhost outside of Tailscale.

#### Scenario: Agent starts and binds correctly
- **WHEN** sentinel-agent starts on a host with Tailscale active
- **THEN** the HTTP listener is bound to the host's Tailscale IP on the configured port (default 9256)

#### Scenario: Agent refuses non-Tailscale connections
- **WHEN** a request arrives from a non-Tailscale IP (e.g., LAN 192.168.x.x)
- **THEN** the connection is refused at the network level (not listening on that interface)

### Requirement: Agent exposes read commands
The sentinel-agent SHALL expose the following read-only endpoints that return structured JSON. Read commands are always available regardless of tier configuration.

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check: pass/warn/fail with reasons |
| `GET /status` | System overview: uptime, load, memory, swap, disk summary |
| `GET /services` | All systemd services with state (running/failed/inactive) |
| `GET /failed` | Failed systemd units only |
| `GET /temperatures` | Hardware sensor readings (CPU, GPU, NVMe, board) |
| `GET /disk` | Disk usage (df) and SMART health summary |
| `GET /gpu` | GPU status: driver info, VRAM usage, clocks, thermals |
| `GET /logs/{unit}?lines={n}` | Last N lines of a systemd unit's journal (default 50) |

#### Scenario: Status returns system overview
- **WHEN** `GET /status` is called
- **THEN** response is JSON containing hostname, uptime, load averages, total/used/available memory, swap usage, and root filesystem usage percentage

#### Scenario: Health check returns structured assessment
- **WHEN** `GET /health` is called
- **THEN** response is JSON with an overall status (`pass`, `warn`, or `fail`) and an array of check results, each with a name, status, and message

#### Scenario: Logs returns journal output for a unit
- **WHEN** `GET /logs/niri?lines=20` is called
- **THEN** response is JSON containing the last 20 journal lines for the `niri` systemd unit

#### Scenario: Temperatures returns sensor data
- **WHEN** `GET /temperatures` is called
- **THEN** response is JSON with sensor readings grouped by device (CPU, GPU, NVMe, board), each with current value, high threshold, and critical threshold where available

### Requirement: Agent exposes tier 1 commands
The sentinel-agent SHALL expose tier 1 operations when `tier1` is enabled in configuration. Tier 1 operations are autonomous and safe to execute without notification.

| Endpoint | Description |
|----------|-------------|
| `POST /restart-service` | Restart a systemd service (body: `{"unit": "<name>"}`) |
| `POST /gpu-reset` | Trigger amdgpu GPU recovery |
| `POST /journal-vacuum` | Reclaim journal disk space (body: `{"max_size": "<size>"}`) |

#### Scenario: Restart a service in the allowlist
- **WHEN** `POST /restart-service` is called with `{"unit": "niri"}` and `niri` is in the host's `restartableServices` list
- **THEN** the agent executes `systemctl restart niri` and returns JSON with the result (success/failure and any error message)

#### Scenario: Reject restart for service not in allowlist
- **WHEN** `POST /restart-service` is called with `{"unit": "sshd"}` and `sshd` is NOT in the host's `restartableServices` list
- **THEN** the agent returns HTTP 403 with an error message indicating the service is not in the allowlist

#### Scenario: Reject tier 1 command when tier 1 is disabled
- **WHEN** `POST /restart-service` is called on a host where `tier1 = false`
- **THEN** the agent returns HTTP 403 with an error message indicating tier 1 operations are disabled

#### Scenario: GPU reset on AMD host
- **WHEN** `POST /gpu-reset` is called on a host with an AMD GPU
- **THEN** the agent triggers GPU recovery via the kernel interface and returns the result

#### Scenario: GPU reset on non-GPU host
- **WHEN** `POST /gpu-reset` is called on a host without an AMD GPU (or without `allowGpuReset`)
- **THEN** the agent returns HTTP 404 indicating GPU reset is not available on this host

### Requirement: Agent exposes tier 2 commands
The sentinel-agent SHALL expose tier 2 operations when `tier2` is enabled in configuration. Tier 2 responses include a `notify: true` field indicating the caller should send a notification.

| Endpoint | Description |
|----------|-------------|
| `POST /reboot` | Reboot the host |
| `POST /kill-process` | Kill a process by PID (body: `{"pid": <number>}`) |

#### Scenario: Reboot the host
- **WHEN** `POST /reboot` is called on a host where `tier2 = true`
- **THEN** the agent initiates a system reboot via `systemctl reboot`, returns JSON with `{"status": "rebooting", "notify": true}`, and the host reboots

#### Scenario: Kill a process by PID
- **WHEN** `POST /kill-process` is called with `{"pid": 12345}` on a host where `tier2 = true`
- **THEN** the agent sends SIGTERM to PID 12345, returns JSON with the result and `notify: true`

#### Scenario: Reject tier 2 command when tier 2 is disabled
- **WHEN** `POST /reboot` is called on a host where `tier2 = false`
- **THEN** the agent returns HTTP 403 with an error message indicating tier 2 operations are disabled

### Requirement: Agent returns consistent JSON responses
All agent endpoints SHALL return JSON with a consistent envelope format.

#### Scenario: Successful response
- **WHEN** any endpoint succeeds
- **THEN** response has HTTP 200, content-type `application/json`, and body `{"ok": true, "data": {...}, "hostname": "<host>"}`

#### Scenario: Error response
- **WHEN** any endpoint fails (bad request, forbidden, internal error)
- **THEN** response has the appropriate HTTP status code, content-type `application/json`, and body `{"ok": false, "error": "<message>", "hostname": "<host>"}`
