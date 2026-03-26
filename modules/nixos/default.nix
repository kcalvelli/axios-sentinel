{ config, lib, pkgs, ... }:

let
  cfg = config.services.sentinel.agent;
in
{
  options.services.sentinel.agent = {
    enable = lib.mkEnableOption "sentinel-agent system operations daemon";

    port = lib.mkOption {
      type = lib.types.port;
      default = 9256;
      description = "Port for the sentinel-agent HTTP API (bound to Tailscale interface only)";
    };

    tier1 = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Allow tier 1 operations (autonomous, safe, reversible):
        restart-service, gpu-reset, journal-vacuum.
      '';
    };

    tier2 = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Allow tier 2 operations (autonomous but notify user):
        reboot, kill-process.
      '';
    };

    restartableServices = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        Systemd units that can be restarted via the restart-service command.
        Only units in this list are allowed — all others are rejected.
      '';
      example = [
        "niri"
        "greetd"
        "ollama"
      ];
    };

    allowGpuReset = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Allow GPU reset via the gpu-reset command (AMD GPUs only)";
    };

    package = lib.mkOption {
      type = lib.types.package;
      description = "The sentinel-agent package to use";
    };

    cliPackage = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      description = "The sentinel-cli package. When set, added to system PATH.";
    };

    fleet = {
      hosts = lib.mkOption {
        type = lib.types.listOf (lib.types.either
          lib.types.str
          (lib.types.submodule {
            options = {
              name = lib.mkOption {
                type = lib.types.str;
                description = "Hostname of the fleet member.";
              };
              availability = lib.mkOption {
                type = lib.types.enum [ "always-on" "transient" ];
                default = "always-on";
                description = "Availability class: always-on (expected to be reachable) or transient (may be offline).";
              };
            };
          })
        );
        default = [ ];
        description = "Hostnames of all sentinel-monitored hosts in the fleet. Each entry is either a plain hostname string (treated as always-on) or an attrset with name and availability.";
        example = lib.literalExpression ''
          [
            "edge"
            "mini"
            { name = "pangolin"; availability = "transient"; }
          ]
        '';
      };

      domain = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Tailnet domain for host resolution (e.g., taile0fb4.ts.net).";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    # Add CLI to system PATH
    environment.systemPackages = lib.mkIf (cfg.cliPackage != null) [ cfg.cliPackage ];

    # Set fleet config as system-wide env vars so sentinel-cli and sentinel-mcp pick them up
    environment.sessionVariables = lib.mkIf (cfg.fleet.hosts != [ ]) {
      SENTINEL_HOSTS = lib.concatStringsSep "," (map (h:
        if builtins.isString h then h
        else if h.availability == "always-on" then h.name
        else "${h.name}:${h.availability}"
      ) cfg.fleet.hosts);
      SENTINEL_DOMAIN = cfg.fleet.domain;
      SENTINEL_PORT = toString cfg.port;
    };

    # Create dedicated system user
    users.users.sentinel = {
      isSystemUser = true;
      group = "sentinel";
      description = "Sentinel agent daemon user";
    };
    users.groups.sentinel = { };

    # Systemd service
    systemd.services.sentinel-agent = {
      description = "Sentinel system operations agent";
      wantedBy = [ "multi-user.target" ];
      after = [
        "network-online.target"
        "tailscaled.service"
      ];
      wants = [ "network-online.target" ];

      environment = {
        SENTINEL_PORT = toString cfg.port;
        SENTINEL_TIER1 = lib.boolToString cfg.tier1;
        SENTINEL_TIER2 = lib.boolToString cfg.tier2;
        SENTINEL_RESTARTABLE = lib.concatStringsSep "," cfg.restartableServices;
        SENTINEL_GPU_RESET = lib.boolToString cfg.allowGpuReset;
      };

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/sentinel-agent";
        Restart = "always";
        RestartSec = 5;

        # Security hardening
        # Run as root for now — needed for systemctl restart, reboot, sensors, journalctl
        # TODO: switch to dedicated user with polkit rules for fine-grained access
        DynamicUser = false;
        ProtectHome = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        NoNewPrivileges = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictSUIDSGID = true;

        # Allow reading sysfs for GPU/temperature data
        ReadOnlyPaths = [
          "/sys"
          "/proc"
        ];
      };

      # Need access to systemctl, sensors, journalctl, tailscale, df, stat, kill
      path = with pkgs; [
        systemd
        lm_sensors
        coreutils
        util-linux
        tailscale
        procps
        smartmontools
      ];
    };

    # Open firewall on Tailscale interface
    networking.firewall.interfaces."tailscale0".allowedTCPPorts = [ cfg.port ];

    # Polkit rules for the sentinel user to restart allowlisted services
    security.polkit.extraConfig = lib.mkIf (cfg.restartableServices != [ ]) ''
      polkit.addRule(function(action, subject) {
        if (action.id == "org.freedesktop.systemd1.manage-units" &&
            subject.user == "root" &&
            action.lookup("verb") == "restart") {
          var unit = action.lookup("unit");
          var allowed = [${
            lib.concatMapStringsSep "," (s: ''"${s}"'') cfg.restartableServices
          }];
          if (allowed.indexOf(unit) >= 0) {
            return polkit.Result.YES;
          }
        }
        return polkit.Result.NOT_HANDLED;
      });
    '';
  };
}
