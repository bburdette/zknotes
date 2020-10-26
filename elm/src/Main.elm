port module Main exposing (main)

import BadError
import Browser
import Browser.Events
import Browser.Navigation
import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Data
import Dict exposing (Dict)
import EditZk
import EditZkListing
import EditZkNote
import EditZkNoteListing
import Element exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region
import Html exposing (Attribute, Html)
import Html.Attributes
import Http
import Json.Decode as JD
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
import ShowMessage
import Url exposing (Url)
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))
import UserInterface as UI
import Util
import View


type Msg
    = LoginMsg Login.Msg
    | BadErrorMsg BadError.Msg
    | ViewMsg View.Msg
    | EditZkMsg EditZk.Msg
    | EditZkListingMsg EditZkListing.Msg
    | EditZkNoteMsg EditZkNote.Msg
    | EditZkNoteListingMsg EditZkNoteListing.Msg
    | ShowMessageMsg ShowMessage.Msg
    | UserReplyData (Result Http.Error UI.ServerResponse)
    | PublicReplyData (Result Http.Error PI.ServerResponse)
    | LoadUrl String
    | InternalUrl Url
    | SelectedText JD.Value
    | UrlChanged Url
    | WindowSize Util.Size
    | Noop


type State
    = Login Login.Model
    | EditZk EditZk.Model Data.Login
    | EditZkListing EditZkListing.Model Data.Login
    | EditZkNote EditZkNote.Model Data.Login
    | EditZkNoteListing EditZkNoteListing.Model Data.Login
    | View View.Model
    | EView View.Model State
    | BadError BadError.Model State
    | ShowMessage ShowMessage.Model Data.Login
    | PubShowMessage ShowMessage.Model
    | Wait State (State -> Msg -> ( State, Cmd Msg ))


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


type Route
    = PublicZkNote Int
    | PublicZkPubId String
    | EditZkNoteR Int
    | Fail


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

        Fail ->
            UB.absolute [] []


routeState : Model -> Route -> ( State, Cmd Msg )
routeState model route =
    let
        _ =
            Debug.log "route: " route
    in
    case route of
        PublicZkNote id ->
            ( PubShowMessage
                { message = "loading article"
                }
            , sendPIMsg model.location
                (PI.GetZkNote id)
            )

        PublicZkPubId pubid ->
            ( PubShowMessage
                { message = "loading article"
                }
            , sendPIMsg model.location
                (PI.GetZkNotePubId pubid)
            )

        EditZkNoteR id ->
            case model.state of
                EditZkNote st login ->
                    let
                        _ =
                            Debug.log "EditZkNote st login ->" id
                    in
                    loadnote model
                        { zk = st.zk
                        , login = login
                        , mbzknotesearchresult = Just st.zknSearchResult
                        , mbzklinks = Nothing
                        , mbzknote = Nothing
                        , spmodel = st.spmodel
                        , navkey = model.navkey
                        , pushUrl = False -- no need to pushUrl after load, since obvs is already in history.
                        }
                        id

                -- load the zknote in question.  will it load?
                -- should ZkNote block the load if unsaved?  I guess.
                EditZkNoteListing st login ->
                    let
                        _ =
                            Debug.log "EditZkNoteListing st login ->" id
                    in
                    loadnote model
                        { zk = st.zk
                        , login = login
                        , mbzknotesearchresult = Just st.notes
                        , mbzklinks = Nothing
                        , mbzknote = Nothing
                        , spmodel = st.spmodel
                        , navkey = model.navkey
                        , pushUrl = False -- no need to pushUrl after load, since obvs is already in history.
                        }
                        id

                st ->
                    case stateLogin st of
                        Just login ->
                            -- uh, no zk?  have to load it?
                            -- I guess check for membership/load zk
                            -- take the search results and state and load away.
                            -- don't know the zk for the note.
                            ( BadError
                                { errorMessage = "note load unimplemented from this state!"
                                }
                                model.state
                            , Browser.Navigation.replaceUrl model.navkey "/"
                            )

                        Nothing ->
                            ( initLogin model.seed
                            , Cmd.none
                              -- Browser.Navigation.replaceUrl model.navkey "/"
                            )

        Fail ->
            ( initLogin model.seed
            , Cmd.none
              --Browser.Navigation.replaceUrl model.navkey "/"
            )


