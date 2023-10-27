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

## install notes

If you want to compile and run this on your own machine, without bothering with development tools:

- start by installing the [nix package manager](https://nixos.org/download.html) on your system, and [enable flakes](https://nixos.wiki/wiki/Flakes).

- install into a 'result' folder in a local directory (no need to clone this repo!)
  ```
  nix build "git+http://github.com/bburdette/zknotes?submodules=1"
  ```

- Make a config.toml file with `zknotes-server -w myconfig.toml`, then edit as needed.

## first login

To create the first zknotes admin account, use the -a option, like so:

`../target/debug/zknotes-server -c myconfig.toml -a the-admin`

By default zknotes uses 'invite links' for new users.  To onboard a new user, an admin gets an invite link from the admin panel, and sends that to the new user via email, signal, slack, etc.  Whoever uses the invite link can set their username and password to get an account.

## run zknotes

After finishing the above setup, run it with:
  ```
  ./result/bin/zknotes-server -c myconfig.toml
  ```

## developing

Strictly speaking you only need git, rust, elm, sqlite and openssl to do development (I think).  But you can use nix to install the full array of gadgets I use which will supply lsp support, formatting, automatic recompiling, etc.

currently my orgauth lib is used here with submodules.  so you have to:
```
$ git submodule update
$ cd orgauth
$ git submodule update
```
yes, there is a submodule within a submodule.  hopefully this will change sooner than later.

to install the dev tools needed, clone the zknotes repo locally.  then run `nix develop` in that directory.

there's a watch_run.sh in the server subdirectory, and a watch_build.sh  in the elm directory.  Run those each in a separate terminal and you'll get automatic rebuilds whenever you make changes to code.
