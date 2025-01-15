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
    | GetJobStatus Int
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
    | JobStatus Data.JobStatus
    | JobNotFound Int
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

        JobStatus _ ->
            "JobStatus"

        JobNotFound _ ->
            "JobNotFound"

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
                , ( "data", Data.zkNoteIdEncoder id )
                ]

        GetZkNoteAndLinks zkne ->
            JE.object
                [ ( "what", JE.string "GetZkNoteAndLinks" )
                , ( "data", Data.getZkNoteAndLinksEncoder zkne )
                ]

        GetZnlIfChanged x ->
            JE.object
                [ ( "what", JE.string "GetZnlIfChanged" )
                , ( "data", Data.getZnlIfChangedEncoder x )
                ]

        GetZkNoteComments msg ->
            JE.object
                [ ( "what", JE.string "GetZkNoteComments" )
                , ( "data", Data.getZkNoteCommentsEncoder msg )
                ]

        GetZkNoteArchives msg ->
            JE.object
                [ ( "what", JE.string "GetZkNoteArchives" )
                , ( "data", Data.getZkNoteArchivesEncoder msg )
                ]

        GetArchiveZkNote msg ->
            JE.object
                [ ( "what", JE.string "GetArchiveZkNote" )
                , ( "data", Data.getArchiveZkNoteEncoder msg )
                ]

        DeleteZkNote id ->
            JE.object
                [ ( "what", JE.string "DeleteZkNote" )
                , ( "data", Data.zkNoteIdEncoder id )
                ]

        SaveZkNote x ->
            JE.object
                [ ( "what", JE.string "SaveZkNote" )
                , ( "data", Data.saveZkNoteEncoder x )
                ]

        SaveZkNoteAndLinks s ->
            JE.object
                [ ( "what", JE.string "SaveZkNoteAndLinks" )
                , ( "data", Data.saveZkNoteAndLinksEncoder s )
                ]

        SaveZkLinks zklinks ->
            JE.object
                [ ( "what", JE.string "SaveZkLinks" )
                , ( "data", Data.zkLinksEncoder zklinks )
                ]

        SearchZkNotes s ->
            JE.object
                [ ( "what", JE.string "SearchZkNotes" )
                , ( "data", S.encodeZkNoteSearch s )
                ]

        SaveImportZkNotes n ->
            JE.object
                [ ( "what", JE.string "SaveImportZkNotes" )
                , ( "data", JE.list Data.importZkNoteEncoder n )
                ]

        PowerDelete s ->
            JE.object
                [ ( "what", JE.string "PowerDelete" )
                , ( "data", S.encodeTagSearch s )
                ]

        SetHomeNote id ->
            JE.object
                [ ( "what", JE.string "SetHomeNote" )
                , ( "data", Data.zkNoteIdEncoder id )
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

        GetJobStatus jobno ->
            JE.object
                [ ( "what", JE.string "GetJobStatus" )
                , ( "data", JE.int jobno )
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
                        JD.map ZkNoteSearchResult (JD.at [ "content" ] <| Data.zkNoteSearchResultDecoder)

                    "ZkListNoteSearchResult" ->
                        JD.map ZkListNoteSearchResult (JD.at [ "content" ] <| Data.zkListNoteSearchResultDecoder)

                    "ZkIdSearchResult" ->
                        JD.map ZkIdSearchResult (JD.at [ "content" ] <| Data.zkIdSearchResultDecoder)

                    "ZkNoteArchives" ->
                        JD.map ZkNoteArchives (JD.at [ "content" ] <| Data.zkNoteArchivesDecoder)

                    "SavedZkNote" ->
                        JD.map SavedZkNote (JD.at [ "content" ] <| Data.savedZkNoteDecoder)

                    "SavedZkNoteAndLinks" ->
                        JD.map SavedZkNoteAndLinks (JD.at [ "content" ] <| Data.savedZkNoteDecoder)

                    "DeletedZkNote" ->
                        JD.map DeletedZkNote (JD.at [ "content" ] <| Data.zkNoteIdDecoder)

                    "ZkNote" ->
                        JD.map ZkNote (JD.at [ "content" ] <| Data.zkNoteDecoder)

                    "ZkNoteAndLinksWhat" ->
                        JD.map ZkNoteAndLinksWhat (JD.at [ "content" ] <| Data.zkNoteAndLinksWhatDecoder)

                    "ZkNoteComments" ->
                        JD.map ZkNoteComments (JD.at [ "content" ] <| JD.list Data.zkNoteDecoder)

                    "SavedZkLinks" ->
                        JD.succeed SavedZkLinks

                    "SavedImportZkNotes" ->
                        JD.succeed SavedImportZkNotes

                    "ZkLinks" ->
                        JD.map ZkLinks (JD.field "content" Data.zkLinksDecoder)

                    "PowerDeleteComplete" ->
                        JD.map PowerDeleteComplete (JD.field "content" JD.int)

                    "HomeNoteSet" ->
                        JD.map HomeNoteSet (JD.field "content" Data.zkNoteIdDecoder)

                    "FilesUploaded" ->
                        JD.map FilesUploaded (JD.field "content" <| JD.list Data.zkListNoteDecoder)

                    "JobStatus" ->
                        JD.map JobStatus (JD.field "content" <| Data.jobStatusDecoder)

                    "JobNotFound" ->
                        JD.map JobNotFound (JD.field "content" <| JD.int)

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
