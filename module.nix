{ config, options, lib, pkgs, ... }:

with lib;

let

  cfg = config.services.zknotes;
  opt = options.services.zknotes;
  settingsFormat = pkgs.formats.toml { };

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

    settings = lib.mkOption {
      inherit (settingsFormat) type;
      default = ''
        ip = '127.0.0.1'
        port = 8000
        createdirs = false
        altmainsite = []
        file_tmp_path = './temp'
        file_path = './files'

        [orgauth_config]
        mainsite = 'http://localhost:8000'
        appname = 'zknotes'
        emaildomain = 'zknotes.com'
        db = './zknotes.db'
        admin_email = 'admin@admin.admin'
        regen_login_tokens = true
        email_token_expiration_ms = 86400000
        reset_token_expiration_ms = 86400000
        invite_token_expiration_ms = 604800000
        open_registration = false
        send_emails = false
        non_admin_invite = true
        remote_registration = true '';
      description = ''
        Configuration included in `starship.toml`.

        See https://starship.rs/config/#prompt for documentation.
      '';
    };

      listenPort = mkOption {
        type = types.nullOr types.int;
        default = null;
        example = 8011;
        description = "Listen on a specific IP port.";
      };

    };
  };

          # ${pkgs.zknotes}/bin/zknotes-server --write_config config.toml
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
        fi
        echo "${cfg.settings}" >> config.toml
        RUST_LOG=info ${pkgs.zknotes}/bin/zknotes-server -c config.toml
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
  };
}
