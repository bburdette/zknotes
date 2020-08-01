module Edit exposing (Command(..), Model, Msg(..), initExample, initFull, initNew, setId, update, view)

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
    = OnMarkdownInput String
    | OnSchelmeCodeChanged String String
    | OnTitleChanged String
    | SavePress
    | DonePress
    | DeletePress
    | ViewPress


type alias Model =
    { id : Maybe Int
    , title : String
    , md : String
    , cells : CellDict
    }


type Command
    = None
    | Save Data.SaveZkNote
    | Done
    | View Data.SaveZkNote
    | Delete Int


view : Model -> Element Msg
view model =
    E.column
        [ E.width E.fill ]
        [ E.row [ E.width E.fill ]
            [ EI.button Common.buttonStyle { onPress = Just SavePress, label = E.text "Save" }
            , EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "Done" }
            , EI.button Common.buttonStyle { onPress = Just ViewPress, label = E.text "View" }
            , EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just DeletePress, label = E.text "Delete" }
            ]
        , EI.text []
            { onChange = OnTitleChanged
            , text = model.title
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "title")
            }
        , E.row [ E.width E.fill ]
            [ EI.multiline [ E.width (E.px 400) ]
                { onChange = OnMarkdownInput
                , text = model.md
                , placeholder = Nothing
                , label = EI.labelHidden "Markdown input"
                , spellcheck = False
                }
            , case markdownView (mkRenderer model.cells OnSchelmeCodeChanged) model.md of
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
initFull blogentry =
    let
        cells =
            blogentry.content
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Just blogentry.id
    , title = blogentry.title
    , md = blogentry.content
    , cells = getCd cc
    }


initNew : Model
initNew =
    let
        cells =
            ""
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Nothing
    , title = ""
    , md = ""
    , cells = getCd cc
    }


initExample : Model
initExample =
    let
        cells =
            markdownBody
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Nothing
    , title = "example"
    , md = markdownBody
    , cells = getCd cc
    }


setId : Model -> Int -> Model
setId model beid =
    { model | id = Just beid }


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SavePress ->
            ( model
            , Save
                { id = model.id
                , title = model.title
                , content = model.md
                }
            )

        ViewPress ->
            ( model
            , View
                { id = model.id
                , title = model.title
                , content = model.md
                }
            )

        DonePress ->
            ( model, Done )

        DeletePress ->
            case model.id of
                Just id ->
                    ( model, Delete id )

                Nothing ->
                    ( model, None )

        OnTitleChanged t ->
            ( { model | title = t }, None )

        OnMarkdownInput newMarkdown ->
            let
                cells =
                    Debug.log "newcells"
                        (newMarkdown
                            |> mdCells
                            |> Result.withDefault (CellDict Dict.empty)
                        )

                ( cc, result ) =
                    evalCellsFully
                        (mkCc cells)
            in
            ( { model
                | md = newMarkdown
                , cells = Debug.log "evaled cells: " <| getCd cc
              }
            , None
            )

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
