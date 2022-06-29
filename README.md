# zknotes

A multiuser zettelkasten implementation in elm and rust, currently storing notes in sqlite.  Its still in the prototype stage.

There are some [docs](https://www.zknotes.com/page/what%20is%20zknotes) hosted in zknotes itself, with a few screenshots.

Notes are in markdown, specifically [elm-markdown](https://package.elm-lang.org/packages/dillonkearns/elm-markdown/latest/), which should allow for some interesting extensions later on.  Currently you can use formulas as in [cellme](https://github.com/bburdette/cellme/).

zknotes is web based, and is intended to be usable on phones.  As of now there's no provision for integration with 3rd party editors like vim or kakoune.  Document editing happens through a typical web page text box.

There's a small search language - you can find documents by title, or by content, or by link with other documents, with boolean expressions combining these queries.

zknotes has some multi-user features.  
 - Notes linked to the 'public' system note are available for the internet at large to view.  
 - You can create 'share notes' by linking ordinary notes to the 'share' system note, and then linking users to the share note.  Those users will be able to see any notes linked to the share note.  
 - Notes can be designated as read-only or editable.  
 - There's also a comment system.  Notes that link to the 'comment' system note and to another note show up as comments in the web UI.

### install notes

If you want to compile and run this on your own machine:

- start by installing the nix package manager on your system, a version with 'flakes' enabled.

- install into a 'result' folder in a local directory:
  ```
  nix build github:bburdette/zknotes
  ```

- Make a config.toml file with `zknotes-server -w myconfig.toml`, then edit as needed.

- run it with:
  ```
  ./result/bin/zknotes-server -c myconfig.toml

  ```

Final note - you're expected to register as a user in order to log in to the website, and this requires an email with a 'magic link' in it.  Chances are the email send won't work when you register (most ISPs prevent this), so look for server/last-email.txt to get your magic link.

### developing.

currently orgauth is used here with submodules.  so you have to:
```
$ git submodule update
$ cd orgauth
$ git submodule update
```
yes, there is a submodule within a submodule.  hopefully this will change sooner than later.

to install the dev tools needed, clone the zknotes repo locally.  then run `nix develop` in that directory.

there's a watch_run.sh in the server subdirectory, and a watch_build.sh  in the elm directory.  Run those each in a separate terminal and you'll get automatic rebuilds whenever you make changes to code.
