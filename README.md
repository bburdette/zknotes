# zknotes

A multiuser zettelkasten implementation in elm and rust, currently storing notes in sqlite.

There are some [docs](https://www.zknotes.com/page/what%20is%20zknotes) hosted in zknotes itself, with a few screenshots.

Notes are in markdown, specifically [elm-markdown](https://package.elm-lang.org/packages/dillonkearns/elm-markdown/latest/), which should allow for some interesting extensions later on.  Currently you can use formulas as in [cellme](https://github.com/bburdette/cellme/).

zknotes is web based, and is intended to be usable on phones.  As of now there's no provision for integration with 3rd party editors like vim or kakoune.  Document editing happens through a typical web page text box.

There's a small search language - you can find documents by title, or by content, or by link with other documents, with boolean expressions combining these queries.

zknotes has some multi-user features.  
 - Notes linked to the 'public' system note are available for the internet at large to view.  
 - You can create 'share notes' by linking ordinary notes to the 'share' system note, and then linking users to the share note.  Those users will be able to see any notes linked to the share note.  
 - Notes can be designated as read-only or editable.  
 - There's also a comment system.  Notes that link to the 'comment' system note and to another note show up as comments in the web UI.

## install locally to try it out:

If you want to compile and run this on your own machine, without bothering with development tools:

- start by installing the [nix package manager](https://nixos.org/download.html) on your system, and [enable flakes](https://nixos.wiki/wiki/Flakes).

- install into a 'result' folder in a local directory (no need to clone this repo!)
  ```
  nix build "git+http://github.com/bburdette/zknotes?submodules=1"
  ```

- Make a config.toml file with `./result/bin/zknotes-server -w myconfig.toml`, then edit as needed.

To create the first zknotes admin account, use the -a option, like so:

`./result/bin/zknotes-server -c myconfig.toml -a the-admin`

After finishing the above setup, run it with:
  ```
  ./result/bin/zknotes-server -c myconfig.toml
  ```

## as a nix service (nixos)

If you have a flake.nix and configuration.nix, then in the flake.nix this line to your inputs:

    zknotes = { url = "git+https://github.com/bburdette/zknotes?submodules=1"; };

Then add this to your modules:

          inputs.zknotes.nixosModules.zknotes

Then in configuration.nix add:
```
  nixpkgs.overlays = [ (final: prev: { zknotes = inputs.zknotes.packages.${pkgs.system}.zknotes; })];
  services.zknotes.enable = true;
```
You can use custom settings:

```
   nixpkgs.overlays = [ (final: prev: { zknotes = inputs.zknotes.packages.${pkgs.system}.zknotes; })];
   services.zknotes = {
     enable = true;
     settings = ''
         ip = '127.0.0.1'
         port = 8010
         createdirs = true
         altmainsite = []
         file_tmp_path = './temp'
         file_path = './files'
 
         [orgauth_config]
         mainsite = 'http://localhost:8010'
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
         remote_registration = true
       '';
   };
```

The default here is to run zknotes in its own user account, 'zknotes', and store data in /home/zknotes/zknotes.  

To create the first user with zknotes, so you can log in:

```
su
cd /home/zknotes/zknotes
zknotes-server -c config.toml -a my-admin-uid
```

## invite users

By default zknotes uses 'invite links' for new users.  To onboard a new user, an admin gets an invite link from the admin panel, and sends that to the new user via email, signal, slack, etc.  Whoever uses the invite link can set their username and password to get an account.

## developing

Strictly speaking you only need git, rust, elm, sqlite and openssl to do development (I think).  But you can use nix to install the full array of gadgets I use which will supply lsp support, formatting, automatic recompiling, etc.

currently my orgauth lib is used here with submodules.  so you have to:
```
 git submodule update --init --recursive
```
yes, there is a submodule within a submodule.  hopefully this will change sooner than later.

to install the dev tools needed, clone the zknotes repo locally.  then run `nix develop` in that directory.

there's a watch_run.sh in the server subdirectory, and a watch_build.sh  in the elm directory.  Run those each in a separate terminal and you'll get automatic rebuilds whenever you make changes to code.
