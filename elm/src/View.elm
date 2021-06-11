module View exposing (Command(..), Model, Msg(..), initFull, initSzn, update, view)

import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Common
import Data
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region as ER
import Html exposing (Attribute, Html)
import Html.Attributes
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..))
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import MdCommon as MC
import Schelme.Show exposing (showTerm)
import TangoColors as TC


type Msg
    = OnSchelmeCodeChanged String String
    | DonePress
    | SwitchPress Int
    | Noop


type alias Model =
    { id : Maybe Int
    , pubid : Maybe String
    , title : String
    , md : String
    , cells : CellDict
    , panelNote : Maybe Data.ZkNote
    , zklinks : List Data.EditLink
    }


type Command
    = None
    | Done
    | Switch Int


zkLinkName : Data.ZkLink -> Int -> String
zkLinkName zklink noteid =
    if noteid == zklink.from then
        zklink.toname |> Maybe.withDefault (String.fromInt zklink.to)

    else if noteid == zklink.to then
        zklink.fromname |> Maybe.withDefault (String.fromInt zklink.from)

    else
        "link error"


showZkl : Int -> Data.EditLink -> Element Msg
showZkl id zkl =
    E.row [ E.spacing 8, E.width E.fill ]
        [ case zkl.direction of
            Data.To ->
                E.text "->"

            Data.From ->
                E.text "<-"
        , E.row
            [ E.clipX
            , E.centerY
            , E.height E.fill
            , E.width E.fill
            ]
            [ E.text (zkl.othername |> Maybe.withDefault "")
            ]
        , EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just (SwitchPress zkl.otherid), label = E.text "↗" }
        ]


view : Int -> Model -> Bool -> Element Msg
view maxw model loggedin =
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
            \x -> E.column [ E.width E.fill ] (List.reverse x)

           else
            E.row [ E.width E.fill ]
          )
            [ case model.panelNote of
                Just panel ->
                    E.el
                        [ if narrow then
                            E.width E.fill

                          else
                            E.width <| E.px 400
                        , E.alignTop
                        ]
                        (case
                            MC.markdownView
                                (MC.mkRenderer (\_ -> Noop) mw model.cells False OnSchelmeCodeChanged)
                                panel.content
                         of
                            Ok rendered ->
                                E.column
                                    [ E.spacing 30
                                    , E.padding 80
                                    , E.width E.fill
                                    , E.centerX
                                    ]
                                    rendered

                            Err errors ->
                                E.text errors
                        )

                Nothing ->
                    E.none
            , E.column
                [ E.width (E.fill |> E.maximum 1000), E.centerX, E.padding 10, E.alignTop ]
                [ E.row [ E.centerX ] [ E.text model.title ]
                , E.row [ E.width E.fill ]
                    [ case MC.markdownView (MC.mkRenderer (\_ -> Noop) mw model.cells False OnSchelmeCodeChanged) model.md of
                        Ok rendered ->
                            E.column
                                [ E.spacing 30
                                , E.padding 80
                                , E.width E.fill
                                , E.centerX
                                ]
                                rendered

                        Err errors ->
                            E.text errors
                    ]
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


initFull : Data.ZkNoteEdit -> Model
initFull zknaa =
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
    , pubid = zknote.pubid
    , title = zknote.title
    , md = zknote.content
    , cells = getCd cc
    , panelNote = zknaa.panelNote
    , zklinks = zknaa.links
    }


initSzn : Data.SaveZkNote -> List Data.EditLink -> Maybe Data.ZkNote -> Model
initSzn zknote links mbpanelnote =
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
    , pubid = zknote.pubid
    , title = zknote.title
    , md = zknote.content
    , cells = getCd cc
    , panelNote = mbpanelnote
    , zklinks = links
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
