port module Main exposing (main)

import ArchiveListing
import Browser
import Browser.Events
import Browser.Navigation
import Common
import Data exposing (EditTab(..), PrivateClosureRequest, ZkNoteId(..))
import DataUtil exposing (FileUrlInfo, LoginData, jobComplete, showPrivateReply)
import Dict exposing (Dict)
import DisplayMessage
import EdMarkdown as EM
import EditZkNote
import EditZkNoteListing
import Either exposing (Either(..))
import Element as E exposing (Element)
import Element.Font as EF
import File as F
import File.Select as FS
import GenDialog as GD
import Html exposing (Html)
import Http
import HttpJsonTask as HE
import Import
import InviteUser
import JobsDialog exposing (TJobs)
import Json.Decode as JD
import Json.Encode as JE
import LocalStorage as LS
import MdCommon as MC
import MdInlineXform
import MessageNLink
import NoteCache as NC exposing (NoteCache)
import Orgauth.ChangeEmail as CE
import Orgauth.ChangePassword as CP
import Orgauth.ChangeRemoteUrl as CRU
import Orgauth.Data as OD
import Orgauth.DataUtil as ODU
import Orgauth.Invited as Invited
import Orgauth.Login as Login
import Orgauth.ResetPassword as ResetPassword
import Orgauth.ShowUrl as ShowUrl
import Orgauth.UserEdit as UserEdit
import Orgauth.UserId exposing (getUserIdVal)
import Orgauth.UserListing as UserListing
import Platform.Cmd as Cmd
import Random exposing (Seed, initialSeed)
import RequestsDialog exposing (TRequest(..), TRequests)
import Route exposing (Route(..), parseUrl, routeTitle, routeUrl)
import SearchStackPanel as SP
import SearchUtil as SU
import SelectString as SS
import ShowMessage
import SpecialNotes as SN
import TSet
import TagAThing
import TagFiles
import TagNotes
import TagNotes2
import Task
import Time
import Toop
import UUID
import Url exposing (Url)
import UserSettings
import Util exposing (andMap)
import View
import WindowKeys
import ZkCommon exposing (StylePalette)


type Msg
    = LoginMsg Login.Msg
    | InvitedMsg Invited.Msg
    | ViewMsg View.Msg
    | EditZkNoteMsg EditZkNote.Msg
    | EditZkNoteListingMsg EditZkNoteListing.Msg
    | ArchiveListingMsg ArchiveListing.Msg
    | UserSettingsMsg UserSettings.Msg
    | UserListingMsg UserListing.Msg
    | UserEditMsg UserEdit.Msg
    | ImportMsg Import.Msg
    | ShowMessageMsg ShowMessage.Msg
    | UserReplyData (Result Http.Error OD.UserResponse)
    | AdminReplyData (Result Http.Error OD.AdminResponse)
    | ZkReplyData (Result Http.Error ( Time.Posix, Data.PrivateReply ))
    | ZkReplyDataSeq (Result Http.Error ( Time.Posix, Data.PrivateReply ) -> Maybe (Cmd Msg)) (Result Http.Error ( Time.Posix, Data.PrivateReply ))
    | TAReplyData DataUtil.TASelection (Result Http.Error ( Time.Posix, Data.PrivateReply ))
    | PublicReplyData (Result Http.Error ( Time.Posix, Data.PublicReply ))
    | ErrorIndexNote (Result Http.Error Data.PublicReply)
    | TauriZkReplyData JD.Value
    | TauriUserReplyData JD.Value
    | TauriAdminReplyData JD.Value
    | TauriPublicReplyData JD.Value
    | TauriTauriReplyData JD.Value
    | LoadUrl String
    | InternalUrl Url
    | TASelection JD.Value
    | TAError JD.Value
    | UrlChanged Url
    | WindowSize Util.Size
    | DisplayMessageMsg (GD.Msg DisplayMessage.Msg)
    | MessageNLinkMsg (GD.Msg MessageNLink.Msg)
    | SelectDialogMsg (GD.Msg (SS.Msg Int))
    | ChangePasswordDialogMsg (GD.Msg CP.Msg)
    | ChangeEmailDialogMsg (GD.Msg CE.Msg)
    | ChangeRemoteUrlDialogMsg (GD.Msg CRU.Msg)
    | ResetPasswordMsg ResetPassword.Msg
    | ShowUrlMsg ShowUrl.Msg
    | Zone Time.Zone
    | WkMsg (Result JD.Error WindowKeys.Key)
    | ReceiveLocalVal { for : String, name : String, value : Maybe String }
    | OnFileSelected F.File (List F.File)
    | FileUploadedButGetTime String (Result Http.Error Data.UploadReply)
    | FileUploaded String (Result Http.Error ( Time.Posix, Data.UploadReply ))
    | RequestProgress String Http.Progress
    | RequestsDialogMsg (GD.Msg RequestsDialog.Msg)
    | JobsDialogMsg (GD.Msg JobsDialog.Msg)
    | MdInlineXformMsg (GD.Msg MdInlineXform.Msg)
    | MdInlineXformCmd MdInlineXform.Command
    | TagFilesMsg (TagAThing.Msg TagFiles.Msg)
    | TagNotesMsg (TagAThing.Msg TagNotes.Msg)
    | TagNotes2Msg TagNotes2.Msg
    | InviteUserMsg (TagAThing.Msg InviteUser.Msg)
    | JobsPollTick Time.Posix
    | Noop


type State
    = Login Login.Model Route
    | Invited Invited.Model
    | EditZkNote EditZkNote.Model LoginData
    | EditZkNoteListing EditZkNoteListing.Model LoginData
    | ArchiveAwait ZkNoteId ZkNoteId LoginData
    | ArchiveListing ArchiveListing.Model LoginData
    | View View.Model
    | EView View.Model State
    | Import Import.Model LoginData
    | ShowMessage ShowMessage.Model LoginData (Maybe State)
    | PubShowMessage ShowMessage.Model (Maybe State)
    | LoginShowMessage ShowMessage.Model LoginData Url
    | SelectDialog (SS.GDModel Int) State
    | ChangePasswordDialog CP.GDModel State
    | ChangeEmailDialog CE.GDModel State
    | ChangeRemoteUrlDialog CRU.GDModel State
    | ResetPassword ResetPassword.Model
    | UserListing UserListing.Model LoginData
    | UserEdit UserEdit.Model LoginData
    | UserSettings UserSettings.Model LoginData State
    | ShowUrl ShowUrl.Model LoginData
    | DisplayMessage DisplayMessage.GDModel State
    | MessageNLink MessageNLink.GDModel State
    | RequestsDialog RequestsDialog.GDModel State
    | JobsDialog JobsDialog.GDModel State
    | TagFiles (TagAThing.Model TagFiles.Model TagFiles.Msg TagFiles.Command) LoginData State
    | TagNotes (TagAThing.Model TagNotes.Model TagNotes.Msg TagNotes.Command) LoginData State
    | TagNotes2 TagNotes2.Model LoginData State
    | InviteUser (TagAThing.Model InviteUser.Model InviteUser.Msg InviteUser.Command) LoginData State
    | MdInlineXform MdInlineXform.GDModel State
    | Wait State (Model -> Msg -> ( Model, Cmd Msg ))


decodeFlags : JD.Decoder Flags
decodeFlags =
    JD.succeed Flags
        |> andMap (JD.field "seed" JD.int)
        |> andMap (JD.field "location" JD.string)
        |> andMap (JD.field "filelocation" JD.string)
        |> andMap (JD.field "useragent" JD.string)
        |> andMap (JD.field "debugstring" JD.string)
        |> andMap (JD.field "width" JD.int)
        |> andMap (JD.field "height" JD.int)
        |> andMap (JD.field "errorid" (JD.maybe Data.zkNoteIdDecoder))
        |> andMap (JD.field "login" (JD.maybe DataUtil.decodeLoginData))
        |> andMap (JD.field "adminsettings" OD.adminSettingsDecoder)
        |> andMap (JD.field "tauri" JD.bool)
        |> andMap (JD.field "mobile" JD.bool)


type alias Flags =
    { seed : Int
    , location : String
    , filelocation : String
    , useragent : String
    , debugstring : String
    , width : Int
    , height : Int
    , errorid : Maybe ZkNoteId
    , login : Maybe DataUtil.LoginData
    , adminsettings : OD.AdminSettings
    , tauri : Bool
    , mobile : Bool
    }


type alias SavedRoute =
    { route : Route
    , save : Bool
    }


maxCacheNotes : Int
maxCacheNotes =
    100


type alias Model =
    { state : State
    , size : Util.Size
    , fui : FileUrlInfo
    , navkey : Browser.Navigation.Key
    , seed : Seed
    , timezone : Time.Zone
    , savedRoute : SavedRoute
    , initialRoute : Route
    , prevSearches : List (List Data.TagSearch)
    , recentNotes : List Data.ZkListNote
    , errorNotes : Dict String String
    , stylePalette : StylePalette
    , adminSettings : OD.AdminSettings
    , trackedRequests : TRequests
    , jobs : TJobs
    , noteCache : NoteCache
    , ziClosureId : Int
    , ziClosures : Dict Int (Result Http.Error ( Time.Posix, Data.PrivateReply ) -> Msg)
    , mobile : Bool
    , spmodel : SP.Model
    , zknSearchResult : Data.ZkListNoteSearchResult
    }


type alias PreInitModel =
    { flags : Flags
    , url : Url
    , key : Browser.Navigation.Key
    , mbzone : Maybe Time.Zone
    , mbfontsize : Maybe Int
    }


type PiModel
    = Ready Model
    | PreInit PreInitModel
    | InitError String


initLoginState : Model -> Route -> State
initLoginState model initroute =
    Login (Login.initialModel Nothing model.adminSettings "zknotes" model.seed) initroute


urlRequest : Browser.UrlRequest -> Msg
urlRequest ur =
    case ur of
        Browser.Internal url ->
            InternalUrl url

        Browser.External str ->
            LoadUrl str


routeState : Model -> Route -> ( State, Cmd Msg )
routeState model route =
    let
        ( st, cmds ) =
            routeStateInternal model route
    in
    ( st, cmds )


