# zknotes

A zettelkasten implementation in elm and rust, currently storing notes in sqlite.  Its still in the prototype stage.

Notes are in markdown, specifically [elm-markdown](https://package.elm-lang.org/packages/dillonkearns/elm-markdown/latest/), which should allow for some interesting extensions later on.  Currently you can use formulas as in [cellme](https://github.com/bburdette/cellme/).

zknotes is web based, and is intended to be usable on phones.  As of now there's no provision for integration with 3rd party editors like vim or kakoune.  Document editing happens through a typical web page text box.

There's a small search language - you can find documents by title, or by content, or by link with other documents, with boolean expressions combining these queries.

zknotes has some multi-user features.  Notes linked to the 'public' system note are available for the internet at large to view.  You can also create 'share notes' by linking ordinary notes to the 'share' system note, and then linking users to the share note.  Those users will be able to see any notes linked to the share note.
