module Main exposing (main)

import BadError
import Browser
import Browser.Navigation
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
import Url exposing (Url)
import Url.Parser as UP exposing ((</>))
import UserInterface as UI
import Util
import View


type Msg
    = LoginMsg Login.Msg
    | BadErrorMsg BadError.Msg
    | ViewMsg View.Msg
    | EditMsg Edit.Msg
    | EditListingMsg EditListing.Msg
    | ShowMessageMsg ShowMessage.Msg
    | UserReplyData (Result Http.Error UI.ServerResponse)
    | PublicReplyData (Result Http.Error PI.ServerResponse)
    | LoadUrl String
    | InternalUrl Url
    | Noop


type ZkMode
    = BmView
    | BmEdit


type State
    = Login Login.Model
    | Edit Edit.Model Data.Login
    | EditListing EditListing.Model Data.Login
    | View View.Model
    | EView View.Model State
    | BadError BadError.Model State
    | ShowMessage ShowMessage.Model Data.Login
    | PubShowMessage ShowMessage.Model
    | ZkWait State ZkMode


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
    , navkey : Browser.Navigation.Key
    , seed : Seed
    }


stateLogin : State -> Maybe Data.Login
stateLogin state =
    case state of
        Login lmod ->
            Just { uid = lmod.userId, pwd = lmod.password }

        Edit _ login ->
            Just login

        EditListing _ login ->
            Just login

        View _ ->
            Nothing

        EView _ evstate ->
            stateLogin evstate

        BadError _ bestate ->
            stateLogin bestate

        ShowMessage _ login ->
            Just login

        PubShowMessage _ ->
            Nothing

        ZkWait bwstate _ ->
            stateLogin bwstate


viewState : Util.Size -> State -> Element Msg
viewState size state =
    case state of
        Login lem ->
            Element.map LoginMsg <| Login.view size lem

        EditListing em _ ->
            Element.map EditListingMsg <| EditListing.view em

        ShowMessage em _ ->
            Element.map ShowMessageMsg <| ShowMessage.view em

        PubShowMessage em ->
            Element.map ShowMessageMsg <| ShowMessage.view em

        View em ->
            Element.map ViewMsg <| View.view em

        EView em _ ->
            Element.map ViewMsg <| View.view em

        Edit em _ ->
            Element.map EditMsg <| Edit.view em

        BadError em _ ->
            Element.map BadErrorMsg <| BadError.view em

        ZkWait innerState _ ->
            Element.map (\_ -> Noop) (viewState size innerState)


