# zknotes

A zettelkasten implementation in elm and rust, currently storing notes in sqlite.  Its still in the prototype stage.

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

- elm/elm-common is a git submodule, so you'll need to:
  ```
  git submodule init
  git submodule update
  ```
- the elm and server directories contain shell.nix files.  If you use nix, just execute nix-shell in those directories and you're ready to build.
- build elm with 
  ```
  cd elm
  nix-shell
  ./watch-build.sh
  ```
  or use `./build-prod.sh` if you only want a one-shot build (watch-build uses elm-live).
- build/run the server with
  ```
  cd server
  nix-shell
  ./watch-run.sh
  ```

Final note - you're expected to register as a user in order to log in to the website, and this requires an email with a 'magic link' in it.  Chances are the email send won't work when you register (most ISPs prevent this), so look for server/last-email.txt to get your magic link.
