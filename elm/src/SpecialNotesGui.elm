module SpecialNotesGui exposing (..)

import ArchiveListing exposing (Command)
import Color
import Common
import Data exposing (AndOr(..), LzLink, SaveLzLink, SearchMod(..), TagSearch(..), ZkListNote, ZkNoteId(..))
import DataUtil exposing (NlLink, zkNoteIdToString)
import Dict exposing (Dict)
import Element as E
import Element.Background as EBg
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import SearchUtil exposing (showTagSearch)
import Set
import SnListEdit as SLE exposing (DragDropWhat(..), nllDndSubscriptions)
import SpecialNotes as SN exposing (CompletedSync, SpecialNote, StyleColor, StylePalette)
import TDict
import Time
import Util


type Msg
    = CopySearchPress
    | CopySyncSearchPress Bool
    | GraphFocusClick
    | SlideShowClick
    | ToMarkdownPress
    | ChangeColorClick ScElement StyleColor
    | ColorChanged ScElement StyleColor
    | SLEMsg SLE.Msg
    | Noop


type Command
    = CopySearch (List TagSearch)
    | CopySyncSearch TagSearch
    | GraphFocus
    | DndCmd (Cmd Msg)
    | SlideShow (Maybe ZkNoteId) (List NlLink)
    | ToMarkdown String
    | PickColor Color.Color (Color.Color -> Msg)
    | None


type SpecialNoteState
    = SnsSearch (List TagSearch)
    | SnsSync CompletedSync
    | SnsList SLE.Model
    | SnsStylePalette StylePalette


type ScElement
    = ScButtons
    | ScButtonFontColor
    | ScTabs
    | ScBackground
    | ScTabBackground
    | ScFontColor
    | ScSavecolor


defaultStylePalette : StylePalette
defaultStylePalette =
    { buttons = { red = 10, green = 10, blue = 10 }
    , buttonFontColor = { red = 10, green = 10, blue = 10 }
    , tabs = { red = 10, green = 10, blue = 10 }
    , background = { red = 10, green = 10, blue = 10 }
    , tabBackground = { red = 10, green = 10, blue = 10 }
    , fontColor = { red = 10, green = 10, blue = 10 }
    , savecolor = { red = 10, green = 10, blue = 10 }
    }


getScColor : ScElement -> StylePalette -> StyleColor
getScColor se stylePalette =
    case se of
        ScButtons ->
            stylePalette.buttons

        ScButtonFontColor ->
            stylePalette.buttonFontColor

        ScTabs ->
            stylePalette.tabs

        ScBackground ->
            stylePalette.background

        ScTabBackground ->
            stylePalette.tabBackground

        ScFontColor ->
            stylePalette.fontColor

        ScSavecolor ->
            stylePalette.savecolor


getSeName : ScElement -> String
getSeName se =
    case se of
        ScButtons ->
            "buttons"

        ScButtonFontColor ->
            "button font color"

        ScTabs ->
            "tabs"

        ScBackground ->
            "background"

        ScTabBackground ->
            "tab background"

        ScFontColor ->
            "font color"

        ScSavecolor ->
            "savecolor"


getColor : ScElement -> StylePalette -> E.Color
getColor se sp =
    let
        c =
            getScColor se sp
    in
    E.rgb255 c.red c.green c.blue


setScColor : ScElement -> StyleColor -> StylePalette -> StylePalette
setScColor styleColors color stylePalette =
    case styleColors of
        ScButtons ->
            { stylePalette | buttons = color }

        ScButtonFontColor ->
            { stylePalette | buttonFontColor = color }

        ScTabs ->
            { stylePalette | tabs = color }

        ScBackground ->
            { stylePalette | background = color }

        ScTabBackground ->
            { stylePalette | tabBackground = color }

        ScFontColor ->
            { stylePalette | fontColor = color }

        ScSavecolor ->
            { stylePalette | savecolor = color }


initSpecialNoteStateLz : ZkNoteId -> SN.SpecialNote -> List LzLink -> SpecialNoteState
initSpecialNoteStateLz znid sn lzls =
    initSpecialNoteState sn (mklzList znid lzls)


