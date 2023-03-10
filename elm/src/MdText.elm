module MdText exposing (renderMdText, stringRenderer)

import Markdown.Block as Block exposing (Block, Inline, ListItem, Task)
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer exposing (Renderer)


renderMdText : String -> Result String String
renderMdText md =
    md
        |> Markdown.Parser.parse
        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        |> Result.andThen (Markdown.Renderer.render stringRenderer)
        |> Result.map String.concat


stringRenderer : Renderer String
stringRenderer =
    { heading =
        \{ level, rawText, children } ->
            (case level of
                Block.H1 ->
                    "# " ++ String.concat children

                Block.H2 ->
                    "## " ++ String.concat children

                Block.H3 ->
                    "### " ++ String.concat children

                Block.H4 ->
                    "#### " ++ String.concat children

                Block.H5 ->
                    "##### " ++ String.concat children

                Block.H6 ->
                    "###### "
                        ++ String.concat children
            )
                |> (\s -> String.append s "\n\n")
    , paragraph =
        \strs ->
            String.concat strs
                ++ "\n\n"
    , hardLineBreak = "  \n"
    , blockQuote =
        \strs ->
            strs
                |> List.map (\s -> "  " ++ s ++ "\n")
                |> String.concat
    , strong =
        \s ->
            String.concat
                ("**" :: s ++ [ "**" ])
    , emphasis =
        \s ->
            String.concat
                ("*" :: s ++ [ "*" ])
    , strikethrough =
        \s ->
            String.concat
                ("~~" :: s ++ [ "~~" ])
    , codeSpan =
        \s ->
            "`" ++ s ++ "`"
    , link =
        \link content ->
            String.concat
                [ "["
                , -- link.title |> Maybe.withDefault "",
                  String.concat content
                , "]("
                , link.destination
                , ")"
                ]
    , image =
        \imageInfo ->
            String.concat
                [ "!["
                , -- link.title |> Maybe.withDefault "",
                  imageInfo.alt
                , "]("
                , imageInfo.src
                , ")"
                ]
    , text = identity
    , unorderedList =
        \items ->
            items
                |> List.map
                    (\listitem ->
                        case listitem of
                            Block.ListItem Block.NoTask childs ->
                                "- " ++ String.concat childs ++ "\n"

                            Block.ListItem Block.IncompleteTask childs ->
                                "- [ ]" ++ String.concat childs ++ "\n"

                            Block.ListItem Block.CompletedTask childs ->
                                "- [x]" ++ String.concat childs ++ "\n"
                    )
                |> String.concat
    , orderedList =
        \startingIndex items ->
            items
                |> List.indexedMap (\i item -> String.fromInt (i + startingIndex) ++ ") " ++ String.concat item ++ "\n")
                |> String.concat
    , html = Markdown.Html.oneOf []
    , codeBlock =
        \{ body, language } ->
            String.concat
                [ "````"
                , language |> Maybe.withDefault ""
                , "\n"
                , body
                , "```\n`"
                ]
    , thematicBreak = "--------------------\n"

    -- tables not currently parsed by elm-markdown.
    , table = String.concat
    , tableHeader = String.concat
    , tableBody = String.concat
    , tableRow = String.concat
    , tableHeaderCell =
        \maybeAlignment strs -> String.concat strs
    , tableCell =
        \maybeAlignment strs -> String.concat strs
    }