view : Model -> { title : String, body : List (Html Msg) }
view model =
    { title = "mah bloag!"
    , body =
        [ Element.layout [] <|
            viewState model.size model.state
        ]
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


sendPIMsg : String -> PI.SendMsg -> Cmd Msg
sendPIMsg location msg =
    Http.post
        { url = location ++ "/public"
        , body =
            Http.jsonBody
                (PI.encodeSendMsg msg)
        , expect = Http.expectJson PublicReplyData PI.serverResponseDecoder
        }


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model.state ) of
        ( InternalUrl url, _ ) ->
            let
                mblogin =
                    stateLogin model.state

                ( state, cmd ) =
                    parseUrl url
                        |> Maybe.map
                            (routeState model.location model.seed)
                        |> Maybe.withDefault ( model.state, Cmd.none )
            in
            ( { model | state = state }, cmd )

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

        ( PublicReplyData prd, state ) ->
            case prd of
                Err e ->
                    ( { model | state = BadError (BadError.initialModel <| Util.httpErrorString e) model.state }, Cmd.none )

                Ok piresponse ->
                    case piresponse of
                        PI.ServerError e ->
                            ( { model | state = BadError (BadError.initialModel e) state }, Cmd.none )

                        PI.ZkNote fbe ->
                            ( { model | state = View (View.initFull fbe) }, Cmd.none )

        ( UserReplyData urd, state ) ->
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
                                        , seed = lmod.seed -- save the seed!
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

                        UI.ZkListing l ->
                            case state of
                                ShowMessage _ login ->
                                    ( { model | state = EditListing { entries = l } login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected login reply") state }
                                    , Cmd.none
                                    )

                        UI.ZkNoteListing l ->
                            case state of
                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "zknotelisiting unimplmeented") state }
                                    , Cmd.none
                                    )

                        UI.ZkNote fbe ->
                            case state of
                                EditListing _ login ->
                                    ( { model | state = Edit (Edit.initFull fbe) login }, Cmd.none )

                                ZkWait bwstate mode ->
                                    case mode of
                                        BmView ->
                                            ( { model | state = EView (View.initFull fbe) bwstate }, Cmd.none )

                                        BmEdit ->
                                            case stateLogin state of
                                                Just login ->
                                                    ( { model | state = Edit (Edit.initFull fbe) login }, Cmd.none )

                                                Nothing ->
                                                    ( { model | state = BadError (BadError.initialModel "can't edit - not logged in!") state }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected blog message") state }, Cmd.none )

                        UI.SavedZkNote beid ->
                            case state of
                                Edit emod login ->
                                    ( { model | state = Edit (Edit.setId emod beid) login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected blog message") state }, Cmd.none )

                        UI.DeletedZkNote beid ->
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

        ( ViewMsg em, View es ) ->
            let
                ( emod, ecmd ) =
                    View.update em es
            in
            case ecmd of
                View.None ->
                    ( { model | state = View emod }, Cmd.none )

                View.Done ->
                    ( { model | state = View emod }, Cmd.none )

        ( ViewMsg em, EView es state ) ->
            let
                ( emod, ecmd ) =
                    View.update em es
            in
            case ecmd of
                View.None ->
                    ( { model | state = EView emod state }, Cmd.none )

                View.Done ->
                    ( { model | state = state }, Cmd.none )

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
                        (UI.SaveZkNote sbe)
                    )

                Edit.None ->
                    ( { model | state = Edit emod login }, Cmd.none )

                Edit.Done ->
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
                        (UI.DeleteZkNote id)
                    )

                Edit.View sbe ->
                    -- issue delete and go back to listing.
                    ( { model
                        | state = EView (View.initSbe sbe) model.state
                      }
                    , Cmd.none
                    )

        ( EditListingMsg em, EditListing es login ) ->
            let
                ( emod, ecmd ) =
                    EditListing.update em es
            in
            case ecmd of
                EditListing.New ->
                    ( { model | state = Edit Edit.initNew login }, Cmd.none )

                EditListing.Example ->
                    ( { model | state = Edit Edit.initExample login }, Cmd.none )

                EditListing.Selected id ->
                    ( { model | state = ZkWait model.state BmEdit }
                    , sendUIMsg model.location
                        login
                        (UI.GetZkNote id)
                    )

                EditListing.View id ->
                    ( { model | state = ZkWait model.state BmView }
                    , sendUIMsg model.location
                        login
                        (UI.GetZkNote id)
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


init : Flags -> Url -> Browser.Navigation.Key -> ( Model, Cmd Msg )
init flags url key =
    let
        _ =
            Debug.log "parsed: " (parseUrl url)

        seed =
            initialSeed (flags.seed + 7)

        ( state, cmd ) =
            parseUrl url
                |> Maybe.map (routeState flags.location seed)
                |> Maybe.withDefault ( initLogin seed, Cmd.none )
    in
    ( { state = state
      , size = { width = flags.width, height = flags.height }
      , location = flags.location
      , navkey = key
      , seed = seed
      }
    , cmd
    )


type Route
    = PublicZk Int
    | Fail


parseUrl : Url -> Maybe Route
parseUrl url =
    UP.parse
        (UP.map (\i -> PublicZk i) <|
            UP.s
                "blog"
                </> UP.int
        )
        url


initLogin : Seed -> State
initLogin seed =
    Login <| Login.initialModel Nothing "mahbloag" seed


routeState : String -> Seed -> Route -> ( State, Cmd Msg )
routeState location seed route =
    case route of
        PublicZk id ->
            ( PubShowMessage
                { message = "loading article"
                }
            , sendPIMsg location
                (PI.GetZkNote id)
            )

        Fail ->
            ( initLogin seed, Cmd.none )


urlRequest : Browser.UrlRequest -> Msg
urlRequest ur =
    let
        _ =
            Debug.log "ur: " ur
    in
    case ur of
        Browser.Internal url ->
            InternalUrl url

        Browser.External str ->
            LoadUrl str


main : Platform.Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = \model -> Sub.none
        , onUrlRequest = urlRequest
        , onUrlChange =
            \uc ->
                let
                    _ =
                        Debug.log "uc: " uc
                in
                Noop

        -- Url -> msg
        }
