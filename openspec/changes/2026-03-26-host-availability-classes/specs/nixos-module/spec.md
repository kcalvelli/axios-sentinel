## CHANGED Requirements

### Requirement: NixOS module supports host availability classes in fleet config
The `services.sentinel.agent.fleet.hosts` option SHALL accept both plain hostname strings (backward compatible) and attribute sets with `name` and `availability` fields.

#### Scenario: Plain string hosts (backward compatible)
- **WHEN** `fleet.hosts = [ "edge" "mini" ]` is set
- **THEN** both hosts are treated as `always-on` and `SENTINEL_HOSTS` is set to `"edge,mini"`

#### Scenario: Hosts with availability classes
- **WHEN** `fleet.hosts = [ { name = "edge"; } { name = "mini"; } { name = "pangolin"; availability = "transient"; } ]` is set
- **THEN** `SENTINEL_HOSTS` is set to `"edge,mini,pangolin:transient"` — hosts without explicit availability default to `always-on` and are serialized without a suffix

#### Scenario: Mixed string and attrset hosts
- **WHEN** `fleet.hosts = [ "edge" "mini" { name = "pangolin"; availability = "transient"; } ]` is set
- **THEN** `SENTINEL_HOSTS` is set to `"edge,mini,pangolin:transient"`

#### Scenario: Invalid availability class
- **WHEN** `fleet.hosts = [ { name = "edge"; availability = "sometimes"; } ]` is set
- **THEN** NixOS evaluation fails with a type error — only `"always-on"` and `"transient"` are accepted