initSpecialNoteState : SN.SpecialNote -> List NlLink -> SpecialNoteState
initSpecialNoteState sn lzls =
    case sn of
        SN.SnSearch tagSearch ->
            SnsSearch tagSearch

        SN.SnSync completedSync ->
            SnsSync completedSync

        SN.SnList notegraph ->
            SnsList (SLE.init notegraph lzls)

        SN.SnStylePalette stylePalette ->
            SnsStylePalette stylePalette


dirty : Maybe SpecialNoteState -> Maybe SpecialNoteState -> Bool
dirty new old =
    case ( new, old ) of
        ( Just (SnsList n), Just (SnsList o) ) ->
            SLE.dirty n o

        _ ->
            new /= old


getSpecialNote : SpecialNoteState -> SpecialNote
getSpecialNote sns =
    case sns of
        SnsSearch tagSearch ->
            SN.SnSearch tagSearch

        SnsSync completedSync ->
            SN.SnSync completedSync

        SnsList slem ->
            SN.SnList slem.ng

        SnsStylePalette stylePalette ->
            SN.SnStylePalette stylePalette


sngSubscriptions : SpecialNoteState -> List (Sub Msg)
sngSubscriptions sns =
    case sns of
        SnsSearch _ ->
            []

        SnsSync _ ->
            []

        SnsList slem ->
            List.map (Sub.map SLEMsg) <| nllDndSubscriptions slem

        SnsStylePalette _ ->
            []


saveLzLinks : ZkNoteId -> SpecialNoteState -> List SaveLzLink
saveLzLinks this sns =
    case sns of
        SnsSearch _ ->
            []

        SnsSync _ ->
            []

        SnsList slem ->
            List.foldl
                (\nll ( to, lst ) ->
                    ( nll.id
                    , { to = to
                      , from = nll.id
                      , delete = Just False
                      }
                        :: lst
                    )
                )
                ( this, [] )
                (filterNotes this slem.nlls)
                |> Tuple.second

        SnsStylePalette _ ->
            []


guiSn :
    Time.Zone
    -> Int
    -> SpecialNoteState
    -> E.Element Msg
guiSn zone fontsize snote =
    case snote of
        SnsSearch tagsearches ->
            E.row
                [ E.alignTop
                , E.width E.fill
                ]
                [ E.paragraph
                    [ E.htmlAttribute (HA.style "overflow-wrap" "break-word")
                    , E.htmlAttribute (HA.style "word-break" "break-word")
                    ]
                    (tagsearches
                        |> List.map (showTagSearch >> E.text)
                    )
                , EI.button (E.alignRight :: Common.buttonStyle)
                    { onPress = Just CopySearchPress
                    , label = E.text ">"
                    }
                ]

        SnsSync completedSync ->
            E.wrappedRow [ E.alignTop, E.width E.fill ]
                [ E.column []
                    [ E.text "sync"
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "start:"
                        , case completedSync.after of
                            Just s ->
                                E.text (Util.showDateTime zone (Time.millisToPosix s))

                            Nothing ->
                                E.text "-∞"
                        ]
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "end:"
                        , E.text (Util.showDateTime zone (Time.millisToPosix completedSync.now))
                        ]
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "local server id:"
                        , completedSync.local |> Maybe.withDefault "" |> E.text
                        ]
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "remote server id:"
                        , completedSync.remote |> Maybe.withDefault "" |> E.text
                        ]
                    ]
                , E.column [ E.alignRight, E.spacing 3 ]
                    [ EI.button (E.alignRight :: Common.buttonStyle)
                        { onPress = Just <| CopySyncSearchPress True
                        , label = E.text "search notes synced from remote >"
                        }
                    , EI.button (E.alignRight :: Common.buttonStyle)
                        { onPress = Just <| CopySyncSearchPress False
                        , label = E.text "search notes synced to remote >"
                        }
                    ]
                ]

        SnsList slem ->
            E.column []
                [ E.row [ E.spacing 3 ]
                    [ EI.button Common.buttonStyle
                        { onPress = Just <| GraphFocusClick
                        , label = E.text "add to list"
                        }
                    , EI.button Common.buttonStyle
                        { onPress = Just <| SlideShowClick
                        , label = E.text "slideshow"
                        }
                    , EI.button Common.buttonStyle
                        { onPress = Just <| ToMarkdownPress
                        , label = E.text "to markdown"
                        }
                    ]
                , E.map SLEMsg <| SLE.view fontsize slem
                ]

        SnsStylePalette sp ->
            E.column []
                (List.map
                    (\sce ->
                        E.row []
                            [ E.text (getSeName sce)
                            , E.row [ E.width <| E.px 15, E.height <| E.px 15, EBg.color <| getColor sce sp, EBd.width 1 ] []
                            , EI.button Common.buttonStyle
                                { onPress = Just <| ChangeColorClick sce (getScColor sce sp)
                                , label = E.text ">"
                                }
                            ]
                    )
                    [ ScButtons
                    , ScButtonFontColor
                    , ScTabs
                    , ScBackground
                    , ScTabBackground
                    , ScFontColor
                    , ScSavecolor
                    ]
                )


