module ZkInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder, showServerResponse)

import Data
import Json.Decode as JD
import Json.Encode as JE
import Search as S
import Util


type SendMsg
    = GetZkNote Int
    | GetZkNoteEdit Data.GetZkNoteEdit
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


type ServerResponse
    = ZkNoteSearchResult Data.ZkNoteSearchResult
    | ZkListNoteSearchResult Data.ZkListNoteSearchResult
    | ArchiveList Data.ZkNoteArchives
    | SavedZkNotePlusLinks Data.SavedZkNote
    | SavedZkNote Data.SavedZkNote
    | DeletedZkNote Int
    | ZkNote Data.ZkNote
    | ZkNoteEdit Data.ZkNoteEdit
    | ZkNoteComments (List Data.ZkNote)
    | ServerError String
    | SavedZkLinks
    | ZkLinks Data.ZkLinks
    | SavedImportZkNotes
    | PowerDeleteComplete Int
    | HomeNoteSet Int
    | FilesUploaded (List Data.ZkListNote)



-- type alias UploadedFiles =
--     { what : String, files : List Data.ZkListNote }
-- decodeUploadedFiles : JD.Decoder UploadedFiles
-- decodeUploadedFiles =
--     JD.succeed UploadedFiles
--         |> Util.andMap (JD.field "what" JD.string)
--         |> Util.andMap (JD.field "files" (JD.list Data.decodeZkListNote))


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

        ZkNoteEdit _ ->
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


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "data", JE.int id )
                ]

        GetZkNoteEdit zkne ->
            JE.object
                [ ( "what", JE.string "getzknoteedit" )
                , ( "data", Data.encodeGetZkNoteEdit zkne )
                ]

        GetZkNoteComments msg ->
            JE.object
                [ ( "what", JE.string "getzknotecomments" )
                , ( "data", Data.encodeGetZkNoteComments msg )
                ]

        GetZkNoteArchives msg ->
            JE.object
                [ ( "what", JE.string "getzknotearchives" )
                , ( "data", Data.encodeGetZkNoteArchives msg )
                ]

        GetArchiveZkNote msg ->
            JE.object
                [ ( "what", JE.string "getarchivezknote" )
                , ( "data", Data.encodeGetArchiveZkNote msg )
                ]

        DeleteZkNote id ->
            JE.object
                [ ( "what", JE.string "deletezknote" )
                , ( "data", JE.int id )
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

        -- GetZkLinks gzl ->
        --     JE.object
        --         [ ( "what", JE.string "getzklinks" )
        --         , ( "data", Data.encodeGetZkLinks gzl )
        --         ]
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

        SetHomeNote id ->
            JE.object
                [ ( "what", JE.string "sethomenote" )
                , ( "data", JE.int id )
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
                    "server error" ->
                        JD.map ServerError (JD.at [ "content" ] JD.string)

                    "zknotesearchresult" ->
                        JD.map ZkNoteSearchResult (JD.at [ "content" ] <| Data.decodeZkNoteSearchResult)

                    "zklistnotesearchresult" ->
                        JD.map ZkListNoteSearchResult (JD.at [ "content" ] <| Data.decodeZkListNoteSearchResult)

                    "zknotearchives" ->
                        JD.map ArchiveList (JD.at [ "content" ] <| Data.decodeZkNoteArchives)

                    "savedzknote" ->
                        JD.map SavedZkNote (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                    "savedzknotepluslinks" ->
                        JD.map SavedZkNotePlusLinks (JD.at [ "content" ] <| Data.decodeSavedZkNote)

                    "deletedzknote" ->
                        JD.map DeletedZkNote (JD.at [ "content" ] <| JD.int)

                    "zknote" ->
                        JD.map ZkNote (JD.at [ "content" ] <| Data.decodeZkNote)

                    "zknoteedit" ->
                        JD.map ZkNoteEdit (JD.at [ "content" ] <| Data.decodeZkNoteEdit)

                    "zknotecomments" ->
                        JD.map ZkNoteComments (JD.at [ "content" ] <| JD.list Data.decodeZkNote)

                    "savedzklinks" ->
                        JD.succeed SavedZkLinks

                    "savedimportzknotes" ->
                        JD.succeed SavedImportZkNotes

                    "zklinks" ->
                        JD.map ZkLinks (JD.field "content" Data.decodeZkLinks)

                    "powerdeletecomplete" ->
                        JD.map PowerDeleteComplete (JD.field "content" JD.int)

                    "homenoteset" ->
                        JD.map HomeNoteSet (JD.field "content" JD.int)

                    "savedfiles" ->
                        JD.map FilesUploaded (JD.field "content" <| JD.list Data.decodeZkListNote)

                    wat ->
                        JD.succeed
                            (ServerError ("invalid 'what' from server: " ++ wat))
            )
