module EdMarkdown exposing (EdMarkdown, getBlocks, getMd, init, updateBlocks, updateMd)

import Markdown.Block as Block exposing (Block, ListItem(..), Task(..), foldl, inlineFoldl)
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import MdCommon as MC


type EdMarkdown
    = EdMarkdown Emd


type alias Emd =
    { md : String
    , elts : Result String (List Block)
    }


init : String -> EdMarkdown
init s =
    updateMd s


getMd : EdMarkdown -> String
getMd (EdMarkdown emd) =
    emd.md


updateMd : String -> EdMarkdown
updateMd md =
    EdMarkdown
        { md = md
        , elts =
            md
                |> Markdown.Parser.parse
                |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        }


getBlocks : EdMarkdown -> Result String (List Block)
getBlocks (EdMarkdown emd) =
    emd.elts


updateBlocks : List Block -> Result String EdMarkdown
updateBlocks blocks =
    -- let
    --     _ =
    --         Debug.log "blocks" blocks
    -- in
    -- render blocks to string!
    -- maybe get tweaky with it and remember the offsets into string to do a faster replacement, rather than re-render whole string.
    Markdown.Renderer.render defaultStringRenderer blocks
        |> Result.map
            (\md ->
                EdMarkdown
                    { md = String.concat md
                    , elts = Ok blocks
                    }
            )


{-| This renders the parsed markdown structs to a string.
TODO: use the one in Markdown lib when its published.
-}
defaultStringRenderer : Markdown.Renderer.Renderer String
defaultStringRenderer =
    { heading =
        \{ level, children } ->
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
                ++ "\n\n"
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
                , String.concat content
                , "]("
                , link.destination
                , ")"
                ]
    , image =
        \imageInfo ->
            String.concat
                [ "!["
                , imageInfo.alt
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
    , html = MC.htmlText
    , codeBlock =
        \{ body, language } ->
            String.concat
                [ "```"
                , language |> Maybe.withDefault ""
                , "\n"
                , body
                , "```\n\n"
                ]
    , thematicBreak = "--------------------\n"
    , table = String.concat >> (++) "\n"
    , tableHeader =
        -- we get the whole header as one string here, contained in a single element list.
        List.map
            twoheads
            >> String.concat
    , tableBody = List.foldr (\s l -> s :: "\n" :: l) [] >> String.concat
    , tableRow = List.intersperse " | " >> String.concat
    , tableHeaderCell =
        \maybeAlignment strs ->
            String.concat strs
                ++ " | "
                ++ (case maybeAlignment of
                        Just Block.AlignLeft ->
                            ":-"

                        Just Block.AlignRight ->
                            "-:"

                        Just Block.AlignCenter ->
                            ":-:"

                        Nothing ->
                            "--"
                   )
    , tableCell =
        \_ strs ->
            String.concat strs
    }


twoheads : String -> String
twoheads headstr =
    headstr
        |> String.split " | "
        |> toheads ( [], [] )
        |> (\( heads, aligns ) ->
                String.concat (List.intersperse " | " heads)
                    ++ "\n"
                    ++ String.concat (List.intersperse "|" aligns)
                    ++ "\n"
           )


toheads : ( List String, List String ) -> List String -> ( List String, List String )
toheads ( llst, rlst ) strs =
    case strs of
        l :: r :: cdr ->
            toheads ( l :: llst, r :: rlst ) cdr

        _ ->
            ( List.reverse llst, List.reverse rlst )
