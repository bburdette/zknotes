{ config, options, lib, pkgs, ... }:

with lib;

let

  cfg = config.services.zknotes-onsave;

in

{

  ###### interface
  options = {
    services.zknotes-onsave = {
      enable = mkEnableOption (lib.mdDoc "zknotes-onsave; perform tasks when notes and files are saved in zknotes.");

      user = mkOption {
        type = types.str;
        default = "zknotes";
        example = "zknotes-user";
        description = "linux user account in which to run zknotes-onsave.";
      };
      group = lib.mkOption {
        type = lib.types.str;
        default = "zknotes";
        description = lib.mdDoc "linux group under which zknotes-onsave runs.";
      };

      amqp_uri = mkOption {
        type = types.nullOr types.str;
        default = "amqp://localhost:5672";
        example = "amqp://localhost:5672";
        description = "uri of the amqp (rabbitmq) server";
      };

      server_uri = mkOption {
        type = types.nullOr types.str;
        default = "http://localhost:8010";
        example = "http://localhost:8010";
        description = "uri of the zknotes server";
      };
    };
  };

  ###### implementation
  config = mkIf cfg.enable {

    systemd.services.zknotes-onsave = {
      description = "zknotes-onsave";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      # didn't work to make yt-dlp available.
      # path = [ "${builtins.dirOf (lib.getExe pkgs.yt-dlp)}" ];
      path = [
        pkgs.yt-dlp
        pkgs.imagemagick
        pkgs.ffmpeg-headless
      ];

      serviceConfig.User = cfg.user;
      serviceConfig.Group = cfg.group;

      script = ''
        cd "/home/${cfg.user}"
        mkdir -p zknotes-onsave
        cd zknotes-onsave
        RUST_LOG=info ${pkgs.zknotes}/bin/zknotes-onsave --amqp_uri "${cfg.amqp_uri}" --server_uri "${cfg.server_uri}" --yt-dlp-path "${lib.getExe pkgs.yt-dlp}"
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
