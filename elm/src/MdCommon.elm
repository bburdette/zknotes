module MdCommon exposing (Panel, ViewMode(..), blockCells, blockPanels, cellView, codeBlock, codeSpan, defCell, heading, imageView, linkDict, markdownView, mdCells, mdPanel, mdPanels, mkRenderer, noteFile, noteIds, panelView, rawTextToId, searchView, showRunState)

import Cellme.Cellme exposing (CellContainer(..), RunState(..))
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr)
import Common exposing (buttonStyle)
import Data exposing (ZkNoteId, ZniSet, emptyZniSet, zkNoteIdFromString, zkNoteIdToString)
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Html
import Html.Attributes as HA
import Markdown.Block as Block exposing (Block, ListItem(..), Task(..), foldl, inlineFoldl)
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import Maybe.Extra as ME
import NoteCache as NC exposing (NoteCache)
import Schelme.Show exposing (showTerm)
import Set exposing (Set(..))
import TSet
import TangoColors as TC
import Util
import ZkCommon


markdownView : Markdown.Renderer.Renderer (Element a) -> String -> Result String (List (Element a))
markdownView renderer markdown =
    markdown
        |> Markdown.Parser.parse
        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        |> Result.andThen (Markdown.Renderer.render renderer)


mdCells : String -> Result String CellDict
mdCells markdown =
    markdown
        |> Markdown.Parser.parse
        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        |> Result.map blockCells


type alias Panel =
    { noteid : ZkNoteId }


mdPanel : String -> Maybe Panel
mdPanel markdown =
    markdown
        |> mdPanels
        |> Result.toMaybe
        |> Maybe.andThen List.head


mdPanels : String -> Result String (List Panel)
mdPanels markdown =
    markdown
        |> Markdown.Parser.parse
        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        |> Result.map blockPanels


blockPanels : List Block -> List Panel
blockPanels blocks =
    blocks
        |> List.filterMap
            (\block ->
                case block of
                    Block.HtmlBlock (Block.HtmlElement tag attribs _) ->
                        if tag == "panel" then
                            let
                                am =
                                    Dict.fromList <| List.map (\trib -> ( trib.name, trib.value )) attribs
                            in
                            am
                                |> Dict.get "noteid"
                                |> Maybe.andThen (zkNoteIdFromString >> Result.toMaybe)
                                |> Maybe.andThen
                                    (\id ->
                                        Just { noteid = id }
                                    )

                        else
                            Nothing

                    _ ->
                        Nothing
            )


blockCells : List Block -> CellDict
blockCells blocks =
    blocks
        |> List.filterMap
            (\block ->
                case block of
                    Block.HtmlBlock (Block.HtmlElement tag attribs _) ->
                        if tag == "cell" then
                            let
                                am =
                                    Dict.fromList <| List.map (\trib -> ( trib.name, trib.value )) attribs
                            in
                            am
                                |> Dict.get "name"
                                |> Maybe.andThen
                                    (\name ->
                                        am
                                            |> Dict.get "schelmecode"
                                            |> Maybe.map (String.replace "/'" "\"")
                                            |> Maybe.andThen
                                                (\schelme ->
                                                    Just ( name, defCell schelme )
                                                )
                                    )

                        else
                            Nothing

                    _ ->
                        Nothing
            )
        |> Dict.fromList
        |> CellDict


defCell : String -> DictCell
defCell s =
    { code = s, prog = Err "", runstate = RsErr "" }


type ViewMode
    = PublicView
    | EditView


link : Maybe String -> String -> List (Element a) -> Element a
link title destination body =
    (if String.contains ":" destination then
        E.newTabLink

     else
        E.link
    )
        [ E.htmlAttribute (HA.style "display" "inline-flex") ]
        { url = destination
        , label =
            E.paragraph
                [ EF.color (E.rgb255 0 0 255)
                , E.htmlAttribute (HA.style "overflow-wrap" "break-word")
                , E.htmlAttribute (HA.style "word-break" "break-word")
                ]
                body
        }