stateRoute : State -> Route
stateRoute state =
    case state of
        View vst ->
            case vst.pubid of
                Just pubid ->
                    PublicZkPubId pubid

                Nothing ->
                    case vst.id of
                        Just id ->
                            PublicZkNote id

                        Nothing ->
                            Fail

        EditZkNote st login ->
            st.id
                |> Maybe.map EditZkNoteR
                |> Maybe.withDefault Fail

        _ ->
            Fail


viewState : Util.Size -> State -> Element Msg
viewState size state =
    case state of
        Login lem ->
            Element.map LoginMsg <| Login.view size lem

        EditZkListing em _ ->
            Element.map EditZkListingMsg <| EditZkListing.view em

        EditZkNote em _ ->
            Element.map EditZkNoteMsg <| EditZkNote.view size em

        EditZkNoteListing em _ ->
            Element.map EditZkNoteListingMsg <| EditZkNoteListing.view size em

        ShowMessage em _ ->
            Element.map ShowMessageMsg <| ShowMessage.view em

        PubShowMessage em ->
            Element.map ShowMessageMsg <| ShowMessage.view em

        View em ->
            Element.map ViewMsg <| View.view em False

        EView em _ ->
            Element.map ViewMsg <| View.view em True

        EditZk em _ ->
            Element.map EditZkMsg <| EditZk.view em

        BadError em _ ->
            Element.map BadErrorMsg <| BadError.view em

        Wait innerState _ ->
            Element.map (\_ -> Noop) (viewState size innerState)


stateLogin : State -> Maybe Data.Login
stateLogin state =
    case state of
        Login lmod ->
            Just { uid = lmod.userId, pwd = lmod.password }

        EditZk _ login ->
            Just login

        EditZkListing _ login ->
            Just login

        EditZkNote _ login ->
            Just login

        EditZkNoteListing _ login ->
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

        Wait wstate _ ->
            stateLogin wstate


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


type alias NwState =
    { zk : Data.Zk
    , login : Data.Login
    , mbzknotesearchresult : Maybe Data.ZkNoteSearchResult
    , mbzklinks : Maybe Data.ZkLinks
    , mbzknote : Maybe Data.FullZkNote
    , spmodel : SP.Model
    , navkey : Browser.Navigation.Key
    , pushUrl : Bool
    }


notewait : NwState -> State -> Msg -> ( State, Cmd Msg )
notewait nwstate state wmsg =
    let
        n =
            case wmsg of
                UserReplyData (Ok (UI.ZkLinks zkl)) ->
                    { nwstate | mbzklinks = Just zkl }

                UserReplyData (Ok (UI.ZkNote zkn)) ->
                    { nwstate | mbzknote = Just zkn }

                UserReplyData (Ok (UI.ZkNoteSearchResult zksr)) ->
                    { nwstate | mbzknotesearchresult = Just zksr }

                UserReplyData (Ok UI.SavedZkLinks) ->
                    nwstate

                UserReplyData (Ok (UI.SavedZkNote _)) ->
                    nwstate

                -- TODO error state for errors, error state for unexpected msgs.
                -- UserReplyData (Err e) ->
                --     BadError (BadError.initialModel <| Util.httpErrorString e) state
                _ ->
                    nwstate
    in
    case ( n.mbzknotesearchresult, n.mbzklinks, n.mbzknote ) of
        ( Just zknl, Just zkl, Just zkn ) ->
            let
                st =
                    EditZkNote (EditZkNote.initFull n.zk zknl zkn zkl n.spmodel) n.login
            in
            ( st
            , if n.pushUrl then
                Browser.Navigation.pushUrl n.navkey (routeUrl (stateRoute st))

              else
                Cmd.none
            )

        _ ->
            ( Wait state (notewait n), Cmd.none )


loadnote : Model -> NwState -> Int -> ( State, Cmd Msg )
loadnote model nwstate zknid =
    let
        nws =
            { nwstate | mbzklinks = Nothing, mbzknote = Nothing }
    in
    ( Wait
        (ShowMessage
            { message = "loading zknote" }
            nws.login
        )
        (notewait nws)
    , Cmd.batch <|
        (case nws.mbzknotesearchresult of
            Nothing ->
                [ sendUIMsg model.location
                    nws.login
                    (UI.SearchZkNotes
                        (SP.getSearch nws.spmodel
                            |> Maybe.withDefault (S.defaultSearch nws.zk.id)
                        )
                    )
                ]

            Just rs ->
                []
        )
            ++ [ sendUIMsg model.location
                    nws.login
                    (UI.GetZkNote zknid)
               , sendUIMsg model.location
                    nws.login
                    (UI.GetZkLinks { zknote = zknid, zk = nws.zk.id })
               ]
    )


