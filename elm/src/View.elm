module View exposing (Command(..), Model, Msg(..), initFull, initSzn, update, view)

import CellCommon exposing (..)
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
import Schelme.Show exposing (showTerm)
import TangoColors as TC


type Msg
    = OnSchelmeCodeChanged String String
    | DonePress
    | SwitchPress Int


type alias Model =
    { id : Maybe Int
    , pubid : Maybe String
    , title : String
    , md : String
    , cells : CellDict
    , zklinks : List Data.ZkLink
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


showZkl : Int -> Data.ZkLink -> Element Msg
showZkl id zkl =
    let
        ( dir, otherid ) =
            case ( zkl.from == id, zkl.to == id ) of
                ( True, False ) ->
                    ( E.text "->", Just zkl.to )

                ( False, True ) ->
                    ( E.text "<-", Just zkl.from )

                _ ->
                    ( E.text "", Nothing )
    in
    E.row [ E.spacing 8, E.width E.fill ]
        [ dir
        , id
            |> zkLinkName zkl
            |> (\s ->
                    E.row
                        [ E.clipX
                        , E.centerY
                        , E.height E.fill
                        , E.width E.fill
                        ]
                        [ E.text s
                        ]
               )
        , case otherid of
            Just zknoteid ->
                EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just (SwitchPress zknoteid), label = E.text "↗" }

            Nothing ->
                E.none
        ]


view : Int -> Model -> Bool -> Element Msg
view maxw model loggedin =
    let
        mw =
            min maxw 1000 - 160
    in
    E.column
        [ E.width (E.fill |> E.maximum 1000), E.centerX, E.padding 10 ]
        [ if loggedin then
            E.row []
                [ EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "Done" }
                ]

          else
            E.none
        , E.row [ E.centerX ] [ E.text model.title ]
        , E.row [ E.width E.fill ]
            [ case markdownHtmlView (mkHtmlRenderer mw model.cells OnSchelmeCodeChanged) model.md of
                Ok rendered ->
                    E.column
                        [ E.spacing 30
                        , E.padding 80
                        , E.width E.fill
                        , E.centerX
                        ]
                        (List.map E.html
                            rendered
                        )

                Err errors ->
                    E.text errors
            ]
        , E.column [ E.centerX, E.width (E.minimum 150 E.shrink), E.spacing 8 ]
            (model.id
                |> Maybe.map
                    (\id ->
                        E.row [ Font.bold ] [ E.text "links" ]
                            :: List.map
                                (showZkl id)
                                model.zklinks
                    )
                |> Maybe.withDefault []
            )
        ]


initFull : Data.ZkNoteAndAccomplices -> Model
initFull zknaa =
    let
        zknote =
            zknaa.zknote

        cells =
            zknote.content
                |> mdCells
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
    , zklinks = zknaa.links
    }


initSzn : Data.SaveZkNote -> List Data.ZkLink -> Model
initSzn zknote links =
    let
        cells =
            zknote.content
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = zknote.id
    , pubid = zknote.pubid
    , title = zknote.title
    , md = zknote.content
    , cells = getCd cc
    , zklinks = links
    }


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
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
                            (Dict.insert name (defCell string) cd
                                |> CellDict
                            )
                        )
            in
            ( { model
                | cells = getCd cc
              }
            , None
            )
