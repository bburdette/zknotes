module Main exposing (main)

import BlogEdit as Edit
import Browser
import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Dict exposing (Dict)
import Element exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region
import Html exposing (Attribute, Html)
import Html.Attributes
import Http
import Login
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..))
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import PublicInterface as PI
import Random exposing (Seed, initialSeed)
import Schelme.Show exposing (showTerm)
import UserInterface as UI
import Util


type Msg
    = LoginMsg Login.Msg
    | EditMsg Edit.Msg
    | ReplyData (Result Http.Error UI.ServerResponse)
    | PublicReplyData (Result Http.Error PI.ServerResponse)


type State
    = Login Login.Model
    | Edit Edit.Model


type alias Flags =
    { seed : Int
    , location : String
    , useragent : String
    , debugstring : String
    , width : Int
    , height : Int
    }


type alias Model =
    { state : State
    , size : Util.Size
    , location : String
    }


view : Model -> { title : String, body : List (Html Msg) }
view model =
    { title = "mah bloag!"
    , body =
        [ Element.layout [] <|
            case model.state of
                Login lem ->
                    Element.map LoginMsg <| Login.view model.size lem

                Edit em ->
                    Element.map EditMsg <| Edit.view em
        ]
    }


main : Platform.Program Flags Model Msg
main =
    Browser.document
        { init =
            \flags ->
                ( { state = Login <| Login.initialModel Nothing "mahbloag" (initialSeed (flags.seed + 7))
                  , size = { width = flags.width, height = flags.height }
                  , location = flags.location
                  }
                , Cmd.none
                )
        , view = view
        , update = update
        , subscriptions = \model -> Sub.none
        }


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model.state ) of
        ( LoginMsg lm, Login ls ) ->
            let
                ( lmod, lcmd ) =
                    Login.update lm ls
            in
            case lcmd of
                Login.None ->
                    ( { model | state = Login lmod }, Cmd.none )

                Login.Register ->
                    ( { model | state = Login lmod }
                    , Http.post
                        { url = model.location ++ "/user"
                        , body =
                            Http.jsonBody
                                (UI.encodeSendMsg (UI.Register ls.email)
                                    lmod.userId
                                    lmod.password
                                )
                        , expect = Http.expectJson ReplyData UI.serverResponseDecoder
                        }
                    )

                Login.Login ->
                    ( { model | state = Login lmod }
                    , Http.post
                        { url = model.location ++ "/user"
                        , body =
                            Http.jsonBody
                                (UI.encodeSendMsg
                                    UI.Login
                                    lmod.userId
                                    lmod.password
                                )
                        , expect = Http.expectJson ReplyData UI.serverResponseDecoder
                        }
                    )

        ( EditMsg em, Edit es ) ->
            let
                ( emod, ecmd ) =
                    Edit.update em es
            in
            ( { model | state = Edit emod }, Cmd.none )

        ( _, _ ) ->
            ( model, Cmd.none )