mkRenderer : Data.Sysids -> ViewMode -> (String -> a) -> Int -> CellDict -> Bool -> (String -> String -> a) -> NoteCache -> Markdown.Renderer.Renderer (Element a)
mkRenderer si viewMode restoreSearchMsg maxw cellDict showPanelElt onchanged noteCache =
    { heading = heading
    , paragraph =
        E.paragraph
            [ E.spacing 8 ]
    , thematicBreak = E.none
    , text = E.text
    , strong = \content -> E.paragraph [ EF.bold ] content
    , emphasis = \content -> E.paragraph [ EF.italic ] content
    , strikethrough = \content -> E.paragraph [ EF.strike ] content
    , codeSpan = codeSpan
    , link =
        \{ title, destination } body -> link title destination body
    , hardLineBreak = Html.br [] [] |> E.html
    , image =
        \image ->
            E.image [ E.width E.fill ]
                { src = image.src, description = image.alt }
    , blockQuote =
        \children ->
            E.column
                [ EBd.widthEach { top = 0, right = 0, bottom = 0, left = 10 }
                , E.padding 10
                , EBd.color (E.rgb255 145 145 145)
                , EBk.color (E.rgb255 245 245 245)
                ]
                children
    , unorderedList =
        \items ->
            E.column [ E.paddingXY 10 0 ]
                (items
                    |> List.map
                        (\(ListItem task children) ->
                            E.paragraph []
                                [ E.row
                                    [ E.alignTop ]
                                    ((case task of
                                        IncompleteTask ->
                                            EI.defaultCheckbox False

                                        CompletedTask ->
                                            EI.defaultCheckbox True

                                        NoTask ->
                                            E.text "•"
                                     )
                                        :: E.text " "
                                        :: children
                                    )
                                ]
                        )
                )
    , orderedList =
        \startingIndex items ->
            E.column [ E.spacingXY 10 0, E.width E.fill ]
                (items
                    |> List.indexedMap
                        (\index itemBlocks ->
                            E.row [ E.width E.fill ]
                                (E.text
                                    (String.fromInt (index + startingIndex)
                                        ++ " "
                                    )
                                    :: itemBlocks
                                )
                        )
                )
    , codeBlock = codeBlock
    , html =
        Markdown.Html.oneOf
            [ Markdown.Html.tag "cell"
                (\name schelmeCode renderedChildren ->
                    cellView cellDict renderedChildren name schelmeCode onchanged
                )
                |> Markdown.Html.withAttribute "name"
                |> Markdown.Html.withAttribute "schelmecode"
            , Markdown.Html.tag "search"
                (\search renderedChildren ->
                    searchView viewMode restoreSearchMsg search renderedChildren
                )
                |> Markdown.Html.withAttribute "query"
            , Markdown.Html.tag "panel"
                (\noteid renderedChildren ->
                    case zkNoteIdFromString noteid of
                        Ok id ->
                            if showPanelElt then
                                panelView id renderedChildren

                            else
                                E.none

                        Err _ ->
                            E.text "error"
                )
                |> Markdown.Html.withAttribute "noteid"
            , Markdown.Html.tag "image" imageView
                |> Markdown.Html.withAttribute "text"
                |> Markdown.Html.withAttribute "url"
                |> Markdown.Html.withOptionalAttribute "width"
            , Markdown.Html.tag "video" (videoView maxw)
                |> Markdown.Html.withAttribute "src"
                |> Markdown.Html.withOptionalAttribute "text"
                |> Markdown.Html.withOptionalAttribute "width"
                |> Markdown.Html.withOptionalAttribute "height"
            , Markdown.Html.tag "note" (noteView si noteCache)
                |> Markdown.Html.withAttribute "id"
            ]
    , table = E.column [ E.width <| E.fill ]
    , tableHeader = E.column [ E.width <| E.fill, EF.bold, EF.underline, E.spacing 8 ]
    , tableBody = E.column [ E.width <| E.fill ]
    , tableRow = E.row [ E.width E.fill ]
    , tableHeaderCell =
        \maybeAlignment children ->
            E.paragraph [] children
    , tableCell =
        \maybeAlignment children ->
            E.paragraph [] children
    }


