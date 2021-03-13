module UserInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder, showServerResponse)

import Data
import Json.Decode as JD
import Json.Encode as JE
import Search as S
import UUID exposing (UUID)


type SendMsg
    = Register Data.Registration
    | Login Data.Login
    | GetZkNote UUID
    | GetZkNoteEdit UUID
    | GetZkLinks UUID
    | DeleteZkNote UUID
    | SaveZkNote Data.SaveZkNote
    | SaveZkLinks Data.ZkLinks
    | SaveZkNotePlusLinks Data.SaveZkNotePlusLinks
    | SearchZkNotes S.ZkNoteSearch
    | SaveImportZkNotes (List Data.ImportZkNote)
    | PowerDelete S.TagSearch


type ServerResponse
    = RegistrationSent
    | UserExists
    | UnregisteredUser
    | InvalidUserOrPwd
    | NotLoggedIn
    | LoggedIn Data.LoginData
    | ZkNoteSearchResult Data.ZkNoteSearchResult
    | SavedZkNotePlusLinks Data.SavedZkNote
    | SavedZkNote Data.SavedZkNote
    | DeletedZkNote UUID
    | ZkNote Data.ZkNote
    | ZkNoteEdit Data.ZkNoteEdit
    | ServerError String
    | SavedZkLinks
    | ZkLinks Data.ZkLinks
    | SavedImportZkNotes
    | PowerDeleteComplete Int


showServerResponse : ServerResponse -> String
showServerResponse sr =
    case sr of
        RegistrationSent ->
            "RegistrationSent"

        UserExists ->
            "UserExists"

        UnregisteredUser ->
            "UnregisteredUser"

        NotLoggedIn ->
            "NotLoggedIn"

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

        SavedZkNotePlusLinks _ ->
            "SavedZkNotePlusLinks"

        SavedImportZkNotes ->
            "SavedImportZkNotes"

        ZkLinks _ ->
            "ZkLinks"

        PowerDeleteComplete _ ->
            "PowerDeleteComplete"


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        Register registration ->
            JE.object
                [ ( "what", JE.string "register" )
                , ( "data", Data.encodeRegistration registration )
                ]

        Login login ->
            JE.object
                [ ( "what", JE.string "login" )
                , ( "data", Data.encodeLogin login )
                ]

        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "data", UUID.toValue id )
                ]

        GetZkNoteEdit zkne ->
            JE.object
                [ ( "what", JE.string "getzknoteedit" )
                , ( "data", UUID.toValue zkne )
                ]

        DeleteZkNote id ->
            JE.object
                [ ( "what", JE.string "deletezknote" )
                , ( "data", UUID.toValue id )
                ]

        SaveZkNote sbe ->
            JE.object
                [ ( "what", JE.string "savezknote" )
                , ( "data", Data.encodeSaveZkNote sbe )
                ]

        SaveZkNotePlusLinks s ->
            JE.object
                [ ( "what", JE.string "savezknotepluslinks" )
                , ( "data", Data.encodeSaveZkNotePlusLinks s )
                ]

        SaveZkLinks zklinks ->
            JE.object
                [ ( "what", JE.string "savezklinks" )
                , ( "data", Data.encodeZkLinks zklinks )
                ]

        GetZkLinks gzl ->
            JE.object
                [ ( "what", JE.string "getzklinks" )
                , ( "data", UUID.toValue gzl )
                ]

        SearchZkNotes s ->
            JE.object
                [ ( "what", JE.string "searchzknotes" )
                , ( "data", S.encodeZkNoteSearch s )
                ]

        SaveImportZkNotes n ->
            JE.object
                [ ( "what", JE.string "saveimportzknotes" )
                , ( "data", JE.list Data.encodeImportZkNote n )
                ]

        PowerDelete s ->
            JE.object
                [ ( "what", JE.string "powerdelete" )
                , ( "data", S.encodeTagSearch s )
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

                "not logged in" ->
                    JD.succeed NotLoggedIn

                "invalid user or pwd" ->
                    JD.succeed InvalidUserOrPwd

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                "zknotesearchresult" ->
                    JD.map ZkNoteSearchResult (JD.at [ "content" ] <| Data.decodeZkNoteSearchResult)

                "savedzknote" ->
                    JD.map SavedZkNote (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                "savedzknotepluslinks" ->
                    JD.map SavedZkNotePlusLinks (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                "deletedzknote" ->
                    JD.map DeletedZkNote (JD.at [ "content" ] <| UUID.jsonDecoder)

                "zknote" ->
                    JD.map ZkNote (JD.at [ "content" ] <| Data.decodeZkNote)

                "zknoteedit" ->
                    JD.map ZkNoteEdit (JD.at [ "content" ] <| Data.decodeZkNoteEdit)

                "savedzklinks" ->
                    JD.succeed SavedZkLinks

                "savedimportzknotes" ->
                    JD.succeed SavedImportZkNotes

                "zklinks" ->
                    JD.map ZkLinks (JD.field "content" Data.decodeZkLinks)

                "powerdeletecomplete" ->
                    JD.map PowerDeleteComplete (JD.field "content" JD.int)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
