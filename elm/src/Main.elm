port module Main exposing (main)

import Array
import Browser
import Browser.Events
import Browser.Navigation
import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Common exposing (buttonStyle)
import Data
import Dict exposing (Dict)
import DisplayError
import EditZkNote
import EditZkNoteListing
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region
import File as F
import File.Select as FS
import GenDialog as GD
import Html exposing (Attribute, Html)
import Html.Attributes
import Html.Events as HE
import Http
import Import
import Json.Decode as JD
import Json.Encode as JE
import LocalStorage as LS
import Login
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..))
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import PublicInterface as PI
import Random exposing (Seed, initialSeed)
import Schelme.Show exposing (showTerm)
import Search as S
import SearchPanel as SP
import SelectString as SS
import ShowMessage
import Toop
import Url exposing (Url)
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))
import UserInterface as UI
import Util
import View


type Msg
    = LoginMsg Login.Msg
    | DisplayErrorMsg DisplayError.Msg
    | ViewMsg View.Msg
    | EditZkNoteMsg EditZkNote.Msg
    | EditZkNoteListingMsg EditZkNoteListing.Msg
    | ImportMsg Import.Msg
    | ShowMessageMsg ShowMessage.Msg
    | UserReplyData (Result Http.Error UI.ServerResponse)
    | PublicReplyData (Result Http.Error PI.ServerResponse)
    | LoadUrl String
    | InternalUrl Url
    | SelectedText JD.Value
    | UrlChanged Url
    | WindowSize Util.Size
    | CtrlS
    | SelectDialogMsg (GD.Msg SS.Msg)
    | Noop


type State
    = Login Login.Model
    | EditZkNote EditZkNote.Model Data.LoginData
    | EditZkNoteListing EditZkNoteListing.Model Data.LoginData
    | View View.Model
    | EView View.Model State
    | Import Import.Model Data.LoginData
    | DisplayError DisplayError.Model State
    | ShowMessage ShowMessage.Model Data.LoginData
    | PubShowMessage ShowMessage.Model
    | LoginShowMessage ShowMessage.Model Data.LoginData Url
    | SelectDialog SS.GDModel State
    | Wait State (Model -> Msg -> ( Model, Cmd Msg ))



-- | WaitM State (Model -> Msg -> ( Model, Cmd Msg ))


type alias Flags =
    { seed : Int
    , location : String
    , useragent : String
    , debugstring : String
    , width : Int
    , height : Int
    , login : Maybe Data.LoginData
    }


type alias SavedRoute =
    { route : Route
    , save : Bool
    }


type alias Model =
    { state : State
    , size : Util.Size
    , location : String
    , navkey : Browser.Navigation.Key
    , seed : Seed
    , savedRoute : SavedRoute
    , prevSearches : List S.TagSearch
    }


type Route
    = PublicZkNote Int
    | PublicZkPubId String
    | EditZkNoteR Int
    | Top


routeTitle : Route -> String
routeTitle route =
    case route of
        PublicZkNote id ->
            "zknote " ++ String.fromInt id

        PublicZkPubId id ->
            "zknote " ++ id

        EditZkNoteR id ->
            "zknote " ++ String.fromInt id

        Top ->
            "zknotes"


urlRequest : Browser.UrlRequest -> Msg
urlRequest ur =
    case ur of
        Browser.Internal url ->
            InternalUrl url

        Browser.External str ->
            LoadUrl str


parseUrl : Url -> Maybe Route
parseUrl url =
    UP.parse
        (UP.oneOf
            [ UP.map PublicZkNote <|
                UP.s
                    "note"
                    </> UP.int
            , UP.map (\i -> PublicZkPubId (Maybe.withDefault "" (Url.percentDecode i))) <|
                UP.s
                    "page"
                    </> UP.string
            , UP.map EditZkNoteR <|
                UP.s
                    "editnote"
                    </> UP.int
            , UP.map Top <| UP.top
            ]
        )
        url


routeUrl : Route -> String
routeUrl route =
    case route of
        PublicZkNote id ->
            UB.absolute [ "note", String.fromInt id ] []

        PublicZkPubId pubid ->
            UB.absolute [ "page", pubid ] []

        EditZkNoteR id ->
            UB.absolute [ "editnote", String.fromInt id ] []

        Top ->
            UB.absolute [] []