searchView : ViewMode -> (String -> a) -> String -> List (Element a) -> Element a
searchView viewMode restoreSearchMsg search renderedChildren =
    E.row [ EBk.color TC.darkGray, E.padding 3, E.spacing 3 ]
        (E.el [ EF.italic ] (E.text "search: ")
            :: E.paragraph [] [ E.text search ]
            :: (case viewMode of
                    PublicView ->
                        E.none

                    EditView ->
                        EI.button
                            (buttonStyle ++ [ EBk.color TC.darkGray ])
                            { label = E.el [ E.centerY, EF.color TC.blue, EF.bold ] <| E.text ">"
                            , onPress = Just <| restoreSearchMsg search
                            }
               )
            :: renderedChildren
        )


panelView : ZkNoteId -> List (Element a) -> Element a
panelView noteid renderedChildren =
    E.el [ E.padding 5, EBk.color TC.darkGray ] <|
        E.text ("Side panel note :" ++ zkNoteIdToString noteid)


imageView : String -> String -> Maybe String -> List (Element a) -> Element a
imageView text url mbwidth renderedChildren =
    case
        mbwidth
            |> Maybe.andThen (\s -> String.toInt s)
    of
        Just w ->
            E.image [ E.width <| E.maximum w E.fill, E.centerX ]
                { src = url, description = text }

        Nothing ->
            E.image [ E.width E.fill ]
                { src = url, description = text }


htmlAudioView : String -> String -> Element a
htmlAudioView url text =
    E.html (Html.audio [ HA.controls True, HA.src url ] [ Html.text text ])


audioNoteView : Data.Sysids -> Data.ZkNote -> Element a
audioNoteView si zkn =
    E.column [ EBd.width 1, E.spacing 5, E.padding 5 ]
        [ link (Just zkn.title) ("/note/" ++ zkNoteIdToString zkn.id) [ E.text zkn.title ]
        , E.row [ E.spacing 20 ]
            [ htmlAudioView ("/file/" ++ zkNoteIdToString zkn.id) zkn.title

            -- TODO pass in url instead of hardcoded
            , if List.filter (\i -> i == si.publicid) zkn.sysids /= [] then
                link
                    (Just "ts↗")
                    ("https://29a.ch/timestretch/#a=https://www.zknotes.com/file/" ++ zkNoteIdToString zkn.id)
                    [ E.text "ts↗" ]

              else
                E.el [ Util.addToolTip E.below (ZkCommon.stringToolTip "disabled for private notes") ]
                    (E.el [ EF.color TC.darkGrey ] <| E.text "ts↗")
            ]
        ]


videoNoteView : Data.ZkNote -> Element a
videoNoteView zknote =
    let
        fileurl =
            "/file/" ++ zkNoteIdToString zknote.id
    in
    E.column [ EBd.width 1, E.spacing 5, E.padding 5 ]
        [ link (Just zknote.title) ("/note/" ++ zkNoteIdToString zknote.id) [ E.text zknote.title ]
        , videoView 500 fileurl (Just zknote.title) Nothing Nothing []
        ]


imageNoteView : Data.ZkNote -> Element a
imageNoteView zknote =
    let
        fileurl =
            "/file/" ++ zkNoteIdToString zknote.id
    in
    E.column [ EBd.width 1, E.spacing 5, E.padding 5 ]
        [ link (Just zknote.title) ("/note/" ++ zkNoteIdToString zknote.id) [ E.text zknote.title ]
        , imageView zknote.title fileurl Nothing []
        ]


noteFile : Data.Sysids -> String -> Data.ZkNote -> Element a
noteFile si filename zknote =
    let
        suffix =
            String.split "." filename
                |> List.drop 1
                |> List.reverse
                |> List.head
    in
    case suffix of
        Nothing ->
            E.text filename

        Just s ->
            case String.toLower s of
                "mp3" ->
                    audioNoteView si zknote

                "m4a" ->
                    audioNoteView si zknote

                "opus" ->
                    audioNoteView si zknote

                "mp4" ->
                    videoNoteView zknote

                "webm" ->
                    videoNoteView zknote

                "mkv" ->
                    videoNoteView zknote

                "jpg" ->
                    imageNoteView zknote

                "gif" ->
                    imageNoteView zknote

                "png" ->
                    imageNoteView zknote

                _ ->
                    link (Just zknote.title) ("/note/" ++ zkNoteIdToString zknote.id) [ E.text zknote.title ]


