{ config, lib, pkgs, ... }:

with lib;

let

  cfg = config.services.zknotes;

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
      enable = mkEnableOption "zknotes";

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

      serviceConfig.User = "bburdette";

      script = ''
          cd /home/bburdette/zknotes
          RUST_LOG=info /home/bburdette/.nix-profile/bin/zknotes-server
          '';
    };
  };
}
