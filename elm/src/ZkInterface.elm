module ZkInterface exposing
    ( SendMsg(..)
    , ServerResponse(..)
    , encodeEmail
    , encodeSendMsg
    , serverResponseDecoder
    , showServerResponse
    )

import Data exposing (ZkNoteId)
import Json.Decode as JD
import Json.Encode as JE
import Search as S


type SendMsg
    = GetZkNote ZkNoteId
    | GetZkNoteAndLinks Data.GetZkNoteAndLinks
    | GetZnlIfChanged Data.GetZnlIfChanged
    | GetZkNoteComments Data.GetZkNoteComments
    | GetZkNoteArchives Data.GetZkNoteArchives
    | GetArchiveZkNote Data.GetArchiveZkNote
    | DeleteZkNote ZkNoteId
    | SaveZkNote Data.SaveZkNote
    | SaveZkLinks Data.ZkLinks
    | SaveZkNoteAndLinks Data.SaveZkNoteAndLinks
    | SearchZkNotes S.ZkNoteSearch
    | SaveImportZkNotes (List Data.ImportZkNote)
    | PowerDelete S.TagSearch
    | SetHomeNote ZkNoteId
    | SyncRemote
    | SyncFiles S.ZkNoteSearch


type ServerResponse
    = ZkNoteSearchResult Data.ZkNoteSearchResult
    | ZkListNoteSearchResult Data.ZkListNoteSearchResult
    | ZkIdSearchResult Data.ZkIdSearchResult
    | ZkNoteArchives Data.ZkNoteArchives
    | SavedZkNoteAndLinks Data.SavedZkNote
    | SavedZkNote Data.SavedZkNote
    | DeletedZkNote ZkNoteId
    | ZkNote Data.ZkNote
    | ZkNoteAndLinksWhat Data.ZkNoteAndLinksWhat
    | ZkNoteComments (List Data.ZkNote)
    | ServerError String
    | SavedZkLinks
    | ZkLinks Data.ZkLinks
    | SavedImportZkNotes
    | PowerDeleteComplete Int
    | HomeNoteSet ZkNoteId
    | FilesUploaded (List Data.ZkListNote)
    | JobStarted Int
    | JobStatus Data.JobStatus
    | JobComplete Int
    | FileSyncComplete
    | Noop
    | NotLoggedIn
    | LoginError


showServerResponse : ServerResponse -> String
showServerResponse sr =
    case sr of
        ZkNoteSearchResult _ ->
            "ZkNoteSearchResult"

        ZkIdSearchResult _ ->
            "ZkIdSearchResult"

        ZkListNoteSearchResult _ ->
            "ZkListNoteSearchResult"

        ZkNoteArchives _ ->
            "ZkNoteArchives"

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

        SavedZkNoteAndLinks _ ->
            "SavedZkNoteAndLinks"

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

        JobComplete _ ->
            "JobComplete"

        JobStatus _ ->
            "JobStatus"

        JobStarted _ ->
            "JobStarted"

        FileSyncComplete ->
            "FileSyncComplete"

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
                , ( "data", Data.encodeZkNoteId id )
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
                , ( "data", Data.encodeZkNoteId id )
                ]

        SaveZkNote x ->
            JE.object
                [ ( "what", JE.string "SaveZkNote" )
                , ( "data", Data.encodeSaveZkNote x )
                ]

        SaveZkNoteAndLinks s ->
            JE.object
                [ ( "what", JE.string "SaveZkNoteAndLinks" )
                , ( "data", Data.encodeSaveZkNoteAndLinks s )
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
                , ( "data", Data.encodeZkNoteId id )
                ]

        SyncRemote ->
            JE.object
                [ ( "what", JE.string "SyncRemote" )
                , ( "data", JE.null )
                ]

        SyncFiles s ->
            JE.object
                [ ( "what", JE.string "SyncFiles" )
                , ( "data", S.encodeZkNoteSearch s )
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

                    "ZkIdSearchResult" ->
                        JD.map ZkIdSearchResult (JD.at [ "content" ] <| Data.decodeZkIdSearchResult)

                    "ZkNoteArchives" ->
                        JD.map ZkNoteArchives (JD.at [ "content" ] <| Data.decodeZkNoteArchives)

                    "SavedZkNote" ->
                        JD.map SavedZkNote (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                    "SavedZkNoteAndLinks" ->
                        JD.map SavedZkNoteAndLinks (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                    "DeletedZkNote" ->
                        JD.map DeletedZkNote (JD.at [ "content" ] <| Data.decodeZkNoteId)

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
                        JD.map HomeNoteSet (JD.field "content" Data.decodeZkNoteId)

                    "FilesUploaded" ->
                        JD.map FilesUploaded (JD.field "content" <| JD.list Data.decodeZkListNote)

                    "JobStarted" ->
                        JD.map JobStarted (JD.field "content" <| JD.int)

                    "JobStatus" ->
                        JD.map JobStatus (JD.field "content" <| Data.decodeJobStatus)

                    "JobComplete" ->
                        JD.map JobComplete (JD.field "content" <| JD.int)

                    "FileSyncComplete" ->
                        JD.succeed FileSyncComplete

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
