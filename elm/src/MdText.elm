module MdText exposing (stringRenderer)

import Markdown.Block as Block exposing (Block, Inline, ListItem, Task)
import Markdown.Html
import Markdown.Renderer exposing (Renderer)


{-| This renders `Html` in an attempt to be as close as possible to
the HTML output in <https://github.github.com/gfm/>.
-}
stringRenderer : Renderer String
stringRenderer =
    { heading =
        -- \{ level, rawText, children } ->
        --     case level of
        --         Block.H1 ->
        --             "# " ++ String.concat children
        --         Block.H2 ->
        --             "## " ++ String.concat children
        --         Block.H3 ->
        --             "### " ++ String.concat children
        --         Block.H4 ->
        --             "#### " ++ String.concat children
        --         Block.H5 ->
        --             "##### " ++ String.concat children
        --         Block.H6 ->
        --             "###### " ++ String.concat children
        \hinfo ->
            case hinfo.level of
                Block.H1 ->
                    "# " ++ String.concat hinfo.children

                Block.H2 ->
                    "## " ++ String.concat hinfo.children

                Block.H3 ->
                    "### " ++ String.concat hinfo.children

                Block.H4 ->
                    "#### " ++ String.concat hinfo.children

                Block.H5 ->
                    "##### " ++ String.concat hinfo.children

                Block.H6 ->
                    "###### " ++ String.concat hinfo.children
    , paragraph = String.concat
    , hardLineBreak = "\n\n"
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

    -- |> (\l -> l ++ "*")
    -- |> (++) "*"
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
                                "- " ++ String.concat childs ++ "\n"

                            Block.ListItem Block.CompletedTask childs ->
                                "- " ++ String.concat childs ++ "\n"
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
    , thematicBreak = "\n"
    , table = String.concat
    , tableHeader = String.concat
    , tableBody = String.concat
    , tableRow = String.concat
    , tableHeaderCell =
        \maybeAlignment strs -> String.concat strs
    , tableCell =
        \maybeAlignment strs -> String.concat strs
    }



-- type alias Renderer String =
--     { heading : { level : Block.HeadingLevel, rawText : String, children : List String } -> String
--     , paragraph : List String -> String
--     , blockQuote : List String -> String
--     , html : Markdown.Html.Renderer (List String -> String)
--     , text : String -> String
--     , codeSpan : String -> String
--     , strong : List String -> String
--     , emphasis : List String -> String
--     , strikethrough : List String -> String
--     , hardLineBreak : String
--     , link : { title : Maybe String, destination : String } -> List String -> String
--     , image : { alt : String, src : String, title : Maybe String } -> String
--     , unorderedList : List (ListItem String) -> String
--     , orderedList : Int -> List (List String) -> String
--     , codeBlock : { body : String, language : Maybe String } -> String
--     , thematicBreak : String
--     , table : List String -> String
--     , tableHeader : List String -> String
--     , tableBody : List String -> String
--     , tableRow : List String -> String
--     , tableCell : Maybe Block.Alignment -> List String -> String
--     , tableHeaderCell : Maybe Block.Alignment -> List String -> String
--     }
{-
   -- { blockQuote : List String -> String
     -- , codeBlock : { body : String, language : Maybe String } -> String
     -- , codeSpan : String -> String
     -- , emphasis : List String -> String
     -- , hardLineBreak : String
     -- , heading :
     --       { children : List String, level : Block.HeadingLevel, rawText : String
     --       }
     --       -> String
     -- , html : Markdown.Html.Renderer (List String -> String)
     -- , image : { alt : String, src : String, title : Maybe String } -> String
     -- , link :
     --       { destination : String, title : Maybe String }
     --       -> List String
     --       -> String
     , orderedList : Int -> List String -> String
     , paragraph : List String -> String
     , strikethrough : List String -> String
     , strong : List String -> String
     , table : List String -> String
     , tableBody : List String -> String
     , tableCell : Maybe Block.Alignment -> List String -> String
     , tableHeader : List String -> String
     , tableHeaderCell : Maybe Block.Alignment -> List String -> String
     , tableRow : List String -> String
     , text : String -> String
     , thematicBreak : String
     , unorderedList : List (ListItem String) -> String
     }
-}
