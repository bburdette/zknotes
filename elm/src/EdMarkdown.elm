module EdMarkdown exposing
    ( EdMarkdown
    , getBlocks
    , getMd
    , getSpecialNote
    , init
    , linkRenderer
    , stringRenderer
    , updateBlocks
    , updateMd
    , updateSpecialNote
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


type EdMarkdown
    = EdMarkdown Emd


type alias Emd =
    { md : String
    , elts : Result String (List Block)
    , specialNote : Result JD.Error SpecialNote
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
        , specialNote = JD.decodeString specialNoteDecoder md
        }


getBlocks : EdMarkdown -> Result String (List Block)
getBlocks (EdMarkdown emd) =
    emd.elts


getSpecialNote : EdMarkdown -> Result JD.Error SpecialNote
getSpecialNote (EdMarkdown emd) =
    emd.specialNote


updateSpecialNote : SpecialNote -> EdMarkdown
updateSpecialNote sn =
    EdMarkdown
        { md = JE.encode 2 (specialNoteEncoder sn)
        , elts = Err "specialnote"
        , specialNote = Ok sn
        }


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
                    , specialNote = Err (JD.Failure "markdown" JE.null)
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
lrConcat : ( Maybe String, List (List Link) ) -> ( Maybe String, List Link )
lrConcat ( mbs, lnks ) =
    ( mbs, List.concat lnks )


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



{-
   { heading : { level : Block.HeadingLevel, rawText : String, children : List (Maybe String, List Link) } -> (Maybe String, List Link)
   , paragraph : List (Maybe String, List Link) -> (Maybe String, List Link)
   , blockQuote : List (Maybe String, List Link) -> (Maybe String, List Link)
   , html : Markdown.Html.Renderer (List (Maybe String, List Link) -> (Maybe String, List Link))
   , text : String -> (Maybe String, List Link)
   , codeSpan : String -> (Maybe String, List Link)
   , strong : List (Maybe String, List Link) -> (Maybe String, List Link)
   , emphasis : List (Maybe String, List Link) -> (Maybe String, List Link)
   , strikethrough : List (Maybe String, List Link) -> (Maybe String, List Link)
   , hardLineBreak : (Maybe String, List Link)
   , link : { title : Maybe String, destination : String } -> List (Maybe String, List Link) -> (Maybe String, List Link)
   , image : { alt : String, src : String, title : Maybe String } -> (Maybe String, List Link)
   , unorderedList : List (ListItem (Maybe String, List Link)) -> (Maybe String, List Link)
   , orderedList : Int -> List (List (Maybe String, List Link)) -> (Maybe String, List Link)
   , codeBlock : { body : String, language : Maybe String } -> (Maybe String, List Link)
   , thematicBreak : (Maybe String, List Link)
   , table : List (Maybe String, List Link) -> (Maybe String, List Link)
   , tableHeader : List (Maybe String, List Link) -> (Maybe String, List Link)
   , tableBody : List (Maybe String, List Link) -> (Maybe String, List Link)
   , tableRow : List (Maybe String, List Link) -> (Maybe String, List Link)
   , tableCell : Maybe Block.Alignment -> List (Maybe String, List Link) -> (Maybe String, List Link)
   , tableHeaderCell : Maybe Block.Alignment -> List (Maybe String, List Link) -> (Maybe String, List Link)

-}


linkRenderer : Markdown.Renderer.Renderer ( Maybe String, List Link )
linkRenderer =
    { heading =
        \{ level, rawText, children } -> otroLrConcat children
    , paragraph = otroLrConcat
    , blockQuote = otroLrConcat
    , html = MC.otroHtmlLinks
    , text = \s -> ( Just s, [] )
    , codeSpan = \_ -> ( Nothing, [] )
    , strong = otroLrConcat
    , emphasis = otroLrConcat
    , strikethrough = otroLrConcat
    , hardLineBreak = ( Nothing, [] )
    , link =
        \{ title, destination } c ->
            let
                _ =
                    Debug.log "  title, destination } content ->" ( title, destination, c )
            in
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



-- linkRenderer : Markdown.Renderer.Renderer ( Maybe String, List Link )
-- linkRenderer =
--     { heading =
--         \{ level, rawText, children } -> List.concat children
--     , paragraph = List.concat
--     , blockQuote = List.concat
--     , html = MC.htmlLinks
--     , text = \_ -> []
--     , codeSpan = \_ -> []
--     , strong = List.concat
--     , emphasis = List.concat
--     , strikethrough = List.concat
--     , hardLineBreak = []
--     , link =
--         \{ title, destination } content ->
--             let
--                 _ =
--                     Debug.log "  title, destination } content ->" ( title, destination, content )
--             in
--             [ { id = Right destination, title = title |> Maybe.withDefault "" } ]
--     , image =
--         \imageInfo ->
--             [ { id = Right imageInfo.src, title = imageInfo.alt } ]
--     , unorderedList =
--         \items ->
--             let
--                 its : List (List Link)
--                 its =
--                     items
--                         |> List.map
--                             (\listitem ->
--                                 case listitem of
--                                     Block.ListItem Block.NoTask childs ->
--                                         List.concat childs
--                                     Block.ListItem Block.IncompleteTask childs ->
--                                         List.concat childs
--                                     Block.ListItem Block.CompletedTask childs ->
--                                         List.concat childs
--                             )
--             in
--             List.concat its
--     , orderedList =
--         \startingIndex items ->
--             List.concat <| List.concat items
--     , codeBlock =
--         \{ body, language } -> []
--     , thematicBreak = []
--     , table = List.concat
--     , tableHeader = List.concat
--     , tableBody = List.concat
--     , tableRow = List.concat
--     , tableCell =
--         \_ lnks ->
--             List.concat lnks
--     , tableHeaderCell =
--         \maybeAlignment lnks ->
--             List.concat lnks
--     }


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
