module View exposing (Command(..), Model, Msg(..), initFull, initSzn, update, view)

import Cellme.Cellme exposing (CellContainer(..), RunState(..), evalCellsFully)
import Cellme.DictCellme exposing (CellDict(..), getCd, mkCc)
import Common
import Data exposing (ZkNote, ZkNoteId)
import DataUtil exposing (FileUrlInfo, zkNoteIdToString, zniEq)
import Dict
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Font as Font
import Element.Input as EI
import Markdown.Block exposing (ListItem(..), Task(..))
import MdCommon as MC
import NoteCache as NC exposing (CacheEntry(..), NoteCache)
import TangoColors as TC
import Time
import Util


type Msg
    = OnSchelmeCodeChanged String String
    | DonePress
    | SwitchPress ZkNoteId
    | Noop


type alias Model =
    { id : Maybe ZkNoteId
    , fui : FileUrlInfo
    , pubid : Maybe String
    , title : String
    , showtitle : Bool
    , md : String
    , cells : CellDict
    , panelNote : Maybe ZkNoteId
    , zklinks : List Data.EditLink
    , createdate : Maybe Int
    , changeddate : Maybe Int
    , zknote : Maybe ZkNote
    }


type Command
    = None
    | Done
    | Switch ZkNoteId


zkLinkName : Data.ZkLink -> ZkNoteId -> String
zkLinkName zklink noteid =
    if zniEq noteid zklink.from then
        zklink.toname |> Maybe.withDefault (zkNoteIdToString zklink.to)

    else if zniEq noteid zklink.to then
        zklink.fromname |> Maybe.withDefault (zkNoteIdToString zklink.from)

    else
        "link error"


showZkl : ZkNoteId -> Data.EditLink -> Element Msg
showZkl id zkl =
    E.row [ E.spacing 8, E.width E.fill ]
        [ case zkl.direction of
            Data.To ->
                E.text "->"

            Data.From ->
                E.text "<-"
        , E.paragraph
            [ E.clipX
            , E.centerY
            , E.height E.fill
            , E.width E.fill
            ]
            [ E.text (zkl.othername |> Maybe.withDefault "")
            ]
        , EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just (SwitchPress zkl.otherid), label = E.text "↗" }
        ]


