{ config, options, lib, pkgs, ... }:

with lib;

let

  cfg = config.services.zknotes;
  opt = options.services.zknotes;

  # Command line arguments for the zknotes daemon
  # data dir.  zknotes.db, config.
  # user to run as.
  # port, I guess.
  # pdf docs directory

in

{

  ###### interface

  options = {
    services.zknotes = {
      enable = mkEnableOption (lib.mdDoc "zknotes; markdown based multi user zettelkasten");

      user = mkOption {
        type = types.str;
        default = "zknotes";
        example = "zknotes-user";
        description = "User account in which to run zknotes.";
      };
      group = lib.mkOption {
        type = lib.types.str;
        default = "zknotes";
        description = lib.mdDoc "Group under which zknotes runs.";
      };


      # dataDir = mkOption {
      #   type = types.path;
      #   default = null;
      #   example = "/home/bburdette/zknotes";
      #   description = "Location where zknotes runs and stores data.";
      # };

      # listenAddress = mkOption {
      #   type = types.nullOr types.str;
      #   default = null;
      #   example = "127.0.0.1";
      #   description = "Listen on a specific IP address.";
      # };

      # listenPort = mkOption {
      #   type = types.nullOr types.int;
      #   default = null;
      #   example = 8011;
      #   description = "Listen on a specific IP port.";
      # };

    };
  };

  ###### implementation
  config = mkIf cfg.enable {

    systemd.services.zknotes = {
      description = "zknotes";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig.User = cfg.user;
      serviceConfig.Group = cfg.group;

      script = ''
        cd "/home/${cfg.user}"
        mkdir -p zknotes
        cd zknotes
        if [ ! -f config.toml ]; then
          mkdir -p files
          mkdir -p temp
          ${pkgs.zknotes}/bin/zknotes-server --write-config config.toml
        fi
        RUST_LOG=info ${pkgs.zknotes}/bin/zknotes-server -c config.toml
        # RUST_LOG=info zknotes-server -c config.toml
        '';
    };

    users.groups = {
      ${cfg.group} = { };
    };

    users.users = lib.mkMerge [
      (lib.mkIf (cfg.user == "zknotes") {
        ${cfg.user} = {
          isSystemUser = true;
          group = cfg.group;
          home = "/home/${cfg.user}";
          createHome = true;
        };
      })
    ];

        # members = "${opt.user}";
        # members = lib.optional cfg.configureNginx config.services.nginx.user;
        # members = [ config.services.nginx.user ];
      # };
    # };

    # environment.systemPackages = [ zknotes ];
  };
}