noteView : Data.Sysids -> NoteCache -> String -> List (Element a) -> Element a
noteView si noteCache id _ =
    case
        zkNoteIdFromString id
            |> Result.toMaybe
            |> Maybe.andThen (NC.getNote noteCache)
    of
        Just zne ->
            if zne.zknote.isFile then
                noteFile si zne.zknote.title zne.zknote

            else
                E.link
                    [ E.htmlAttribute (HA.style "display" "inline-flex") ]
                    { url = "/note/" ++ id
                    , label =
                        E.paragraph
                            [ EF.color (E.rgb255 0 0 255)
                            , E.htmlAttribute (HA.style "overflow-wrap" "break-word")
                            , E.htmlAttribute (HA.style "word-break" "break-word")
                            ]
                            [ E.text zne.zknote.title ]
                    }

        -- , E.column [] (List.map (.othername >> Maybe.withDefault "" >> E.text) zne.links)
        Nothing ->
            E.text <| "note " ++ id


videoView : Int -> String -> Maybe String -> Maybe String -> Maybe String -> List (Element a) -> Element a
videoView maxw url mbtext mbwidth mbheight renderedChildren =
    let
        attribs =
            List.filterMap identity
                [ mbwidth
                    |> Maybe.andThen (\s -> String.toInt s)
                    |> Maybe.map (\i -> HA.width (min i maxw))
                    |> ME.orElse (Just <| HA.width maxw)
                , mbheight
                    |> Maybe.andThen (\s -> String.toInt s)
                    |> Maybe.map (\i -> HA.height i)
                , Just <| HA.controls True
                ]
    in
    E.el [] <|
        E.html <|
            Html.video
                attribs
                [ Html.source
                    [ HA.attribute "src" url ]
                    [ mbtext
                        |> Maybe.map (\s -> Html.text s)
                        |> Maybe.withDefault (Html.text "video")
                    ]
                ]


cellView : CellDict -> List (Element a) -> String -> String -> (String -> String -> a) -> Element a
cellView (CellDict cellDict) renderedChildren name schelmeCode onchanged =
    E.column
        [ EBd.shadow
            { offset = ( 0.3, 0.3 )
            , size = 2
            , blur = 0.5
            , color = E.rgba255 0 0 0 0.22
            }
        , E.padding 20
        , E.spacing 30
        , E.centerX
        , EF.center
        ]
        (E.row [ E.spacing 20 ]
            [ E.el
                [ EF.bold
                , EF.size 30
                ]
                (E.text name)
            , EI.text []
                { onChange = onchanged name
                , placeholder = Nothing
                , label = EI.labelHidden name
                , text =
                    cellDict
                        |> Dict.get name
                        |> Maybe.map .code
                        |> Maybe.withDefault "<err>"
                }
            , cellDict
                |> Dict.get name
                |> Maybe.map showRunState
                |> Maybe.withDefault
                    (E.text "<reserr>")
            ]
            :: renderedChildren
        )


showRunState : DictCell -> Element a
showRunState cell =
    E.el [ E.width E.fill ] <|
        case cell.runstate of
            RsOk term ->
                E.text <| showTerm term

            RsErr s ->
                E.el [ EF.color <| E.rgb 1 0.1 0.1 ] <| E.text <| "err: " ++ s

            RsUnevaled ->
                E.text <| "unevaled"

            RsBlocked _ id ->
                E.text <| "blocked on cell: " ++ dictCcr.showId id


rawTextToId : String -> String
rawTextToId rawText =
    rawText
        |> String.toLower
        |> String.replace " " ""