routeStateInternal : Model -> Route -> ( State, Cmd Msg )
routeStateInternal model route =
    case route of
        LoginR ->
            ( initLoginState model route, Cmd.none )

        PublicZkNote id ->
            case stateLogin model.state of
                Just login ->
                    ( ShowMessage
                        { message = "loading article"
                        }
                        login
                        (Just model.state)
                    , case model.state of
                        EView _ _ ->
                            -- if we're in "EView" then do this request to stay in EView.
                            sendPIMsg model.fui
                                (Data.PbrGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = Nothing
                                    }
                                )

                        _ ->
                            sendZIMsg model.fui
                                (Data.PvqGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = Nothing
                                    }
                                )
                    )

                Nothing ->
                    ( PubShowMessage
                        { message = "loading article"
                        }
                        (Just model.state)
                    , sendPIMsg model.fui
                        (Data.PbrGetZkNoteAndLinks
                            { zknote = id
                            , what = ""
                            , edittab = Nothing
                            }
                        )
                    )

        PublicZkPubId pubid ->
            ( case stateLogin model.state of
                Just login ->
                    ShowMessage
                        { message = "loading article"
                        }
                        login
                        (Just model.state)

                Nothing ->
                    PubShowMessage
                        { message = "loading article"
                        }
                        (Just model.state)
            , sendPIMsg model.fui (Data.PbrGetZkNotePubId pubid)
            )

        EditZkNoteR id mbtab ->
            case model.state of
                -- if the id is the same but the edit tab has changed, just change the edit tab.
                EditZkNote st login ->
                    case ( mbtab, st.id == Just id ) of
                        ( Just et, True ) ->
                            ( EditZkNote (EditZkNote.setTab et st) login, Cmd.none )

                        _ ->
                            ( EditZkNote st login
                            , sendZIMsg model.fui
                                (Data.PvqGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = mbtab
                                    }
                                )
                            )

                EditZkNoteListing st login ->
                    ( EditZkNoteListing st login
                    , sendZIMsg model.fui
                        (Data.PvqGetZkNoteAndLinks
                            { zknote = id
                            , what = ""
                            , edittab = mbtab
                            }
                        )
                    )

                EView st login ->
                    ( EView st login
                    , sendPIMsg model.fui
                        (Data.PbrGetZkNoteAndLinks
                            { zknote = id
                            , what = ""
                            , edittab = mbtab
                            }
                        )
                    )

                st ->
                    case stateLogin st of
                        Just login ->
                            ( ShowMessage { message = "loading note..." }
                                login
                                (Just model.state)
                            , sendZIMsg model.fui
                                (Data.PvqGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = mbtab
                                    }
                                )
                            )

                        Nothing ->
                            ( PubShowMessage { message = "loading note..." }
                                (Just model.state)
                            , sendPIMsg model.fui
                                (Data.PbrGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = mbtab
                                    }
                                )
                            )

        EditZkNoteNew ->
            case model.state of
                EditZkNote st login ->
                    -- handleEditZkNoteCmd should return state probably, or this function should return model.
                    let
                        ( nm, cmd ) =
                            handleEditZkNoteCmd model login (EditZkNote.newWithSave st)
                    in
                    ( nm.state, cmd )

                EditZkNoteListing st login ->
                    ( EditZkNote (EditZkNote.initNew model.fui login [] model.mobile) login, Cmd.none )

                st ->
                    case stateLogin st of
                        Just login ->
                            ( EditZkNote
                                (EditZkNote.initNew model.fui
                                    login
                                    []
                                    model.mobile
                                )
                                login
                            , Cmd.none
                            )

                        Nothing ->
                            -- err 'you're not logged in.'
                            ( (displayMessageDialog { model | state = initLoginState model route } "can't create a new note; you're not logged in!").state, Cmd.none )

        ArchiveNoteListingR id ->
            case model.state of
                ArchiveListing st login ->
                    ( ArchiveListing st login
                    , sendZIMsg model.fui
                        (Data.PvqGetZkNoteArchives
                            { zknote = id
                            , offset = 0
                            , limit = Just SU.defaultSearchLimit
                            }
                        )
                    )

                st ->
                    case stateLogin st of
                        Just login ->
                            ( ShowMessage { message = "loading archives..." }
                                login
                                (Just model.state)
                            , sendZIMsg model.fui
                                (Data.PvqGetZkNoteArchives
                                    { zknote = id
                                    , offset = 0
                                    , limit = Just SU.defaultSearchLimit
                                    }
                                )
                            )

                        Nothing ->
                            ( model.state, Cmd.none )

        ArchiveNoteR id aid ->
            case stateLogin model.state of
                Just login ->
                    ( ArchiveAwait id aid login
                    , sendZIMsg
                        model.fui
                        (Data.PvqGetZkNoteArchives
                            { zknote = id
                            , offset = 0
                            , limit = Just SU.defaultSearchLimit
                            }
                        )
                    )

                Nothing ->
                    ( model.state, Cmd.none )

        ResetPasswordR username key ->
            ( ResetPassword <| ResetPassword.initialModel username key "zknotes", Cmd.none )

        SettingsR ->
            case stateLogin model.state of
                Just login ->
                    ( UserSettings (UserSettings.init login model.stylePalette.fontSize) login model.state, Cmd.none )

                Nothing ->
                    ( (displayMessageDialog { model | state = initLoginState model route } "can't view user settings; you're not logged in!").state, Cmd.none )

        Invite token ->
            ( PubShowMessage { message = "retrieving invite" } Nothing
            , sendUIMsg model.fui (OD.UrqReadInvite token)
            )

        Top ->
            case stateLogin model.state of
                Just login ->
                    case login.homenote of
                        Just id ->
                            ( model.state
                            , Cmd.batch
                                [ sendZIMsg model.fui
                                    (Data.PvqSearchZkNotes <| prevSearchQuery login)
                                , sendZIMsg model.fui
                                    (Data.PvqGetZkNoteAndLinks
                                        { zknote = id
                                        , what = ""
                                        , edittab = Nothing
                                        }
                                    )
                                ]
                            )

                        Nothing ->
                            ( EditZkNote
                                (EditZkNote.initNew model.fui
                                    login
                                    []
                                    model.mobile
                                )
                                login
                            , sendZIMsg model.fui
                                (Data.PvqSearchZkNotes <| prevSearchQuery login)
                            )

                Nothing ->
                    ( initLoginState model route, Cmd.none )


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

        EView vst _ ->
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

        EditZkNote st _ ->
            st.id
                |> Maybe.map (\id -> { route = EditZkNoteR id (Just st.tab), save = True })
                |> Maybe.withDefault { route = EditZkNoteNew, save = False }

        ArchiveListing almod _ ->
            almod.selected
                |> Maybe.map (\sid -> { route = ArchiveNoteR almod.noteid sid, save = True })
                |> Maybe.withDefault { route = ArchiveNoteListingR almod.noteid, save = True }

        Login _ _ ->
            { route = LoginR
            , save = False
            }

        UserSettings _ _ _ ->
            { route = SettingsR
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

        InvitedMsg _ ->
            "InvitedMsg"

        DisplayMessageMsg _ ->
            "DisplayMessage"

        MessageNLinkMsg _ ->
            "MessageNLink"

        ViewMsg _ ->
            "ViewMsg"

        EditZkNoteMsg _ ->
            "EditZkNoteMsg"

        EditZkNoteListingMsg _ ->
            "EditZkNoteListingMsg"

        ArchiveListingMsg _ ->
            "ArchiveListingMsg"

        UserSettingsMsg _ ->
            "UserSettingsMsg"

        ImportMsg _ ->
            "ImportMsg"

        ShowMessageMsg _ ->
            "ShowMessageMsg"

        UserReplyData urd ->
            "UserReplyData: "
                ++ (Result.map ODU.showUserResponse urd
                        |> Result.mapError Util.httpErrorString
                        |> (\r ->
                                case r of
                                    Ok m ->
                                        "message: " ++ m

                                    Err e ->
                                        "error: " ++ e
                           )
                   )

        AdminReplyData urd ->
            "AdminReplyData: "
                ++ (Result.map ODU.showAdminResponse urd
                        |> Result.mapError Util.httpErrorString
                        |> (\r ->
                                case r of
                                    Ok m ->
                                        "message: " ++ m

                                    Err e ->
                                        "error: " ++ e
                           )
                   )

        ZkReplyData urd ->
            "ZkReplyData: "
                ++ (case urd of
                        Ok ( _, m ) ->
                            "message: " ++ showPrivateReply m

                        Err e ->
                            "error: " ++ Util.httpErrorString e
                   )

        ZkReplyDataSeq _ urd ->
            "ZkReplyDataSeq : "
                ++ (case urd of
                        Ok ( _, m ) ->
                            "message: " ++ showPrivateReply m

                        Err e ->
                            "error: " ++ Util.httpErrorString e
                   )

        TAReplyData _ urd ->
            "TAReplyData: "
                ++ (case urd of
                        Ok ( _, m ) ->
                            "message: " ++ showPrivateReply m

                        Err e ->
                            "error: " ++ Util.httpErrorString e
                   )

        PublicReplyData _ ->
            "PublicReplyData"

        ErrorIndexNote _ ->
            "ErrorIndexNote"

        LoadUrl _ ->
            "LoadUrl"

        InternalUrl _ ->
            "InternalUrl"

        TASelection _ ->
            "TASelection"

        TAError _ ->
            "TAError"

        UrlChanged _ ->
            "UrlChanged"

        WindowSize _ ->
            "WindowSize"

        Noop ->
            "Noop"

        WkMsg _ ->
            "WkMsg"

        ReceiveLocalVal _ ->
            "ReceiveLocalVal"

        TauriZkReplyData _ ->
            "TauriReceiveZkReplyData"

        TauriUserReplyData _ ->
            "TauriUserReplyData"

        TauriAdminReplyData _ ->
            "TauriAdminReplyData"

        TauriPublicReplyData _ ->
            "TauriPublicReplyData"

        TauriTauriReplyData _ ->
            "TauriTauriReplyData"

        SelectDialogMsg _ ->
            "SelectDialogMsg"

        ChangePasswordDialogMsg _ ->
            "ChangePasswordDialogMsg"

        ChangeEmailDialogMsg _ ->
            "ChangeEmailDialogMsg"

        ChangeRemoteUrlDialogMsg _ ->
            "ChangeRemoteUrlDialogMsg"

        ResetPasswordMsg _ ->
            "ResetPasswordMsg"

        Zone _ ->
            "Zone"

        UserListingMsg _ ->
            "UserListingMsg"

        UserEditMsg _ ->
            "UserEditMsg"

        ShowUrlMsg _ ->
            "ShowUrlMsg"

        OnFileSelected _ _ ->
            "OnFileSelected"

        FileUploadedButGetTime _ _ ->
            "FileUploadedButGetTime"

        FileUploaded _ _ ->
            "FileUploaded"

        JobsDialogMsg _ ->
            "JobsDialogMsg"

        MdInlineXformMsg _ ->
            "MdInlineXformMsg"

        MdInlineXformCmd _ ->
            "MdInlineXformCmd"

        RequestsDialogMsg _ ->
            "RequestsDialogMsg"

        RequestProgress _ _ ->
            "RequestProgress"

        TagFilesMsg _ ->
            "TagFilesMsg"

        TagNotesMsg _ ->
            "TagNotesMsg"

        TagNotes2Msg _ ->
            "TagNotes2Msg"

        InviteUserMsg _ ->
            "InviteUserMsg"

        JobsPollTick _ ->
            "JobsPollTick"


showState : State -> String
showState state =
    case state of
        Login _ _ ->
            "Login"

        Invited _ ->
            "Invited"

        EditZkNote _ _ ->
            "EditZkNote"

        EditZkNoteListing _ _ ->
            "EditZkNoteListing"

        ArchiveListing _ _ ->
            "ArchiveListing"

        ArchiveAwait _ _ _ ->
            "ArchiveAwait"

        View _ ->
            "View"

        EView _ _ ->
            "EView"

        UserSettings _ _ _ ->
            "UserSettings"

        Import _ _ ->
            "Import"

        DisplayMessage _ _ ->
            "DisplayMessage"

        MessageNLink _ _ ->
            "MessageNLink"

        ShowMessage _ _ _ ->
            "ShowMessage"

        PubShowMessage _ _ ->
            "PubShowMessage"

        LoginShowMessage _ _ _ ->
            "LoginShowMessage"

        Wait _ _ ->
            "Wait"

        SelectDialog _ _ ->
            "SelectDialog"

        ChangePasswordDialog _ _ ->
            "ChangePasswordDialog"

        ChangeEmailDialog _ _ ->
            "ChangeEmailDialog"

        ChangeRemoteUrlDialog _ _ ->
            "ChangeRemoteUrlDialog"

        ResetPassword _ ->
            "ResetPassword"

        UserListing _ _ ->
            "UserListing"

        UserEdit _ _ ->
            "UserEdit"

        ShowUrl _ _ ->
            "ShowUrl"

        JobsDialog _ _ ->
            "JobsDialog"

        MdInlineXform _ _ ->
            "MdInlineXform"

        RequestsDialog _ _ ->
            "RequestsDialog"

        TagFiles _ _ _ ->
            "TagFiles"

        TagNotes _ _ _ ->
            "TagNotes"

        TagNotes2 _ _ _ ->
            "TagNotes2"

        InviteUser _ _ _ ->
            "InviteUser"


unexpectedMsg : Model -> Msg -> Model
unexpectedMsg model msg =
    unexpectedMessage model (showMessage msg)


unexpectedMessage : Model -> String -> Model
unexpectedMessage model msg =
    displayMessageDialog model
        ("unexpected message - " ++ msg ++ "; state was " ++ showState model.state)


viewState : Util.Size -> State -> Model -> Element Msg
viewState size state model =
    case state of
        Login lem _ ->
            E.map LoginMsg <| Login.view model.stylePalette size lem

        Invited em ->
            E.map InvitedMsg <| Invited.view model.stylePalette size em

        EditZkNote em _ ->
            E.map EditZkNoteMsg <| EditZkNote.view model.stylePalette model.timezone size model.spmodel model.zknSearchResult model.recentNotes model.trackedRequests model.jobs model.noteCache em

        EditZkNoteListing em ld ->
            E.map EditZkNoteListingMsg <| EditZkNoteListing.view model.stylePalette.fontSize ld size em model.spmodel model.zknSearchResult

        ArchiveListing em ld ->
            E.map ArchiveListingMsg <| ArchiveListing.view ld model.timezone size em

        ArchiveAwait _ _ _ ->
            E.text "loading archive..."

        ShowMessage em _ _ ->
            E.map ShowMessageMsg <| ShowMessage.view em

        PubShowMessage em _ ->
            E.map ShowMessageMsg <| ShowMessage.view em

        LoginShowMessage em _ _ ->
            E.map ShowMessageMsg <| ShowMessage.view em

        Import em _ ->
            E.map ImportMsg <| Import.view size em model.spmodel model.zknSearchResult

        View em ->
            E.map ViewMsg <| View.view model.timezone size.width model.noteCache em False

        EView em _ ->
            E.map ViewMsg <| View.view model.timezone size.width model.noteCache em True

        UserSettings em _ _ ->
            E.map UserSettingsMsg <| UserSettings.view em

        DisplayMessage _ _ ->
            -- render is at the layout level, not here.
            E.none

        MessageNLink _ _ ->
            -- render is at the layout level, not here.
            E.none

        Wait innerState _ ->
            E.map (\_ -> Noop) (viewState size innerState model)

        SelectDialog _ _ ->
            -- render is at the layout level, not here.
            E.none

        ChangePasswordDialog _ _ ->
            -- render is at the layout level, not here.
            E.none

        ChangeEmailDialog _ _ ->
            -- render is at the layout level, not here.
            E.none

        ChangeRemoteUrlDialog _ _ ->
            -- render is at the layout level, not here.
            E.none

        ResetPassword st ->
            E.map ResetPasswordMsg (ResetPassword.view size st)

        UserListing st _ ->
            E.map UserListingMsg (UserListing.view Common.buttonStyle st)

        UserEdit st _ ->
            E.map UserEditMsg (UserEdit.view Common.buttonStyle st)

        ShowUrl st _ ->
            E.map
                ShowUrlMsg
                (ShowUrl.view Common.buttonStyle st)

        JobsDialog _ _ ->
            -- render is at the layout level, not here.
            E.none

        MdInlineXform _ _ ->
            E.none

        RequestsDialog _ _ ->
            -- render is at the layout level, not here.
            E.none

        TagFiles tfmod _ _ ->
            E.map TagFilesMsg <| TagAThing.view model.stylePalette model.recentNotes (Just size) model.spmodel model.zknSearchResult tfmod

        TagNotes tfmod _ _ ->
            E.map TagNotesMsg <| TagAThing.view model.stylePalette model.recentNotes (Just size) model.spmodel model.zknSearchResult tfmod

        TagNotes2 tfmod _ _ ->
            E.map TagNotes2Msg <| TagNotes2.view model.stylePalette (Just size) model.recentNotes model.spmodel model.zknSearchResult tfmod

        InviteUser tfmod _ _ ->
            E.map InviteUserMsg <| TagAThing.view model.stylePalette model.recentNotes (Just size) model.spmodel model.zknSearchResult tfmod


stateLogin : State -> Maybe LoginData
stateLogin state =
    case state of
        Login _ _ ->
            Nothing

        Invited _ ->
            Nothing

        EditZkNote _ login ->
            Just login

        EditZkNoteListing _ login ->
            Just login

        ArchiveListing _ login ->
            Just login

        ArchiveAwait _ _ login ->
            Just login

        Import _ login ->
            Just login

        View _ ->
            Nothing

        EView _ evstate ->
            stateLogin evstate

        UserSettings _ login _ ->
            Just login

        DisplayMessage _ bestate ->
            stateLogin bestate

        MessageNLink _ bestate ->
            stateLogin bestate

        ShowMessage _ login _ ->
            Just login

        PubShowMessage _ _ ->
            Nothing

        LoginShowMessage _ _ _ ->
            Nothing

        Wait wstate _ ->
            stateLogin wstate

        SelectDialog _ instate ->
            stateLogin instate

        ChangePasswordDialog _ instate ->
            stateLogin instate

        ChangeEmailDialog _ instate ->
            stateLogin instate

        ChangeRemoteUrlDialog _ instate ->
            stateLogin instate

        ResetPassword _ ->
            Nothing

        UserListing _ login ->
            Just login

        UserEdit _ login ->
            Just login

        ShowUrl _ login ->
            Just login

        JobsDialog _ instate ->
            stateLogin instate

        MdInlineXform _ instate ->
            stateLogin instate

        RequestsDialog _ instate ->
            stateLogin instate

        TagFiles _ login _ ->
            Just login

        TagNotes _ login _ ->
            Just login

        TagNotes2 _ login _ ->
            Just login

        InviteUser _ login _ ->
            Just login


sendUIMsg : FileUrlInfo -> OD.UserRequest -> Cmd Msg
sendUIMsg fui msg =
    if fui.tauri then
        sendUIValueTauri (OD.userRequestEncoder msg)

    else
        Http.post
            { url = fui.location ++ "/user"
            , body = Http.jsonBody (OD.userRequestEncoder msg)
            , expect = Http.expectJson UserReplyData OD.userResponseDecoder
            }


sendAIMsg : String -> OD.AdminRequest -> Cmd Msg
sendAIMsg location msg =
    sendAIMsgExp location msg AdminReplyData


sendAIMsgExp : String -> OD.AdminRequest -> (Result Http.Error OD.AdminResponse -> Msg) -> Cmd Msg
sendAIMsgExp location msg tomsg =
    Http.post
        { url = location ++ "/admin"
        , body = Http.jsonBody (OD.adminRequestEncoder msg)
        , expect = Http.expectJson tomsg OD.adminResponseDecoder
        }


sendZIMsg : FileUrlInfo -> Data.PrivateRequest -> Cmd Msg
sendZIMsg fui msg =
    if fui.tauri then
        sendZIMsgTauri <| PrivateClosureRequest Nothing msg

    else
        HE.postJsonTask
            { url = fui.location ++ "/private"
            , body = Http.jsonBody (Data.privateRequestEncoder msg)
            , decoder = Data.privateReplyDecoder
            }
            |> Task.andThen (\x -> Task.map (\posix -> ( posix, x )) Time.now)
            |> Task.attempt ZkReplyData


sendZIMsgTauri : Data.PrivateClosureRequest -> Cmd Msg
sendZIMsgTauri msg =
    sendZIValueTauri <| Data.privateClosureRequestEncoder msg


sendZIMsgExp : Model -> FileUrlInfo -> Data.PrivateRequest -> (Result Http.Error ( Time.Posix, Data.PrivateReply ) -> Msg) -> ( Model, Cmd Msg )
sendZIMsgExp model fui msg tomsg =
    if fui.tauri then
        ( { model
            | ziClosureId = model.ziClosureId + 1
            , ziClosures = Dict.insert model.ziClosureId tomsg model.ziClosures
          }
        , sendZIValueTauri <|
            Data.privateClosureRequestEncoder
                { closureId = Just model.ziClosureId
                , request = msg
                }
        )

    else
        ( model
        , HE.postJsonTask
            { url = fui.location ++ "/private"
            , body = Http.jsonBody (Data.privateRequestEncoder msg)
            , decoder = Data.privateReplyDecoder
            }
            |> Task.andThen (\x -> Task.map (\posix -> ( posix, x )) Time.now)
            |> Task.attempt tomsg
        )


{-| send search AND save search in db as a zknote
-}
sendSearch : Model -> Data.ZkNoteSearch -> ( Model, Cmd Msg )
sendSearch model search =
    case stateLogin model.state of
        Just ldata ->
            let
                searchnote =
                    { note =
                        { id = Nothing
                        , pubid = Nothing
                        , title = SU.printTagSearch (SU.andifySearches search.tagsearch)
                        , content = SN.SnSearch search.tagsearch |> SN.specialNoteEncoder |> JE.encode 2
                        , editable = False
                        , showtitle = True
                        , deleted = False
                        , what = Nothing
                        }
                    , links =
                        [ { otherid = DataUtil.sysids.searchid
                          , direction = Data.To
                          , user = ldata.userid
                          , zknote = Nothing
                          , delete = Nothing
                          }
                        ]
                    , lzlinks = []
                    }

                datesearch =
                    List.map (SU.tagSearchDates model.timezone) search.tagsearch
                        |> Util.rslist
                        |> Result.map
                            (\tsl ->
                                { search | tagsearch = tsl }
                            )
            in
            case datesearch of
                Err (SU.InvalidDateFormat d) ->
                    ( displayMessageDialog model
                        ("invalid date in search: "
                            ++ d
                            ++ "\nvalid format examples:"
                            ++ "\n ac'1756706400000' - milliseconds from jan 1 1970"
                            ++ "\n bm'2025/09/01' - created before date"
                            ++ "\n bm'2025/09/01 12:30:01' - created before datetime"
                        )
                    , Cmd.none
                    )

                Err (SU.InvalidDateMods d) ->
                    ( displayMessageDialog model
                        ("invalid date search in term: "
                            ++ d
                            ++ "\ndate searches must include: : "
                            ++ "\na or b : before or after"
                            ++ "\nc or m : create or modification date"
                            ++ "\nfor example: ma'2025/01/01': Modified After indicated date."
                        )
                    , Cmd.none
                    )

                Err (SU.InvalidServerMods d) ->
                    ( displayMessageDialog model d, Cmd.none )

                Err (SU.InvalidUuid d) ->
                    ( displayMessageDialog model d, Cmd.none )

                Ok dsearch ->
                    -- if this is the same search as last time, don't save.
                    if
                        (List.head model.prevSearches == Just dsearch.tagsearch)
                            || (dsearch.tagsearch == [ Data.SearchTerm { mods = [], term = "" } ])
                    then
                        ( model
                        , sendZIMsg model.fui (Data.PvqSearchZkNotes dsearch)
                        )

                    else
                        let
                            ( nm, cmd ) =
                                sendZIMsgExp model
                                    model.fui
                                    (Data.PvqSaveZkNoteAndLinks searchnote)
                                    -- ignore the reply!  otherwise if you search while
                                    -- creating a new note, that new note gets the search note
                                    -- id.
                                    (\_ -> Noop)
                        in
                        ( { nm | prevSearches = dsearch.tagsearch :: model.prevSearches }
                        , Cmd.batch
                            [ sendZIMsg model.fui (Data.PvqSearchZkNotes dsearch)
                            , cmd
                            ]
                        )

        Nothing ->
            ( model
            , Cmd.none
            )


sendPIMsg : FileUrlInfo -> Data.PublicRequest -> Cmd Msg
sendPIMsg fui msg =
    sendPIMsgExp fui msg PublicReplyData


sendPIMsgExp : FileUrlInfo -> Data.PublicRequest -> (Result Http.Error ( Time.Posix, Data.PublicReply ) -> Msg) -> Cmd Msg
sendPIMsgExp fui msg tomsg =
    if fui.tauri then
        sendPIValueTauri <| Data.publicRequestEncoder msg

    else
        HE.postJsonTask
            { url = fui.location ++ "/public"
            , body = Http.jsonBody (Data.publicRequestEncoder msg)
            , decoder = Data.publicReplyDecoder
            }
            |> Task.andThen (\x -> Task.map (\posix -> ( posix, x )) Time.now)
            |> Task.attempt tomsg


addRecentZkListNote : List Data.ZkListNote -> Data.ZkListNote -> List Data.ZkListNote
addRecentZkListNote recent zkln =
    List.take 50 <|
        zkln
            :: List.filter (\x -> x.id /= zkln.id) recent


piview : PiModel -> { title : String, body : List (Html Msg) }
piview pimodel =
    case pimodel of
        Ready model ->
            view model

        PreInit _ ->
            { title = "zknotes: initializing"
            , body = []
            }

        InitError e ->
            { title = "zknotes: init error!"
            , body = [ E.layout [ E.width E.fill ] (E.column [] [ E.text "zknotes init error! ", E.text e ]) ]
            }


view : Model -> { title : String, body : List (Html Msg) }
view model =
    let
        dndif =
            (case model.state of
                EditZkNote ezn _ ->
                    Maybe.map (E.inFront << E.map EditZkNoteMsg) <|
                        EditZkNote.ghostView ezn model.timezone model.noteCache MC.EditView 500

                _ ->
                    Nothing
            )
                |> Maybe.map List.singleton
                |> Maybe.withDefault []
    in
    { title =
        case model.state of
            EditZkNote ezn _ ->
                ezn.title ++ " - zknote"

            _ ->
                routeTitle model.savedRoute.route
    , body =
        [ case model.state of
            DisplayMessage dm _ ->
                Html.map DisplayMessageMsg <|
                    GD.layout
                        (Just { width = min 600 model.size.width, height = min 500 model.size.height })
                        dm

            MessageNLink dm _ ->
                Html.map MessageNLinkMsg <|
                    GD.layout
                        (Just { width = min 600 model.size.width, height = min 500 model.size.height })
                        dm

            SelectDialog sdm _ ->
                if model.mobile then
                    E.layout [] <|
                        E.map SelectDialogMsg <|
                            GD.dialogView Nothing sdm

                else
                    Html.map SelectDialogMsg <|
                        GD.layout
                            (Just { width = min 600 model.size.width, height = min 500 model.size.height })
                            sdm

            ChangePasswordDialog cdm _ ->
                Html.map ChangePasswordDialogMsg <|
                    GD.layout
                        (Just { width = min 600 model.size.width, height = min 200 model.size.height })
                        cdm

            ChangeEmailDialog cdm _ ->
                Html.map ChangeEmailDialogMsg <|
                    GD.layout
                        (Just { width = min 600 model.size.width, height = min 200 model.size.height })
                        cdm

            ChangeRemoteUrlDialog cdm _ ->
                Html.map ChangeRemoteUrlDialogMsg <|
                    GD.layout
                        (Just { width = min 600 model.size.width, height = min 200 model.size.height })
                        cdm

            JobsDialog dm _ ->
                if model.mobile then
                    E.layout [] <|
                        E.map JobsDialogMsg <|
                            GD.dialogView Nothing { dm | model = model.jobs }

                else
                    Html.map JobsDialogMsg <|
                        GD.layout
                            (Just { width = min 600 model.size.width, height = min 500 model.size.height })
                            { dm | model = model.jobs }

            MdInlineXform gdm _ ->
                if model.mobile then
                    E.layout [] <|
                        E.map MdInlineXformMsg <|
                            GD.dialogView Nothing gdm

                else
                    Html.map MdInlineXformMsg <|
                        GD.layout
                            (Just { width = min 600 model.size.width, height = min 500 model.size.height })
                            gdm

            RequestsDialog dm _ ->
                if model.mobile then
                    E.layout [] <|
                        E.map RequestsDialogMsg <|
                            GD.dialogView Nothing { dm | model = model.trackedRequests }

                else
                    Html.map RequestsDialogMsg <|
                        GD.layout
                            (Just { width = min 600 model.size.width, height = min 500 model.size.height })
                            -- use the live-updated model
                            { dm | model = model.trackedRequests }

            _ ->
                E.layout
                    ([ EF.size model.stylePalette.fontSize, E.width E.fill ]
                        ++ dndif
                    )
                <|
                    viewState model.size model.state model
        ]
    }


piupdate : Msg -> PiModel -> ( PiModel, Cmd Msg )
piupdate msg initmodel =
    case initmodel of
        Ready model ->
            let
                ( m, c ) =
                    urlupdate msg model
            in
            ( Ready m, c )

        PreInit imod ->
            let
                nmod =
                    case msg of
                        Zone zone ->
                            { imod | mbzone = Just zone }

                        ReceiveLocalVal lv ->
                            let
                                default =
                                    16
                            in
                            case lv.name of
                                "fontsize" ->
                                    case lv.value of
                                        Just v ->
                                            case String.toInt v of
                                                Just i ->
                                                    { imod | mbfontsize = Just i }

                                                Nothing ->
                                                    { imod | mbfontsize = Just default }

                                        Nothing ->
                                            { imod | mbfontsize = Just default }

                                _ ->
                                    { imod | mbfontsize = Nothing }

                        _ ->
                            imod
            in
            case ( nmod.mbzone, nmod.mbfontsize ) of
                ( Just zone, Just fontsize ) ->
                    let
                        ( m, c ) =
                            init imod.flags imod.url imod.key zone fontsize
                    in
                    ( Ready m, c )

                _ ->
                    ( PreInit nmod, Cmd.none )

        InitError e ->
            ( InitError e, Cmd.none )


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
                                |> Maybe.map (routeState model)
                                |> Maybe.withDefault ( model.state, Cmd.none )

                        bcmd =
                            case model.state of
                                EditZkNote s _ ->
                                    if EditZkNote.dirty s then
                                        Cmd.batch
                                            [ icmd
                                            , sendZIMsg model.fui
                                                (Data.PvqSaveZkNoteAndLinks <| EditZkNote.fullSave s)
                                            ]

                                    else
                                        icmd

                                _ ->
                                    icmd
                    in
                    ( { model | state = state }, bcmd )

                LoadUrl _ ->
                    -- load foreign site
                    -- ( model, Browser.Navigation.load urlstr )
                    ( model, Cmd.none )

                UrlChanged url ->
                    -- we get this from forward and back buttons.  if the user changes the url
                    -- in the browser address bar, its a site reload so this isn't called.
                    case parseUrl url of
                        Just route ->
                            let
                                reroot =
                                    let
                                        ( st, rscmd ) =
                                            routeState model route
                                    in
                                    -- swap out the savedRoute, so we don't write over history.
                                    ( { model
                                        | state = st
                                        , savedRoute =
                                            let
                                                nssr =
                                                    stateRoute st
                                            in
                                            { nssr | save = False }
                                      }
                                    , rscmd
                                    )
                            in
                            if route == (stateRoute model.state).route then
                                ( model, Cmd.none )

                            else
                                case ( route, model.state ) of
                                    ( ArchiveNoteR pid nid, ArchiveListing almod _ ) ->
                                        if almod.noteid == pid then
                                            ( model, Cmd.none )

                                        else
                                            reroot

                                    _ ->
                                        reroot

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


shDialog : Model -> Model
shDialog model =
    { model
        | state =
            SelectDialog
                (SS.init
                    { choices =
                        List.indexedMap
                            (\i ps -> ( i, SU.printTagSearch (SU.andifySearches ps) ))
                            model.prevSearches
                    , selected = Nothing
                    , search = ""
                    , mobile = model.mobile
                    }
                    Common.buttonStyle
                    (E.map (\_ -> ()) (viewState model.size model.state model))
                )
                model.state
    }


displayMessageDialog : Model -> String -> Model
displayMessageDialog model message =
    { model
        | state =
            DisplayMessage
                (DisplayMessage.init Common.buttonStyle
                    message
                    (E.map (\_ -> ()) (viewState model.size model.state model))
                )
                model.state
    }


displayMessageNLinkDialog : Model -> String -> String -> String -> Model
displayMessageNLinkDialog model message url text =
    { model
        | state =
            MessageNLink
                (MessageNLink.init Common.buttonStyle
                    message
                    url
                    text
                    (E.map (\_ -> ()) (viewState model.size model.state model))
                )
                model.state
    }


onZkNoteEditWhat : Model -> Time.Posix -> Data.ZkNoteAndLinksWhat -> ( Model, Cmd Msg )
onZkNoteEditWhat model pt znew =
    let
        state =
            model.state
    in
    if znew.what == "cache" then
        ( { model
            | noteCache =
                NC.addNote pt znew.znl.zknote.id (NC.ZNAL znew.znl) model.noteCache
                    |> NC.purgeNotes
          }
        , Cmd.none
        )

    else
        case stateLogin state of
            Just login ->
                let
                    ( nst, c ) =
                        EditZkNote.initFull model.fui
                            login
                            znew.znl.zknote
                            znew.znl.links
                            znew.znl.lzlinks
                            znew.edittab
                            model.mobile

                    ngets =
                        makeNoteCacheGets (EM.getMd nst.edMarkdown) model
                in
                ( { model
                    | state =
                        EditZkNote
                            nst
                            login
                    , recentNotes =
                        let
                            zknote =
                                znew.znl.zknote
                        in
                        addRecentZkListNote model.recentNotes
                            { id = zknote.id
                            , user = zknote.user
                            , title = zknote.title
                            , filestatus = zknote.filestatus
                            , createdate = zknote.createdate
                            , changeddate = zknote.changeddate
                            , sysids = zknote.sysids
                            }
                    , noteCache = NC.setKeeps (MC.noteIds (EM.getMd nst.edMarkdown)) model.noteCache
                  }
                , Cmd.batch ((sendZIMsg model.fui <| Data.PvqGetZkNoteComments c) :: ngets)
                )

            _ ->
                ( unexpectedMessage model "ZkNoteEditWhat"
                , Cmd.none
                )


type alias TauriData a =
    { utc : Time.Posix
    , data : a
    }


makeTDDecoder : JD.Decoder a -> JD.Decoder (TauriData a)
makeTDDecoder ad =
    JD.map2 TauriData
        (JD.field "utcmillis" (JD.map Time.millisToPosix JD.int))
        (JD.field "data" ad)


handleMdInlineXformOk : Model -> State -> GD.Transition MdInlineXform.GDModel MdInlineXform.Command -> ( Model, Cmd Msg )
handleMdInlineXformOk model prevstate gdmsg =
    case gdmsg of
        GD.Dialog nmod ->
            ( { model
                | state = MdInlineXform nmod prevstate
              }
            , Cmd.none
            )

        GD.Ok return ->
            case return of
                MdInlineXform.Close ->
                    ( { model | state = prevstate }, Cmd.none )

                MdInlineXform.UpdateInline umsg ->
                    case prevstate of
                        EditZkNote em login ->
                            let
                                emod =
                                    EditZkNote.updateEditBlock umsg em
                            in
                            ( { model | state = EditZkNote emod login }, Cmd.none )

                        _ ->
                            ( { model | state = prevstate }, Cmd.none )

                MdInlineXform.LinkBack title mkmsg ->
                    case prevstate of
                        EditZkNote ezst _ ->
                            case EditZkNote.initLinkBackNote ezst title of
                                Ok nst ->
                                    sendZIMsgExp model
                                        model.fui
                                        (Data.PvqSaveZkNoteAndLinks (EditZkNote.fullSave nst))
                                        (\ziresponse ->
                                            case ziresponse of
                                                Ok ( _, Data.PvyServerError _ ) ->
                                                    MdInlineXformCmd MdInlineXform.Close

                                                Ok ( _, Data.PvySavedZkNoteAndLinks szkn ) ->
                                                    MdInlineXformCmd (mkmsg szkn)

                                                _ ->
                                                    MdInlineXformCmd MdInlineXform.Close
                                        )

                                Err e ->
                                    ( displayMessageDialog model e, Cmd.none )

                        _ ->
                            ( model, Cmd.none )

        GD.Cancel ->
            ( { model | state = prevstate }, Cmd.none )


actualupdate : Msg -> Model -> ( Model, Cmd Msg )
actualupdate msg model =
    case ( msg, model.state ) of
        ( _, Wait _ wfn ) ->
            let
                ( nmd, cmd ) =
                    wfn model msg
            in
            ( nmd, cmd )

        ( ReceiveLocalVal _, _ ) ->
            -- update the font size
            ( model, Cmd.none )

        ( TauriZkReplyData jd, _ ) ->
            case JD.decodeValue (makeTDDecoder Data.privateClosureReplyDecoder) jd of
                Ok td ->
                    case td.data.closureId of
                        Just id ->
                            case Dict.get id model.ziClosures of
                                Just closure ->
                                    let
                                        cmsg =
                                            closure (Ok ( td.utc, td.data.reply ))
                                    in
                                    actualupdate cmsg model

                                Nothing ->
                                    actualupdate (ZkReplyData (Ok ( td.utc, td.data.reply ))) model

                        Nothing ->
                            actualupdate (ZkReplyData (Ok ( td.utc, td.data.reply ))) model

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e ++ "\n" ++ JE.encode 2 jd
                    , Cmd.none
                    )

        ( TauriUserReplyData jd, _ ) ->
            case JD.decodeValue OD.userResponseDecoder jd of
                Ok d ->
                    actualupdate (UserReplyData (Ok d)) model

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e
                    , Cmd.none
                    )

        ( TauriAdminReplyData jd, _ ) ->
            case JD.decodeValue OD.adminResponseDecoder jd of
                Ok d ->
                    actualupdate (AdminReplyData (Ok d)) model

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e
                    , Cmd.none
                    )

        ( TauriPublicReplyData jd, _ ) ->
            case JD.decodeValue (makeTDDecoder Data.publicReplyDecoder) jd of
                Ok td ->
                    actualupdate (PublicReplyData (Ok ( td.utc, td.data ))) model

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e
                    , Cmd.none
                    )

        ( TauriTauriReplyData jd, _ ) ->
            case JD.decodeValue Data.tauriReplyDecoder jd of
                Ok td ->
                    case td of
                        Data.TyUploadedFiles uf ->
                            let
                                fc =
                                    1 + List.length uf.notes

                                tr =
                                    model.trackedRequests

                                nrid =
                                    String.fromInt (model.trackedRequests.requestCount + 1)

                                nrq =
                                    uf.notes
                                        |> List.map (\n -> n.title)
                                        |> (\names ->
                                                FileUpload
                                                    { filenames = names
                                                    , progress = Just (Http.Sending { sent = fc, size = fc })
                                                    , files = Just uf.notes
                                                    }
                                           )

                                ntr =
                                    { tr
                                        | requestCount = tr.requestCount + fc
                                        , requests =
                                            Dict.insert nrid
                                                nrq
                                                tr.requests
                                    }
                            in
                            ( { model
                                | trackedRequests = ntr
                                , state =
                                    RequestsDialog
                                        (RequestsDialog.init
                                            { requestCount = 1, mobile = model.mobile, requests = Dict.fromList [ ( nrid, nrq ) ] }
                                            Common.buttonStyle
                                            (E.map (\_ -> ()) (viewState model.size model.state model))
                                        )
                                        model.state
                              }
                            , Cmd.none
                            )

                        Data.TyServerError e ->
                            ( displayMessageDialog model <| e, Cmd.none )

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e
                    , Cmd.none
                    )

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
                                sendsearch =
                                    sendZIMsg model.fui
                                        (Data.PvqSearchZkNotes
                                            { tagsearch = ts
                                            , offset = 0
                                            , limit = Nothing
                                            , what = ""
                                            , resulttype = Data.RtListNote
                                            , archives = Data.Current
                                            , deleted = False
                                            , ordering = Nothing
                                            }
                                        )

                                nmod =
                                    updateSearch ts model
                            in
                            ( { nmod | state = instate }, sendsearch )

                        Nothing ->
                            ( { model | state = instate }, Cmd.none )

                GD.Cancel ->
                    ( { model | state = instate }, Cmd.none )

        ( ChangePasswordDialogMsg sdmsg, ChangePasswordDialog sdmod instate ) ->
            case GD.update sdmsg sdmod of
                GD.Dialog nmod ->
                    ( { model | state = ChangePasswordDialog nmod instate }, Cmd.none )

                GD.Ok return ->
                    ( { model | state = instate }
                    , sendUIMsg model.fui <| OD.UrqAuthedRequest <| OD.AthChangePassword return
                    )

                GD.Cancel ->
                    ( { model | state = instate }, Cmd.none )

        ( ChangeEmailDialogMsg sdmsg, ChangeEmailDialog sdmod instate ) ->
            case GD.update sdmsg sdmod of
                GD.Dialog nmod ->
                    ( { model | state = ChangeEmailDialog nmod instate }, Cmd.none )

                GD.Ok return ->
                    ( { model | state = instate }
                    , sendUIMsg model.fui <| OD.UrqAuthedRequest <| OD.AthChangeEmail return
                    )

                GD.Cancel ->
                    ( { model | state = instate }, Cmd.none )

        ( ChangeRemoteUrlDialogMsg sdmsg, ChangeRemoteUrlDialog sdmod instate ) ->
            case GD.update sdmsg sdmod of
                GD.Dialog nmod ->
                    ( { model | state = ChangeRemoteUrlDialog nmod instate }, Cmd.none )

                GD.Ok return ->
                    ( { model | state = instate }
                    , sendUIMsg model.fui <| OD.UrqAuthedRequest <| OD.AthChangeRemoteUrl return
                    )

                GD.Cancel ->
                    ( { model | state = instate }, Cmd.none )

        ( ResetPasswordMsg rmsg, ResetPassword rst ) ->
            let
                ( nst, cmd ) =
                    ResetPassword.update rmsg rst
            in
            case cmd of
                ResetPassword.Ok ->
                    ( { model | state = ResetPassword nst }
                    , sendUIMsg model.fui
                        (OD.UrqSetPassword { uid = nst.userId, newpwd = nst.password, resetKey = UUID.toString nst.reset_key })
                    )

                ResetPassword.None ->
                    ( { model | state = ResetPassword nst }, Cmd.none )

        ( TASelection jv, state ) ->
            case JD.decodeValue DataUtil.decodeTASelection jv of
                Ok tas ->
                    case state of
                        EditZkNote emod login ->
                            handleTASelection model emod login tas

                        _ ->
                            ( model, Cmd.none )

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e, Cmd.none )

        ( TAError jv, state ) ->
            case JD.decodeValue DataUtil.decodeTAError jv of
                Ok tae ->
                    case state of
                        EditZkNote emod login ->
                            handleTASelection model
                                emod
                                login
                                { text = ""
                                , offset = 0
                                , what = tae.what
                                }

                        _ ->
                            ( model, Cmd.none )

                Err e ->
                    ( displayMessageDialog model <| JD.errorToString e, Cmd.none )

        ( UserSettingsMsg umsg, UserSettings umod login prevstate ) ->
            let
                ( numod, c ) =
                    UserSettings.update umsg umod
            in
            case c of
                UserSettings.Done ->
                    ( { model | state = prevstate }, Cmd.none )

                UserSettings.LogOut ->
                    ( { model | state = initLoginState model Top }
                    , sendUIMsg model.fui OD.UrqLogout
                    )

                UserSettings.ChangePassword ->
                    ( { model
                        | state =
                            ChangePasswordDialog (CP.init (DataUtil.toOaLd login) Common.buttonStyle (UserSettings.view numod |> E.map (always ())))
                                (UserSettings numod login prevstate)
                      }
                    , Cmd.none
                    )

                UserSettings.ChangeEmail ->
                    ( { model
                        | state =
                            ChangeEmailDialog
                                (CE.init (DataUtil.toOaLd login) Common.buttonStyle (UserSettings.view numod |> E.map (always ())))
                                (UserSettings numod login prevstate)
                      }
                    , Cmd.none
                    )

                UserSettings.ChangeRemoteUrl ->
                    ( { model
                        | state =
                            ChangeRemoteUrlDialog
                                (CRU.init (DataUtil.toOaLd login) Common.buttonStyle (UserSettings.view numod |> E.map (always ())))
                                (UserSettings numod login prevstate)
                      }
                    , Cmd.none
                    )

                UserSettings.ChangeFontSize size ->
                    let
                        s =
                            model.stylePalette
                    in
                    ( { model
                        | state = UserSettings numod login prevstate
                        , stylePalette = { s | fontSize = size }
                      }
                    , LS.storeLocalVal { name = "fontsize", value = String.fromInt size }
                    )

                UserSettings.None ->
                    ( { model | state = UserSettings numod login prevstate }, Cmd.none )

        ( UserListingMsg umsg, UserListing umod login ) ->
            let
                ( numod, c ) =
                    UserListing.update umsg umod
            in
            case c of
                UserListing.Done ->
                    initToRoute model Top

                UserListing.InviteUser ->
                    ( { model
                        | state =
                            InviteUser
                                (TagAThing.init
                                    (InviteUser.initThing "")
                                    TagAThing.AddLinksOnly
                                    []
                                    login
                                )
                                login
                                (UserListing numod login)
                      }
                    , Cmd.none
                    )

                UserListing.EditUser ld ->
                    ( { model | state = UserEdit (UserEdit.init ld) login }, Cmd.none )

                UserListing.None ->
                    ( { model | state = UserListing numod login }, Cmd.none )

        ( UserEditMsg umsg, UserEdit umod login ) ->
            let
                ( numod, c ) =
                    UserEdit.update umsg umod
            in
            case c of
                UserEdit.Done ->
                    ( model
                    , sendAIMsg model.fui.location OD.ArqGetUsers
                    )

                UserEdit.Delete id ->
                    ( model
                    , sendAIMsg model.fui.location <| OD.ArqDeleteUser id
                    )

                UserEdit.Save ld ->
                    ( model
                    , sendAIMsg model.fui.location <| OD.ArqUpdateUser ld
                    )

                UserEdit.ResetPwd id ->
                    ( model
                    , sendAIMsg model.fui.location <| OD.ArqGetPwdReset id
                    )

                UserEdit.None ->
                    ( { model | state = UserEdit numod login }, Cmd.none )

        ( ShowUrlMsg umsg, ShowUrl umod login ) ->
            let
                ( numod, c ) =
                    ShowUrl.update umsg umod
            in
            case c of
                ShowUrl.Done ->
                    if login.admin then
                        ( model
                        , sendAIMsg model.fui.location OD.ArqGetUsers
                        )

                    else
                        initToRoute model Top

                ShowUrl.None ->
                    ( { model | state = ShowUrl numod login }, Cmd.none )

        ( WkMsg (Ok key), Login ls url ) ->
            handleLogin model url (Login.onWkKeyPress key ls)

        ( WkMsg (Ok key), Invited ls ) ->
            handleInvited model (Invited.onWkKeyPress key ls)

        ( WkMsg (Ok key), TagFiles mod ld ps ) ->
            handleTagFiles model (TagAThing.onWkKeyPress key mod) ld ps

        ( WkMsg (Ok key), TagNotes mod ld ps ) ->
            handleTagNotes model (TagAThing.onWkKeyPress key mod) ld ps

        ( WkMsg (Ok key), InviteUser mod ld ps ) ->
            handleInviteUser model (TagAThing.onWkKeyPress key mod) ld ps

        ( WkMsg (Ok key), DisplayMessage _ state ) ->
            case Toop.T4 key.key key.ctrl key.alt key.shift of
                Toop.T4 "Enter" False False False ->
                    ( { model | state = state }, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        ( WkMsg (Ok key), EditZkNote es login ) ->
            handleEditZkNoteCmd model login (EditZkNote.onWkKeyPress key es)

        ( WkMsg (Ok key), EditZkNoteListing es login ) ->
            handleEditZkNoteListing model
                login
                (EditZkNoteListing.onWkKeyPress key es)

        ( WkMsg (Err e), _ ) ->
            ( displayMessageDialog model <| "error decoding windowkeys message: " ++ JD.errorToString e
            , Cmd.none
            )

        ( TagFilesMsg lm, TagFiles mod ld st ) ->
            handleTagFiles model (TagAThing.update lm mod) ld st

        ( TagNotesMsg lm, TagNotes mod ld st ) ->
            handleTagNotes model (TagAThing.update lm mod) ld st

        ( TagNotes2Msg lm, TagNotes2 mod ld st ) ->
            handleTagNotes2 model (TagNotes2.update lm mod) ld st

        ( InviteUserMsg lm, InviteUser mod ld st ) ->
            handleInviteUser model (TagAThing.update lm mod) ld st

        ( LoginMsg lm, Login ls route ) ->
            handleLogin model route (Login.update lm ls)

        ( InvitedMsg lm, Invited ls ) ->
            handleInvited model (Invited.update lm ls)

        ( ArchiveListingMsg lm, ArchiveListing mod ld ) ->
            handleArchiveListing model ld (ArchiveListing.update lm mod ld)

        ( ZkReplyData (Ok ( _, Data.PvyZkNoteArchives lm )), ArchiveAwait id aid ld ) ->
            ( { model | state = ArchiveListing (ArchiveListing.init lm) ld }
            , sendZIMsg model.fui
                (Data.PvqGetZkNote (Data.ArchiveZni (DataUtil.zkNoteIdToString aid) (DataUtil.zkNoteIdToString id)))
            )

        ( PublicReplyData prd, state ) ->
            case prd of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e
                    , Cmd.none
                    )

                Ok ( pt, piresponse ) ->
                    case piresponse of
                        Data.PbyServerError e ->
                            case e of
                                Data.PbeNoteNotFound publicrequest ->
                                    case DataUtil.getPrqNoteInfo publicrequest of
                                        Just ( zknoteid, "cache" ) ->
                                            ( { model
                                                | noteCache =
                                                    NC.addNote pt zknoteid NC.NotFound model.noteCache
                                                        |> NC.purgeNotes
                                              }
                                            , Cmd.none
                                            )

                                        _ ->
                                            let
                                                prevstate =
                                                    case stateLogin state of
                                                        Just _ ->
                                                            state

                                                        Nothing ->
                                                            initLoginState model model.initialRoute
                                            in
                                            ( displayMessageDialog { model | state = prevstate } "note not found", Cmd.none )

                                Data.PbeNoteIsPrivate publicrequest ->
                                    case DataUtil.getPrqNoteInfo publicrequest of
                                        Just ( zknoteid, "cache" ) ->
                                            ( { model
                                                | noteCache =
                                                    NC.addNote pt zknoteid NC.Private model.noteCache
                                                        |> NC.purgeNotes
                                              }
                                            , Cmd.none
                                            )

                                        _ ->
                                            let
                                                prevstate =
                                                    case stateLogin state of
                                                        Just _ ->
                                                            state

                                                        Nothing ->
                                                            initLoginState model model.initialRoute
                                            in
                                            ( displayMessageDialog { model | state = prevstate } "note is private", Cmd.none )

                                Data.PbeString estr ->
                                    let
                                        prevstate =
                                            case stateLogin state of
                                                Just _ ->
                                                    state

                                                Nothing ->
                                                    initLoginState model model.initialRoute
                                    in
                                    case Dict.get estr model.errorNotes of
                                        Just url ->
                                            ( displayMessageNLinkDialog { model | state = prevstate } estr url "more info"
                                            , Cmd.none
                                            )

                                        Nothing ->
                                            ( displayMessageDialog { model | state = prevstate } estr, Cmd.none )

                        Data.PbyZkNoteAndLinks znl ->
                            let
                                vstate =
                                    case stateLogin state of
                                        Just _ ->
                                            EView
                                                (View.initFull
                                                    model.fui
                                                    znl
                                                )
                                                state

                                        Nothing ->
                                            View (View.initFull model.fui znl)

                                ngets =
                                    makePubNoteCacheGets model znl.zknote.content
                            in
                            ( { model | state = vstate }
                            , Cmd.batch ngets
                            )

                        Data.PbyZkNoteAndLinksWhat znlw ->
                            if znlw.what == "cache" then
                                let
                                    gets =
                                        (case state of
                                            EView vs _ ->
                                                Just vs

                                            View vs ->
                                                Just vs

                                            _ ->
                                                Nothing
                                        )
                                            |> Maybe.andThen .panelNote
                                            |> Maybe.andThen
                                                (\pn ->
                                                    if pn == znlw.znl.zknote.id then
                                                        Just znlw.znl.zknote.content

                                                    else
                                                        Nothing
                                                )
                                            |> Maybe.map (makePubNoteCacheGets model >> Cmd.batch)
                                            |> Maybe.withDefault Cmd.none
                                in
                                ( { model
                                    | noteCache =
                                        NC.addNote pt znlw.znl.zknote.id (NC.ZNAL znlw.znl) model.noteCache
                                            |> NC.purgeNotes
                                  }
                                , gets
                                )

                            else
                                let
                                    vstate =
                                        case stateLogin state of
                                            Just _ ->
                                                EView
                                                    (View.initFull
                                                        model.fui
                                                        znlw.znl
                                                    )
                                                    state

                                            Nothing ->
                                                View (View.initFull model.fui znlw.znl)

                                    ngets =
                                        makePubNoteCacheGets model znlw.znl.zknote.content
                                in
                                ( { model | state = vstate }
                                , Cmd.batch ngets
                                )

                        Data.PbyNoop ->
                            ( model, Cmd.none )

        ( ErrorIndexNote rsein, _ ) ->
            case rsein of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e
                    , Cmd.none
                    )

                Ok resp ->
                    case resp of
                        Data.PbyServerError e ->
                            -- if there's an error on getting the error index note, just display it.
                            ( displayMessageDialog model <| DataUtil.showPublicError e, Cmd.none )

                        Data.PbyZkNoteAndLinks fbe ->
                            ( { model | errorNotes = MC.linkDict fbe.zknote.content }
                            , Cmd.none
                            )

                        Data.PbyZkNoteAndLinksWhat fbe ->
                            ( { model | errorNotes = MC.linkDict fbe.znl.zknote.content }
                            , Cmd.none
                            )

                        Data.PbyNoop ->
                            ( model, Cmd.none )

        ( TAReplyData tas urd, state ) ->
            case urd of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e, Cmd.none )

                Ok ( _, uiresponse ) ->
                    case uiresponse of
                        Data.PvyServerError e ->
                            ( displayMessageDialog model <| DataUtil.showPrivateError e, Cmd.none )

                        Data.PvySavedZkNoteAndLinks szkn ->
                            case state of
                                EditZkNote emod login ->
                                    let
                                        ( eznst, cmd ) =
                                            EditZkNote.onLinkBackSaved
                                                emod
                                                (Just tas)
                                                szkn
                                    in
                                    handleEditZkNoteCmd model login ( eznst, cmd )

                                _ ->
                                    -- just ignore if we're not editing a new note.
                                    ( model, Cmd.none )

                        _ ->
                            ( unexpectedMsg model msg, Cmd.none )

        ( UserReplyData urd, state ) ->
            case urd of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e, Cmd.none )

                Ok uiresponse ->
                    case uiresponse of
                        OD.UrpServerError e ->
                            ( displayMessageDialog model <| e, Cmd.none )

                        OD.UrpRegistrationSent ->
                            case model.state of
                                Login lgst url ->
                                    ( { model | state = Login (Login.registrationSent lgst) url }, Cmd.none )

                                _ ->
                                    ( model, Cmd.none )

                        OD.UrpLoggedIn oalogin ->
                            case DataUtil.fromOaLd oalogin of
                                Ok login ->
                                    let
                                        lgmod =
                                            { model
                                                | state =
                                                    ShowMessage { message = "logged in" }
                                                        login
                                                        Nothing
                                            }
                                    in
                                    case state of
                                        Login _ _ ->
                                            -- we're logged in!
                                            initToRoute lgmod lgmod.initialRoute

                                        LoginShowMessage _ _ url ->
                                            let
                                                ( m, cmd ) =
                                                    let
                                                        mbroute =
                                                            parseUrl url
                                                    in
                                                    mbroute
                                                        |> Maybe.andThen
                                                            (\s ->
                                                                case s of
                                                                    Top ->
                                                                        Nothing

                                                                    _ ->
                                                                        Just s
                                                            )
                                                        |> Maybe.map
                                                            (routeState
                                                                lgmod
                                                            )
                                                        |> Maybe.map (\( st, cm ) -> ( { model | state = st }, cm ))
                                                        |> Maybe.withDefault (initToRoute lgmod (mbroute |> Maybe.withDefault Top))
                                            in
                                            ( m, cmd )

                                        _ ->
                                            -- we're logged in!
                                            initToRoute lgmod Top

                                Err e ->
                                    ( displayMessageDialog model (JD.errorToString e)
                                    , Cmd.none
                                    )

                        OD.UrpLoggedOut ->
                            ( model, Cmd.none )

                        OD.UrpResetPasswordAck ->
                            let
                                nmod =
                                    { model
                                        | state = initLoginState model Top
                                    }
                            in
                            ( displayMessageDialog nmod "password reset attempted!  if you're a valid user, check your inbox for a reset email."
                            , Cmd.none
                            )

                        OD.UrpSetPasswordAck ->
                            let
                                nmod =
                                    { model
                                        | state = initLoginState model Top
                                    }
                            in
                            ( displayMessageDialog nmod "password reset complete!"
                            , Cmd.none
                            )

                        OD.UrpChangedPassword ->
                            ( displayMessageDialog model "password changed"
                            , Cmd.none
                            )

                        OD.UrpChangedEmail ->
                            ( displayMessageDialog model "email change confirmation sent!  check your inbox (or spam folder) for an email with title 'change zknotes email', and follow the enclosed link to change to the new address."
                            , Cmd.none
                            )

                        OD.UrpChangedRemoteUrl url ->
                            let
                                nmod =
                                    case model.state of
                                        UserSettings us lg ps ->
                                            let
                                                nlg =
                                                    { lg | remoteUrl = Just url }
                                            in
                                            { model | state = UserSettings { us | login = nlg } nlg ps }

                                        _ ->
                                            model
                            in
                            ( displayMessageDialog nmod "changed remote url"
                            , Cmd.none
                            )

                        OD.UrpUserExists ->
                            case state of
                                Login lmod route ->
                                    ( { model | state = Login (Login.userExists lmod) route }, Cmd.none )

                                _ ->
                                    ( unexpectedMessage model (ODU.showUserResponse uiresponse)
                                    , Cmd.none
                                    )

                        OD.UrpUnregisteredUser ->
                            case state of
                                Login lmod route ->
                                    ( { model | state = Login (Login.unregisteredUser lmod) route }, Cmd.none )

                                _ ->
                                    ( unexpectedMessage model (ODU.showUserResponse uiresponse)
                                    , Cmd.none
                                    )

                        OD.UrpNotLoggedIn ->
                            case state of
                                Login lmod route ->
                                    ( { model | state = Login lmod route }, Cmd.none )

                                _ ->
                                    ( { model | state = initLoginState model Top }, Cmd.none )

                        OD.UrpInvalidUserOrPwd ->
                            case state of
                                Login lmod route ->
                                    ( { model | state = Login (Login.invalidUserOrPwd lmod) route }, Cmd.none )

                                _ ->
                                    ( unexpectedMessage { model | state = initLoginState model Top }
                                        (ODU.showUserResponse uiresponse)
                                    , Cmd.none
                                    )

                        OD.UrpInvalidUserUuid ->
                            ( displayMessageDialog model "user UUID on remote server doesn't match local user UUID!"
                            , Cmd.none
                            )

                        OD.UrpInvalidUserId ->
                            ( displayMessageDialog model "invalid user id!"
                            , Cmd.none
                            )

                        OD.UrpAccountDeactivated ->
                            ( displayMessageDialog model "account deactivated!"
                            , Cmd.none
                            )

                        OD.UrpRemoteRegistrationFailed ->
                            ( displayMessageDialog model "remote registration failed!"
                            , Cmd.none
                            )

                        OD.UrpRemoteUser ru ->
                            ( displayMessageDialog model <| "remote user: " ++ ru.name
                            , Cmd.none
                            )

                        OD.UrpNoData ->
                            ( displayMessageDialog model <| "no data!"
                            , Cmd.none
                            )

                        OD.UrpBlankUserName ->
                            case state of
                                Invited lmod ->
                                    ( { model | state = Invited <| Invited.blankUserName lmod }, Cmd.none )

                                Login lmod route ->
                                    ( { model | state = Login (Login.blankUserName lmod) route }, Cmd.none )

                                _ ->
                                    ( unexpectedMessage { model | state = initLoginState model Top }
                                        (ODU.showUserResponse uiresponse)
                                    , Cmd.none
                                    )

                        OD.UrpBlankPassword ->
                            case state of
                                Invited lmod ->
                                    ( { model | state = Invited <| Invited.blankPassword lmod }, Cmd.none )

                                Login lmod route ->
                                    ( { model | state = Login (Login.blankPassword lmod) route }, Cmd.none )

                                _ ->
                                    ( unexpectedMessage { model | state = initLoginState model Top }
                                        (ODU.showUserResponse uiresponse)
                                    , Cmd.none
                                    )

                        OD.UrpInvite invite ->
                            ( { model | state = Invited (Invited.initialModel invite model.adminSettings "zknotes") }
                            , Cmd.none
                            )

        ( AdminReplyData ard, state ) ->
            case ard of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e, Cmd.none )

                Ok airesponse ->
                    case airesponse of
                        OD.ArpNotLoggedIn ->
                            case state of
                                Login lmod route ->
                                    ( { model | state = Login lmod route }, Cmd.none )

                                _ ->
                                    ( { model | state = initLoginState model Top }, Cmd.none )

                        OD.ArpUsers users ->
                            case stateLogin model.state of
                                Just login ->
                                    ( { model | state = UserListing (UserListing.init users) login }, Cmd.none )

                                Nothing ->
                                    ( displayMessageDialog model "not logged in", Cmd.none )

                        OD.ArpUserDeleted _ ->
                            ( displayMessageDialog model "user deleted!"
                            , sendAIMsg model.fui.location OD.ArqGetUsers
                            )

                        OD.ArpUserUpdated ld ->
                            case model.state of
                                UserEdit ue login ->
                                    ( displayMessageDialog { model | state = UserEdit (UserEdit.onUserUpdated ue ld) login } "user updated"
                                    , Cmd.none
                                    )

                                _ ->
                                    ( model, Cmd.none )

                        OD.ArpUserInvite ui ->
                            case stateLogin model.state of
                                Just login ->
                                    ( { model
                                        | state =
                                            ShowUrl
                                                (ShowUrl.init ui.url "Send this url to the invited user!" "invite url")
                                                login
                                      }
                                    , Cmd.none
                                    )

                                Nothing ->
                                    ( displayMessageDialog model "not logged in!"
                                    , Cmd.none
                                    )

                        OD.ArpPwdReset pr ->
                            case state of
                                UserEdit _ login ->
                                    ( { model
                                        | state =
                                            ShowUrl
                                                (ShowUrl.init pr.url "Send this url to the user for password reset!" "reset url")
                                                login
                                      }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( model, Cmd.none )

                        OD.ArpServerError e ->
                            ( displayMessageDialog model <| e, Cmd.none )

                        OD.ArpUserNotDeleted _ ->
                            ( displayMessageDialog model <| ODU.showAdminResponse airesponse, Cmd.none )

                        OD.ArpNoUserId ->
                            ( displayMessageDialog model <| ODU.showAdminResponse airesponse, Cmd.none )

                        OD.ArpNoData ->
                            ( displayMessageDialog model <| ODU.showAdminResponse airesponse, Cmd.none )

                        OD.ArpInvalidUserOrPassword ->
                            ( displayMessageDialog model <| ODU.showAdminResponse airesponse, Cmd.none )

                        OD.ArpAccessDenied ->
                            ( displayMessageDialog model <| ODU.showAdminResponse airesponse, Cmd.none )

        ( ZkReplyDataSeq f zrd, _ ) ->
            let
                ( nmod, ncmd ) =
                    actualupdate (ZkReplyData zrd) model
            in
            case f zrd of
                Just cmd ->
                    ( nmod, Cmd.batch [ ncmd, cmd ] )

                Nothing ->
                    ( nmod, ncmd )

        ( ZkReplyData zrd, state ) ->
            case zrd of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e, Cmd.none )

                Ok ( pt, ziresponse ) ->
                    case ziresponse of
                        Data.PvyServerError e ->
                            ( displayMessageDialog model <| DataUtil.showPrivateError e, Cmd.none )

                        Data.PvyPowerDeleteComplete count ->
                            case model.state of
                                EditZkNoteListing mod li ->
                                    ( { model | state = EditZkNoteListing (EditZkNoteListing.onPowerDeleteComplete model.stylePalette.fontSize count li mod model.spmodel model.zknSearchResult) li }, Cmd.none )

                                _ ->
                                    ( model, Cmd.none )

                        Data.PvyZkNoteSearchResult sr ->
                            if sr.what == "prevSearches" then
                                let
                                    -- TODO decode specialnote instead.
                                    pses =
                                        List.filterMap
                                            (\zknote ->
                                                JD.decodeString SN.specialNoteDecoder zknote.content
                                                    |> Result.toMaybe
                                                    |> Maybe.andThen
                                                        (\r ->
                                                            case r of
                                                                SN.SnSearch s ->
                                                                    Just s

                                                                _ ->
                                                                    Nothing
                                                        )
                                            )
                                            sr.notes

                                    -- TODO: fix
                                    laststack =
                                        pses
                                            |> List.filter (\l -> List.length l > 1)
                                            |> List.head
                                            |> Maybe.withDefault []
                                            |> List.reverse
                                            |> List.drop 1
                                            |> List.reverse
                                in
                                ( updateSearchStack laststack { model | prevSearches = pses }, Cmd.none )

                            else
                                ( model, Cmd.none )

                        Data.PvyZkListNoteSearchResult sr ->
                            case state of
                                ShowMessage _ login _ ->
                                    ( { model | state = EditZkNoteListing { dialog = Nothing, zone = model.timezone } login }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( updateSearchResult sr model, Cmd.none )

                        Data.PvyZkNoteArchives ar ->
                            case model.state of
                                ArchiveListing al login ->
                                    ( { model | state = ArchiveListing (ArchiveListing.updateSearchResult ar.results al) login }
                                    , Cmd.none
                                    )

                                _ ->
                                    case stateLogin state of
                                        Just login ->
                                            ( { model | state = ArchiveListing (ArchiveListing.init ar) login }
                                            , Cmd.none
                                            )

                                        Nothing ->
                                            ( displayMessageDialog
                                                { model | state = initLoginState model Top }
                                                "can't access note archives; you're not logged in!"
                                            , Cmd.none
                                            )

                        Data.PvyZkNote zkn ->
                            case state of
                                ArchiveListing st login ->
                                    handleArchiveListing model login (ArchiveListing.onZkNote zkn st)

                                _ ->
                                    ( unexpectedMessage model (showPrivateReply ziresponse)
                                    , Cmd.none
                                    )

                        Data.PvyZkNoteAndLinksWhat znew ->
                            onZkNoteEditWhat model pt znew

                        Data.PvyZkNoteComments zc ->
                            case state of
                                EditZkNote s login ->
                                    ( { model | state = EditZkNote (EditZkNote.commentsRecieved zc s) login }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( unexpectedMessage model (showPrivateReply ziresponse)
                                    , Cmd.none
                                    )

                        Data.PvySavedZkNote szkn ->
                            case state of
                                EditZkNote emod login ->
                                    let
                                        eznst =
                                            EditZkNote.onSaved
                                                emod
                                                szkn

                                        rn =
                                            EditZkNote.toZkListNote eznst
                                                |> Maybe.map
                                                    (\zkln ->
                                                        addRecentZkListNote model.recentNotes zkln
                                                    )
                                                |> Maybe.withDefault model.recentNotes

                                        st =
                                            EditZkNote eznst login
                                    in
                                    ( { model | state = st, recentNotes = rn }
                                    , Cmd.none
                                    )

                                _ ->
                                    -- just ignore if we're not editing a new note.
                                    ( model, Cmd.none )

                        Data.PvySavedZkNoteAndLinks szkn ->
                            case state of
                                EditZkNote emod login ->
                                    let
                                        eznst =
                                            EditZkNote.onSaved
                                                emod
                                                szkn

                                        rn =
                                            EditZkNote.toZkListNote eznst
                                                |> Maybe.map
                                                    (\zkln ->
                                                        addRecentZkListNote model.recentNotes zkln
                                                    )
                                                |> Maybe.withDefault model.recentNotes

                                        st =
                                            EditZkNote eznst login
                                    in
                                    ( { model | state = st, recentNotes = rn }
                                    , Cmd.none
                                    )

                                _ ->
                                    -- just ignore if we're not editing a new note.
                                    ( model, Cmd.none )

                        Data.PvyDeletedZkNote _ ->
                            ( model, Cmd.none )

                        Data.PvySavedZkLinks ->
                            ( model, Cmd.none )

                        Data.PvyZkLinks _ ->
                            ( model, Cmd.none )

                        Data.PvySavedImportZkNotes ->
                            ( model, Cmd.none )

                        Data.PvyHomeNoteSet id ->
                            case model.state of
                                EditZkNote eznstate login ->
                                    let
                                        x =
                                            EditZkNote.setHomeNote eznstate id
                                    in
                                    ( { model
                                        | state =
                                            EditZkNote x { login | homenote = Just id }
                                      }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( model, Cmd.none )

                        Data.PvyJobStatus jobstatus ->
                            let
                                js =
                                    case Dict.get jobstatus.jobno model.jobs.jobs of
                                        Just j ->
                                            case j.state of
                                                Data.Completed ->
                                                    -- TODO fix , do counter
                                                    { jobstatus | state = Data.Completed }

                                                _ ->
                                                    jobstatus

                                        _ ->
                                            jobstatus

                                nm =
                                    { model | jobs = { mobile = model.mobile, jobs = Dict.insert jobstatus.jobno js model.jobs.jobs } }
                            in
                            ( if jobstatus.state == Data.Started then
                                { nm
                                    | state =
                                        JobsDialog
                                            (JobsDialog.init
                                                nm.jobs
                                                Common.buttonStyle
                                                (E.map (\_ -> ()) (viewState model.size model.state model))
                                            )
                                            nm.state
                                }

                              else
                                nm
                            , Cmd.none
                            )

                        Data.PvyJobNotFound jobno ->
                            let
                                nm =
                                    { model
                                        | jobs =
                                            { mobile = model.mobile
                                            , jobs =
                                                Dict.insert jobno
                                                    { jobno = jobno, state = Data.Failed, message = "job not found" }
                                                    model.jobs.jobs
                                            }
                                    }
                            in
                            ( nm, Cmd.none )

                        Data.PvyNoop ->
                            -- just ignore these.
                            ( model, Cmd.none )

                        -- unused messages!  remove?
                        Data.PvyArchives _ ->
                            ( model, Cmd.none )

                        Data.PvyArchiveZkLinks _ ->
                            ( model, Cmd.none )

                        Data.PvyZkNoteIdSearchResult _ ->
                            ( model, Cmd.none )

                        Data.PvyZkNoteAndLinksSearchResult _ ->
                            ( model, Cmd.none )

                        -- TODO: still used?
                        Data.PvyFileSyncComplete ->
                            ( model, Cmd.none )

                        -- TODO: still used?
                        Data.PvySyncComplete ->
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
                                (Just model.state)
                      }
                    , sendPIMsg model.fui
                        (Data.PbrGetZkNoteAndLinks
                            { zknote = id
                            , what = ""
                            , edittab = Nothing
                            }
                        )
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
                    case state of
                        EditZkNote _ _ ->
                            -- revert to the edit state.
                            ( { model | state = state }, Cmd.none )

                        _ ->
                            case es.id of
                                Just id ->
                                    ( { model | state = state }
                                    , sendZIMsg model.fui
                                        (Data.PvqGetZkNoteAndLinks
                                            { zknote = id
                                            , what = ""
                                            , edittab = Nothing
                                            }
                                        )
                                    )

                                Nothing ->
                                    -- uh, initial page I guess.  would expect prev state to be edit if no id.
                                    -- initialPage model ((stateRoute state).route) |> Maybe.withDefault Top)
                                    initToRoute model (stateRoute state).route

                View.Switch id ->
                    ( model
                    , sendPIMsg model.fui
                        (Data.PbrGetZkNoteAndLinks
                            { zknote = id
                            , what = ""
                            , edittab = Nothing
                            }
                        )
                    )

        ( EditZkNoteMsg em, EditZkNote es login ) ->
            handleEditZkNoteCmd model login (EditZkNote.update em es)

        ( EditZkNoteMsg EditZkNote.Noop, _ ) ->
            ( model, Cmd.none )

        ( EditZkNoteListingMsg em, EditZkNoteListing es login ) ->
            handleEditZkNoteListing model login (EditZkNoteListing.update em es model.spmodel model.zknSearchResult login)

        ( ImportMsg em, Import es login ) ->
            let
                ( emod, ecmd ) =
                    Import.update em es

                backtolisting =
                    let
                        nm =
                            { model
                                | state =
                                    EditZkNoteListing { dialog = Nothing, zone = model.timezone } login
                            }
                    in
                    case SP.getSearch model.spmodel of
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
                            backtolisting

                        notecmds =
                            List.map
                                (\n ->
                                    sendZIMsg model.fui
                                        (Data.PvqSaveImportZkNotes [ n ])
                                )
                                notes
                    in
                    ( m
                    , Cmd.batch
                        (c
                            :: notecmds
                        )
                    )

                Import.SelectFiles ->
                    ( { model | state = Import emod login }
                    , FS.files []
                        (\a b -> ImportMsg (Import.FilesSelected a b))
                    )

                Import.Cancel ->
                    backtolisting

                Import.Command cmd ->
                    ( model, Cmd.map ImportMsg cmd )

                Import.SPMod fn ->
                    handleSPMod model fn

        ( DisplayMessageMsg bm, DisplayMessage bs prevstate ) ->
            case GD.update bm bs of
                GD.Dialog nmod ->
                    ( { model | state = DisplayMessage nmod prevstate }, Cmd.none )

                GD.Ok _ ->
                    case prevstate of
                        ShowMessage _ _ (Just ps) ->
                            ( { model | state = ps }, Cmd.none )

                        PubShowMessage _ (Just ps) ->
                            ( { model | state = ps }, Cmd.none )

                        _ ->
                            ( { model | state = prevstate }, Cmd.none )

                GD.Cancel ->
                    ( { model | state = prevstate }, Cmd.none )

        ( MessageNLinkMsg bm, MessageNLink bs prevstate ) ->
            case GD.update bm bs of
                GD.Dialog nmod ->
                    ( { model | state = MessageNLink nmod prevstate }, Cmd.none )

                GD.Ok _ ->
                    case prevstate of
                        ShowMessage _ _ (Just ps) ->
                            ( { model | state = ps }, Cmd.none )

                        PubShowMessage _ (Just ps) ->
                            ( { model | state = ps }, Cmd.none )

                        _ ->
                            ( { model | state = prevstate }, Cmd.none )

                GD.Cancel ->
                    ( { model | state = prevstate }, Cmd.none )

        ( MessageNLinkMsg GD.Noop, _ ) ->
            ( model, Cmd.none )

        ( Noop, _ ) ->
            ( model, Cmd.none )

        ( ChangePasswordDialogMsg GD.Noop, _ ) ->
            ( model, Cmd.none )

        ( ChangeEmailDialogMsg GD.Noop, _ ) ->
            ( model, Cmd.none )

        ( ChangeRemoteUrlDialogMsg GD.Noop, _ ) ->
            ( model, Cmd.none )

        ( SelectDialogMsg GD.Noop, _ ) ->
            ( model, Cmd.none )

        ( DisplayMessageMsg GD.Noop, _ ) ->
            ( model, Cmd.none )

        ( OnFileSelected file files, _ ) ->
            let
                fc =
                    1 + List.length files

                tr =
                    model.trackedRequests

                nrid =
                    String.fromInt (model.trackedRequests.requestCount + 1)

                nrq =
                    (file :: files)
                        |> List.map F.name
                        |> (\names ->
                                FileUpload { filenames = names, progress = Nothing, files = Nothing }
                           )

                ntr =
                    { tr
                        | requestCount = tr.requestCount + fc
                        , requests =
                            Dict.insert nrid
                                nrq
                                tr.requests
                    }
            in
            ( { model
                | trackedRequests = ntr
                , state =
                    RequestsDialog
                        (RequestsDialog.init
                            -- dummy state we won't use
                            { requestCount = 0, mobile = model.mobile, requests = Dict.empty }
                            Common.buttonStyle
                            (E.map (\_ -> ()) (viewState model.size model.state model))
                        )
                        model.state
              }
            , Http.request
                { method = "POST"
                , headers = []
                , url = model.fui.location ++ "/upload"
                , body =
                    file
                        :: files
                        |> List.map (\f -> Http.filePart (F.name f) f)
                        |> Http.multipartBody
                , expect = Http.expectJson (FileUploadedButGetTime nrid) Data.uploadReplyDecoder
                , timeout = Nothing
                , tracker = Just nrid
                }
            )

        ( FileUploadedButGetTime what zrd, state ) ->
            ( { model | state = state }
            , Time.now
                |> Task.perform (\pt -> FileUploaded what (Result.map (\zi -> ( pt, zi )) zrd))
            )

        ( FileUploaded what zrd, _ ) ->
            case zrd of
                Err e ->
                    ( displayMessageDialog model <| Util.httpErrorString e, Cmd.none )

                Ok ( _, ziresponse ) ->
                    case ziresponse of
                        Data.UrFilesUploaded files ->
                            ( { model
                                | trackedRequests =
                                    case Dict.get what model.trackedRequests.requests of
                                        Just (FileUpload fu) ->
                                            let
                                                trqs =
                                                    model.trackedRequests
                                            in
                                            { trqs
                                                | requests =
                                                    Dict.insert what
                                                        (FileUpload
                                                            { fu
                                                                | files =
                                                                    Just files
                                                            }
                                                        )
                                                        trqs.requests
                                            }

                                        _ ->
                                            model.trackedRequests
                              }
                            , Cmd.none
                            )

        ( RequestProgress a b, _ ) ->
            let
                tr =
                    model.trackedRequests
            in
            case Dict.get a tr.requests of
                Just (FileUpload trq) ->
                    ( { model
                        | trackedRequests =
                            { tr | requests = Dict.insert a (FileUpload { trq | progress = Just b }) tr.requests }
                      }
                    , Cmd.none
                    )

                Nothing ->
                    ( model, Cmd.none )

        ( MdInlineXformCmd cm, MdInlineXform _ prevstate ) ->
            case cm of
                MdInlineXform.Close ->
                    ( { model | state = prevstate }, Cmd.none )

                MdInlineXform.UpdateInline umsg ->
                    case prevstate of
                        EditZkNote em login ->
                            let
                                emod =
                                    EditZkNote.updateEditBlock umsg em
                            in
                            ( { model | state = EditZkNote emod login }, Cmd.none )

                        _ ->
                            ( { model | state = prevstate }, Cmd.none )

                _ ->
                    ( { model | state = prevstate }, Cmd.none )

        ( MdInlineXformCmd cm, EditZkNote em login ) ->
            case cm of
                MdInlineXform.UpdateInline umsg ->
                    let
                        emod =
                            EditZkNote.updateEditBlock umsg em
                    in
                    ( { model | state = EditZkNote emod login }, Cmd.none )

                _ ->
                    ( model, Cmd.none )

        ( WkMsg (Ok key), MdInlineXform bs prevstate ) ->
            case Toop.T4 key.key key.ctrl key.alt key.shift of
                Toop.T4 "Enter" False False False ->
                    handleMdInlineXformOk model prevstate (GD.update (GD.EltMsg MdInlineXform.OkClick) bs)

                _ ->
                    ( model, Cmd.none )

        ( MdInlineXformMsg bm, MdInlineXform bs prevstate ) ->
            handleMdInlineXformOk model prevstate (GD.update bm bs)

        ( MdInlineXformMsg _, _ ) ->
            ( model, Cmd.none )

        ( JobsDialogMsg bm, JobsDialog bs prevstate ) ->
            -- TODO address this hack!
            case GD.update bm { bs | model = model.jobs } of
                GD.Dialog nmod ->
                    ( { model
                        | state = JobsDialog nmod prevstate
                        , jobs = nmod.model
                      }
                    , Cmd.none
                    )

                GD.Ok return ->
                    case return of
                        JobsDialog.Close ->
                            ( { model | state = prevstate }, Cmd.none )

                        JobsDialog.Tag s ->
                            case stateLogin prevstate of
                                Just login ->
                                    ( { model
                                        | state =
                                            TagFiles
                                                (TagAThing.init
                                                    (TagFiles.initThing s)
                                                    TagAThing.AddLinksOnly
                                                    []
                                                    login
                                                )
                                                login
                                                prevstate
                                      }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( { model | state = prevstate }, Cmd.none )

                GD.Cancel ->
                    ( { model | state = prevstate }, Cmd.none )

        ( JobsDialogMsg _, _ ) ->
            ( model, Cmd.none )

        ( JobsPollTick _, _ ) ->
            ( model
            , model.jobs.jobs
                |> Dict.keys
                |> List.map
                    (\jobno ->
                        sendZIMsg model.fui
                            (Data.PvqGetJobStatus jobno)
                    )
                |> Cmd.batch
            )

        ( RequestsDialogMsg bm, RequestsDialog bs prevstate ) ->
            -- TODO address this hack!
            case GD.update bm { bs | model = model.trackedRequests } of
                GD.Dialog nmod ->
                    ( { model
                        | state = RequestsDialog nmod prevstate
                        , trackedRequests = nmod.model
                      }
                    , Cmd.none
                    )

                GD.Ok return ->
                    case return of
                        RequestsDialog.Close ->
                            ( { model | state = prevstate }, Cmd.none )

                        RequestsDialog.Tag s ->
                            case stateLogin prevstate of
                                Just login ->
                                    ( { model
                                        | state =
                                            TagFiles
                                                (TagAThing.init
                                                    (TagFiles.initThing s)
                                                    TagAThing.AddLinksOnly
                                                    []
                                                    login
                                                )
                                                login
                                                prevstate
                                      }
                                    , Cmd.none
                                    )

                                _ ->
                                    ( { model | state = prevstate }, Cmd.none )

                GD.Cancel ->
                    ( { model | state = prevstate }, Cmd.none )

        ( RequestsDialogMsg _, _ ) ->
            ( model, Cmd.none )

        ( x, _ ) ->
            ( unexpectedMsg model x
            , Cmd.none
            )


updateSearchResult : Data.ZkListNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | zknSearchResult = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


updateSearch : List Data.TagSearch -> Model -> Model
updateSearch ts model =
    { model
        | spmodel = SP.setSearch model.spmodel ts
    }


updateSearchStack : List Data.TagSearch -> Model -> Model
updateSearchStack tsl model =
    let
        spm =
            model.spmodel
    in
    { model
        | spmodel = { spm | searchStack = tsl }
    }


handleSPMod : Model -> (SP.Model -> ( SP.Model, SP.Command )) -> ( Model, Cmd Msg )
handleSPMod model fn =
    let
        ( nspm, spcmd ) =
            fn model.spmodel
    in
    case spcmd of
        SP.None ->
            ( { model | spmodel = nspm }
            , Cmd.none
            )

        SP.Save ->
            ( { model | spmodel = nspm }
            , Cmd.none
            )

        SP.Copy _ ->
            -- TODO
            ( { model | spmodel = nspm }
            , Cmd.none
            )

        SP.Search ts ->
            -- clear search result on new search
            let
                zsr =
                    model.zknSearchResult
            in
            sendSearch { model | spmodel = nspm, zknSearchResult = { zsr | notes = [] } } ts

        SP.SyncFiles ts ->
            ( { model | spmodel = nspm }
            , sendZIMsg model.fui (Data.PvqSyncFiles ts)
            )


handleTASelection : Model -> EditZkNote.Model -> LoginData -> DataUtil.TASelection -> ( Model, Cmd Msg )
handleTASelection model emod login tas =
    case EditZkNote.onTASelection emod model.zknSearchResult model.recentNotes tas of
        EditZkNote.TAError e ->
            ( displayMessageDialog model e, Cmd.none )

        EditZkNote.TASave s ->
            sendZIMsgExp model
                model.fui
                (Data.PvqSaveZkNoteAndLinks s)
                (TAReplyData tas)

        EditZkNote.TAUpdated nemod s ->
            ( { model | state = EditZkNote nemod login }
            , Cmd.batch
                ((case s of
                    Just sel ->
                        setTASelection (DataUtil.encodeSetSelection sel)

                    Nothing ->
                        Cmd.none
                 )
                    :: makeNewNoteCacheGets (EM.getMd nemod.edMarkdown) model
                )
            )

        EditZkNote.TANoop ->
            ( model, Cmd.none )


makeNoteCacheGets : String -> Model -> List (Cmd Msg)
makeNoteCacheGets md model =
    MC.noteIds md
        |> TSet.toList
        |> List.map
            (\id ->
                case NC.getNote model.noteCache id of
                    Just (NC.ZNAL zkn) ->
                        sendZIMsg model.fui
                            (Data.PvqGetZnlIfChanged
                                { zknote = id
                                , what = "cache"
                                , edittab = Nothing
                                , changeddate = zkn.zknote.changeddate
                                }
                            )

                    Just NC.Private ->
                        sendZIMsg model.fui
                            (Data.PvqGetZkNoteAndLinks
                                { zknote = id
                                , what = "cache"
                                , edittab = Nothing
                                }
                            )

                    Just NC.NotFound ->
                        sendZIMsg model.fui
                            (Data.PvqGetZkNoteAndLinks
                                { zknote = id
                                , what = "cache"
                                , edittab = Nothing
                                }
                            )

                    Nothing ->
                        sendZIMsg model.fui
                            (Data.PvqGetZkNoteAndLinks
                                { zknote = id
                                , what = "cache"
                                , edittab = Nothing
                                }
                            )
            )


makePubNoteCacheGets : Model -> String -> List (Cmd Msg)
makePubNoteCacheGets model md =
    MC.noteIds md
        |> TSet.toList
        |> List.map
            (makePubNoteCacheGet model)


makePubNoteCacheGet : Model -> ZkNoteId -> Cmd Msg
makePubNoteCacheGet model id =
    case NC.getNote model.noteCache id of
        Just (NC.ZNAL zkn) ->
            sendPIMsg
                model.fui
                (Data.PbrGetZnlIfChanged
                    { zknote = id
                    , what = "cache"
                    , edittab = Nothing
                    , changeddate = zkn.zknote.changeddate
                    }
                )

        Just NC.NotFound ->
            sendPIMsg
                model.fui
                (Data.PbrGetZkNoteAndLinks
                    { zknote = id
                    , what = "cache"
                    , edittab = Nothing
                    }
                )

        Just NC.Private ->
            sendPIMsg
                model.fui
                (Data.PbrGetZkNoteAndLinks
                    { zknote = id
                    , what = "cache"
                    , edittab = Nothing
                    }
                )

        Nothing ->
            sendPIMsg
                model.fui
                (Data.PbrGetZkNoteAndLinks
                    { zknote = id
                    , what = "cache"
                    , edittab = Nothing
                    }
                )


makeNewNoteCacheGets : String -> Model -> List (Cmd Msg)
makeNewNoteCacheGets md model =
    -- only retreive not-found notes.
    MC.noteIds md
        |> TSet.toList
        |> List.filterMap
            (\id ->
                case NC.getNote model.noteCache id of
                    Just _ ->
                        Nothing

                    Nothing ->
                        Just <|
                            sendZIMsg model.fui
                                (Data.PvqGetZkNoteAndLinks
                                    { zknote = id
                                    , what = "cache"
                                    , edittab = Nothing
                                    }
                                )
            )


handleEditZkNoteCmd : Model -> LoginData -> ( EditZkNote.Model, EditZkNote.Command ) -> ( Model, Cmd Msg )
handleEditZkNoteCmd model login ( emod, ecmd ) =
    let
        backtolisting =
            let
                nm =
                    { model
                        | state =
                            EditZkNoteListing { dialog = Nothing, zone = model.timezone } login
                    }
            in
            case SP.getSearch model.spmodel of
                Just s ->
                    sendSearch nm s

                Nothing ->
                    ( nm, Cmd.none )

        ngets =
            makeNewNoteCacheGets (EM.getMd emod.edMarkdown) model

        ( rm, rcmd ) =
            case ecmd of
                EditZkNote.SaveExit snpl ->
                    let
                        gotres =
                            let
                                nm =
                                    { model
                                        | state =
                                            EditZkNoteListing { dialog = Nothing, zone = model.timezone } login
                                    }
                            in
                            case SP.getSearch model.spmodel of
                                Just s ->
                                    sendSearch nm s

                                Nothing ->
                                    ( nm, Cmd.none )

                        onmsg : Model -> Msg -> ( Model, Cmd Msg )
                        onmsg _ ms =
                            case ms of
                                ZkReplyData (Ok ( _, Data.PvySavedZkNoteAndLinks _ )) ->
                                    gotres

                                ZkReplyData (Ok ( _, Data.PvyServerError e )) ->
                                    ( displayMessageDialog model (DataUtil.showPrivateError e)
                                    , Cmd.none
                                    )

                                _ ->
                                    ( unexpectedMsg model ms
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
                                    (Just model.state)
                                )
                                onmsg
                      }
                    , sendZIMsg model.fui
                        (Data.PvqSaveZkNoteAndLinks snpl)
                    )

                EditZkNote.Save snpl ->
                    ( { model
                        | state = EditZkNote emod login

                        -- reset keeps on save, to get rid of unused notes.
                        , noteCache = NC.setKeeps (MC.noteIds (EM.getMd emod.edMarkdown)) model.noteCache
                      }
                    , sendZIMsg model.fui
                        (Data.PvqSaveZkNoteAndLinks snpl)
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
                                    -- stop waiting, issue listing query when a message
                                    -- is received. (presumably delete reply)
                                    ( { mod | state = m.state }, c )
                                )
                      }
                    , sendZIMsg model.fui
                        (Data.PvqDeleteZkNote id)
                    )

                EditZkNote.Switch id ->
                    let
                        ( st, cmd ) =
                            ( ShowMessage { message = "loading note..." }
                                login
                                (Just model.state)
                            , sendZIMsg model.fui
                                (Data.PvqGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = Nothing
                                    }
                                )
                            )
                    in
                    ( { model | state = st }, cmd )

                EditZkNote.SaveSwitch s id ->
                    let
                        ( st, cmd ) =
                            ( ShowMessage { message = "loading note..." }
                                login
                                (Just model.state)
                            , sendZIMsg model.fui
                                (Data.PvqGetZkNoteAndLinks
                                    { zknote = id
                                    , what = ""
                                    , edittab = Nothing
                                    }
                                )
                            )
                    in
                    ( { model | state = st }
                    , Cmd.batch
                        [ cmd
                        , sendZIMsg model.fui
                            (Data.PvqSaveZkNoteAndLinks s)
                        ]
                    )

                EditZkNote.View v ->
                    ( { model
                        | state =
                            EView
                                (case v.note of
                                    Left szn ->
                                        View.initSzn model.fui
                                            szn
                                            v.createdate
                                            v.changeddate
                                            v.links
                                            v.panelnote

                                    Right zknal ->
                                        View.initFull model.fui zknal
                                )
                                (EditZkNote emod login)
                      }
                    , Cmd.batch <|
                        makeNoteCacheGets
                            (case v.note of
                                Left szn ->
                                    szn.content

                                Right zknal ->
                                    zknal.zknote.content
                            )
                            model
                    )

                EditZkNote.GetTASelection id what ->
                    ( { model | state = EditZkNote emod login }
                    , getTASelection (JE.object [ ( "id", JE.string id ), ( "what", JE.string what ) ])
                    )

                EditZkNote.SyncFiles s ->
                    ( { model | state = EditZkNote emod login }
                    , sendZIMsg model.fui (Data.PvqSyncFiles s)
                    )

                EditZkNote.SearchHistory ->
                    ( shDialog model
                    , Cmd.none
                    )

                EditZkNote.BigSearch ->
                    backtolisting

                EditZkNote.Settings ->
                    ( { model | state = UserSettings (UserSettings.init login model.stylePalette.fontSize) login (EditZkNote emod login) }
                    , Cmd.none
                    )

                EditZkNote.Admin ->
                    ( model
                    , sendAIMsg model.fui.location OD.ArqGetUsers
                    )

                EditZkNote.SetHomeNote id ->
                    ( { model | state = EditZkNote emod login }
                    , sendZIMsg model.fui (Data.PvqSetHomeNote id)
                    )

                EditZkNote.AddToRecent zklns ->
                    ( { model
                        | state = EditZkNote emod login
                        , recentNotes =
                            List.foldl
                                (\zkln rns ->
                                    addRecentZkListNote rns zkln
                                )
                                model.recentNotes
                                zklns
                      }
                    , Cmd.none
                    )

                EditZkNote.ShowMessage e ->
                    ( displayMessageDialog model e, Cmd.none )

                EditZkNote.ShowArchives id ->
                    ( model
                    , sendZIMsg model.fui (Data.PvqGetZkNoteArchives { zknote = id, offset = 0, limit = Just SU.defaultSearchLimit })
                    )

                EditZkNote.FileUpload ->
                    ( model
                      -- can use rust open dialog on tauri desktop, but panics on android.
                      -- , if model.fui.tauri then
                    , if False then
                        -- keep this fn here even though never called, otherwise error on JS side.
                        sendTIValueTauri <|
                            Data.tauriRequestEncoder
                                Data.TrqUploadFiles

                      else
                        -- using normal http upload.
                        FS.files [] OnFileSelected
                    )

                EditZkNote.Sync ->
                    ( model
                    , sendZIMsg model.fui Data.PvqSyncRemote
                    )

                EditZkNote.PowerTag ->
                    ( { model
                        | state =
                            TagNotes2
                                (TagNotes2.init
                                    login
                                    []
                                    []
                                    TagNotes2.AddNotes
                                )
                                login
                                model.state
                      }
                    , Cmd.none
                    )

                EditZkNote.Requests ->
                    ( { model
                        | state =
                            RequestsDialog
                                (RequestsDialog.init
                                    model.trackedRequests
                                    Common.buttonStyle
                                    (E.map (\_ -> ()) (viewState model.size model.state model))
                                )
                                model.state
                      }
                    , Cmd.none
                    )

                EditZkNote.Jobs ->
                    ( { model
                        | state =
                            JobsDialog
                                (JobsDialog.init
                                    model.jobs
                                    Common.buttonStyle
                                    (E.map (\_ -> ()) (viewState model.size model.state model))
                                )
                                model.state
                      }
                    , Cmd.none
                    )

                EditZkNote.SPMod fn ->
                    let
                        ( nspm, spcmd ) =
                            fn model.spmodel

                        nmod =
                            { model | spmodel = nspm }
                    in
                    case spcmd of
                        SP.Copy s ->
                            -- kind of messed up to have this here and not in the EditZkNote file
                            ( { nmod
                                | state =
                                    EditZkNote
                                        { emod
                                            | mbReplaceString =
                                                Just <|
                                                    (if EM.getMd emod.edMarkdown == "" then
                                                        "<search query=\""

                                                     else
                                                        "<search query=\""
                                                    )
                                                        ++ String.replace "&" "&amp;" s
                                                        ++ "\"/>"
                                        }
                                        login
                              }
                            , getTASelection (JE.object [ ( "id", JE.string "mdtext" ), ( "what", JE.string "replacestring" ) ])
                            )

                        _ ->
                            -- otherwise its all the usual stuff.
                            handleSPMod nmod fn

                EditZkNote.InlineXform inline f ->
                    ( { model
                        | state =
                            MdInlineXform
                                (MdInlineXform.init
                                    inline
                                    f
                                    model.mobile
                                    Common.buttonStyle
                                    (E.map (\_ -> ()) (viewState model.size model.state model))
                                )
                                model.state
                      }
                    , Cmd.none
                    )

                EditZkNote.SaveLinks szl ->
                    ( { model | state = EditZkNote emod login }
                    , sendZIMsg model.fui (Data.PvqSaveZkLinks szl)
                    )

                EditZkNote.Cmd cmd mbcommand ->
                    let
                        ( nmod, mbcmd ) =
                            case mbcommand of
                                Just emd ->
                                    handleEditZkNoteCmd model login ( emod, emd )

                                Nothing ->
                                    ( { model | state = EditZkNote emod login }, Cmd.none )
                    in
                    ( nmod
                    , Cmd.batch [ Cmd.map EditZkNoteMsg cmd, mbcmd ]
                    )
    in
    ( rm, Cmd.batch (rcmd :: ngets) )


