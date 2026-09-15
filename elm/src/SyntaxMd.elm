module SyntaxMd exposing (langParser, showLang)

import Html exposing (Html)
import Parser
import SyntaxHighlight exposing (HCode)
import Util


langParser : String -> (String -> Result (List Parser.DeadEnd) HCode)
langParser s =
    case s of
        "elm" ->
            SyntaxHighlight.elm

        "xml" ->
            SyntaxHighlight.xml

        "javascript" ->
            SyntaxHighlight.javascript

        "css" ->
            SyntaxHighlight.css

        "python" ->
            SyntaxHighlight.python

        "go" ->
            SyntaxHighlight.go

        "sql" ->
            SyntaxHighlight.sql

        "json" ->
            SyntaxHighlight.json

        "nix" ->
            SyntaxHighlight.nix

        "kotlin" ->
            SyntaxHighlight.kotlin

        _ ->
            SyntaxHighlight.noLang


showLang : String -> Maybe String -> Html m
showLang body mblang =
    let
        _ =
            Debug.log "showlang " ( body, mblang )

        ps =
            mblang |> Maybe.map langParser |> Maybe.withDefault SyntaxHighlight.noLang

        rhc =
            ps body
    in
    case rhc of
        Err e ->
            Html.text (Util.deadEndsToString e)

        Ok hc ->
            SyntaxHighlight.toInlineHtml hc