lzToDict : Dict String LzLink -> DataUtil.LzlDict
lzToDict lzls =
    lzls
        |> Dict.values
        |> List.foldl
            (\lzl lzll ->
                TDict.insert lzl.to lzl lzll
            )
            DataUtil.emptyLzlDict


lzToDict2 : List LzLink -> DataUtil.LzlDict
lzToDict2 lzls =
    lzls
        |> List.foldl
            (\lzl lzll ->
                TDict.insert lzl.to lzl lzll
            )
            DataUtil.emptyLzlDict


addNotes :
    ZkNoteId
    -> List ZkListNote
    -> SpecialNoteState
    -> SpecialNoteState
addNotes this zlns sns =
    case sns of
        SnsList slem ->
            -- disallow linking 'this' and disallow multiple occurances
            -- of notes.
            let
                notes =
                    List.map (\zln -> { id = zln.id, title = zln.title }) zlns
            in
            case slem.ng.currentUuid of
                Nothing ->
                    SnsList
                        { slem | nlls = filterNotes this <| notes ++ slem.nlls }

                Just uuid ->
                    let
                        zni =
                            Zni uuid

                        nlnks =
                            List.foldr
                                (\n lst ->
                                    if n.id == zni then
                                        n :: notes ++ lst

                                    else
                                        n :: lst
                                )
                                []
                                slem.nlls

                        flnks =
                            filterNotes this nlnks
                    in
                    SnsList { slem | nlls = flnks }

        SnsSearch s ->
            SnsSearch s

        SnsSync s ->
            SnsSync s

        SnsStylePalette s ->
            SnsStylePalette s


filterNotes : ZkNoteId -> List NlLink -> List NlLink
filterNotes this nlls =
    -- filter out duplicates and 'this'
    List.foldr
        (\nll ( nls, nlst ) ->
            let
                nllid =
                    zkNoteIdToString nll.id
            in
            if Set.member nllid nls then
                ( nls, nlst )

            else
                ( Set.insert nllid nls, nll :: nlst )
        )
        ( Set.singleton (zkNoteIdToString this), [] )
        nlls
        |> Tuple.second


mklzList : ZkNoteId -> List Data.LzLink -> List { id : ZkNoteId, title : String }
mklzList this links =
    List.reverse <| dolst this (lzToDict2 links) []


dolst : ZkNoteId -> DataUtil.LzlDict -> List { id : ZkNoteId, title : String } -> List { id : ZkNoteId, title : String }
dolst toid lz2d lst =
    case TDict.get toid lz2d of
        Nothing ->
            lst

        Just l ->
            dolst l.from
                (TDict.remove toid lz2d)
                ({ id = l.from, title = l.fromname } :: lst)


syncSearch : Bool -> SN.CompletedSync -> TagSearch
syncSearch fromremote csync =
    case csync.after of
        Just a ->
            Boolex
                { ts1 =
                    let
                        st =
                            SearchTerm
                                { mods = [ Server ]
                                , term = "local"
                                }
                    in
                    if fromremote then
                        Not
                            { ts = st
                            }

                    else
                        st
                , ao = And
                , ts2 =
                    Boolex
                        { ts1 =
                            SearchTerm
                                { mods = [ After, Mod ]
                                , term = String.fromInt a
                                }
                        , ao = And
                        , ts2 =
                            SearchTerm
                                { mods = [ Before, Mod ]
                                , term = String.fromInt csync.now
                                }
                        }
                }

        Nothing ->
            Boolex
                { ts1 =
                    let
                        st =
                            SearchTerm
                                { mods = [ Server ]
                                , term = "local"
                                }
                    in
                    if fromremote then
                        Not
                            { ts = st
                            }

                    else
                        st
                , ao = And
                , ts2 =
                    SearchTerm
                        { mods = [ Before, Mod ]
                        , term = String.fromInt csync.now
                        }
                }