routeState : Model -> Route -> Maybe ( State, Cmd Msg )
routeState model route =
    case route of
        PublicZkNote id ->
            case stateLogin model.state of
                Just login ->
                    Just
                        ( ShowMessage
                            { message = "loading article"
                            }
                            login
                        , sendUIMsg model.location
                            (UI.GetZkNoteEdit { zknote = id })
                        )

                Nothing ->
                    Just
                        ( PubShowMessage
                            { message = "loading article"
                            }
                        , sendPIMsg model.location
                            (PI.GetZkNote id)
                        )

        PublicZkPubId pubid ->
            Just
                ( PubShowMessage
                    { message = "loading article"
                    }
                , sendPIMsg model.location
                    (PI.GetZkNotePubId pubid)
                )

        EditZkNoteR id ->
            case model.state of
                EditZkNote st login ->
                    Just <|
                        ( EditZkNote st login
                        , sendUIMsg model.location (UI.GetZkNoteEdit { zknote = id })
                        )

                EditZkNoteListing st login ->
                    Just <|
                        ( EditZkNoteListing st login
                        , sendUIMsg model.location (UI.GetZkNoteEdit { zknote = id })
                        )

                st ->
                    case stateLogin st of
                        Just login ->
                            Just <|
                                ( ShowMessage { message = "loading note..." } login
                                , sendUIMsg model.location (UI.GetZkNoteEdit { zknote = id })
                                )

                        Nothing ->
                            Nothing

        Top ->
            if (stateRoute model.state).route == Top then
                Just ( model.state, Cmd.none )

            else
                Nothing


stateRoute : State -> SavedRoute
stateRoute state =
    case state of
        View vst ->
            case vst.pubid of
                Just pubid ->
                    { route = PublicZkPubId pubid
                    , save = True
                    }

                Nothing ->
                    case vst.id of
                        Just id ->
                            { route = PublicZkNote id
                            , save = True
                            }

                        Nothing ->
                            { route = Top
                            , save = False
                            }

        EditZkNote st login ->
            { route =
                st.id
                    |> Maybe.map EditZkNoteR
                    |> Maybe.withDefault Top
            , save = True
            }

        _ ->
            { route = Top
            , save = False
            }


showMessage : Msg -> String
showMessage msg =
    case msg of
        LoginMsg _ ->
            "LoginMsg"

        DisplayErrorMsg _ ->
            "DisplayErrorMsg"

        ViewMsg _ ->
            "ViewMsg"

        EditZkNoteMsg _ ->
            "EditZkNoteMsg"

        EditZkNoteListingMsg _ ->
            "EditZkNoteListingMsg"

        ImportMsg _ ->
            "ImportMsg"

        ShowMessageMsg _ ->
            "ShowMessageMsg"

        UserReplyData urd ->
            "UserReplyData: "
                ++ (Result.map UI.showServerResponse urd
                        |> Result.mapError Util.httpErrorString
                        |> (\r ->
                                case r of
                                    Ok m ->
                                        "message: " ++ m

                                    Err e ->
                                        "error: " ++ e
                           )
                   )

        PublicReplyData _ ->
            "PublicReplyData"

        LoadUrl _ ->
            "LoadUrl"

        InternalUrl _ ->
            "InternalUrl"

        SelectedText _ ->
            "SelectedText"

        UrlChanged _ ->
            "UrlChanged"

        WindowSize _ ->
            "WindowSize"

        Noop ->
            "Noop"

        CtrlS ->
            "CtrlS"

        SelectDialogMsg _ ->
            "SelectDialogMsg"


showState : State -> String
showState state =
    case state of
        Login _ ->
            "Login"

        EditZkNote _ _ ->
            "EditZkNote"

        EditZkNoteListing _ _ ->
            "EditZkNoteListing"

        View _ ->
            "View"

        EView _ _ ->
            "EView"

        Import _ _ ->
            "Import"

        DisplayError _ _ ->
            "DisplayError"

        ShowMessage _ _ ->
            "ShowMessage"

        PubShowMessage _ ->
            "PubShowMessage"

        LoginShowMessage _ _ _ ->
            "LoginShowMessage"

        Wait _ _ ->
            "Wait"

        SelectDialog _ _ ->
            "SelectDialog"


unexpectedMsg : State -> Msg -> State
unexpectedMsg state msg =
    unexpectedMessage state (showMessage msg)


unexpectedMessage : State -> String -> State
unexpectedMessage state msg =
    DisplayError (DisplayError.initialModel <| "unexpected message - " ++ msg ++ "; state was " ++ showState state) state


viewState : Util.Size -> State -> Element Msg
viewState size state =
    case state of
        Login lem ->
            E.map LoginMsg <| Login.view size lem

        EditZkNote em _ ->
            E.map EditZkNoteMsg <| EditZkNote.view size em

        EditZkNoteListing em ld ->
            E.map EditZkNoteListingMsg <| EditZkNoteListing.view ld size em

        ShowMessage em _ ->
            E.map ShowMessageMsg <| ShowMessage.view em

        PubShowMessage em ->
            E.map ShowMessageMsg <| ShowMessage.view em

        LoginShowMessage em _ _ ->
            E.map ShowMessageMsg <| ShowMessage.view em

        Import em _ ->
            E.map ImportMsg <| Import.view size em

        View em ->
            E.map ViewMsg <| View.view size.width em False

        EView em _ ->
            E.map ViewMsg <| View.view size.width em True

        DisplayError em _ ->
            E.map DisplayErrorMsg <| DisplayError.view em

        Wait innerState _ ->
            E.map (\_ -> Noop) (viewState size innerState)

        SelectDialog _ _ ->
            -- render is at the layout level, not here.
            E.none