handleEditZkNoteListing : Model -> LoginData -> ( EditZkNoteListing.Model, EditZkNoteListing.Command ) -> ( Model, Cmd Msg )
handleEditZkNoteListing model login ( emod, ecmd ) =
    case ecmd of
        EditZkNoteListing.None ->
            ( { model | state = EditZkNoteListing emod login }, Cmd.none )

        EditZkNoteListing.New ->
            ( { model | state = EditZkNote (EditZkNote.initNew model.fui login [] model.mobile) login }, Cmd.none )

        EditZkNoteListing.Done ->
            ( { model | state = UserSettings (UserSettings.init login model.stylePalette.fontSize) login (EditZkNoteListing emod login) }
            , Cmd.none
            )

        EditZkNoteListing.Import ->
            ( { model | state = Import (Import.init login) login }
            , Cmd.none
            )

        EditZkNoteListing.PowerDelete s ->
            ( { model | state = EditZkNoteListing emod login }
            , sendZIMsg model.fui
                (Data.PvqPowerDelete s)
            )

        EditZkNoteListing.SearchHistory ->
            ( shDialog model
            , Cmd.none
            )

        EditZkNoteListing.SPMod fn ->
            handleSPMod model fn


handleArchiveListing : Model -> LoginData -> ( ArchiveListing.Model, ArchiveListing.Command ) -> ( Model, Cmd Msg )
handleArchiveListing model login ( emod, ecmd ) =
    case ecmd of
        ArchiveListing.None ->
            ( { model | state = ArchiveListing emod login }, Cmd.none )

        ArchiveListing.Selected id ->
            ( { model | state = ArchiveListing emod login }
            , sendZIMsg model.fui (Data.PvqGetZkNote id)
            )

        ArchiveListing.Done ->
            ( { model | state = UserSettings (UserSettings.init login model.stylePalette.fontSize) login (ArchiveListing emod login) }
            , Cmd.none
            )

        ArchiveListing.GetArchives msg ->
            ( { model | state = ArchiveListing emod login }
            , sendZIMsg model.fui <| Data.PvqGetZkNoteArchives msg
            )


