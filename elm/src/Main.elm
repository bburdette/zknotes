module Main exposing (main)

import BadError
import Browser
import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Data
import Dict exposing (Dict)
import Edit
import EditListing
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
import ShowMessage
import UserInterface as UI
import Util


type Msg
    = LoginMsg Login.Msg
    | BadErrorMsg BadError.Msg
    | EditMsg Edit.Msg
    | EditListingMsg EditListing.Msg
    | ShowMessageMsg ShowMessage.Msg
    | UserReplyData (Result Http.Error UI.ServerResponse)
    | PublicReplyData (Result Http.Error PI.ServerResponse)


type State
    = Login Login.Model
    | Edit Edit.Model Data.Login
    | EditListing EditListing.Model Data.Login
    | BadError BadError.Model State
    | ShowMessage ShowMessage.Model Data.Login


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

                EditListing em _ ->
                    Element.map EditListingMsg <| EditListing.view em

                ShowMessage em _ ->
                    Element.map ShowMessageMsg <| ShowMessage.view em

                Edit em _ ->
                    Element.map EditMsg <| Edit.view em

                BadError em _ ->
                    Element.map BadErrorMsg <| BadError.view em
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


sendUIMsg : String -> Data.Login -> UI.SendMsg -> Cmd Msg
sendUIMsg location login msg =
    Http.post
        { url = location ++ "/user"
        , body =
            Http.jsonBody
                (UI.encodeSendMsg msg
                    login.uid
                    login.pwd
                )
        , expect = Http.expectJson UserReplyData UI.serverResponseDecoder
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
                    , sendUIMsg model.location
                        { uid =
                            lmod.userId
                        , pwd =
                            lmod.password
                        }
                        (UI.Register ls.email)
                    )

                Login.Login ->
                    ( { model | state = Login lmod }
                    , sendUIMsg model.location
                        { uid =
                            lmod.userId
                        , pwd =
                            lmod.password
                        }
                        UI.Login
                    )

        ( UserReplyData urd, state ) ->
            let
                _ =
                    Debug.log "( UserReplyData urd," urd
            in
            case urd of
                Err e ->
                    ( { model | state = BadError (BadError.initialModel <| Util.httpErrorString e) model.state }, Cmd.none )

                Ok uiresponse ->
                    case uiresponse of
                        UI.ServerError e ->
                            ( { model | state = BadError (BadError.initialModel e) state }, Cmd.none )

                        UI.RegistrationSent ->
                            ( model, Cmd.none )

                        UI.LoggedIn ->
                            case state of
                                Login lmod ->
                                    -- we're logged in!  Get article listing.
                                    ( { model
                                        | state =
                                            ShowMessage
                                                { message = "loading articles"
                                                }
                                                { uid = lmod.userId, pwd = lmod.password }
                                      }
                                    , sendUIMsg model.location
                                        { uid =
                                            lmod.userId
                                        , pwd =
                                            lmod.password
                                        }
                                        UI.GetListing
                                    )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected login reply") state }
                                    , Cmd.none
                                    )

                        UI.EntryListing l ->
                            case state of
                                ShowMessage _ login ->
                                    ( { model | state = EditListing { entries = l } login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected login reply") state }
                                    , Cmd.none
                                    )

                        UI.BlogEntry fbe ->
                            case state of
                                EditListing _ login ->
                                    ( { model | state = Edit (Edit.initFull fbe) login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected blog message") state }, Cmd.none )

                        UI.SavedBlogEntry beid ->
                            case state of
                                Edit emod login ->
                                    ( { model | state = Edit (Edit.setId emod beid) login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected blog message") state }, Cmd.none )

                        UI.DeletedBlogEntry beid ->
                            case state of
                                ShowMessage _ login ->
                                    ( model
                                    , sendUIMsg model.location login UI.GetListing
                                    )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected message") state }, Cmd.none )

                        UI.UserExists ->
                            ( { model | state = BadError (BadError.initialModel "Can't register - User exists already!") state }, Cmd.none )

                        UI.UnregisteredUser ->
                            ( { model | state = BadError (BadError.initialModel "Unregistered user.  Check your spam folder!") state }, Cmd.none )

                        UI.InvalidUserOrPwd ->
                            ( { model | state = BadError (BadError.initialModel "Invalid username or password.") state }, Cmd.none )

        ( EditMsg em, Edit es login ) ->
            let
                ( emod, ecmd ) =
                    Edit.update em es
            in
            case ecmd of
                Edit.Save sbe ->
                    ( { model | state = Edit emod login }
                    , sendUIMsg model.location
                        login
                        (UI.SaveBlogEntry sbe)
                    )

                Edit.None ->
                    ( { model | state = Edit emod login }, Cmd.none )

                Edit.Cancel ->
                    ( { model
                        | state =
                            ShowMessage
                                { message = "loading articles"
                                }
                                login
                      }
                    , sendUIMsg model.location
                        login
                        UI.GetListing
                    )

                Edit.Delete id ->
                    -- issue delete and go back to listing.
                    ( { model
                        | state =
                            ShowMessage
                                { message = "loading articles"
                                }
                                login
                      }
                    , sendUIMsg model.location
                        login
                        (UI.DeleteBlogEntry id)
                    )

        ( EditListingMsg em, EditListing es login ) ->
            let
                ( emod, ecmd ) =
                    EditListing.update em es
            in
            case ecmd of
                EditListing.New ->
                    ( { model | state = Edit Edit.initNew login }, Cmd.none )

                EditListing.Selected id ->
                    ( model
                    , sendUIMsg model.location
                        login
                        (UI.GetBlogEntry id)
                    )

        ( BadErrorMsg bm, BadError bs prevstate ) ->
            let
                ( bmod, bcmd ) =
                    BadError.update bm bs
            in
            case bcmd of
                BadError.Okay ->
                    ( { model | state = prevstate }, Cmd.none )

        ( _, _ ) ->
            ( model, Cmd.none )
