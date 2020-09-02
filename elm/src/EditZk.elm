module EditZk exposing (Command(..), Model, Msg(..), addedZkMember, deletedZkMember, initExample, initFull, initNew, setId, update, view)

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
    | OnNameChanged String
    | OnAddMemberNameChanged String
    | SavePress
    | DonePress
    | DeletePress
    | ViewPress
    | DeleteMemberPress String
    | AddMemberPress


type alias Model =
    { id : Maybe Int
    , name : String
    , md : String
    , cells : CellDict
    , addMemberName : String
    , members : List String
    }


type Command
    = None
    | Save Data.SaveZk
    | Done
    | View Data.SaveZk
    | Delete Int
    | AddZkMember Data.ZkMember
    | DeleteZkMember Data.ZkMember


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
            { onChange = OnNameChanged
            , text = model.name
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "name")
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
        , E.row []
            [ EI.button Common.buttonStyle { onPress = Just AddMemberPress, label = E.text "Add Member" }
            , EI.text []
                { onChange = OnAddMemberNameChanged
                , text = model.addMemberName
                , placeholder = Nothing
                , label = EI.labelHidden "name"
                }
            ]
        , E.row [ E.spacing 8, E.padding 8 ]
            [ E.column [ E.alignTop ]
                [ E.text "members:"
                ]
            , E.column [ E.spacing 8 ] <|
                List.map
                    (\name ->
                        E.row [ E.spacing 8 ]
                            [ E.text name
                            , EI.button Common.buttonStyle { onPress = Just (DeleteMemberPress name), label = E.text "Delete" }
                            ]
                    )
                    model.members
            ]
        ]


initFull : Data.Zk -> List String -> Model
initFull zk members =
    let
        cells =
            zk.description
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Just zk.id
    , name = zk.name
    , md = zk.description
    , cells = getCd cc
    , addMemberName = ""
    , members = members
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
    , name = ""
    , md = ""
    , cells = getCd cc
    , addMemberName = ""
    , members = []
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
    , name = "example"
    , md = markdownBody
    , cells = getCd cc
    , addMemberName = ""
    , members = []
    }


addedZkMember : Model -> Data.ZkMember -> Model
addedZkMember model zkm =
    { model | members = List.sort (zkm.name :: model.members) }


deletedZkMember : Model -> Data.ZkMember -> Model
deletedZkMember model zkm =
    { model | members = List.filter (\n -> n /= zkm.name) model.members }


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
                , name = model.name
                , description = model.md
                }
            )

        ViewPress ->
            ( model
            , View
                { id = model.id
                , name = model.name
                , description = model.md
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

        OnNameChanged t ->
            ( { model | name = t }, None )

        OnAddMemberNameChanged t ->
            ( { model | addMemberName = t }, None )

        DeleteMemberPress name ->
            case model.id of
                Just id ->
                    ( model, DeleteZkMember { name = name, zkid = id } )

                Nothing ->
                    ( model, None )

        AddMemberPress ->
            case model.id of
                Just id ->
                    ( model, AddZkMember { name = model.addMemberName, zkid = id } )

                Nothing ->
                    ( model, None )

        OnMarkdownInput newMarkdown ->
            let
                cells =
                    newMarkdown
                        |> mdCells
                        |> Result.withDefault (CellDict Dict.empty)

                ( cc, result ) =
                    evalCellsFully
                        (mkCc cells)
            in
            ( { model
                | md = newMarkdown
                , cells = getCd cc
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