handleLogin : Model -> Route -> ( Login.Model, Login.Cmd ) -> ( Model, Cmd Msg )
handleLogin model route ( lmod, lcmd ) =
    case lcmd of
        Login.None ->
            ( { model | state = Login lmod route }, Cmd.none )

        Login.Register ->
            ( { model | state = Login lmod route }
            , sendUIMsg model.fui
                (OD.UrqRegister
                    { uid = lmod.userId
                    , pwd = lmod.password
                    , email = lmod.email
                    , remoteUrl = lmod.remoteUrl
                    }
                )
            )

        Login.Login ->
            ( { model | state = Login lmod route }
            , sendUIMsg model.fui <|
                OD.UrqLogin
                    { uid = lmod.userId
                    , pwd = lmod.password
                    }
            )

        Login.Reset ->
            ( { model | state = Login lmod route }
            , sendUIMsg model.fui <|
                OD.UrqResetPassword
                    { uid = lmod.userId
                    }
            )


handleInvited : Model -> ( Invited.Model, Invited.Cmd ) -> ( Model, Cmd Msg )
handleInvited model ( lmod, lcmd ) =
    case lcmd of
        Invited.None ->
            ( { model | state = Invited lmod }, Cmd.none )

        Invited.RSVP ->
            ( { model | state = Invited lmod }
            , sendUIMsg model.fui
                (OD.UrqRsvp
                    { uid = lmod.userId
                    , pwd = lmod.password
                    , email = lmod.email
                    , invite = lmod.invite
                    }
                )
            )


