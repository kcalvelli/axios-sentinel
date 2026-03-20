## ADDED Requirements

### Requirement: CLI provides direct access to any sentinel-agent
The sentinel-cli SHALL communicate directly with sentinel-agents over Tailscale HTTP, bypassing the mcp-gateway entirely. It MUST resolve hosts via Tailscale DNS (`<host>.<tailnet>.ts.net`).

#### Scenario: Query a single host
- **WHEN** `sentinel-cli edge status` is called
- **THEN** the CLI sends `GET /status` to `edge.<tailnet>.ts.net:9256` and prints the response

#### Scenario: Query all hosts
- **WHEN** `sentinel-cli all health` is called
- **THEN** the CLI queries the health endpoint on every configured host in parallel and prints a summary table

#### Scenario: Host unreachable
- **WHEN** `sentinel-cli edge status` is called and edge is unreachable
- **THEN** the CLI prints an error indicating the host is unreachable (connection timeout) and exits with a non-zero status code

### Requirement: CLI supports all agent commands
The sentinel-cli SHALL support every command in the agent's vocabulary as a subcommand.

#### Scenario: Read command
- **WHEN** `sentinel-cli edge temperatures` is called
- **THEN** the CLI sends `GET /temperatures` to edge's agent and prints formatted output

#### Scenario: Tier 1 command
- **WHEN** `sentinel-cli edge restart-service niri` is called
- **THEN** the CLI sends `POST /restart-service` with `{"unit": "niri"}` to edge's agent and prints the result

#### Scenario: Tier 2 command
- **WHEN** `sentinel-cli edge reboot` is called
- **THEN** the CLI sends `POST /reboot` to edge's agent and prints the result

#### Scenario: View logs
- **WHEN** `sentinel-cli edge logs ollama 100` is called
- **THEN** the CLI sends `GET /logs/ollama?lines=100` to edge's agent and prints the journal output

### Requirement: CLI output is human-readable by default and machine-parseable with --json
The sentinel-cli SHALL format output as human-readable tables/text by default and as JSON when `--json` is passed.

#### Scenario: Default output
- **WHEN** `sentinel-cli edge status` is called without flags
- **THEN** output is a formatted, human-readable summary

#### Scenario: JSON output
- **WHEN** `sentinel-cli edge status --json` is called
- **THEN** output is the raw JSON response from the agent

### Requirement: CLI reads configuration from environment or config file
The sentinel-cli SHALL read the tailnet domain and host list from environment variables or a config file, with environment taking precedence.

#### Scenario: Environment variables
- **WHEN** `SENTINEL_DOMAIN=taile0fb4.ts.net` and `SENTINEL_HOSTS=edge,mini,pangolin` are set
- **THEN** the CLI uses these for host resolution and the `all` target

#### Scenario: Config file
- **WHEN** environment variables are not set and `~/.config/sentinel/config.toml` exists
- **THEN** the CLI reads domain and hosts from the config file