stateSearch : State -> Maybe ( SP.Model, Data.ZkListNoteSearchResult )
stateSearch state =
    case state of
        EditZkNote emod _ ->
            Just ( emod.spmodel, emod.zknSearchResult )

        EditZkNoteListing emod _ ->
            Just ( emod.spmodel, emod.notes )

        _ ->
            Nothing


stateLogin : State -> Maybe Data.LoginData
stateLogin state =
    case state of
        Login lmod ->
            Nothing

        EditZkNote _ login ->
            Just login

        EditZkNoteListing _ login ->
            Just login

        Import _ login ->
            Just login

        View _ ->
            Nothing

        EView _ evstate ->
            stateLogin evstate

        DisplayError _ bestate ->
            stateLogin bestate

        ShowMessage _ login ->
            Just login

        PubShowMessage _ ->
            Nothing

        LoginShowMessage _ _ _ ->
            Nothing

        Wait wstate _ ->
            stateLogin wstate

        SelectDialog _ instate ->
            stateLogin instate


sendUIMsg : String -> UI.SendMsg -> Cmd Msg
sendUIMsg location msg =
    Http.post
        { url = location ++ "/user"
        , body =
            Http.jsonBody
                (UI.encodeSendMsg msg)
        , expect = Http.expectJson UserReplyData UI.serverResponseDecoder
        }


{-| send search AND save search in db as a zknote
-}
sendSearch : Model -> S.ZkNoteSearch -> ( Model, Cmd Msg )
sendSearch model search =
    case stateLogin model.state of
        Just ldata ->
            let
                searchnote =
                    { note =
                        { id = Nothing
                        , pubid = Nothing
                        , title = S.printTagSearch search.tagSearch
                        , content = S.encodeTagSearch search.tagSearch |> JE.encode 2
                        , editable = False
                        }
                    , links =
                        [ { otherid = ldata.searchid
                          , direction = Data.To
                          , user = ldata.userid
                          , zknote = Nothing
                          , delete = Nothing
                          }
                        ]
                    }
            in
            ( { model | prevSearches = search.tagSearch :: model.prevSearches }
            , if List.head model.prevSearches == Just search.tagSearch then
                sendUIMsg model.location (UI.SearchZkNotes search)

              else
                Cmd.batch
                    [ sendUIMsg model.location (UI.SearchZkNotes search)
                    , sendUIMsg model.location (UI.SaveZkNotePlusLinks searchnote)
                    ]
            )

        Nothing ->
            ( model
            , Cmd.none
            )


sendPIMsg : String -> PI.SendMsg -> Cmd Msg
sendPIMsg location msg =
    Http.post
        { url = location ++ "/public"
        , body =
            Http.jsonBody
                (PI.encodeSendMsg msg)
        , expect = Http.expectJson PublicReplyData PI.serverResponseDecoder
        }


type alias NwState =
    { login : Data.LoginData
    , mbzknotesearchresult : Maybe Data.ZkListNoteSearchResult
    , mbzklinks : Maybe Data.ZkLinks
    , mbzknote : Maybe Data.ZkNote
    , spmodel : SP.Model
    , navkey : Browser.Navigation.Key
    , location : String
    , seed : Seed
    }


listingwait : Data.LoginData -> State -> Msg -> ( State, Cmd Msg )
listingwait login st ms =
    case ms of
        UserReplyData (Ok (UI.ZkListNoteSearchResult rs)) ->
            ( EditZkNoteListing
                { notes = rs
                , spmodel =
                    SP.searchResultUpdated rs SP.initModel
                , dialog = Nothing
                }
                login
            , Cmd.none
            )

        UserReplyData (Ok (UI.ServerError e)) ->
            ( DisplayError (DisplayError.initialModel e) st, Cmd.none )

        _ ->
            ( unexpectedMsg st ms
            , Cmd.none
            )


getListing : Model -> Data.LoginData -> ( Model, Cmd Msg )
getListing model login =
    sendSearch
        { model
            | state =
                ShowMessage
                    { message = "loading articles"
                    }
                    login
            , seed =
                case model.state of
                    -- save the seed if we're leaving login state.
                    Login lmod ->
                        lmod.seed

                    _ ->
                        model.seed
        }
        S.defaultSearch


view : Model -> { title : String, body : List (Html Msg) }
view model =
    { title =
        case model.state of
            EditZkNote ezn _ ->
                ezn.title ++ " - zknote"

            _ ->
                routeTitle model.savedRoute.route
    , body =
        [ Html.div
            [ -- important to prevent ctrl-s on non-input item focus.
              -- also has to be done in a div enclosing E.layout, rather than using
              -- E.htmlAttribute to attach it directly.
              Html.Attributes.tabindex 0

            -- blocks on ctrl-s, lets others through.
            , onKeyDown
            ]
            [ case model.state of
                SelectDialog sdm prevstate ->
                    Html.map SelectDialogMsg <| GD.layout sdm

                _ ->
                    E.layout [] <| viewState model.size model.state
            ]
        ]
    }