handleTagFiles :
    Model
    -> ( TagAThing.Model TagFiles.Model TagFiles.Msg TagFiles.Command, TagAThing.Command TagFiles.Command )
    -> LoginData
    -> State
    -> ( Model, Cmd Msg )
handleTagFiles model ( lmod, lcmd ) login st =
    let
        updstate =
            TagFiles lmod login st
    in
    case lcmd of
        TagAThing.Search s ->
            sendSearch { model | state = updstate } s

        TagAThing.SyncFiles s ->
            ( { model | state = updstate }
            , sendZIMsg model.fui (Data.PvqSyncFiles s)
            )

        TagAThing.SearchHistory ->
            ( shDialog { model | state = updstate }
            , Cmd.none
            )

        TagAThing.None ->
            ( { model | state = updstate }, Cmd.none )

        TagAThing.AddToRecent zklns ->
            ( { model
                | state = updstate
                , recentNotes =
                    List.foldl
                        (\zkln rns ->
                            addRecentZkListNote rns zkln
                        )
                        model.recentNotes
                        zklns
              }
            , Cmd.none
            )

        TagAThing.ThingCommand tc ->
            case tc of
                TagFiles.Ok ->
                    let
                        zklns =
                            lmod.thing.model.files

                        zkls =
                            Dict.values lmod.zklDict

                        zklinks : List Data.SaveZkLink2
                        zklinks =
                            List.foldl
                                (\zkln links ->
                                    List.map (\el -> DataUtil.elToSzl2 zkln.id el) zkls
                                        ++ links
                                )
                                []
                                zklns
                    in
                    ( { model | state = st }
                    , sendZIMsg model.fui
                        (Data.PvqSaveZkLinks { links = zklinks })
                    )

                TagFiles.Cancel ->
                    ( { model | state = st }, Cmd.none )

                TagFiles.None ->
                    ( { model | state = updstate }, Cmd.none )

        TagAThing.SPMod fn ->
            handleSPMod model fn


