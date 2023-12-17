module ZkInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder, showServerResponse)

import Data
import Json.Decode as JD
import Json.Encode as JE
import Search as S
import Util


type SendMsg
    = GetZkNote Int
    | GetZkNoteAndLinks Data.GetZkNoteAndLinks
    | GetZnlIfChanged Data.GetZnlIfChanged
    | GetZkNoteComments Data.GetZkNoteComments
    | GetZkNoteArchives Data.GetZkNoteArchives
    | GetArchiveZkNote Data.GetArchiveZkNote
    | DeleteZkNote Int
    | SaveZkNote Data.SaveZkNote
    | SaveZkLinks Data.ZkLinks
    | SaveZkNotePlusLinks Data.SaveZkNotePlusLinks
    | SearchZkNotes S.ZkNoteSearch
    | SaveImportZkNotes (List Data.ImportZkNote)
    | PowerDelete S.TagSearch
    | SetHomeNote Int
    | SyncRemote


type ServerResponse
    = ZkNoteSearchResult Data.ZkNoteSearchResult
    | ZkListNoteSearchResult Data.ZkListNoteSearchResult
    | ArchiveList Data.ZkNoteArchives
    | SavedZkNotePlusLinks Data.SavedZkNote
    | SavedZkNote Data.SavedZkNote
    | DeletedZkNote Int
    | ZkNote Data.ZkNote
    | ZkNoteAndLinksWhat Data.ZkNoteAndLinksWhat
    | ZkNoteComments (List Data.ZkNote)
    | ServerError String
    | SavedZkLinks
    | ZkLinks Data.ZkLinks
    | SavedImportZkNotes
    | PowerDeleteComplete Int
    | HomeNoteSet Int
    | FilesUploaded (List Data.ZkListNote)
    | SyncComplete
    | Noop
    | NotLoggedIn
    | LoginError


showServerResponse : ServerResponse -> String
showServerResponse sr =
    case sr of
        ZkNoteSearchResult _ ->
            "ZkNoteSearchResult"

        ZkListNoteSearchResult _ ->
            "ZkListNoteSearchResult"

        ArchiveList _ ->
            "ArchiveList"

        SavedZkNote _ ->
            "SavedZkNote"

        DeletedZkNote _ ->
            "DeletedZkNote"

        ZkNote _ ->
            "ZkNote"

        ZkNoteAndLinksWhat _ ->
            "ZkNoteEdit"

        ZkNoteComments _ ->
            "ZkNoteComments"

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

        HomeNoteSet _ ->
            "HomeNoteSet"

        FilesUploaded _ ->
            "FilesUploaded"

        SyncComplete ->
            "SyncComplete"

        Noop ->
            "Noop"

        NotLoggedIn ->
            "NotLoggedIn"

        LoginError ->
            "LoginError"


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "GetZkNote" )
                , ( "data", JE.int id )
                ]

        GetZkNoteAndLinks zkne ->
            JE.object
                [ ( "what", JE.string "GetZkNoteAndLinks" )
                , ( "data", Data.encodeGetZkNoteEdit zkne )
                ]

        GetZnlIfChanged x ->
            JE.object
                [ ( "what", JE.string "GetZnlIfChanged" )
                , ( "data", Data.encodeGetZnlIfChanged x )
                ]

        GetZkNoteComments msg ->
            JE.object
                [ ( "what", JE.string "GetZkNoteComments" )
                , ( "data", Data.encodeGetZkNoteComments msg )
                ]

        GetZkNoteArchives msg ->
            JE.object
                [ ( "what", JE.string "GetZkNoteArchives" )
                , ( "data", Data.encodeGetZkNoteArchives msg )
                ]

        GetArchiveZkNote msg ->
            JE.object
                [ ( "what", JE.string "GetArchiveZkNote" )
                , ( "data", Data.encodeGetArchiveZkNote msg )
                ]

        DeleteZkNote id ->
            JE.object
                [ ( "what", JE.string "DeleteZkNote" )
                , ( "data", JE.int id )
                ]

        SaveZkNote x ->
            JE.object
                [ ( "what", JE.string "SaveZkNote" )
                , ( "data", Data.encodeSaveZkNote x )
                ]

        SaveZkNotePlusLinks s ->
            JE.object
                [ ( "what", JE.string "SaveZkNotePlusLinks" )
                , ( "data", Data.encodeSaveZkNotePlusLinks s )
                ]

        SaveZkLinks zklinks ->
            JE.object
                [ ( "what", JE.string "SaveZkLinks" )
                , ( "data", Data.encodeZkLinks zklinks )
                ]

        SearchZkNotes s ->
            JE.object
                [ ( "what", JE.string "SearchZkNotes" )
                , ( "data", S.encodeZkNoteSearch s )
                ]

        SaveImportZkNotes n ->
            JE.object
                [ ( "what", JE.string "SaveImportZkNotes" )
                , ( "data", JE.list Data.encodeImportZkNote n )
                ]

        PowerDelete s ->
            JE.object
                [ ( "what", JE.string "PowerDelete" )
                , ( "data", S.encodeTagSearch s )
                ]

        SetHomeNote id ->
            JE.object
                [ ( "what", JE.string "SetHomeNote" )
                , ( "data", JE.int id )
                ]

        SyncRemote ->
            JE.object
                [ ( "what", JE.string "SyncRemote" )
                , ( "data", JE.null )
                ]