onKeyDown : Attribute Msg
onKeyDown =
    HE.preventDefaultOn "keydown"
        (JD.map4
            (\key ctrl alt shift ->
                case Toop.T4 key ctrl alt shift of
                    Toop.T4 "s" True False False ->
                        -- ctrl-s -> prevent default!
                        -- also, CtrlS message.
                        ( CtrlS, True )

                    _ ->
                        -- anything else, don't prevent default!
                        ( Noop, False )
            )
            (JD.field "key" JD.string)
            (JD.field "ctrlKey" JD.bool)
            (JD.field "altKey" JD.bool)
            (JD.field "shiftKey" JD.bool)
        )


onKeyUp : msg -> Attribute msg
onKeyUp msg =
    HE.preventDefaultOn "keyup" (JD.map alwaysPreventDefault (JD.succeed msg))


alwaysPreventDefault : msg -> ( msg, Bool )
alwaysPreventDefault msg =
    ( msg, True )


{-| urlUpdate: all URL code shall go here! regular code shall not worry about urls!
this function calls actualupdate where the app stuff happens.
url messages and state based url changes are done here.
-}
urlupdate : Msg -> Model -> ( Model, Cmd Msg )
urlupdate msg model =
    let
        ( nm, cmd ) =
            case msg of
                InternalUrl url ->
                    let
                        ( state, icmd ) =
                            parseUrl url
                                |> Maybe.andThen (routeState model)
                                |> Maybe.withDefault ( model.state, Cmd.none )

                        bcmd =
                            case model.state of
                                EditZkNote s ld ->
                                    if EditZkNote.dirty s then
                                        Cmd.batch
                                            [ icmd
                                            , sendUIMsg model.location
                                                (UI.SaveZkNotePlusLinks <| EditZkNote.fullSave s)
                                            ]

                                    else
                                        icmd

                                _ ->
                                    icmd
                    in
                    ( { model | state = state }, bcmd )

                LoadUrl urlstr ->
                    -- load foreign site
                    -- ( model, Browser.Navigation.load urlstr )
                    ( model, Cmd.none )

                UrlChanged url ->
                    -- we get this from forward and back buttons.  if the user changes the url
                    -- in the browser address bar, its a site reload so this isn't called.
                    case parseUrl url of
                        Just route ->
                            if route == (stateRoute model.state).route then
                                ( model, Cmd.none )

                            else
                                case routeState model route of
                                    Just ( st, rscmd ) ->
                                        -- swap out the savedRoute, so we don't write over history.
                                        ( { model | state = st, savedRoute = stateRoute st }, rscmd )

                                    Nothing ->
                                        ( model, Cmd.none )

                        Nothing ->
                            -- load foreign site
                            -- ( model, Browser.Navigation.load (Url.toString url) )
                            ( model, Cmd.none )

                _ ->
                    -- not an url related message!  pass it on to the 'actualupdate'
                    -- this is where all the app stuff happens.
                    actualupdate msg model

        sr =
            stateRoute nm.state
    in
    -- when the route changes, change the address bar, optionally pushing what's there to
    -- browser history.
    if sr.route /= nm.savedRoute.route then
        ( { nm | savedRoute = sr }
        , if model.savedRoute.save then
            Cmd.batch
                [ cmd
                , Browser.Navigation.pushUrl nm.navkey
                    (routeUrl sr.route)
                ]

          else
            Cmd.batch
                [ cmd
                , Browser.Navigation.replaceUrl nm.navkey
                    (routeUrl sr.route)
                ]
        )

    else
        ( nm, cmd )