handleTagNotes :
    Model
    -> ( TagAThing.Model TagNotes.Model TagNotes.Msg TagNotes.Command, TagAThing.Command TagNotes.Command )
    -> LoginData
    -> State
    -> ( Model, Cmd Msg )
handleTagNotes model ( lmod, lcmd ) login st =
    let
        updstate =
            TagNotes lmod login st
    in
    case lcmd of
        TagAThing.Search s ->
            sendSearch { model | state = updstate } s

        TagAThing.SyncFiles s ->
            ( { model | state = updstate }
            , sendZIMsg model.fui (Data.PvqSyncFiles s)
            )

        TagAThing.SearchHistory ->
            ( shDialog { model | state = updstate }
            , Cmd.none
            )

        TagAThing.None ->
            ( { model | state = updstate }, Cmd.none )

        TagAThing.AddToRecent zklns ->
            ( { model
                | state = updstate
                , recentNotes =
                    List.foldl
                        (\zkln rns ->
                            addRecentZkListNote rns zkln
                        )
                        model.recentNotes
                        zklns
              }
            , Cmd.none
            )

        TagAThing.ThingCommand tc ->
            case tc of
                TagNotes.Ok ->
                    let
                        zklns =
                            lmod.thing.model.notes

                        zkls =
                            Dict.values lmod.zklDict

                        zklinks : List Data.SaveZkLink2
                        zklinks =
                            List.foldl
                                (\zkln links ->
                                    List.map (\el -> DataUtil.elToSzl2 zkln.id el) zkls
                                        ++ links
                                )
                                []
                                zklns
                    in
                    ( { model | state = st }
                    , sendZIMsg model.fui
                        (Data.PvqSaveZkLinks { links = zklinks })
                    )

                TagNotes.Cancel ->
                    ( { model | state = st }, Cmd.none )

                TagNotes.Which w ->
                    ( { model | state = TagNotes { lmod | addWhich = w } login st }, Cmd.none )

                TagNotes.None ->
                    ( { model | state = updstate }, Cmd.none )

        TagAThing.SPMod fn ->
            handleSPMod { model | state = updstate } fn


