module EdMarkdown exposing
    ( EdMarkdown
    , getBlocks
    , getContent
    , getSpecialNote
    , getSpecialNoteState
    , initMd
    , initSpecial
    , linkRenderer
    , stringRenderer
    , updateBlocks
    , updateMd
    , updateSpecialNoteState
    )

import Data exposing (ZkNoteId)
import DataUtil exposing (NlLink)
import Either exposing (Either(..))
import Json.Decode as JD
import Json.Encode as JE
import Markdown.Block as Block exposing (Block, ListItem(..), Task(..))
import Markdown.Parser
import Markdown.Renderer
import MdCommon as MC exposing (Link)
import SpecialNotes exposing (SpecialNote, specialNoteDecoder, specialNoteEncoder)
import SpecialNotesGui as SNG exposing (SpecialNoteState)



-- TRY THIS


type EdMarkdown
    = EdMarkdown Emd
    | EdSpecial Special


type alias Emd =
    { md : String
    , elts : Result String (List Block)
    }


type alias Special =
    { snState : SpecialNoteState
    }


initMd : String -> EdMarkdown
initMd s =
    updateMd s


initSpecial : SpecialNoteState -> EdMarkdown
initSpecial sns =
    EdSpecial { snState = sns }


getContent : EdMarkdown -> String
getContent em =
    case em of
        EdSpecial s ->
            JE.encode 2 (specialNoteEncoder (SNG.getSpecialNote s.snState))

        EdMarkdown emd ->
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
getBlocks em =
    case em of
        EdMarkdown emd ->
            emd.elts

        EdSpecial _ ->
            Err "special note"


getSpecialNote : EdMarkdown -> Maybe SpecialNote
getSpecialNote em =
    case em of
        EdMarkdown _ ->
            Nothing

        EdSpecial s ->
            Just <| SNG.getSpecialNote s.snState


getSpecialNoteState : EdMarkdown -> Maybe SpecialNoteState
getSpecialNoteState em =
    case em of
        EdMarkdown _ ->
            Nothing

        EdSpecial s ->
            Just <| s.snState


updateSpecialNoteState : SpecialNoteState -> EdMarkdown
updateSpecialNoteState sns =
    EdSpecial { snState = sns }


updateBlocks : List Block -> Result String EdMarkdown
updateBlocks blocks =
    -- render blocks to string!
    -- maybe get tweaky with it and remember the offsets into string to do a faster replacement, rather than re-render whole string.
    Markdown.Renderer.render stringRenderer blocks
        |> Result.map
            (\md ->
                EdMarkdown
                    { md = String.concat md
                    , elts = Ok blocks
                    }
            )


spacedash : String -> Bool
spacedash s =
    case String.left 1 s of
        "-" ->
            True

        " " ->
            spacedash <| String.dropLeft 1 s

        _ ->
            False


{-| This renders the parsed markdown structs to a string.
TODO: use the one in Markdown lib when its published.
-}
stringRenderer : Markdown.Renderer.Renderer String
stringRenderer =
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
            strs
                |> String.concat
                |> String.trimRight
                |> (\s -> s ++ "\n\n")
    , hardLineBreak = "  \n"
    , blockQuote =
        \strs ->
            strs
                |> List.map (\s -> "> " ++ s ++ "\n")
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
            let
                its : List String
                its =
                    items
                        |> List.map
                            (\listitem ->
                                let
                                    childz =
                                        \childs ->
                                            ((String.concat
                                                (List.map
                                                    (\s ->
                                                        if spacedash s then
                                                            "\n  "
                                                                ++ String.replace "\n"
                                                                    "\n  "
                                                                    s

                                                        else
                                                            s
                                                    )
                                                    childs
                                                )
                                                |> String.trimRight
                                             )
                                                |> String.replace "\n" "\n  "
                                            )
                                                ++ "\n"
                                in
                                case listitem of
                                    Block.ListItem Block.NoTask childs ->
                                        "- "
                                            -- ++ (String.concat childs |> String.trimRight)
                                            ++ childz childs

                                    Block.ListItem Block.IncompleteTask childs ->
                                        "- [ ] "
                                            ++ childz childs

                                    Block.ListItem Block.CompletedTask childs ->
                                        "- [x] "
                                            ++ childz childs
                            )
            in
            its
                ++ [ "\n" ]
                |> String.concat
    , orderedList =
        \startingIndex items ->
            items
                |> List.indexedMap (\i item -> String.fromInt (i + startingIndex) ++ ") " ++ String.concat item ++ "\n")
                |> (\l ->
                        List.append l [ "\n" ]
                   )
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


{-| This renders the parsed markdown structs to a list of links.
-}
linkRenderer : Markdown.Renderer.Renderer ( Maybe String, List Link )
linkRenderer =
    { heading =
        \{ level, rawText, children } -> otroLrConcat children
    , paragraph = otroLrConcat
    , blockQuote = otroLrConcat
    , html = MC.htmlLinks
    , text = \s -> ( Just s, [] )
    , codeSpan = \_ -> ( Nothing, [] )
    , strong = otroLrConcat
    , emphasis = otroLrConcat
    , strikethrough = otroLrConcat
    , hardLineBreak = ( Nothing, [] )
    , link =
        \{ title, destination } c ->
            ( Nothing, [ { id = Right destination, title = otroLrConcat c |> Tuple.first |> Maybe.withDefault "" } ] )
    , image =
        \imageInfo ->
            ( Nothing, [ { id = Right imageInfo.src, title = imageInfo.alt } ] )
    , unorderedList =
        \items ->
            let
                its : List ( Maybe String, List Link )
                its =
                    items
                        |> List.map
                            (\listitem ->
                                case listitem of
                                    Block.ListItem Block.NoTask childs ->
                                        otroLrConcat childs

                                    Block.ListItem Block.IncompleteTask childs ->
                                        otroLrConcat childs

                                    Block.ListItem Block.CompletedTask childs ->
                                        otroLrConcat childs
                            )
            in
            otroLrConcat its
    , orderedList =
        -- List (List (Maybe String, List Link))
        \startingIndex items ->
            -- otroLrConcat <| otroLrConcat items
            otroLrConcat (List.map otroLrConcat items)
    , codeBlock =
        \{ body, language } -> ( Nothing, [] )
    , thematicBreak = ( Nothing, [] )
    , table = otroLrConcat
    , tableHeader = otroLrConcat
    , tableBody = otroLrConcat
    , tableRow = otroLrConcat
    , tableCell =
        \_ lnks ->
            otroLrConcat lnks
    , tableHeaderCell =
        \maybeAlignment lnks ->
            otroLrConcat lnks
    }


otroLrConcat : List ( Maybe String, List Link ) -> ( Maybe String, List Link )
otroLrConcat lst =
    List.foldl
        (\( mbs, links ) ( smbs, slinks ) ->
            ( case ( mbs, smbs ) of
                ( Just s, _ ) ->
                    Just s

                ( Nothing, Just s ) ->
                    Just s

                ( Nothing, Nothing ) ->
                    Nothing
            , List.concat [ links, slinks ]
            )
        )
        ( Nothing, [] )
        lst


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