actualupdate : Msg -> Model -> ( Model, Cmd Msg )
actualupdate msg model =
    case ( msg, model.state ) of
        ( _, Wait wst wfn ) ->
            let
                ( nmd, cmd ) =
                    wfn model msg
            in
            ( nmd, cmd )

        ( WindowSize s, _ ) ->
            ( { model | size = s }, Cmd.none )

        ( SelectDialogMsg sdmsg, SelectDialog sdmod instate ) ->
            case GD.update sdmsg sdmod of
                GD.Dialog nmod ->
                    ( { model | state = SelectDialog nmod instate }, Cmd.none )

                GD.Ok return ->
                    case List.head (List.drop return model.prevSearches) of
                        Just ts ->
                            let
                                ns =
                                    case instate of
                                        EditZkNote ezn login ->
                                            EditZkNote (Tuple.first <| EditZkNote.updateSearch ts ezn) login

                                        _ ->
                                            instate
                            in
                            ( { model | state = ns }, Cmd.none )

                        Nothing ->
                            ( { model | state = instate }, Cmd.none )

                GD.Cancel ->
                    ( { model | state = instate }, Cmd.none )

        ( SelectedText jv, state ) ->
            case JD.decodeValue JD.string jv of
                Ok str ->
                    case state of
                        EditZkNote emod login ->
                            let
                                ( newnote_st, cmd ) =
                                    EditZkNote.gotSelectedText emod str
                            in
                            case cmd of
                                EditZkNote.Save szkpl ->
                                    ( { model
                                        | state =
                                            Wait
                                                (ShowMessage
                                                    { message = "waiting for save"
                                                    }
                                                    login
                                                )
                                                (\md ms ->
                                                    case ms of
                                                        UserReplyData (Ok (UI.SavedZkNotePlusLinks _)) ->
                                                            ( { model
                                                                | state =
                                                                    EditZkNote
                                                                        newnote_st
                                                                        login
                                                              }
                                                            , Cmd.none
                                                            )

                                                        _ ->
                                                            ( { model
                                                                | state =
                                                                    unexpectedMsg model.state ms
                                                              }
                                                            , Cmd.none
                                                            )
                                                )
                                      }
                                    , Cmd.batch
                                        [ sendUIMsg model.location
                                            (UI.SaveZkNotePlusLinks szkpl)
                                        ]
                                    )

                                _ ->
                                    ( { model | state = EditZkNote newnote_st login }, Cmd.none )

                        _ ->
                            ( model, Cmd.none )

                Err e ->
                    ( { model | state = DisplayError (DisplayError.initialModel <| JD.errorToString e) model.state }, Cmd.none )

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
                        (UI.Register
                            { uid = lmod.userId
                            , pwd = lmod.password
                            , email = ls.email
                            }
                        )
                    )

                Login.Login ->
                    ( { model | state = Login lmod }
                    , sendUIMsg model.location <|
                        UI.Login
                            { uid = lmod.userId
                            , pwd = lmod.password
                            }
                    )

        ( PublicReplyData prd, state ) ->
            case prd of
                Err e ->
                    ( { model | state = DisplayError (DisplayError.initialModel <| Util.httpErrorString e) model.state }, Cmd.none )

                Ok piresponse ->
                    case piresponse of
                        PI.ServerError e ->
                            ( { model | state = DisplayError (DisplayError.initialModel e) state }, Cmd.none )

                        PI.ZkNote fbe ->
                            let
                                vstate =
                                    View (View.initFull fbe)
                            in
                            ( { model | state = vstate }
                            , Cmd.none
                            )

        ( UserReplyData urd, state ) ->
            case urd of
                Err e ->
                    ( { model | state = DisplayError (DisplayError.initialModel <| Util.httpErrorString e) model.state }, Cmd.none )

                Ok uiresponse ->
                    case uiresponse of
                        UI.ServerError e ->
                            ( { model | state = DisplayError (DisplayError.initialModel e) state }, Cmd.none )

                        UI.RegistrationSent ->
                            ( model, Cmd.none )

                        UI.PowerDeleteComplete count ->
                            case model.state of
                                EditZkNoteListing mod li ->
                                    ( { model | state = EditZkNoteListing (EditZkNoteListing.onPowerDeleteComplete count li mod) li }, Cmd.none )

                                _ ->
                                    ( model, Cmd.none )

                        UI.LoggedIn login ->
                            let
                                getlisting =
                                    sendSearch
                                        { model
                                            | state =
                                                ShowMessage
                                                    { message = "loading articles"
                                                    }
                                                    login
                                            , seed =
                                                case state of
                                                    -- save the seed if we're leaving login state.
                                                    Login lmod ->
                                                        lmod.seed

                                                    _ ->
                                                        model.seed
                                        }
                                        S.defaultSearch
                            in
                            case state of
                                Login lm ->
                                    -- we're logged in!  Get article listing.
                                    getlisting

                                LoginShowMessage _ li url ->
                                    let
                                        lgmod =
                                            { model | state = ShowMessage { message = "logged in" } login }

                                        ( m, cmd ) =
                                            parseUrl url
                                                |> Maybe.andThen
                                                    (\s ->
                                                        case s of
                                                            Top ->
                                                                Nothing

                                                            _ ->
                                                                Just s
                                                    )
                                                |> Maybe.andThen
                                                    (routeState
                                                        lgmod
                                                    )
                                                |> Maybe.map (\( st, cm ) -> ( { model | state = st }, cm ))
                                                |> Maybe.withDefault
                                                    getlisting
                                    in
                                    ( m, cmd )

                                -- we're logged in!  Get article listing.
                                -- getlisting login
                                _ ->
                                    ( { model | state = unexpectedMessage state "logged in" }
                                    , Cmd.none
                                    )

                        UI.ZkNoteSearchResult sr ->
                            let
                                _ =
                                    Debug.log " UI.ZkNoteSearchResult " sr
                            in
                            if sr.what == "prevSearches" then
                                -- build prev searches list.
                                ( { model
                                    | prevSearches =
                                        Debug.log "prevsearches:" <|
                                            List.filterMap
                                                (\zknote ->
                                                    JD.decodeString S.decodeTagSearch zknote.content
                                                        |> Result.toMaybe
                                                )
                                                sr.notes
                                  }
                                , Cmd.none
                                )

                            else
                                ( model, Cmd.none )

                        UI.ZkListNoteSearchResult sr ->
                            let
                                _ =
                                    Debug.log "UI.ZkListNoteSearchResult" sr
                            in
                            case state of
                                EditZkNoteListing znlstate login_ ->
                                    ( { model | state = EditZkNoteListing (EditZkNoteListing.updateSearchResult sr znlstate) login_ }
                                    , Cmd.none
                                    )

                                EditZkNote znstate login_ ->
                                    ( { model | state = EditZkNote (EditZkNote.updateSearchResult sr znstate) login_ }
                                    , Cmd.none
                                    )

                                Import istate login_ ->
                                    ( { model | state = Import (Import.updateSearchResult sr istate) login_ }
                                    , Cmd.none
                                    )

                                ShowMessage _ login ->
                                    ( { model | state = EditZkNoteListing { notes = sr, spmodel = SP.initModel, dialog = Nothing } login }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( { model | state = unexpectedMessage state (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.ZkNote _ ->
                            case state of
                                _ ->
                                    ( { model | state = unexpectedMessage state (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.ZkNoteEdit zne ->
                            case stateLogin state of
                                Just login ->
                                    let
                                        ( spmod, sres ) =
                                            stateSearch state
                                                |> Maybe.withDefault ( SP.initModel, { notes = [], offset = 0, what = "" } )

                                        ( s, c ) =
                                            EditZkNote.initFull login
                                                sres
                                                zne.zknote
                                                zne.links
                                                spmod
                                    in
                                    ( { model
                                        | state =
                                            EditZkNote
                                                s
                                                login
                                      }
                                    , sendUIMsg model.location <| UI.GetZkNoteComments c
                                    )

                                _ ->
                                    ( { model | state = unexpectedMessage state (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.ZkNoteComments zc ->
                            case state of
                                EditZkNote s login ->
                                    ( { model | state = EditZkNote (EditZkNote.commentsRecieved zc s) login }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( { model | state = unexpectedMessage state (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.SavedZkNote szkn ->
                            case state of
                                EditZkNote emod login ->
                                    let
                                        eznst =
                                            EditZkNote.onSaved emod szkn

                                        st =
                                            EditZkNote eznst login
                                    in
                                    ( { model | state = st }
                                    , Cmd.none
                                    )

                                _ ->
                                    -- just ignore if we're not editing a new note.
                                    ( model, Cmd.none )

                        UI.SavedZkNotePlusLinks szkn ->
                            case state of
                                EditZkNote emod login ->
                                    let
                                        eznst =
                                            EditZkNote.onSaved emod szkn

                                        st =
                                            EditZkNote eznst login
                                    in
                                    ( { model | state = st }
                                    , Cmd.none
                                    )

                                _ ->
                                    -- just ignore if we're not editing a new note.
                                    ( model, Cmd.none )

                        UI.DeletedZkNote beid ->
                            ( model, Cmd.none )

                        UI.SavedZkLinks ->
                            ( model, Cmd.none )

                        UI.ZkLinks zkl ->
                            ( model, Cmd.none )

                        UI.UserExists ->
                            case state of
                                Login lmod ->
                                    ( { model | state = Login <| Login.userExists lmod }, Cmd.none )

                                _ ->
                                    ( { model | state = unexpectedMessage state (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.UnregisteredUser ->
                            case state of
                                Login lmod ->
                                    ( { model | state = Login <| Login.unregisteredUser lmod }, Cmd.none )

                                _ ->
                                    ( { model | state = unexpectedMessage state (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.NotLoggedIn ->
                            case state of
                                Login lmod ->
                                    ( { model | state = Login lmod }, Cmd.none )

                                _ ->
                                    ( { model | state = Login <| Login.initialModel Nothing "zknotes" model.seed }, Cmd.none )

                        UI.InvalidUserOrPwd ->
                            case state of
                                Login lmod ->
                                    ( { model | state = Login <| Login.invalidUserOrPwd lmod }, Cmd.none )

                                _ ->
                                    ( { model | state = unexpectedMessage (Login (Login.initialModel Nothing "zknotes" model.seed)) (UI.showServerResponse uiresponse) }
                                    , Cmd.none
                                    )

                        UI.SavedImportZkNotes ->
                            ( model, Cmd.none )

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

                View.Switch id ->
                    ( { model
                        | state =
                            PubShowMessage
                                { message = "loading article"
                                }
                      }
                    , sendPIMsg model.location
                        (PI.GetZkNote id)
                    )

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

                View.Switch _ ->
                    ( model, Cmd.none )

        ( EditZkNoteMsg em, EditZkNote es login ) ->
            let
                ( emod, ecmd ) =
                    EditZkNote.update em es
            in
            handleEditZkNoteCmd model login emod ecmd

        ( CtrlS, EditZkNote es login ) ->
            let
                ( emod, ecmd ) =
                    EditZkNote.onCtrlS es
            in
            handleEditZkNoteCmd model login emod ecmd

        ( EditZkNoteListingMsg em, EditZkNoteListing es login ) ->
            let
                ( emod, ecmd ) =
                    EditZkNoteListing.update em es login
            in
            case ecmd of
                EditZkNoteListing.None ->
                    ( { model | state = EditZkNoteListing emod login }, Cmd.none )

                EditZkNoteListing.New ->
                    ( { model | state = EditZkNote (EditZkNote.initNew login es.notes emod.spmodel) login }, Cmd.none )

                EditZkNoteListing.Selected id ->
                    ( { model | state = EditZkNoteListing emod login }
                    , sendUIMsg model.location (UI.GetZkNoteEdit { zknote = id })
                    )

                EditZkNoteListing.Done ->
                    ( { model | state = Login (Login.initialModel Nothing "zknotes" model.seed) }
                    , Cmd.batch
                        [ LS.storeLocalVal { name = "uid", value = "" }
                        , LS.storeLocalVal { name = "pwd", value = "" }
                        ]
                    )

                EditZkNoteListing.Import ->
                    ( { model | state = Import (Import.init login emod.notes emod.spmodel) login }
                    , Cmd.none
                    )

                EditZkNoteListing.Search s ->
                    sendSearch { model | state = EditZkNoteListing emod login } s

                EditZkNoteListing.PowerDelete s ->
                    ( { model | state = EditZkNoteListing emod login }
                    , sendUIMsg model.location
                        (UI.PowerDelete s)
                    )

        ( ImportMsg em, Import es login ) ->
            let
                ( emod, ecmd ) =
                    Import.update em es

                backtolisting =
                    \imod ->
                        let
                            nm =
                                { model
                                    | state =
                                        EditZkNoteListing
                                            { notes = imod.zknSearchResult
                                            , spmodel = imod.spmodel
                                            , dialog = Nothing
                                            }
                                            login
                                }
                        in
                        case SP.getSearch imod.spmodel of
                            Just s ->
                                sendSearch nm s

                            Nothing ->
                                ( nm, Cmd.none )
            in
            case ecmd of
                Import.None ->
                    ( { model | state = Import emod login }, Cmd.none )

                Import.SaveExit notes ->
                    let
                        ( m, c ) =
                            backtolisting emod

                        notecmds =
                            List.map
                                (\n ->
                                    sendUIMsg model.location
                                        (UI.SaveImportZkNotes [ n ])
                                )
                                notes
                    in
                    ( m
                    , Cmd.batch
                        (c
                            :: notecmds
                        )
                    )

                Import.Search s ->
                    sendSearch { model | state = Import emod login } s

                Import.SelectFiles ->
                    ( { model | state = Import emod login }
                    , FS.files []
                        (\a b -> ImportMsg (Import.FilesSelected a b))
                    )

                Import.Cancel ->
                    backtolisting emod

                Import.Command cmd ->
                    ( model, Cmd.map ImportMsg cmd )

        ( DisplayErrorMsg bm, DisplayError bs prevstate ) ->
            let
                ( bmod, bcmd ) =
                    DisplayError.update bm bs
            in
            case bcmd of
                DisplayError.Okay ->
                    ( { model | state = prevstate }, Cmd.none )

        ( _, _ ) ->
            ( model, Cmd.none )


handleEditZkNoteCmd model login emod ecmd =
    let
        backtolisting =
            let
                nm =
                    { model
                        | state =
                            EditZkNoteListing
                                { notes = emod.zknSearchResult
                                , spmodel = emod.spmodel
                                , dialog = Nothing
                                }
                                login
                    }
            in
            case SP.getSearch emod.spmodel of
                Just s ->
                    sendSearch nm s

                Nothing ->
                    ( nm, Cmd.none )
    in
    case ecmd of
        EditZkNote.SaveExit snpl ->
            let
                gotres =
                    let
                        nm =
                            { model
                                | state =
                                    EditZkNoteListing
                                        { notes = emod.zknSearchResult
                                        , spmodel = emod.spmodel
                                        , dialog = Nothing
                                        }
                                        login
                            }
                    in
                    case SP.getSearch emod.spmodel of
                        Just s ->
                            sendSearch nm s

                        Nothing ->
                            ( nm, Cmd.none )

                onmsg : Model -> Msg -> ( Model, Cmd Msg )
                onmsg st ms =
                    case ms of
                        UserReplyData (Ok (UI.SavedZkNotePlusLinks szn)) ->
                            gotres

                        UserReplyData (Ok (UI.ServerError e)) ->
                            ( { model | state = DisplayError (DisplayError.initialModel e) model.state }
                            , Cmd.none
                            )

                        _ ->
                            ( { model | state = unexpectedMsg model.state ms }
                            , Cmd.none
                            )
            in
            ( { model
                | state =
                    Wait
                        (ShowMessage
                            { message = "loading articles"
                            }
                            login
                        )
                        onmsg
              }
            , sendUIMsg model.location
                (UI.SaveZkNotePlusLinks snpl)
            )

        EditZkNote.Save snpl ->
            ( { model | state = EditZkNote emod login }
            , sendUIMsg model.location
                (UI.SaveZkNotePlusLinks snpl)
            )

        EditZkNote.None ->
            ( { model | state = EditZkNote emod login }, Cmd.none )

        EditZkNote.Revert ->
            backtolisting

        EditZkNote.Delete id ->
            -- issue delete and go back to listing.
            let
                ( m, c ) =
                    backtolisting
            in
            ( { m
                | state =
                    Wait m.state
                        (\mod _ ->
                            ( mod, c )
                        )
              }
            , sendUIMsg model.location
                (UI.DeleteZkNote id)
            )

        EditZkNote.Switch id ->
            let
                ( st, cmd ) =
                    ( ShowMessage { message = "loading note..." } login
                    , sendUIMsg model.location (UI.GetZkNoteEdit { zknote = id })
                    )
            in
            ( { model | state = st }, cmd )

        EditZkNote.SaveSwitch s id ->
            let
                ( st, cmd ) =
                    ( ShowMessage { message = "loading note..." } login
                    , sendUIMsg model.location (UI.GetZkNoteEdit { zknote = id })
                    )
            in
            ( { model | state = st }
            , Cmd.batch
                [ cmd
                , sendUIMsg model.location
                    (UI.SaveZkNotePlusLinks s)
                ]
            )

        EditZkNote.View szn ->
            ( { model | state = EView (View.initSzn szn []) (EditZkNote emod login) }, Cmd.none )

        EditZkNote.GetSelectedText ids ->
            ( { model | state = EditZkNote emod login }
            , getSelectedText ids
            )

        EditZkNote.Search s ->
            sendSearch { model | state = EditZkNote emod login } s

        EditZkNote.SearchHistory ->
            ( { model
                | state =
                    SelectDialog
                        (SS.init
                            { choices = Array.fromList <| List.map S.printTagSearch model.prevSearches
                            , selected = Nothing
                            , search = ""
                            , buttonStyle = List.map (E.mapAttribute (\_ -> SS.Noop)) Common.buttonStyle
                            }
                            (E.map (\_ -> ()) (viewState model.size model.state))
                        )
                        model.state
              }
            , Cmd.none
            )


init : Flags -> Url -> Browser.Navigation.Key -> ( Model, Cmd Msg )
init flags url key =
    let
        seed =
            initialSeed (flags.seed + 7)

        imodel =
            { state =
                case flags.login of
                    Nothing ->
                        PubShowMessage { message = "loading..." }

                    Just l ->
                        ShowMessage { message = "loading..." } l
            , size = { width = flags.width, height = flags.height }
            , location = flags.location
            , navkey = key
            , seed = seed
            , savedRoute = { route = Top, save = False }
            , prevSearches = []
            }

        prevSearchQuery =
            \login ->
                let
                    ts : S.TagSearch
                    ts =
                        S.Boolex (S.SearchTerm [ S.ExactMatch, S.Tag ] "search")
                            S.And
                            (S.SearchTerm [ S.User ] login.name)
                in
                { tagSearch = ts
                , offset = 0
                , limit = Just 50
                , what = "prevSearches"
                , list = False
                }

        ( model, cmd ) =
            parseUrl url
                |> Maybe.andThen
                    (\s ->
                        case s of
                            Top ->
                                Nothing

                            _ ->
                                Just s
                    )
                |> Maybe.andThen
                    (routeState
                        imodel
                    )
                |> Maybe.map
                    (\( rs, rcmd ) ->
                        ( { imodel
                            | state = rs
                          }
                        , rcmd
                        )
                    )
                |> Maybe.withDefault
                    (let
                        ( m, c ) =
                            case flags.login of
                                Just login ->
                                    let
                                        ( m2, c2 ) =
                                            getListing imodel login
                                    in
                                    ( m2
                                    , Cmd.batch
                                        [ sendUIMsg
                                            flags.location
                                            (UI.SearchZkNotes <| prevSearchQuery login)
                                        , c2
                                        ]
                                    )

                                Nothing ->
                                    ( { imodel | state = initLogin seed }, Cmd.none )
                     in
                     ( m
                     , Cmd.batch
                        [ c
                        , Browser.Navigation.replaceUrl key "/"
                        ]
                     )
                    )
    in
    ( model, cmd )


initLogin : Seed -> State
initLogin seed =
    Login <| Login.initialModel Nothing "zknotes" seed


main : Platform.Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = urlupdate
        , subscriptions =
            \_ ->
                Sub.batch
                    [ receiveSelectedText SelectedText
                    , Browser.Events.onResize (\w h -> WindowSize { width = w, height = h })
                    ]
        , onUrlRequest = urlRequest
        , onUrlChange = UrlChanged
        }


port getSelectedText : List String -> Cmd msg


port receiveSelectedText : (JD.Value -> msg) -> Sub msg