updateSn : Msg -> SpecialNoteState -> ( SpecialNoteState, Command )
updateSn msg snote =
    case snote of
        SnsSearch tagsearches ->
            case msg of
                GraphFocusClick ->
                    ( SnsSearch tagsearches, None )

                ChangeColorClick _ _ ->
                    ( SnsSearch tagsearches, None )

                ColorChanged _ _ ->
                    ( SnsSearch tagsearches, None )

                SlideShowClick ->
                    ( SnsSearch tagsearches, None )

                CopySearchPress ->
                    ( SnsSearch tagsearches, CopySearch tagsearches )

                ToMarkdownPress ->
                    ( SnsSearch tagsearches, None )

                CopySyncSearchPress _ ->
                    ( SnsSearch tagsearches, None )

                SLEMsg _ ->
                    ( SnsSearch tagsearches, None )

                Noop ->
                    ( SnsSearch tagsearches, None )

        SnsSync completedSync ->
            case msg of
                GraphFocusClick ->
                    ( SnsSync completedSync, None )

                ChangeColorClick _ _ ->
                    ( SnsSync completedSync, None )

                ColorChanged _ _ ->
                    ( SnsSync completedSync, None )

                SlideShowClick ->
                    ( SnsSync completedSync, None )

                CopySearchPress ->
                    ( SnsSync completedSync, None )

                ToMarkdownPress ->
                    ( SnsSync completedSync, None )

                CopySyncSearchPress fromremote ->
                    ( SnsSync completedSync, CopySyncSearch (syncSearch fromremote completedSync) )

                SLEMsg _ ->
                    ( SnsSync completedSync, None )

                Noop ->
                    ( SnsSync completedSync, None )

        SnsList slem ->
            case msg of
                GraphFocusClick ->
                    ( SnsList slem, GraphFocus )

                ChangeColorClick _ _ ->
                    ( SnsList slem, GraphFocus )

                ColorChanged _ _ ->
                    ( SnsList slem, GraphFocus )

                SlideShowClick ->
                    ( SnsList slem, SlideShow (Maybe.map Zni slem.ng.currentUuid) slem.nlls )

                ToMarkdownPress ->
                    ( SnsList slem
                    , ToMarkdown
                        (List.map
                            (\nl ->
                                "<note id=\"" ++ zkNoteIdToString nl.id ++ "\" text=\"" ++ nl.title ++ "\"/>"
                            )
                            slem.nlls
                            |> List.intersperse "\n"
                            |> String.concat
                        )
                    )

                CopySearchPress ->
                    ( SnsList slem, None )

                CopySyncSearchPress _ ->
                    ( SnsList slem, None )

                SLEMsg m ->
                    let
                        nm =
                            SLE.update m slem
                    in
                    ( SnsList nm, DndCmd <| Cmd.map SLEMsg <| SLE.commands nm )

                Noop ->
                    ( SnsList slem, None )

        SnsStylePalette ssp ->
            case msg of
                GraphFocusClick ->
                    ( SnsStylePalette ssp, None )

                ChangeColorClick se sc ->
                    ( SnsStylePalette ssp
                    , PickColor
                        (Color.rgb255 sc.red sc.green sc.blue)
                        (\c ->
                            ColorChanged
                                se
                                (let
                                    rgba =
                                        Color.toRgba c
                                 in
                                 { red = rgba.red * 255 |> round, green = rgba.green * 255 |> round, blue = rgba.blue * 255 |> round }
                                )
                        )
                    )

                ColorChanged se sc ->
                    ( SnsStylePalette (setScColor se sc ssp), None )

                SlideShowClick ->
                    ( SnsStylePalette ssp, None )

                CopySearchPress ->
                    ( SnsStylePalette ssp, None )

                ToMarkdownPress ->
                    ( SnsStylePalette ssp, None )

                CopySyncSearchPress _ ->
                    ( SnsStylePalette ssp, None )

                SLEMsg _ ->
                    ( SnsStylePalette ssp, None )

                Noop ->
                    ( SnsStylePalette ssp, None )
