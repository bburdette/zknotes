module View exposing
    ( Command(..)
    , Config
    , Model
    , Msg(..)
    , defaultConfig
    , initFull
    , initSzn
    , update
    , view
    )

import Cellme.Cellme exposing (CellContainer(..), RunState(..), evalCellsFully)
import Cellme.DictCellme exposing (CellDict(..), getCd, mkCc)
import Common
import Data exposing (ZkNote, ZkNoteId)
import DataUtil exposing (FileUrlInfo)
import Dict
import EdMarkdown as EM
import Either exposing (..)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Json.Decode as JD
import Markdown.Block exposing (ListItem(..), Task(..))
import MdCommon as MC
import NoteCache as NC exposing (CacheEntry(..), NoteCache)
import SpecialNotes
import SpecialNotesGui as SNG
import TangoColors as TC
import Time
import Util
import ZkCommon as ZC


type Msg
    = OnSchelmeCodeChanged String String
    | DonePress
    | SwitchPress ZkNoteId -- TODO: remove?
    | OnPlaybackEnd
    | SNGMsg SNG.Msg
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


type alias Config =
    { showLinks : Bool
    , alwaysShowTitle : Bool
    , showContents : Bool
    , showMedia : Bool
    , showDates : Bool
    , showPanel : Bool
    , loggedin : Bool
    , autoplay : Bool
    , mobile : Bool
    }


defaultConfig : Config
defaultConfig =
    { showLinks = True
    , alwaysShowTitle = False
    , showContents = True
    , showMedia = True
    , showDates = True
    , showPanel = True
    , loggedin = True
    , autoplay = True
    , mobile = False
    }


type Command
    = None
    | Done
    | Switch ZkNoteId
    | OnPlaybackEnded


showZkl : Data.EditLink -> Element Msg
showZkl zkl =
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
        , EI.button (E.alignRight :: Common.buttonStyle)
            { onPress = Just (SwitchPress zkl.otherid), label = E.text "↗" }
        ]


view : ZC.StylePalette -> Time.Zone -> Int -> NoteCache -> Config -> Model -> Element Msg
view stylePalette zone maxw noteCache config model =
    let
        mw =
            min maxw 1000 - 160

        narrow =
            maxw < 1300

        snedit =
            \sn ->
                E.row
                    [ E.padding 10
                    , EBd.rounded 10
                    , E.width E.fill
                    , EBk.color TC.lightGray
                    , E.height E.fill
                    ]
                    [ E.el
                        [ EBd.color TC.black
                        , EBd.width 1
                        , E.width E.fill
                        , E.centerX
                        , E.padding 3
                        ]
                        (SNG.guiSn zone stylePalette.fontSize sn |> E.map SNGMsg)
                    ]
    in
    E.column [ E.width E.fill ]
        [ if config.loggedin then
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
                |> Maybe.andThen
                    (\pn ->
                        if config.showPanel then
                            Just pn

                        else
                            Nothing
                    )
                |> Maybe.andThen (NC.getCacheEntry noteCache)
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
                                                , mobile = config.mobile
                                                , cellDict = model.cells
                                                , showPanelElt = False
                                                , onchanged = OnSchelmeCodeChanged
                                                , noteCache = noteCache
                                                , isDirty = False
                                                , noop = Noop
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
                [ if model.showtitle || config.alwaysShowTitle then
                    E.row [ E.centerX ]
                        [ E.paragraph
                            [ Font.bold, Font.size 20 ]
                            [ E.text model.title ]
                        ]

                  else
                    E.none

                -- if has a file, show file view
                , model.zknote
                    |> Maybe.andThen
                        (\n ->
                            if config.showMedia then
                                Just n

                            else
                                Nothing
                        )
                    |> Maybe.map
                        (\zkn ->
                            case zkn.filestatus of
                                Data.FilePresent ->
                                    MC.noteFile model.fui mw config.autoplay (Just OnPlaybackEnd) model.title zkn

                                Data.FileMissing ->
                                    E.text <| "file missing"

                                Data.NotAFile ->
                                    E.none
                        )
                    |> Maybe.withDefault E.none
                , if config.showContents then
                    E.row [ E.width E.fill ]
                        [ case
                            MC.markdownView
                                (MC.mkRenderer
                                    { zone = zone
                                    , fui = model.fui
                                    , viewMode = MC.PublicView
                                    , addToSearchMsg = \_ -> Noop
                                    , maxw = mw
                                    , mobile = config.mobile
                                    , cellDict = model.cells
                                    , showPanelElt = False
                                    , onchanged = OnSchelmeCodeChanged
                                    , noteCache = noteCache
                                    , isDirty = False
                                    , noop = Noop
                                    }
                                )
                                model.md
                          of
                            Ok rendered ->
                                E.column
                                    [ E.spacing 8
                                    , E.width E.fill
                                    , E.centerX
                                    ]
                                    rendered

                            Err errors ->
                                E.text errors
                        ]

                  else
                    E.none
                , if config.showDates then
                    case ( model.createdate, model.changeddate ) of
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

                  else
                    E.none
                , if config.showLinks then
                    E.column [ E.centerX, E.width (E.minimum 150 E.shrink), E.spacing 8 ]
                        (if List.isEmpty model.zklinks then
                            []

                         else
                            E.row [ Font.bold ] [ E.text "links" ]
                                :: List.map
                                    showZkl
                                    model.zklinks
                        )

                  else
                    E.none
                ]
            ]
        ]


type InitModel
    = LoadLocal String (Maybe String -> Model)
    | Ready Model


initFull : FileUrlInfo -> Data.ZkNoteAndLinks -> InitModel
initFull fui zknaa =
    let
        zknote =
            zknaa.zknote

        cells =
            zknote.content
                |> MC.mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, _ ) =
            evalCellsFully
                (mkCc cells)

        lrEdMarkdown =
            case JD.decodeString SpecialNotes.specialNoteDecoder zknote.content of
                Ok sn ->
                    case SNG.mbLocalDataId zknote.id sn of
                        Just id ->
                            Right
                                ( id
                                , \mbdata ->
                                    EM.initSpecial (SNG.initSpecialNoteStateLz zknote.id sn mbdata zknaa.lzlinks)
                                )

                        Nothing ->
                            Left <| EM.initSpecial (SNG.initSpecialNoteStateLz zknote.id sn Nothing zknaa.lzlinks)

                Err _ ->
                    Left <| EM.initMd zknote.content

        tost =
            \edm ->
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
    in
    case lrEdMarkdown of
        Left em ->
            Ready <| tost em

        Right ( id, mbf ) ->
            LoadLocal id (mbf >> tost)


initSzn : FileUrlInfo -> Data.SaveZkNote -> Maybe Int -> Maybe Int -> List Data.EditLink -> Maybe ZkNoteId -> Model
initSzn fui zknote mbcreatedate mbchangeddate links mbpanelid =
    let
        cells =
            zknote.content
                |> MC.mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, _ ) =
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

        SNGMsg _ ->
            ( model, None )

        DonePress ->
            ( model, Done )

        SwitchPress id ->
            ( model, Switch id )

        OnSchelmeCodeChanged name string ->
            let
                (CellDict cd) =
                    model.cells

                ( cc, _ ) =
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

        OnPlaybackEnd ->
            ( model, OnPlaybackEnded )