view : Time.Zone -> Int -> NoteCache -> Model -> Bool -> Element Msg
view zone maxw noteCache model loggedin =
    let
        mw =
            min maxw 1000 - 160

        narrow =
            maxw < 1300
    in
    E.column [ E.width E.fill ]
        [ if loggedin then
            E.row []
                [ EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "Edit" }
                ]

          else
            E.none
        , (if narrow then
            \x -> E.column [ E.width E.fill ] [ E.column [ E.centerX ] (List.reverse x) ]

           else
            \x -> E.row [ E.width E.fill ] [ E.row [ E.centerX, E.spacing 10 ] x ]
          )
            [ model.panelNote
                |> Maybe.andThen (NC.getNote noteCache)
                |> Maybe.map
                    (\ce ->
                        case ce of
                            ZNAL pn ->
                                E.el
                                    [ if narrow then
                                        E.width E.fill

                                      else
                                        E.width <| E.px 300
                                    , E.alignTop
                                    , EBk.color TC.darkGrey
                                    , E.padding 10
                                    ]
                                    (case
                                        MC.markdownView
                                            (MC.mkRenderer
                                                { zone = zone
                                                , fui = model.fui
                                                , viewMode = MC.PublicView
                                                , addToSearchMsg = \_ -> Noop
                                                , maxw = mw
                                                , cellDict = model.cells
                                                , showPanelElt = False
                                                , onchanged = OnSchelmeCodeChanged
                                                , noteCache = noteCache
                                                }
                                            )
                                            pn.zknote.content
                                     of
                                        Ok rendered ->
                                            E.column
                                                [ E.spacing 30
                                                , E.width E.fill
                                                , E.centerX
                                                ]
                                                rendered

                                        Err errors ->
                                            E.text errors
                                    )

                            Private ->
                                E.text "private note"

                            NotFound ->
                                E.text "note not found"
                    )
                |> Maybe.withDefault E.none
            , E.column
                [ E.width (E.fill |> E.maximum 1000), E.centerX, E.spacing 20, E.padding 10, E.alignTop ]
                [ if model.showtitle then
                    E.row [ E.centerX ] [ E.paragraph [ Font.bold, Font.size 20 ] [ E.text model.title ] ]

                  else
                    E.none

                -- if has a file, show file view
                , model.zknote
                    |> Maybe.map
                        (\zkn ->
                            case zkn.filestatus of
                                Data.FilePresent ->
                                    MC.noteFile model.fui Nothing model.title zkn

                                Data.FileMissing ->
                                    E.text <| "file missing"

                                Data.NotAFile ->
                                    E.none
                        )
                    |> Maybe.withDefault E.none
                , E.row [ E.width E.fill ]
                    [ case
                        MC.markdownView
                            (MC.mkRenderer
                                { zone = zone
                                , fui = model.fui
                                , viewMode = MC.PublicView
                                , addToSearchMsg = \_ -> Noop
                                , maxw = mw
                                , cellDict = model.cells
                                , showPanelElt = False
                                , onchanged = OnSchelmeCodeChanged
                                , noteCache = noteCache
                                }
                            )
                            model.md
                      of
                        Ok rendered ->
                            E.column
                                [ E.spacing 3
                                , E.width E.fill
                                , E.centerX
                                ]
                                rendered

                        Err errors ->
                            E.text errors
                    ]
                , case ( model.createdate, model.changeddate ) of
                    ( Just cd, Just chd ) ->
                        E.row [ E.width E.fill, Font.italic ]
                            [ E.paragraph []
                                [ E.text "created: "
                                , E.text (Util.showDateTime zone (Time.millisToPosix cd))
                                ]
                            , E.paragraph [ Font.alignRight ]
                                [ E.text "updated: "
                                , E.text (Util.showDateTime zone (Time.millisToPosix chd))
                                ]
                            ]

                    _ ->
                        E.none
                , E.column [ E.centerX, E.width (E.minimum 150 E.shrink), E.spacing 8 ]
                    (model.id
                        |> Maybe.map
                            (\id ->
                                if List.isEmpty model.zklinks then
                                    []

                                else
                                    E.row [ Font.bold ] [ E.text "links" ]
                                        :: List.map
                                            (showZkl id)
                                            model.zklinks
                            )
                        |> Maybe.withDefault []
                    )
                ]
            ]
        ]


initFull : FileUrlInfo -> Data.ZkNoteAndLinks -> Model
initFull fui zknaa =
    let
        zknote =
            zknaa.zknote

        cells =
            zknote.content
                |> MC.mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Just zknote.id
    , fui = fui
    , pubid = zknote.pubid
    , title = zknote.title
    , showtitle = zknote.showtitle
    , md = zknote.content
    , cells = getCd cc
    , panelNote = zknote.content |> MC.mdPanel |> Maybe.map .noteid
    , zklinks = zknaa.links
    , createdate = Just zknote.createdate
    , changeddate = Just zknote.changeddate
    , zknote = Just zknote
    }


initSzn : FileUrlInfo -> Data.SaveZkNote -> Maybe Int -> Maybe Int -> List Data.EditLink -> Maybe ZkNoteId -> Model
initSzn fui zknote mbcreatedate mbchangeddate links mbpanelid =
    let
        cells =
            zknote.content
                |> MC.mdCells
                |> Result.withDefault (CellDict Dict.empty)

        panels =
            zknote.content
                |> MC.mdPanels
                |> Result.withDefault []

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = zknote.id
    , fui = fui
    , pubid = zknote.pubid
    , title = zknote.title
    , showtitle = zknote.showtitle
    , md = zknote.content
    , cells = getCd cc
    , panelNote = mbpanelid
    , zklinks = links
    , createdate = mbcreatedate
    , changeddate = mbchangeddate
    , zknote = Nothing
    }


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        Noop ->
            ( model, None )

        DonePress ->
            ( model, Done )

        SwitchPress id ->
            ( model, Switch id )

        OnSchelmeCodeChanged name string ->
            let
                (CellDict cd) =
                    model.cells

                ( cc, result ) =
                    evalCellsFully
                        (mkCc
                            (Dict.insert name (MC.defCell string) cd
                                |> CellDict
                            )
                        )
            in
            ( { model
                | cells = getCd cc
              }
            , None
            )
