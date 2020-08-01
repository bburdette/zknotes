module View exposing (Command(..), Model, Msg(..), initFull, initNew, initSbe, setId, update, view)

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


type Msg
    = OnSchelmeCodeChanged String String
    | DonePress


type alias Model =
    { id : Maybe Int
    , title : String
    , md : String
    , cells : CellDict
    }


type Command
    = None
    | Done


view : Model -> Element Msg
view model =
    E.column
        [ E.width E.fill ]
        [ E.row []
            [ EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "Done" }
            ]
        , E.text model.title
        , E.row [ E.width E.fill ]
            [ case markdownView (mkRenderer model.cells OnSchelmeCodeChanged) model.md of
                Ok rendered ->
                    E.column
                        [ E.spacing 30
                        , E.padding 80
                        , E.width (E.fill |> E.maximum 1000)
                        , E.centerX
                        ]
                        rendered

                Err errors ->
                    E.text errors
            ]
        ]


initFull : Data.FullZkNote -> Model
initFull zknote =
    let
        cells =
            Debug.log "newcells"
                (zknote.content
                    |> mdCells
                    |> Result.withDefault (CellDict Dict.empty)
                )

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Just zknote.id
    , title = zknote.title
    , md = zknote.content
    , cells = Debug.log "evaled cells: " <| getCd cc
    }


initSbe : Data.SaveZkNote -> Model
initSbe zknote =
    let
        cells =
            Debug.log "newcells"
                (zknote.content
                    |> mdCells
                    |> Result.withDefault (CellDict Dict.empty)
                )

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = zknote.id
    , title = zknote.title
    , md = zknote.content
    , cells = Debug.log "evaled cells: " <| getCd cc
    }


initNew : Model
initNew =
    let
        cells =
            Debug.log "newcells"
                (markdownBody
                    |> mdCells
                    |> Result.withDefault (CellDict Dict.empty)
                )

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Nothing
    , title = "example"
    , md = markdownBody
    , cells = Debug.log "evaled cells: " <| getCd cc
    }


setId : Model -> Int -> Model
setId model beid =
    { model | id = Just beid }


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        DonePress ->
            ( model, Done )

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