handleTagNotes2 :
    Model
    -> ( TagNotes2.Model, TagNotes2.Command )
    -> LoginData
    -> State
    -> ( Model, Cmd Msg )
handleTagNotes2 model ( lmod, lcmd ) login st =
    let
        updstate =
            TagNotes2 lmod login st
    in
    case lcmd of
        TagNotes2.AddToRecent zklns ->
            ( { model
                | state = updstate
                , recentNotes =
                    List.foldl
                        (\zkln rns ->
                            addRecentZkListNote rns zkln
                        )
                        model.recentNotes
                        zklns
              }
            , Cmd.none
            )

        TagNotes2.SearchHistory ->
            ( shDialog { model | state = updstate }
            , Cmd.none
            )

        TagNotes2.Ok ->
            let
                zklns =
                    lmod.notes

                zkls =
                    Dict.values lmod.zklDict

                zklinks : List Data.SaveZkLink2
                zklinks =
                    List.foldl
                        (\zkln links ->
                            List.map (\el -> DataUtil.elToSzl2 zkln.id el) zkls
                                ++ links
                        )
                        []
                        zklns
            in
            ( { model | state = st }
            , sendZIMsg model.fui
                (Data.PvqSaveZkLinks { links = zklinks })
            )

        TagNotes2.Cancel ->
            ( { model | state = st }, Cmd.none )

        TagNotes2.Which w ->
            ( { model | state = TagNotes2 { lmod | addWhich = w } login st }, Cmd.none )

        TagNotes2.SPMod fn ->
            handleSPMod { model | state = updstate } fn

        TagNotes2.None ->
            ( { model | state = updstate }, Cmd.none )


