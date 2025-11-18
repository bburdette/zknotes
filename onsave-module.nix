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
      yeet-service = mkOption {
        type = types.nullOr types.boolean;
        default = "true";
        example = "true";
        description = "consume on_save_note amqp messages and check for yeetlinks.  yeet accordingly.";
      };
      thumb-service = mkOption {
        type = types.nullOr types.boolean;
        default = "true";
        example = "true";
        description = "consume on_make_file_note amqp messages and generate thumb files for movies/images.";
      };
      amqp-uid-file = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "/run/secrets/onsave-uid";
        description = "file containing amqp (rabbitmq) user name";
      };
      amqp-pwd-file = mkOption {
        type = types.nullOr types.str;
        default = null;
        example = "/run/secrets/onsave-pwd";
        description = "file containing amqp (rabbitmq) password";
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
        RUST_LOG=info ${pkgs.zknotes}/bin/zknotes-onsave --amqp_uri "${cfg.amqp_uri}" --server_uri "${cfg.server_uri}" --yt-dlp-path "${lib.getExe pkgs.yt-dlp}" --thumb-service ${cfg.thumb-service} --yeet-service ${cfg.yeet-service} 
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