encodeEmail : String -> JE.Value
encodeEmail email =
    JE.object
        [ ( "email", JE.string email )
        ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.at [ "what" ]
        JD.string
        |> JD.andThen
            (\what ->
                case what of
                    "ServerError" ->
                        JD.map ServerError (JD.at [ "content" ] JD.string)

                    "ZkNoteSearchResult" ->
                        JD.map ZkNoteSearchResult (JD.at [ "content" ] <| Data.decodeZkNoteSearchResult)

                    "ZkListNoteSearchResult" ->
                        JD.map ZkListNoteSearchResult (JD.at [ "content" ] <| Data.decodeZkListNoteSearchResult)

                    "ArchiveList" ->
                        JD.map ArchiveList (JD.at [ "content" ] <| Data.decodeZkNoteArchives)

                    "SavedZkNote" ->
                        JD.map SavedZkNote (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                    "SavedZkNotePlusLinks" ->
                        JD.map SavedZkNotePlusLinks (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                    "DeletedZkNote" ->
                        JD.map DeletedZkNote (JD.at [ "content" ] <| JD.int)

                    "ZkNote" ->
                        JD.map ZkNote (JD.at [ "content" ] <| Data.decodeZkNote)

                    "ZkNoteAndLinksWhat" ->
                        JD.map ZkNoteAndLinksWhat (JD.at [ "content" ] <| Data.decodeZkNoteEditWhat)

                    "ZkNoteComments" ->
                        JD.map ZkNoteComments (JD.at [ "content" ] <| JD.list Data.decodeZkNote)

                    "SavedZkLinks" ->
                        JD.succeed SavedZkLinks

                    "SavedImportZkNotes" ->
                        JD.succeed SavedImportZkNotes

                    "ZkLinks" ->
                        JD.map ZkLinks (JD.field "content" Data.decodeZkLinks)

                    "PowerDeleteComplete" ->
                        JD.map PowerDeleteComplete (JD.field "content" JD.int)

                    "HomeNoteSet" ->
                        JD.map HomeNoteSet (JD.field "content" JD.int)

                    "FilesUploaded" ->
                        JD.map FilesUploaded (JD.field "content" <| JD.list Data.decodeZkListNote)

                    "SyncComplete" ->
                        JD.succeed SyncComplete

                    "Noop" ->
                        JD.succeed Noop

                    "NotLoggedIn" ->
                        JD.succeed NotLoggedIn

                    "LoginError" ->
                        JD.succeed LoginError

                    wat ->
                        JD.succeed
                            (ServerError ("invalid 'what' from server: " ++ wat))
            )