handleInviteUser :
    Model
    -> ( TagAThing.Model InviteUser.Model InviteUser.Msg InviteUser.Command, TagAThing.Command InviteUser.Command )
    -> LoginData
    -> State
    -> ( Model, Cmd Msg )
handleInviteUser model ( lmod, lcmd ) login st =
    let
        updstate =
            InviteUser lmod login st
    in
    case lcmd of
        TagAThing.Search s ->
            sendSearch { model | state = updstate } s

        TagAThing.SyncFiles s ->
            ( { model | state = updstate }
            , sendZIMsg model.fui (Data.PvqSyncFiles s)
            )

        TagAThing.SearchHistory ->
            ( { model | state = updstate }, Cmd.none )

        TagAThing.None ->
            ( { model | state = updstate }, Cmd.none )

        TagAThing.AddToRecent zklns ->
            ( { model
                | state = updstate
                , recentNotes =
                    List.foldl
                        (\zkln rns ->
                            addRecentZkListNote rns zkln
                        )
                        model.recentNotes
                        zklns
              }
            , Cmd.none
            )

        TagAThing.ThingCommand tc ->
            case tc of
                InviteUser.Ok ->
                    ( { model | state = updstate }
                    , sendAIMsg model.fui.location
                        (OD.ArqGetInvite
                            { email =
                                if lmod.thing.model.email /= "" then
                                    Just lmod.thing.model.email

                                else
                                    Nothing
                            , data =
                                DataUtil.encodeZkInviteData (List.map DataUtil.elToSzl (Dict.values lmod.zklDict))
                                    |> JE.encode 2
                                    |> Just
                            }
                        )
                    )

                InviteUser.Cancel ->
                    ( { model | state = st }, Cmd.none )

                InviteUser.None ->
                    ( { model | state = updstate }, Cmd.none )

        TagAThing.SPMod fn ->
            handleSPMod model fn


prevSearchQuery : LoginData -> Data.ZkNoteSearch
prevSearchQuery login =
    let
        ts : Data.TagSearch
        ts =
            Data.Boolex
                { ts1 = Data.SearchTerm { mods = [ Data.ExactMatch, Data.Tag ], term = "search" }
                , ao = Data.And
                , ts2 =
                    Data.SearchTerm { mods = [ Data.User ], term = login.name }
                }
    in
    { tagsearch = [ ts ]
    , offset = 0
    , limit = Just 50
    , what = "prevSearches"
    , resulttype = Data.RtNote
    , archives = Data.Current
    , deleted = False
    , ordering = Nothing
    }


preinit : JD.Value -> Url -> Browser.Navigation.Key -> ( PiModel, Cmd Msg )
preinit jsflags url key =
    ( case JD.decodeValue decodeFlags jsflags of
        Ok flags ->
            PreInit
                { flags = flags
                , url = url
                , key = key
                , mbzone = Nothing
                , mbfontsize = Nothing
                }

        Err e ->
            InitError (JD.errorToString e)
    , Cmd.batch
        [ Task.perform Zone Time.here
        , LS.getLocalVal { for = "", name = "fontsize" }
        ]
    )


initToRoute : Model -> Route -> ( Model, Cmd Msg )
initToRoute model route =
    let
        ( initialstate, c ) =
            routeState model route
    in
    ( { model | state = initialstate }
    , Cmd.batch
        [ Browser.Navigation.replaceUrl model.navkey
            (routeUrl (stateRoute initialstate).route)
        , c
        ]
    )


init : Flags -> Url -> Browser.Navigation.Key -> Time.Zone -> Int -> ( Model, Cmd Msg )
init flags url key zone fontsize =
    let
        seed =
            initialSeed (flags.seed + 7)

        initialroute =
            parseUrl url
                |> Maybe.withDefault Top
                |> (\r ->
                        -- Don't go back to login once we've logged in!  Ha.
                        if r == LoginR then
                            Top

                        else
                            r
                   )

        imodel : Model
        imodel =
            { state =
                case flags.login of
                    Nothing ->
                        PubShowMessage { message = "loading..." } Nothing

                    Just l ->
                        ShowMessage { message = "loading..." } l Nothing
            , size = { width = flags.width, height = flags.height }
            , fui =
                { location = flags.location
                , filelocation = flags.filelocation
                , tauri = flags.tauri
                }
            , navkey = key
            , seed = seed
            , timezone = zone
            , savedRoute = { route = Top, save = False }
            , initialRoute = initialroute
            , prevSearches = []
            , recentNotes = []
            , errorNotes = Dict.empty
            , stylePalette = { defaultSpacing = 10, fontSize = fontsize }
            , adminSettings = flags.adminsettings
            , trackedRequests = { requestCount = 0, mobile = flags.mobile, requests = Dict.empty }
            , jobs = { mobile = flags.mobile, jobs = Dict.empty }
            , noteCache = NC.empty maxCacheNotes
            , ziClosureId = 0
            , ziClosures = Dict.empty
            , mobile = flags.mobile
            , spmodel = SP.initModel
            , zknSearchResult =
                { notes = []
                , offset = 0
                , what = ""
                }
            }

        geterrornote =
            flags.errorid
                |> Maybe.map
                    (\id ->
                        DataUtil.getErrorIndexNote flags.location id ErrorIndexNote
                    )
                |> Maybe.withDefault Cmd.none

        setkeys =
            skcommand <|
                WindowKeys.SetWindowKeys
                    [ { key = "s", ctrl = True, alt = False, shift = False, preventDefault = True }
                    , { key = "s", ctrl = True, alt = True, shift = False, preventDefault = True }
                    , { key = "e", ctrl = True, alt = True, shift = False, preventDefault = True }
                    , { key = "r", ctrl = True, alt = True, shift = False, preventDefault = True }
                    , { key = "v", ctrl = True, alt = True, shift = False, preventDefault = True }
                    , { key = "Enter", ctrl = False, alt = False, shift = False, preventDefault = False }
                    , { key = "Escape", ctrl = False, alt = False, shift = False, preventDefault = False }
                    , { key = "l", ctrl = True, alt = True, shift = False, preventDefault = True }
                    ]

        ( m, c ) =
            initToRoute imodel imodel.initialRoute
    in
    ( m
    , Cmd.batch
        [ c
        , geterrornote
        , setkeys
        ]
    )


main : Platform.Program JD.Value PiModel Msg
main =
    Browser.application
        { init = preinit
        , view = piview
        , update = piupdate
        , subscriptions =
            \model ->
                let
                    rdysubs : List (Sub Msg)
                    rdysubs =
                        case model of
                            Ready rmd ->
                                let
                                    tracks : List (Sub Msg)
                                    tracks =
                                        rmd.trackedRequests.requests
                                            |> Dict.keys
                                            |> List.map (\k -> Http.track k (RequestProgress k))

                                    jobtick : List (Sub Msg)
                                    jobtick =
                                        if
                                            Dict.values rmd.jobs.jobs
                                                |> List.filter
                                                    (\j ->
                                                        not <| jobComplete j.state
                                                    )
                                                |> List.isEmpty
                                                |> not
                                        then
                                            [ Time.every 1000 JobsPollTick
                                            ]

                                        else
                                            []

                                    stsubs : List (Sub Msg)
                                    stsubs =
                                        case rmd.state of
                                            EditZkNote st _ ->
                                                List.map (Sub.map EditZkNoteMsg) <|
                                                    EditZkNote.blockDndSubscriptions st

                                            _ ->
                                                []
                                in
                                tracks ++ jobtick ++ stsubs

                            PreInit _ ->
                                []

                            InitError _ ->
                                []
                in
                Sub.batch <|
                    [ receiveTASelection TASelection
                    , receiveTAError TAError
                    , Browser.Events.onResize (\w h -> WindowSize { width = w, height = h })
                    , keyreceive
                    , LS.localVal ReceiveLocalVal
                    , receiveZITauriResponse TauriZkReplyData
                    , receiveAITauriResponse TauriAdminReplyData
                    , receiveUITauriResponse TauriUserReplyData
                    , receivePITauriResponse TauriPublicReplyData
                    , receiveTITauriResponse TauriTauriReplyData
                    ]
                        ++ rdysubs
        , onUrlRequest = urlRequest
        , onUrlChange = UrlChanged
        }


port getTASelection : JE.Value -> Cmd msg


port setTASelection : JE.Value -> Cmd msg


port receiveTASelection : (JD.Value -> msg) -> Sub msg


port receiveTAError : (JD.Value -> msg) -> Sub msg


port receiveKeyMsg : (JD.Value -> msg) -> Sub msg


port sendZIValueTauri : JD.Value -> Cmd msg


port receiveZITauriResponse : (JD.Value -> msg) -> Sub msg


port sendAIValueTauri : JD.Value -> Cmd msg


port receiveAITauriResponse : (JD.Value -> msg) -> Sub msg


port sendUIValueTauri : JD.Value -> Cmd msg


port receiveUITauriResponse : (JD.Value -> msg) -> Sub msg


port sendPIValueTauri : JD.Value -> Cmd msg


port receivePITauriResponse : (JD.Value -> msg) -> Sub msg


port sendTIValueTauri : JD.Value -> Cmd msg


port receiveTITauriResponse : (JD.Value -> msg) -> Sub msg


keyreceive : Sub Msg
keyreceive =
    receiveKeyMsg <| WindowKeys.receive WkMsg


port sendKeyCommand : JE.Value -> Cmd msg


skcommand : WindowKeys.WindowKeyCmd -> Cmd Msg
skcommand =
    WindowKeys.send sendKeyCommand