listingwait : Data.Login -> Data.Zk -> State -> Msg -> ( State, Cmd Msg )
listingwait login zk st ms =
    case ms of
        UserReplyData (Ok (UI.ZkNoteSearchResult rs)) ->
            ( EditZkNoteListing
                { zk = zk
                , notes = rs
                , spmodel =
                    SP.searchResultUpdated rs (SP.initModel zk.id)
                }
                login
            , Cmd.none
            )

        UserReplyData (Ok (UI.ServerError e)) ->
            ( BadError (BadError.initialModel e) st, Cmd.none )

        _ ->
            ( BadError (BadError.initialModel "unexpected message!") st
            , Cmd.none
            )


noteviewwait : State -> State -> Msg -> ( State, Cmd Msg )
noteviewwait backstate st ms =
    case ms of
        UserReplyData (Ok (UI.ZkNote zkn)) ->
            ( EView (View.initFull zkn) backstate, Cmd.none )

        UserReplyData (Ok (UI.ServerError e)) ->
            ( BadError (BadError.initialModel e) st, Cmd.none )

        _ ->
            ( BadError (BadError.initialModel "unexpected message!") st
            , Cmd.none
            )


view : Model -> { title : String, body : List (Html Msg) }
view model =
    { title = "zknotes"
    , body =
        [ Element.layout [] <|
            viewState model.size model.state
        ]
    }


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case ( msg, model.state ) of
        ( _, Wait wst wfn ) ->
            let
                ( nst, cmd ) =
                    wfn model.state msg
            in
            ( { model | state = nst }, cmd )

        ( WindowSize s, _ ) ->
            ( { model | size = s }, Cmd.none )

        ( UrlChanged url, state ) ->
            case parseUrl url of
                Just route ->
                    let
                        _ =
                            Debug.log "urlchanged: " ( url, route )
                    in
                    if route == stateRoute state then
                        ( model, Cmd.none )

                    else
                        let
                            ( st, cmd ) =
                                routeState model route
                        in
                        ( { model | state = st }, cmd )

                Nothing ->
                    let
                        _ =
                            Debug.log "urlchanged, nothing: " url
                    in
                    -- load other site??
                    ( model, Browser.Navigation.load (Url.toString url) )

        ( SelectedText jv, state ) ->
            case JD.decodeValue JD.string jv of
                Ok str ->
                    case state of
                        EditZkNote emod login ->
                            let
                                ( s, cmd ) =
                                    EditZkNote.gotSelectedText emod str
                            in
                            case cmd of
                                EditZkNote.Save szk zklinks ->
                                    ( { model
                                        | state =
                                            Wait
                                                (ShowMessage
                                                    { message = "waiting for zknote id"
                                                    }
                                                    login
                                                )
                                                (\st ms ->
                                                    -- discard
                                                    case ms of
                                                        UserReplyData (Ok (UI.SavedZkNote szkn)) ->
                                                            ( EditZkNote
                                                                (EditZkNote.addListNote s szk szkn)
                                                                login
                                                            , Cmd.none
                                                            )

                                                        _ ->
                                                            ( BadError (BadError.initialModel "unexpected message after zknote save") st, Cmd.none )
                                                )
                                      }
                                    , Cmd.batch
                                        [ sendUIMsg model.location
                                            login
                                            (UI.SaveZkNote szk)
                                        , sendUIMsg model.location
                                            login
                                          <|
                                            UI.SaveZkLinks
                                                { zk = emod.zk.id
                                                , links = zklinks
                                                }
                                        ]
                                    )

                                _ ->
                                    ( { model | state = EditZkNote s login }, Cmd.none )

                        _ ->
                            ( model, Cmd.none )

                Err e ->
                    ( { model | state = BadError (BadError.initialModel <| JD.errorToString e) model.state }, Cmd.none )

        ( InternalUrl url, _ ) ->
            let
                mblogin =
                    stateLogin model.state

                ( state, cmd ) =
                    parseUrl url
                        |> Maybe.map (routeState model)
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
                            let
                                vstate =
                                    View (View.initFull fbe)
                            in
                            ( { model | state = vstate }
                            , Browser.Navigation.pushUrl model.navkey
                                (routeUrl (stateRoute vstate))
                            )

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
                                        UI.GetZkListing
                                    )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected login reply") state }
                                    , Cmd.none
                                    )

                        UI.ZkListing l ->
                            case state of
                                ShowMessage _ login ->
                                    ( { model | state = EditZkListing { zks = l } login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected login reply") state }
                                    , Cmd.none
                                    )

                        UI.ZkNoteSearchResult sr ->
                            case state of
                                EditZkNoteListing znlstate login_ ->
                                    ( { model | state = EditZkNoteListing (EditZkNoteListing.updateSearchResult sr znlstate) login_ }
                                    , Cmd.none
                                    )

                                EditZkNote znstate login_ ->
                                    ( { model | state = EditZkNote (EditZkNote.updateSearchResult sr znstate) login_ }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected zknote listing") state }
                                    , Cmd.none
                                    )

                        UI.ZkNote _ ->
                            case state of
                                _ ->
                                    ( { model | state = BadError (BadError.initialModel <| "unexpected message: zknote") state }, Cmd.none )

                        UI.SavedZk beid ->
                            case state of
                                EditZk emod login ->
                                    ( { model | state = EditZk (EditZk.setId emod beid) login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected message: savedzk") state }, Cmd.none )

                        UI.DeletedZk _ ->
                            case state of
                                ShowMessage _ login ->
                                    ( model
                                    , sendUIMsg model.location login UI.GetZkListing
                                    )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected message") state }, Cmd.none )

                        UI.ZkMembers _ ->
                            ( { model | state = BadError (BadError.initialModel "unexpected zkmembers message") state }, Cmd.none )

                        UI.AddedZkMember zkm ->
                            case state of
                                EditZk ezk login ->
                                    ( { model | state = EditZk (EditZk.addedZkMember ezk zkm) login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected zkmembers message") state }, Cmd.none )

                        UI.DeletedZkMember zkm ->
                            case state of
                                EditZk ezk login ->
                                    ( { model | state = EditZk (EditZk.deletedZkMember ezk zkm) login }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected zkmembers message") state }, Cmd.none )

                        UI.SavedZkNote szkn ->
                            case state of
                                EditZkNote emod login ->
                                    let
                                        ( eznst, pushurl ) =
                                            EditZkNote.gotId emod szkn.id

                                        st =
                                            EditZkNote eznst login
                                    in
                                    ( { model | state = st }
                                    , if pushurl then
                                        Browser.Navigation.pushUrl model.navkey (routeUrl (stateRoute st))

                                      else
                                        Cmd.none
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
                                    ( { model | state = BadError (BadError.initialModel "unexpected message") state }, Cmd.none )

                        UI.UnregisteredUser ->
                            case state of
                                Login lmod ->
                                    ( { model | state = Login <| Login.unregisteredUser lmod }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected message") state }, Cmd.none )

                        UI.InvalidUserOrPwd ->
                            case state of
                                Login lmod ->
                                    ( { model | state = Login <| Login.invalidUserOrPwd lmod }, Cmd.none )

                                _ ->
                                    ( { model | state = BadError (BadError.initialModel "unexpected message") state }, Cmd.none )

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

        ( EditZkMsg em, EditZk es login ) ->
            let
                ( emod, ecmd ) =
                    EditZk.update em es
            in
            case ecmd of
                EditZk.Save zk ->
                    ( { model | state = EditZk emod login }
                    , sendUIMsg model.location
                        login
                        (UI.SaveZk zk)
                    )

                EditZk.None ->
                    ( { model | state = EditZk emod login }, Cmd.none )

                EditZk.Done ->
                    ( { model
                        | state =
                            ShowMessage
                                { message = "loading articles"
                                }
                                login
                      }
                    , sendUIMsg model.location
                        login
                        UI.GetZkListing
                    )

                EditZk.Delete id ->
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
                        (UI.DeleteZk id)
                    )

                EditZk.View sbe ->
                    ( { model | state = BadError (BadError.initialModel "EditZk.View sbe -> unimplmeented") model.state }
                    , Cmd.none
                    )

                EditZk.AddZkMember zkm ->
                    ( model
                    , sendUIMsg model.location
                        login
                        (UI.AddZkMember zkm)
                    )

                EditZk.DeleteZkMember zkm ->
                    ( model
                    , sendUIMsg model.location
                        login
                        (UI.DeleteZkMember zkm)
                    )

        ( EditZkNoteMsg em, EditZkNote es login ) ->
            let
                ( emod, ecmd ) =
                    EditZkNote.update em es

                backtolisting =
                    ( { model
                        | state =
                            EditZkNoteListing { zk = emod.zk, notes = emod.zknSearchResult, spmodel = emod.spmodel } login
                      }
                    , sendUIMsg model.location
                        login
                        (UI.SearchZkNotes (S.defaultSearch es.zk.id))
                    )
            in
            case ecmd of
                EditZkNote.SaveExit szk zklinks ->
                    let
                        gotres =
                            ( EditZkNoteListing
                                { zk = es.zk
                                , notes = emod.zknSearchResult
                                , spmodel = emod.spmodel
                                }
                                login
                            , case SP.getSearch emod.spmodel of
                                Just s ->
                                    sendUIMsg model.location
                                        login
                                        (UI.SearchZkNotes s)

                                Nothing ->
                                    Cmd.none
                            )

                        savefn : Bool -> Bool -> State -> Msg -> ( State, Cmd Msg )
                        savefn gotsn gotsl st ms =
                            case ms of
                                UserReplyData (Ok (UI.SavedZkNote szn)) ->
                                    if gotsl then
                                        gotres

                                    else
                                        ( Wait st (savefn True False), Cmd.none )

                                UserReplyData (Ok UI.SavedZkLinks) ->
                                    if gotsn then
                                        gotres

                                    else
                                        ( Wait st (savefn False True), Cmd.none )

                                UserReplyData (Ok (UI.ServerError e)) ->
                                    ( BadError (BadError.initialModel e) st, Cmd.none )

                                _ ->
                                    ( BadError (BadError.initialModel "unexpected message!") model.state
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
                                (savefn False False)
                      }
                    , Cmd.batch
                        [ sendUIMsg model.location
                            login
                            (UI.SaveZkNote szk)
                        , sendUIMsg model.location
                            login
                          <|
                            UI.SaveZkLinks
                                { zk = emod.zk.id
                                , links = zklinks
                                }
                        ]
                    )

                EditZkNote.Save szk zklinks ->
                    ( { model | state = EditZkNote emod login }
                    , Cmd.batch
                        [ sendUIMsg model.location
                            login
                            (UI.SaveZkNote szk)
                        , sendUIMsg model.location
                            login
                          <|
                            UI.SaveZkLinks
                                { zk = emod.zk.id
                                , links = zklinks
                                }
                        ]
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
                                (\state _ ->
                                    ( m.state, c )
                                )
                      }
                    , sendUIMsg model.location
                        login
                        (UI.DeleteZkNote id)
                    )

                EditZkNote.Switch id ->
                    let
                        ( st, cmd ) =
                            loadnote model
                                { zk = emod.zk
                                , login = login
                                , mbzknotesearchresult = Nothing
                                , mbzklinks = Nothing
                                , mbzknote = Nothing
                                , spmodel = emod.spmodel
                                , navkey = model.navkey
                                , pushUrl = True
                                }
                                id
                    in
                    ( { model | state = st }, cmd )

                EditZkNote.SaveSwitch szkn zklinks id ->
                    let
                        ( st, cmd ) =
                            loadnote model
                                { zk = emod.zk
                                , login = login
                                , mbzknotesearchresult = Nothing
                                , mbzklinks = Nothing
                                , mbzknote = Nothing
                                , spmodel = emod.spmodel
                                , navkey = model.navkey
                                , pushUrl = True
                                }
                                id
                    in
                    ( { model | state = st }
                    , Cmd.batch
                        [ cmd
                        , sendUIMsg model.location
                            login
                            (UI.SaveZkNote szkn)
                        , sendUIMsg model.location
                            login
                          <|
                            UI.SaveZkLinks
                                { zk = emod.zk.id
                                , links = zklinks
                                }
                        ]
                    )

                EditZkNote.View szn ->
                    ( { model | state = EView (View.initSzn szn) (EditZkNote es login) }, Cmd.none )

                EditZkNote.GetSelectedText id ->
                    ( { model | state = EditZkNote emod login }
                    , getSelectedText (Just id)
                    )

                EditZkNote.Search s ->
                    ( { model | state = EditZkNote emod login }
                    , sendUIMsg model.location
                        login
                        (UI.SearchZkNotes s)
                    )

        ( EditZkListingMsg em, EditZkListing es login ) ->
            let
                ( emod, ecmd ) =
                    EditZkListing.update em es
            in
            case ecmd of
                EditZkListing.New ->
                    ( { model | state = EditZk EditZk.initNew login }, Cmd.none )

                EditZkListing.Selected zk ->
                    ( { model
                        | state =
                            Wait
                                (ShowMessage
                                    { message = "loading zk members"
                                    }
                                    login
                                )
                                (\st ms ->
                                    case ms of
                                        UserReplyData (Ok (UI.ZkMembers list)) ->
                                            ( EditZk (EditZk.initFull zk list) login, Cmd.none )

                                        _ ->
                                            ( BadError (BadError.initialModel "unexpected message instead of zk member list") st, Cmd.none )
                                )
                      }
                    , sendUIMsg model.location
                        login
                        (UI.GetZkMembers zk.id)
                    )

                EditZkListing.Notes zk ->
                    ( { model
                        | state =
                            Wait
                                (ShowMessage
                                    { message = "loading articles"
                                    }
                                    login
                                )
                                (listingwait login zk)
                      }
                    , sendUIMsg model.location
                        login
                        (UI.SearchZkNotes <| S.defaultSearch zk.id)
                    )

                EditZkListing.View id ->
                    ( { model
                        | state =
                            Wait
                                (ShowMessage
                                    { message = "loading zk members"
                                    }
                                    login
                                )
                                (noteviewwait model.state)
                      }
                    , sendUIMsg model.location
                        login
                        (UI.GetZk id)
                    )

        ( EditZkNoteListingMsg em, EditZkNoteListing es login ) ->
            let
                ( emod, ecmd ) =
                    EditZkNoteListing.update em es
            in
            case ecmd of
                EditZkNoteListing.None ->
                    ( { model | state = EditZkNoteListing emod login }, Cmd.none )

                EditZkNoteListing.New ->
                    ( { model | state = EditZkNote (EditZkNote.initNew emod.zk es.notes emod.spmodel) login }, Cmd.none )

                EditZkNoteListing.Example ->
                    ( { model | state = EditZkNote (EditZkNote.initExample emod.zk es.notes emod.spmodel) login }, Cmd.none )

                EditZkNoteListing.Selected id ->
                    let
                        ( st, cmd ) =
                            loadnote model
                                { zk = emod.zk
                                , login = login
                                , mbzknotesearchresult = Just emod.notes
                                , mbzklinks = Nothing
                                , mbzknote = Nothing
                                , spmodel = emod.spmodel
                                , navkey = model.navkey
                                , pushUrl = True
                                }
                                id
                    in
                    ( { model | state = st }, cmd )

                EditZkNoteListing.View id ->
                    ( { model
                        | state =
                            Wait
                                (ShowMessage
                                    { message = "loading zknote" }
                                    login
                                )
                                (noteviewwait model.state)
                      }
                    , sendUIMsg model.location
                        login
                        (UI.GetZkNote id)
                    )

                EditZkNoteListing.Done ->
                    -- back to the Zk listing.
                    ( { model
                        | state =
                            ShowMessage
                                { message = "loading zk listing"
                                }
                                login
                      }
                    , sendUIMsg model.location
                        login
                        UI.GetZkListing
                    )

                EditZkNoteListing.Search s ->
                    ( { model | state = EditZkNoteListing emod login }
                    , sendUIMsg model.location
                        login
                        (UI.SearchZkNotes s)
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
        seed =
            initialSeed (flags.seed + 7)

        model =
            { state = PubShowMessage { message = "initial state" }
            , size = { width = flags.width, height = flags.height }
            , location = flags.location
            , navkey = key
            , seed = seed
            }

        ( state, cmd ) =
            parseUrl url
                |> Maybe.map
                    (routeState
                        model
                    )
                |> Maybe.withDefault
                    ( initLogin seed
                    , Cmd.none
                      -- , Browser.Navigation.replaceUrl key "/"
                    )
    in
    ( { model | state = state }
    , cmd
    )


initLogin : Seed -> State
initLogin seed =
    Login <| Login.initialModel Nothing "zknotes" seed


main : Platform.Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions =
            \_ ->
                Sub.batch
                    [ receiveSelectedText SelectedText
                    , Browser.Events.onResize (\w h -> WindowSize { width = w, height = h })
                    ]
        , onUrlRequest = urlRequest
        , onUrlChange = UrlChanged
        }


port getSelectedText : Maybe String -> Cmd msg


port receiveSelectedText : (JD.Value -> msg) -> Sub msg