heading : { level : Block.HeadingLevel, rawText : String, children : List (Element msg) } -> Element msg
heading { level, rawText, children } =
    E.paragraph
        [ EF.size
            (case level of
                Block.H1 ->
                    36

                Block.H2 ->
                    24

                _ ->
                    20
            )
        , EF.bold
        , EF.family [ EF.typeface "Montserrat" ]
        , ER.heading (Block.headingLevelToInt level)
        , E.htmlAttribute
            (HA.attribute "name" (rawTextToId rawText))
        , E.htmlAttribute
            (HA.id (rawTextToId rawText))
        ]
        children


codeSpan : String -> Element msg
codeSpan snippet =
    E.el
        [ EBk.color
            (E.rgba 0 0 0 0.13)
        , EBd.rounded 2
        , E.paddingXY 5 3
        , EF.family [ EF.monospace ]
        ]
        (E.text snippet)


codeBlock : { body : String, language : Maybe String } -> Element msg
codeBlock details =
    E.column
        [ EBk.color (E.rgba 0 0 0 0.13)
        , E.padding 5
        , EF.family [ EF.monospace ]
        ]
        [ E.html <|
            Html.div
                [ HA.style "white-space" "pre-wrap"
                , HA.style "word-break" "break-word"
                ]
                [ Html.text <|
                    details.body
                ]
        ]


linkDict : String -> Dict String String
linkDict markdown =
    -- build a dict of description->url
    let
        parsedmd =
            markdown
                |> Markdown.Parser.parse
                |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
    in
    case parsedmd of
        Err _ ->
            Dict.empty

        Ok blocks ->
            inlineFoldl
                (\inline links ->
                    case inline of
                        Block.Link str mbdesc moreinlines ->
                            case mbdesc of
                                Just desc ->
                                    ( desc, str )
                                        :: links

                                Nothing ->
                                    case moreinlines of
                                        [ Block.Text desc2 ] ->
                                            ( desc2, str )
                                                :: links

                                        _ ->
                                            links

                        _ ->
                            links
                )
                []
                blocks
                |> Dict.fromList


noteIds : String -> ZniSet
noteIds markdown =
    -- build a dict of description->url
    let
        parsedmd =
            markdown
                |> Markdown.Parser.parse
                |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
    in
    case parsedmd of
        Err _ ->
            emptyZniSet

        Ok blocks ->
            foldl
                (\block ids ->
                    case block of
                        Block.HtmlBlock (Block.HtmlElement tag attr childs) ->
                            case tag of
                                "note" ->
                                    case
                                        List.foldl
                                            (\i mbv ->
                                                if i.name == "id" then
                                                    zkNoteIdFromString i.value |> Result.toMaybe

                                                else
                                                    Nothing
                                            )
                                            Nothing
                                            attr
                                    of
                                        Just id ->
                                            TSet.insert id ids

                                        Nothing ->
                                            ids

                                "panel" ->
                                    let
                                        am =
                                            Dict.fromList <| List.map (\trib -> ( trib.name, trib.value )) attr
                                    in
                                    am
                                        |> Dict.get "noteid"
                                        |> Maybe.andThen (zkNoteIdFromString >> Result.toMaybe)
                                        |> Maybe.map
                                            (\id ->
                                                TSet.insert id ids
                                            )
                                        |> Maybe.withDefault ids

                                _ ->
                                    ids

                        _ ->
                            ids
                )
                emptyZniSet
                blocks
                |> (\bids ->
                        inlineFoldl
                            (\inline ids ->
                                case inline of
                                    Block.HtmlInline (Block.HtmlElement tag attr childs) ->
                                        case
                                            ( tag
                                            , List.foldl
                                                (\i mbv ->
                                                    if i.name == "id" then
                                                        zkNoteIdFromString i.value |> Result.toMaybe

                                                    else
                                                        Nothing
                                                )
                                                Nothing
                                                attr
                                            )
                                        of
                                            ( "note", Just id ) ->
                                                TSet.insert id ids

                                            _ ->
                                                ids

                                    _ ->
                                        ids
                            )
                            bids
                            blocks
                   )
