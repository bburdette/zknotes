module UserInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder, showServerResponse)

import Data
import Json.Decode as JD
import Json.Encode as JE
import Search as S


type SendMsg
    = Register String
    | Login
    | GetZkNote Int
    | GetZkNoteEdit Data.GetZkNoteEdit
    | DeleteZkNote Int
    | SaveZkNote Data.SaveZkNote
    | SaveZkLinks Data.ZkLinks
    | GetZkLinks Data.GetZkLinks
    | SearchZkNotes S.ZkNoteSearch
    | SaveImportZkNotes (List Data.ImportZkNote)


type ServerResponse
    = RegistrationSent
    | UserExists
    | UnregisteredUser
    | InvalidUserOrPwd
    | LoggedIn Data.LoginData
    | ZkNoteSearchResult Data.ZkNoteSearchResult
    | SavedZkNote Data.SavedZkNote
    | DeletedZkNote Int
    | ZkNote Data.ZkNote
    | ZkNoteEdit Data.ZkNoteEdit
    | ServerError String
    | SavedZkLinks
    | ZkLinks Data.ZkLinks
    | SavedImportZkNotes


showServerResponse : ServerResponse -> String
showServerResponse sr =
    case sr of
        RegistrationSent ->
            "RegistrationSent"

        UserExists ->
            "UserExists"

        UnregisteredUser ->
            "UnregisteredUser"

        InvalidUserOrPwd ->
            "InvalidUserOrPwd"

        LoggedIn _ ->
            "LoggedIn"

        ZkNoteSearchResult _ ->
            "ZkNoteSearchResult"

        SavedZkNote _ ->
            "SavedZkNote"

        DeletedZkNote _ ->
            "DeletedZkNote"

        ZkNote _ ->
            "ZkNote"

        ZkNoteEdit _ ->
            "ZkNoteEdit"

        ServerError _ ->
            "ServerError"

        SavedZkLinks ->
            "SavedZkLinks"

        SavedImportZkNotes ->
            "SavedImportZkNotes"

        ZkLinks _ ->
            "ZkLinks"


encodeSendMsg : SendMsg -> String -> String -> JE.Value
encodeSendMsg sm uid pwd =
    case sm of
        Register email ->
            JE.object
                [ ( "what", JE.string "register" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", encodeEmail email )
                ]

        Login ->
            JE.object
                [ ( "what", JE.string "login" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                ]

        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.int id )
                ]

        GetZkNoteEdit zkne ->
            JE.object
                [ ( "what", JE.string "getzknoteedit" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeGetZkNoteEdit zkne )
                ]

        DeleteZkNote id ->
            JE.object
                [ ( "what", JE.string "deletezknote" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.int id )
                ]

        SaveZkNote sbe ->
            JE.object
                [ ( "what", JE.string "savezknote" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeSaveZkNote sbe )
                ]

        SaveZkLinks zklinks ->
            JE.object
                [ ( "what", JE.string "savezklinks" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeZkLinks zklinks )
                ]

        GetZkLinks gzl ->
            JE.object
                [ ( "what", JE.string "getzklinks" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeGetZkLinks gzl )
                ]

        SearchZkNotes s ->
            JE.object
                [ ( "what", JE.string "searchzknotes" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", S.encodeZkNoteSearch s )
                ]

        SaveImportZkNotes n ->
            JE.object
                [ ( "what", JE.string "saveimportzknotes" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.list Data.encodeImportZkNote n )
                ]


encodeEmail : String -> JE.Value
encodeEmail email =
    JE.object
        [ ( "email", JE.string email )
        ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "registration sent" ->
                    JD.succeed RegistrationSent

                "unregistered user" ->
                    JD.succeed UnregisteredUser

                "user exists" ->
                    JD.succeed UserExists

                "logged in" ->
                    JD.map LoggedIn (JD.at [ "content" ] Data.decodeLoginData)

                "invalid user or pwd" ->
                    JD.succeed InvalidUserOrPwd

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                "zknotesearchresult" ->
                    JD.map ZkNoteSearchResult (JD.at [ "content" ] <| Data.decodeZkNoteSearchResult)

                "savedzknote" ->
                    JD.map SavedZkNote (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                "deletedzknote" ->
                    JD.map DeletedZkNote (JD.at [ "content" ] <| JD.int)

                "zknote" ->
                    JD.map ZkNote (JD.at [ "content" ] <| Data.decodeZkNote)

                "zknoteedit" ->
                    JD.map ZkNoteEdit (JD.at [ "content" ] <| Data.decodeZkNoteEdit)

                "savedzklinks" ->
                    JD.succeed SavedZkLinks

                "zklinks" ->
                    JD.map ZkLinks (JD.field "content" Data.decodeZkLinks)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
