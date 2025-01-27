module Data exposing (..)

import Dict exposing (Dict)
import Json.Decode
import Json.Encode
import Orgauth.UserId exposing (UserId, userIdDecoder, userIdEncoder)
import Url.Builder


resultEncoder : (e -> Json.Encode.Value) -> (t -> Json.Encode.Value) -> (Result e t -> Json.Encode.Value)
resultEncoder errEncoder okEncoder enum =
    case enum of
        Ok inner ->
            Json.Encode.object [ ( "Ok", okEncoder inner ) ]

        Err inner ->
            Json.Encode.object [ ( "Err", errEncoder inner ) ]


resultDecoder : Json.Decode.Decoder e -> Json.Decode.Decoder t -> Json.Decode.Decoder (Result e t)
resultDecoder errDecoder okDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map Ok (Json.Decode.field "Ok" okDecoder)
        , Json.Decode.map Err (Json.Decode.field "Err" errDecoder)
        ]


type ZkNoteId
    = Zni String


zkNoteIdEncoder : ZkNoteId -> Json.Encode.Value
zkNoteIdEncoder enum =
    case enum of
        Zni inner ->
            Json.Encode.object [ ( "Zni", Json.Encode.string inner ) ]


type alias ExtraLoginData =
    { userid : UserId
    , zknote : ZkNoteId
    , homenote : Maybe ZkNoteId
    }


extraLoginDataEncoder : ExtraLoginData -> Json.Encode.Value
extraLoginDataEncoder struct =
    Json.Encode.object
        [ ( "userid", userIdEncoder struct.userid )
        , ( "zknote", zkNoteIdEncoder struct.zknote )
        , ( "homenote", (Maybe.withDefault Json.Encode.null << Maybe.map zkNoteIdEncoder) struct.homenote )
        ]


type alias ZkNote =
    { id : ZkNoteId
    , title : String
    , content : String
    , user : UserId
    , username : String
    , usernote : ZkNoteId
    , editable : Bool
    , editableValue : Bool
    , showtitle : Bool
    , pubid : Maybe String
    , createdate : Int
    , changeddate : Int
    , deleted : Bool
    , filestatus : FileStatus
    , sysids : List ZkNoteId
    }


zkNoteEncoder : ZkNote -> Json.Encode.Value
zkNoteEncoder struct =
    Json.Encode.object
        [ ( "id", zkNoteIdEncoder struct.id )
        , ( "title", Json.Encode.string struct.title )
        , ( "content", Json.Encode.string struct.content )
        , ( "user", userIdEncoder struct.user )
        , ( "username", Json.Encode.string struct.username )
        , ( "usernote", zkNoteIdEncoder struct.usernote )
        , ( "editable", Json.Encode.bool struct.editable )
        , ( "editableValue", Json.Encode.bool struct.editableValue )
        , ( "showtitle", Json.Encode.bool struct.showtitle )
        , ( "pubid", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.pubid )
        , ( "createdate", Json.Encode.int struct.createdate )
        , ( "changeddate", Json.Encode.int struct.changeddate )
        , ( "deleted", Json.Encode.bool struct.deleted )
        , ( "filestatus", fileStatusEncoder struct.filestatus )
        , ( "sysids", Json.Encode.list zkNoteIdEncoder struct.sysids )
        ]


type FileStatus
    = NotAFile
    | FileMissing
    | FilePresent


fileStatusEncoder : FileStatus -> Json.Encode.Value
fileStatusEncoder enum =
    case enum of
        NotAFile ->
            Json.Encode.string "NotAFile"

        FileMissing ->
            Json.Encode.string "FileMissing"

        FilePresent ->
            Json.Encode.string "FilePresent"


type alias ZkListNote =
    { id : ZkNoteId
    , title : String
    , filestatus : FileStatus
    , user : UserId
    , createdate : Int
    , changeddate : Int
    , sysids : List ZkNoteId
    }


zkListNoteEncoder : ZkListNote -> Json.Encode.Value
zkListNoteEncoder struct =
    Json.Encode.object
        [ ( "id", zkNoteIdEncoder struct.id )
        , ( "title", Json.Encode.string struct.title )
        , ( "filestatus", fileStatusEncoder struct.filestatus )
        , ( "user", userIdEncoder struct.user )
        , ( "createdate", Json.Encode.int struct.createdate )
        , ( "changeddate", Json.Encode.int struct.changeddate )
        , ( "sysids", Json.Encode.list zkNoteIdEncoder struct.sysids )
        ]


type alias SavedZkNote =
    { id : ZkNoteId
    , changeddate : Int
    }


savedZkNoteEncoder : SavedZkNote -> Json.Encode.Value
savedZkNoteEncoder struct =
    Json.Encode.object
        [ ( "id", zkNoteIdEncoder struct.id )
        , ( "changeddate", Json.Encode.int struct.changeddate )
        ]


type alias SaveZkNote =
    { id : Maybe ZkNoteId
    , title : String
    , pubid : Maybe String
    , content : String
    , editable : Bool
    , showtitle : Bool
    , deleted : Bool
    }


saveZkNoteEncoder : SaveZkNote -> Json.Encode.Value
saveZkNoteEncoder struct =
    Json.Encode.object
        [ ( "id", (Maybe.withDefault Json.Encode.null << Maybe.map zkNoteIdEncoder) struct.id )
        , ( "title", Json.Encode.string struct.title )
        , ( "pubid", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.pubid )
        , ( "content", Json.Encode.string struct.content )
        , ( "editable", Json.Encode.bool struct.editable )
        , ( "showtitle", Json.Encode.bool struct.showtitle )
        , ( "deleted", Json.Encode.bool struct.deleted )
        ]


type Direction
    = From
    | To


directionEncoder : Direction -> Json.Encode.Value
directionEncoder enum =
    case enum of
        From ->
            Json.Encode.string "From"

        To ->
            Json.Encode.string "To"


type alias SaveZkLink =
    { otherid : ZkNoteId
    , direction : Direction
    , user : UserId
    , zknote : Maybe ZkNoteId
    , delete : Maybe Bool
    }


saveZkLinkEncoder : SaveZkLink -> Json.Encode.Value
saveZkLinkEncoder struct =
    Json.Encode.object
        [ ( "otherid", zkNoteIdEncoder struct.otherid )
        , ( "direction", directionEncoder struct.direction )
        , ( "user", userIdEncoder struct.user )
        , ( "zknote", (Maybe.withDefault Json.Encode.null << Maybe.map zkNoteIdEncoder) struct.zknote )
        , ( "delete", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.bool) struct.delete )
        ]


type alias SaveZkNoteAndLinks =
    { note : SaveZkNote
    , links : List SaveZkLink
    }


saveZkNoteAndLinksEncoder : SaveZkNoteAndLinks -> Json.Encode.Value
saveZkNoteAndLinksEncoder struct =
    Json.Encode.object
        [ ( "note", saveZkNoteEncoder struct.note )
        , ( "links", Json.Encode.list saveZkLinkEncoder struct.links )
        ]


type alias ZkLink =
    { from : ZkNoteId
    , to : ZkNoteId
    , user : UserId
    , linkzknote : Maybe ZkNoteId
    , delete : Maybe Bool
    , fromname : Maybe String
    , toname : Maybe String
    }


zkLinkEncoder : ZkLink -> Json.Encode.Value
zkLinkEncoder struct =
    Json.Encode.object
        [ ( "from", zkNoteIdEncoder struct.from )
        , ( "to", zkNoteIdEncoder struct.to )
        , ( "user", userIdEncoder struct.user )
        , ( "linkzknote", (Maybe.withDefault Json.Encode.null << Maybe.map zkNoteIdEncoder) struct.linkzknote )
        , ( "delete", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.bool) struct.delete )
        , ( "fromname", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.fromname )
        , ( "toname", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.toname )
        ]


type alias EditLink =
    { otherid : ZkNoteId
    , direction : Direction
    , user : UserId
    , zknote : Maybe ZkNoteId
    , othername : Maybe String
    , delete : Maybe Bool
    , sysids : List ZkNoteId
    }


editLinkEncoder : EditLink -> Json.Encode.Value
editLinkEncoder struct =
    Json.Encode.object
        [ ( "otherid", zkNoteIdEncoder struct.otherid )
        , ( "direction", directionEncoder struct.direction )
        , ( "user", userIdEncoder struct.user )
        , ( "zknote", (Maybe.withDefault Json.Encode.null << Maybe.map zkNoteIdEncoder) struct.zknote )
        , ( "othername", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.othername )
        , ( "delete", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.bool) struct.delete )
        , ( "sysids", Json.Encode.list zkNoteIdEncoder struct.sysids )
        ]


type alias ZkLinks =
    { links : List ZkLink
    }


zkLinksEncoder : ZkLinks -> Json.Encode.Value
zkLinksEncoder struct =
    Json.Encode.object
        [ ( "links", Json.Encode.list zkLinkEncoder struct.links )
        ]


type alias ImportZkNote =
    { title : String
    , content : String
    , fromLinks : List String
    , toLinks : List String
    }


importZkNoteEncoder : ImportZkNote -> Json.Encode.Value
importZkNoteEncoder struct =
    Json.Encode.object
        [ ( "title", Json.Encode.string struct.title )
        , ( "content", Json.Encode.string struct.content )
        , ( "fromLinks", Json.Encode.list Json.Encode.string struct.fromLinks )
        , ( "toLinks", Json.Encode.list Json.Encode.string struct.toLinks )
        ]


type alias GetZkLinks =
    { zknote : ZkNoteId
    }


getZkLinksEncoder : GetZkLinks -> Json.Encode.Value
getZkLinksEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteIdEncoder struct.zknote )
        ]


type alias GetZkNoteAndLinks =
    { zknote : ZkNoteId
    , what : String
    }


getZkNoteAndLinksEncoder : GetZkNoteAndLinks -> Json.Encode.Value
getZkNoteAndLinksEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteIdEncoder struct.zknote )
        , ( "what", Json.Encode.string struct.what )
        ]


type alias GetZnlIfChanged =
    { zknote : ZkNoteId
    , changeddate : Int
    , what : String
    }


getZnlIfChangedEncoder : GetZnlIfChanged -> Json.Encode.Value
getZnlIfChangedEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteIdEncoder struct.zknote )
        , ( "changeddate", Json.Encode.int struct.changeddate )
        , ( "what", Json.Encode.string struct.what )
        ]


type alias GetZkNoteArchives =
    { zknote : ZkNoteId
    , offset : Int
    , limit : Maybe Int
    }


getZkNoteArchivesEncoder : GetZkNoteArchives -> Json.Encode.Value
getZkNoteArchivesEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteIdEncoder struct.zknote )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "limit", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.int) struct.limit )
        ]


type alias ZkNoteArchives =
    { zknote : ZkNoteId
    , results : ZkListNoteSearchResult
    }


zkNoteArchivesEncoder : ZkNoteArchives -> Json.Encode.Value
zkNoteArchivesEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteIdEncoder struct.zknote )
        , ( "results", zkListNoteSearchResultEncoder struct.results )
        ]


type alias GetArchiveZkNote =
    { parentnote : ZkNoteId
    , noteid : ZkNoteId
    }


getArchiveZkNoteEncoder : GetArchiveZkNote -> Json.Encode.Value
getArchiveZkNoteEncoder struct =
    Json.Encode.object
        [ ( "parentnote", zkNoteIdEncoder struct.parentnote )
        , ( "noteid", zkNoteIdEncoder struct.noteid )
        ]


type alias GetArchiveZkLinks =
    { createddateAfter : Maybe Int
    }


getArchiveZkLinksEncoder : GetArchiveZkLinks -> Json.Encode.Value
getArchiveZkLinksEncoder struct =
    Json.Encode.object
        [ ( "createddate_after", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.int) struct.createddateAfter )
        ]


type alias GetZkLinksSince =
    { createddateAfter : Maybe Int
    }


getZkLinksSinceEncoder : GetZkLinksSince -> Json.Encode.Value
getZkLinksSinceEncoder struct =
    Json.Encode.object
        [ ( "createddate_after", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.int) struct.createddateAfter )
        ]


type alias FileInfo =
    { hash : String
    , size : Int
    }


fileInfoEncoder : FileInfo -> Json.Encode.Value
fileInfoEncoder struct =
    Json.Encode.object
        [ ( "hash", Json.Encode.string struct.hash )
        , ( "size", Json.Encode.int struct.size )
        ]


type alias ArchiveZkLink =
    { userUuid : String
    , fromUuid : String
    , toUuid : String
    , linkUuid : Maybe String
    , createdate : Int
    , deletedate : Int
    }


archiveZkLinkEncoder : ArchiveZkLink -> Json.Encode.Value
archiveZkLinkEncoder struct =
    Json.Encode.object
        [ ( "userUuid", Json.Encode.string struct.userUuid )
        , ( "fromUuid", Json.Encode.string struct.fromUuid )
        , ( "toUuid", Json.Encode.string struct.toUuid )
        , ( "linkUuid", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.linkUuid )
        , ( "createdate", Json.Encode.int struct.createdate )
        , ( "deletedate", Json.Encode.int struct.deletedate )
        ]


type alias UuidZkLink =
    { userUuid : String
    , fromUuid : String
    , toUuid : String
    , linkUuid : Maybe String
    , createdate : Int
    }


uuidZkLinkEncoder : UuidZkLink -> Json.Encode.Value
uuidZkLinkEncoder struct =
    Json.Encode.object
        [ ( "userUuid", Json.Encode.string struct.userUuid )
        , ( "fromUuid", Json.Encode.string struct.fromUuid )
        , ( "toUuid", Json.Encode.string struct.toUuid )
        , ( "linkUuid", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.linkUuid )
        , ( "createdate", Json.Encode.int struct.createdate )
        ]


type alias GetZkNoteComments =
    { zknote : ZkNoteId
    , offset : Int
    , limit : Maybe Int
    }


getZkNoteCommentsEncoder : GetZkNoteComments -> Json.Encode.Value
getZkNoteCommentsEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteIdEncoder struct.zknote )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "limit", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.int) struct.limit )
        ]


type alias ZkNoteAndLinks =
    { zknote : ZkNote
    , links : List EditLink
    }


zkNoteAndLinksEncoder : ZkNoteAndLinks -> Json.Encode.Value
zkNoteAndLinksEncoder struct =
    Json.Encode.object
        [ ( "zknote", zkNoteEncoder struct.zknote )
        , ( "links", Json.Encode.list editLinkEncoder struct.links )
        ]


type alias ZkNoteAndLinksWhat =
    { what : String
    , znl : ZkNoteAndLinks
    }


zkNoteAndLinksWhatEncoder : ZkNoteAndLinksWhat -> Json.Encode.Value
zkNoteAndLinksWhatEncoder struct =
    Json.Encode.object
        [ ( "what", Json.Encode.string struct.what )
        , ( "znl", zkNoteAndLinksEncoder struct.znl )
        ]


type JobState
    = Started
    | Running
    | Completed
    | Failed


jobStateEncoder : JobState -> Json.Encode.Value
jobStateEncoder enum =
    case enum of
        Started ->
            Json.Encode.string "Started"

        Running ->
            Json.Encode.string "Running"

        Completed ->
            Json.Encode.string "Completed"

        Failed ->
            Json.Encode.string "Failed"


type alias JobStatus =
    { jobno : Int
    , state : JobState
    , message : String
    }


jobStatusEncoder : JobStatus -> Json.Encode.Value
jobStatusEncoder struct =
    Json.Encode.object
        [ ( "jobno", Json.Encode.int struct.jobno )
        , ( "state", jobStateEncoder struct.state )
        , ( "message", Json.Encode.string struct.message )
        ]


type PublicRequest
    = PbrGetZkNoteAndLinks GetZkNoteAndLinks
    | PbrGetZnlIfChanged GetZnlIfChanged
    | PbrGetZkNotePubId String


publicRequestEncoder : PublicRequest -> Json.Encode.Value
publicRequestEncoder enum =
    case enum of
        PbrGetZkNoteAndLinks inner ->
            Json.Encode.object [ ( "PbrGetZkNoteAndLinks", getZkNoteAndLinksEncoder inner ) ]

        PbrGetZnlIfChanged inner ->
            Json.Encode.object [ ( "PbrGetZnlIfChanged", getZnlIfChangedEncoder inner ) ]

        PbrGetZkNotePubId inner ->
            Json.Encode.object [ ( "PbrGetZkNotePubId", Json.Encode.string inner ) ]


type PublicReply
    = PbyServerError PublicError
    | PbyZkNoteAndLinks ZkNoteAndLinks
    | PbyZkNoteAndLinksWhat ZkNoteAndLinksWhat
    | PbyNoop


publicReplyEncoder : PublicReply -> Json.Encode.Value
publicReplyEncoder enum =
    case enum of
        PbyServerError inner ->
            Json.Encode.object [ ( "PbyServerError", publicErrorEncoder inner ) ]

        PbyZkNoteAndLinks inner ->
            Json.Encode.object [ ( "PbyZkNoteAndLinks", zkNoteAndLinksEncoder inner ) ]

        PbyZkNoteAndLinksWhat inner ->
            Json.Encode.object [ ( "PbyZkNoteAndLinksWhat", zkNoteAndLinksWhatEncoder inner ) ]

        PbyNoop ->
            Json.Encode.string "PbyNoop"


type PublicError
    = PbeString String
    | PbeNoteNotFound PublicRequest
    | PbeNoteIsPrivate PublicRequest


publicErrorEncoder : PublicError -> Json.Encode.Value
publicErrorEncoder enum =
    case enum of
        PbeString inner ->
            Json.Encode.object [ ( "PbeString", Json.Encode.string inner ) ]

        PbeNoteNotFound inner ->
            Json.Encode.object [ ( "PbeNoteNotFound", publicRequestEncoder inner ) ]

        PbeNoteIsPrivate inner ->
            Json.Encode.object [ ( "PbeNoteIsPrivate", publicRequestEncoder inner ) ]


type PrivateRequest
    = PvqGetZkNote ZkNoteId
    | PvqGetZkNoteAndLinks GetZkNoteAndLinks
    | PvqGetZnlIfChanged GetZnlIfChanged
    | PvqGetZkNoteComments GetZkNoteComments
    | PvqGetZkNoteArchives GetZkNoteArchives
    | PvqGetArchiveZkNote GetArchiveZkNote
    | PvqGetArchiveZklinks GetArchiveZkLinks
    | PvqGetZkLinksSince GetZkLinksSince
    | PvqSearchZkNotes ZkNoteSearch
    | PvqPowerDelete (List TagSearch)
    | PvqDeleteZkNote ZkNoteId
    | PvqSaveZkNote SaveZkNote
    | PvqSaveZkLinks ZkLinks
    | PvqSaveZkNoteAndLinks SaveZkNoteAndLinks
    | PvqSaveImportZkNotes (List ImportZkNote)
    | PvqSetHomeNote ZkNoteId
    | PvqSyncRemote
    | PvqSyncFiles ZkNoteSearch
    | PvqGetJobStatus Int


privateRequestEncoder : PrivateRequest -> Json.Encode.Value
privateRequestEncoder enum =
    case enum of
        PvqGetZkNote inner ->
            Json.Encode.object [ ( "PvqGetZkNote", zkNoteIdEncoder inner ) ]

        PvqGetZkNoteAndLinks inner ->
            Json.Encode.object [ ( "PvqGetZkNoteAndLinks", getZkNoteAndLinksEncoder inner ) ]

        PvqGetZnlIfChanged inner ->
            Json.Encode.object [ ( "PvqGetZnlIfChanged", getZnlIfChangedEncoder inner ) ]

        PvqGetZkNoteComments inner ->
            Json.Encode.object [ ( "PvqGetZkNoteComments", getZkNoteCommentsEncoder inner ) ]

        PvqGetZkNoteArchives inner ->
            Json.Encode.object [ ( "PvqGetZkNoteArchives", getZkNoteArchivesEncoder inner ) ]

        PvqGetArchiveZkNote inner ->
            Json.Encode.object [ ( "PvqGetArchiveZkNote", getArchiveZkNoteEncoder inner ) ]

        PvqGetArchiveZklinks inner ->
            Json.Encode.object [ ( "PvqGetArchiveZklinks", getArchiveZkLinksEncoder inner ) ]

        PvqGetZkLinksSince inner ->
            Json.Encode.object [ ( "PvqGetZkLinksSince", getZkLinksSinceEncoder inner ) ]

        PvqSearchZkNotes inner ->
            Json.Encode.object [ ( "PvqSearchZkNotes", zkNoteSearchEncoder inner ) ]

        PvqPowerDelete inner ->
            Json.Encode.object [ ( "PvqPowerDelete", Json.Encode.list tagSearchEncoder inner ) ]

        PvqDeleteZkNote inner ->
            Json.Encode.object [ ( "PvqDeleteZkNote", zkNoteIdEncoder inner ) ]

        PvqSaveZkNote inner ->
            Json.Encode.object [ ( "PvqSaveZkNote", saveZkNoteEncoder inner ) ]

        PvqSaveZkLinks inner ->
            Json.Encode.object [ ( "PvqSaveZkLinks", zkLinksEncoder inner ) ]

        PvqSaveZkNoteAndLinks inner ->
            Json.Encode.object [ ( "PvqSaveZkNoteAndLinks", saveZkNoteAndLinksEncoder inner ) ]

        PvqSaveImportZkNotes inner ->
            Json.Encode.object [ ( "PvqSaveImportZkNotes", Json.Encode.list importZkNoteEncoder inner ) ]

        PvqSetHomeNote inner ->
            Json.Encode.object [ ( "PvqSetHomeNote", zkNoteIdEncoder inner ) ]

        PvqSyncRemote ->
            Json.Encode.string "PvqSyncRemote"

        PvqSyncFiles inner ->
            Json.Encode.object [ ( "PvqSyncFiles", zkNoteSearchEncoder inner ) ]

        PvqGetJobStatus inner ->
            Json.Encode.object [ ( "PvqGetJobStatus", Json.Encode.int inner ) ]


type PrivateReply
    = PvyServerError PrivateError
    | PvyZkNote ZkNote
    | PvyZkNoteAndLinksWhat ZkNoteAndLinksWhat
    | PvyNoop
    | PvyZkNoteComments (List ZkNote)
    | PvyArchives (List ZkListNote)
    | PvyZkNoteArchives ZkNoteArchives
    | PvyArchiveZkLinks (List ArchiveZkLink)
    | PvyZkLinks (List UuidZkLink)
    | PvyZkListNoteSearchResult ZkListNoteSearchResult
    | PvyZkNoteSearchResult ZkNoteSearchResult
    | PvyZkNoteIdSearchResult ZkIdSearchResult
    | PvyZkNoteAndLinksSearchResult ZkNoteAndLinksSearchResult
    | PvyPowerDeleteComplete Int
    | PvyDeletedZkNote ZkNoteId
    | PvySavedZkNote SavedZkNote
    | PvySavedZkLinks
    | PvySavedZkNoteAndLinks SavedZkNote
    | PvySavedImportZkNotes
    | PvyHomeNoteSet ZkNoteId
    | PvyJobStatus JobStatus
    | PvyJobNotFound Int
    | PvyFileSyncComplete
    | PvySyncComplete


privateReplyEncoder : PrivateReply -> Json.Encode.Value
privateReplyEncoder enum =
    case enum of
        PvyServerError inner ->
            Json.Encode.object [ ( "PvyServerError", privateErrorEncoder inner ) ]

        PvyZkNote inner ->
            Json.Encode.object [ ( "PvyZkNote", zkNoteEncoder inner ) ]

        PvyZkNoteAndLinksWhat inner ->
            Json.Encode.object [ ( "PvyZkNoteAndLinksWhat", zkNoteAndLinksWhatEncoder inner ) ]

        PvyNoop ->
            Json.Encode.string "PvyNoop"

        PvyZkNoteComments inner ->
            Json.Encode.object [ ( "PvyZkNoteComments", Json.Encode.list zkNoteEncoder inner ) ]

        PvyArchives inner ->
            Json.Encode.object [ ( "PvyArchives", Json.Encode.list zkListNoteEncoder inner ) ]

        PvyZkNoteArchives inner ->
            Json.Encode.object [ ( "PvyZkNoteArchives", zkNoteArchivesEncoder inner ) ]

        PvyArchiveZkLinks inner ->
            Json.Encode.object [ ( "PvyArchiveZkLinks", Json.Encode.list archiveZkLinkEncoder inner ) ]

        PvyZkLinks inner ->
            Json.Encode.object [ ( "PvyZkLinks", Json.Encode.list uuidZkLinkEncoder inner ) ]

        PvyZkListNoteSearchResult inner ->
            Json.Encode.object [ ( "PvyZkListNoteSearchResult", zkListNoteSearchResultEncoder inner ) ]

        PvyZkNoteSearchResult inner ->
            Json.Encode.object [ ( "PvyZkNoteSearchResult", zkNoteSearchResultEncoder inner ) ]

        PvyZkNoteIdSearchResult inner ->
            Json.Encode.object [ ( "PvyZkNoteIdSearchResult", zkIdSearchResultEncoder inner ) ]

        PvyZkNoteAndLinksSearchResult inner ->
            Json.Encode.object [ ( "PvyZkNoteAndLinksSearchResult", zkNoteAndLinksSearchResultEncoder inner ) ]

        PvyPowerDeleteComplete inner ->
            Json.Encode.object [ ( "PvyPowerDeleteComplete", Json.Encode.int inner ) ]

        PvyDeletedZkNote inner ->
            Json.Encode.object [ ( "PvyDeletedZkNote", zkNoteIdEncoder inner ) ]

        PvySavedZkNote inner ->
            Json.Encode.object [ ( "PvySavedZkNote", savedZkNoteEncoder inner ) ]

        PvySavedZkLinks ->
            Json.Encode.string "PvySavedZkLinks"

        PvySavedZkNoteAndLinks inner ->
            Json.Encode.object [ ( "PvySavedZkNoteAndLinks", savedZkNoteEncoder inner ) ]

        PvySavedImportZkNotes ->
            Json.Encode.string "PvySavedImportZkNotes"

        PvyHomeNoteSet inner ->
            Json.Encode.object [ ( "PvyHomeNoteSet", zkNoteIdEncoder inner ) ]

        PvyJobStatus inner ->
            Json.Encode.object [ ( "PvyJobStatus", jobStatusEncoder inner ) ]

        PvyJobNotFound inner ->
            Json.Encode.object [ ( "PvyJobNotFound", Json.Encode.int inner ) ]

        PvyFileSyncComplete ->
            Json.Encode.string "PvyFileSyncComplete"

        PvySyncComplete ->
            Json.Encode.string "PvySyncComplete"


type PrivateError
    = PveString String
    | PveNoteNotFound ZkNoteRq
    | PveNoteIsPrivate ZkNoteRq
    | PveNotLoggedIn
    | PveLoginError String


privateErrorEncoder : PrivateError -> Json.Encode.Value
privateErrorEncoder enum =
    case enum of
        PveString inner ->
            Json.Encode.object [ ( "PveString", Json.Encode.string inner ) ]

        PveNoteNotFound inner ->
            Json.Encode.object [ ( "PveNoteNotFound", zkNoteRqEncoder inner ) ]

        PveNoteIsPrivate inner ->
            Json.Encode.object [ ( "PveNoteIsPrivate", zkNoteRqEncoder inner ) ]

        PveNotLoggedIn ->
            Json.Encode.string "PveNotLoggedIn"

        PveLoginError inner ->
            Json.Encode.object [ ( "PveLoginError", Json.Encode.string inner ) ]


type alias ZkNoteRq =
    { zknoteid : ZkNoteId
    , what : Maybe String
    }


zkNoteRqEncoder : ZkNoteRq -> Json.Encode.Value
zkNoteRqEncoder struct =
    Json.Encode.object
        [ ( "zknoteid", zkNoteIdEncoder struct.zknoteid )
        , ( "what", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.string) struct.what )
        ]


type UploadReply
    = UrFilesUploaded (List ZkListNote)


uploadReplyEncoder : UploadReply -> Json.Encode.Value
uploadReplyEncoder enum =
    case enum of
        UrFilesUploaded inner ->
            Json.Encode.object [ ( "UrFilesUploaded", Json.Encode.list zkListNoteEncoder inner ) ]


type alias ZkNoteSearch =
    { tagsearch : List TagSearch
    , offset : Int
    , limit : Maybe Int
    , what : String
    , resulttype : ResultType
    , archives : Bool
    , deleted : Bool
    , ordering : Maybe Ordering
    }


zkNoteSearchEncoder : ZkNoteSearch -> Json.Encode.Value
zkNoteSearchEncoder struct =
    Json.Encode.object
        [ ( "tagsearch", Json.Encode.list tagSearchEncoder struct.tagsearch )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "limit", (Maybe.withDefault Json.Encode.null << Maybe.map Json.Encode.int) struct.limit )
        , ( "what", Json.Encode.string struct.what )
        , ( "resulttype", resultTypeEncoder struct.resulttype )
        , ( "archives", Json.Encode.bool struct.archives )
        , ( "deleted", Json.Encode.bool struct.deleted )
        , ( "ordering", (Maybe.withDefault Json.Encode.null << Maybe.map orderingEncoder) struct.ordering )
        ]


type alias Ordering =
    { field : OrderField
    , direction : OrderDirection
    }


orderingEncoder : Ordering -> Json.Encode.Value
orderingEncoder struct =
    Json.Encode.object
        [ ( "field", orderFieldEncoder struct.field )
        , ( "direction", orderDirectionEncoder struct.direction )
        ]


type OrderDirection
    = Ascending
    | Descending


orderDirectionEncoder : OrderDirection -> Json.Encode.Value
orderDirectionEncoder enum =
    case enum of
        Ascending ->
            Json.Encode.string "Ascending"

        Descending ->
            Json.Encode.string "Descending"


type OrderField
    = Title
    | Created
    | Changed


orderFieldEncoder : OrderField -> Json.Encode.Value
orderFieldEncoder enum =
    case enum of
        Title ->
            Json.Encode.string "Title"

        Created ->
            Json.Encode.string "Created"

        Changed ->
            Json.Encode.string "Changed"


type ResultType
    = RtId
    | RtListNote
    | RtNote
    | RtNoteAndLinks


resultTypeEncoder : ResultType -> Json.Encode.Value
resultTypeEncoder enum =
    case enum of
        RtId ->
            Json.Encode.string "RtId"

        RtListNote ->
            Json.Encode.string "RtListNote"

        RtNote ->
            Json.Encode.string "RtNote"

        RtNoteAndLinks ->
            Json.Encode.string "RtNoteAndLinks"


type TagSearch
    = SearchTerm { mods : List SearchMod, term : String }
    | Not { ts : TagSearch }
    | Boolex { ts1 : TagSearch, ao : AndOr, ts2 : TagSearch }


tagSearchEncoder : TagSearch -> Json.Encode.Value
tagSearchEncoder enum =
    case enum of
        SearchTerm { mods, term } ->
            Json.Encode.object [ ( "SearchTerm", Json.Encode.object [ ( "mods", Json.Encode.list searchModEncoder mods ), ( "term", Json.Encode.string term ) ] ) ]

        Not { ts } ->
            Json.Encode.object [ ( "Not", Json.Encode.object [ ( "ts", tagSearchEncoder ts ) ] ) ]

        Boolex { ts1, ao, ts2 } ->
            Json.Encode.object [ ( "Boolex", Json.Encode.object [ ( "ts1", tagSearchEncoder ts1 ), ( "ao", andOrEncoder ao ), ( "ts2", tagSearchEncoder ts2 ) ] ) ]


type SearchMod
    = ExactMatch
    | ZkNoteId
    | Tag
    | Note
    | User
    | File
    | Before
    | After
    | Create
    | Mod


searchModEncoder : SearchMod -> Json.Encode.Value
searchModEncoder enum =
    case enum of
        ExactMatch ->
            Json.Encode.string "ExactMatch"

        ZkNoteId ->
            Json.Encode.string "ZkNoteId"

        Tag ->
            Json.Encode.string "Tag"

        Note ->
            Json.Encode.string "Note"

        User ->
            Json.Encode.string "User"

        File ->
            Json.Encode.string "File"

        Before ->
            Json.Encode.string "Before"

        After ->
            Json.Encode.string "After"

        Create ->
            Json.Encode.string "Create"

        Mod ->
            Json.Encode.string "Mod"


type AndOr
    = And
    | Or


andOrEncoder : AndOr -> Json.Encode.Value
andOrEncoder enum =
    case enum of
        And ->
            Json.Encode.string "And"

        Or ->
            Json.Encode.string "Or"


type alias ZkIdSearchResult =
    { notes : List ZkNoteId
    , offset : Int
    , what : String
    }


zkIdSearchResultEncoder : ZkIdSearchResult -> Json.Encode.Value
zkIdSearchResultEncoder struct =
    Json.Encode.object
        [ ( "notes", Json.Encode.list zkNoteIdEncoder struct.notes )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "what", Json.Encode.string struct.what )
        ]


type alias ZkListNoteSearchResult =
    { notes : List ZkListNote
    , offset : Int
    , what : String
    }


zkListNoteSearchResultEncoder : ZkListNoteSearchResult -> Json.Encode.Value
zkListNoteSearchResultEncoder struct =
    Json.Encode.object
        [ ( "notes", Json.Encode.list zkListNoteEncoder struct.notes )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "what", Json.Encode.string struct.what )
        ]


type alias ZkNoteSearchResult =
    { notes : List ZkNote
    , offset : Int
    , what : String
    }


zkNoteSearchResultEncoder : ZkNoteSearchResult -> Json.Encode.Value
zkNoteSearchResultEncoder struct =
    Json.Encode.object
        [ ( "notes", Json.Encode.list zkNoteEncoder struct.notes )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "what", Json.Encode.string struct.what )
        ]


type alias ZkSearchResultHeader =
    { what : String
    , resulttype : ResultType
    , offset : Int
    }


zkSearchResultHeaderEncoder : ZkSearchResultHeader -> Json.Encode.Value
zkSearchResultHeaderEncoder struct =
    Json.Encode.object
        [ ( "what", Json.Encode.string struct.what )
        , ( "resulttype", resultTypeEncoder struct.resulttype )
        , ( "offset", Json.Encode.int struct.offset )
        ]


type alias ZkNoteAndLinksSearchResult =
    { notes : List ZkNoteAndLinks
    , offset : Int
    , what : String
    }


zkNoteAndLinksSearchResultEncoder : ZkNoteAndLinksSearchResult -> Json.Encode.Value
zkNoteAndLinksSearchResultEncoder struct =
    Json.Encode.object
        [ ( "notes", Json.Encode.list zkNoteAndLinksEncoder struct.notes )
        , ( "offset", Json.Encode.int struct.offset )
        , ( "what", Json.Encode.string struct.what )
        ]


zkNoteIdDecoder : Json.Decode.Decoder ZkNoteId
zkNoteIdDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map Zni (Json.Decode.field "Zni" Json.Decode.string)
        ]


extraLoginDataDecoder : Json.Decode.Decoder ExtraLoginData
extraLoginDataDecoder =
    Json.Decode.succeed ExtraLoginData
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "userid" userIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "homenote" (Json.Decode.nullable zkNoteIdDecoder)))


zkNoteDecoder : Json.Decode.Decoder ZkNote
zkNoteDecoder =
    Json.Decode.succeed ZkNote
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "id" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "title" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "content" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "user" userIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "username" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "usernote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "editable" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "editableValue" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "showtitle" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "pubid" (Json.Decode.nullable Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "createdate" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "changeddate" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "deleted" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "filestatus" fileStatusDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "sysids" (Json.Decode.list zkNoteIdDecoder)))


fileStatusDecoder : Json.Decode.Decoder FileStatus
fileStatusDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "NotAFile" ->
                            Json.Decode.succeed NotAFile

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "FileMissing" ->
                            Json.Decode.succeed FileMissing

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "FilePresent" ->
                            Json.Decode.succeed FilePresent

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


zkListNoteDecoder : Json.Decode.Decoder ZkListNote
zkListNoteDecoder =
    Json.Decode.succeed ZkListNote
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "id" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "title" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "filestatus" fileStatusDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "user" userIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "createdate" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "changeddate" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "sysids" (Json.Decode.list zkNoteIdDecoder)))


savedZkNoteDecoder : Json.Decode.Decoder SavedZkNote
savedZkNoteDecoder =
    Json.Decode.succeed SavedZkNote
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "id" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "changeddate" Json.Decode.int))


saveZkNoteDecoder : Json.Decode.Decoder SaveZkNote
saveZkNoteDecoder =
    Json.Decode.succeed SaveZkNote
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "id" (Json.Decode.nullable zkNoteIdDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "title" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "pubid" (Json.Decode.nullable Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "content" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "editable" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "showtitle" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "deleted" Json.Decode.bool))


directionDecoder : Json.Decode.Decoder Direction
directionDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "From" ->
                            Json.Decode.succeed From

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "To" ->
                            Json.Decode.succeed To

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


saveZkLinkDecoder : Json.Decode.Decoder SaveZkLink
saveZkLinkDecoder =
    Json.Decode.succeed SaveZkLink
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "otherid" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "direction" directionDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "user" userIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" (Json.Decode.nullable zkNoteIdDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "delete" (Json.Decode.nullable Json.Decode.bool)))


saveZkNoteAndLinksDecoder : Json.Decode.Decoder SaveZkNoteAndLinks
saveZkNoteAndLinksDecoder =
    Json.Decode.succeed SaveZkNoteAndLinks
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "note" saveZkNoteDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "links" (Json.Decode.list saveZkLinkDecoder)))


zkLinkDecoder : Json.Decode.Decoder ZkLink
zkLinkDecoder =
    Json.Decode.succeed ZkLink
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "from" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "to" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "user" userIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "linkzknote" (Json.Decode.nullable zkNoteIdDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "delete" (Json.Decode.nullable Json.Decode.bool)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "fromname" (Json.Decode.nullable Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "toname" (Json.Decode.nullable Json.Decode.string)))


editLinkDecoder : Json.Decode.Decoder EditLink
editLinkDecoder =
    Json.Decode.succeed EditLink
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "otherid" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "direction" directionDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "user" userIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" (Json.Decode.nullable zkNoteIdDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "othername" (Json.Decode.nullable Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "delete" (Json.Decode.nullable Json.Decode.bool)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "sysids" (Json.Decode.list zkNoteIdDecoder)))


zkLinksDecoder : Json.Decode.Decoder ZkLinks
zkLinksDecoder =
    Json.Decode.succeed ZkLinks
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "links" (Json.Decode.list zkLinkDecoder)))


importZkNoteDecoder : Json.Decode.Decoder ImportZkNote
importZkNoteDecoder =
    Json.Decode.succeed ImportZkNote
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "title" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "content" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "fromLinks" (Json.Decode.list Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "toLinks" (Json.Decode.list Json.Decode.string)))


getZkLinksDecoder : Json.Decode.Decoder GetZkLinks
getZkLinksDecoder =
    Json.Decode.succeed GetZkLinks
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))


getZkNoteAndLinksDecoder : Json.Decode.Decoder GetZkNoteAndLinks
getZkNoteAndLinksDecoder =
    Json.Decode.succeed GetZkNoteAndLinks
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))


getZnlIfChangedDecoder : Json.Decode.Decoder GetZnlIfChanged
getZnlIfChangedDecoder =
    Json.Decode.succeed GetZnlIfChanged
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "changeddate" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))


getZkNoteArchivesDecoder : Json.Decode.Decoder GetZkNoteArchives
getZkNoteArchivesDecoder =
    Json.Decode.succeed GetZkNoteArchives
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "limit" (Json.Decode.nullable Json.Decode.int)))


zkNoteArchivesDecoder : Json.Decode.Decoder ZkNoteArchives
zkNoteArchivesDecoder =
    Json.Decode.succeed ZkNoteArchives
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "results" zkListNoteSearchResultDecoder))


getArchiveZkNoteDecoder : Json.Decode.Decoder GetArchiveZkNote
getArchiveZkNoteDecoder =
    Json.Decode.succeed GetArchiveZkNote
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "parentnote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "noteid" zkNoteIdDecoder))


getArchiveZkLinksDecoder : Json.Decode.Decoder GetArchiveZkLinks
getArchiveZkLinksDecoder =
    Json.Decode.succeed GetArchiveZkLinks
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "createddate_after" (Json.Decode.nullable Json.Decode.int)))


getZkLinksSinceDecoder : Json.Decode.Decoder GetZkLinksSince
getZkLinksSinceDecoder =
    Json.Decode.succeed GetZkLinksSince
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "createddate_after" (Json.Decode.nullable Json.Decode.int)))


fileInfoDecoder : Json.Decode.Decoder FileInfo
fileInfoDecoder =
    Json.Decode.succeed FileInfo
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "hash" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "size" Json.Decode.int))


archiveZkLinkDecoder : Json.Decode.Decoder ArchiveZkLink
archiveZkLinkDecoder =
    Json.Decode.succeed ArchiveZkLink
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "userUuid" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "fromUuid" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "toUuid" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "linkUuid" (Json.Decode.nullable Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "createdate" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "deletedate" Json.Decode.int))


uuidZkLinkDecoder : Json.Decode.Decoder UuidZkLink
uuidZkLinkDecoder =
    Json.Decode.succeed UuidZkLink
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "userUuid" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "fromUuid" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "toUuid" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "linkUuid" (Json.Decode.nullable Json.Decode.string)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "createdate" Json.Decode.int))


getZkNoteCommentsDecoder : Json.Decode.Decoder GetZkNoteComments
getZkNoteCommentsDecoder =
    Json.Decode.succeed GetZkNoteComments
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "limit" (Json.Decode.nullable Json.Decode.int)))


zkNoteAndLinksDecoder : Json.Decode.Decoder ZkNoteAndLinks
zkNoteAndLinksDecoder =
    Json.Decode.succeed ZkNoteAndLinks
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknote" zkNoteDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "links" (Json.Decode.list editLinkDecoder)))


zkNoteAndLinksWhatDecoder : Json.Decode.Decoder ZkNoteAndLinksWhat
zkNoteAndLinksWhatDecoder =
    Json.Decode.succeed ZkNoteAndLinksWhat
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "znl" zkNoteAndLinksDecoder))


jobStateDecoder : Json.Decode.Decoder JobState
jobStateDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Started" ->
                            Json.Decode.succeed Started

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Running" ->
                            Json.Decode.succeed Running

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Completed" ->
                            Json.Decode.succeed Completed

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Failed" ->
                            Json.Decode.succeed Failed

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


jobStatusDecoder : Json.Decode.Decoder JobStatus
jobStatusDecoder =
    Json.Decode.succeed JobStatus
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "jobno" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "state" jobStateDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "message" Json.Decode.string))


publicRequestDecoder : Json.Decode.Decoder PublicRequest
publicRequestDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map PbrGetZkNoteAndLinks (Json.Decode.field "PbrGetZkNoteAndLinks" getZkNoteAndLinksDecoder)
        , Json.Decode.map PbrGetZnlIfChanged (Json.Decode.field "PbrGetZnlIfChanged" getZnlIfChangedDecoder)
        , Json.Decode.map PbrGetZkNotePubId (Json.Decode.field "PbrGetZkNotePubId" Json.Decode.string)
        ]


publicReplyDecoder : Json.Decode.Decoder PublicReply
publicReplyDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map PbyServerError (Json.Decode.field "PbyServerError" publicErrorDecoder)
        , Json.Decode.map PbyZkNoteAndLinks (Json.Decode.field "PbyZkNoteAndLinks" zkNoteAndLinksDecoder)
        , Json.Decode.map PbyZkNoteAndLinksWhat (Json.Decode.field "PbyZkNoteAndLinksWhat" zkNoteAndLinksWhatDecoder)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PbyNoop" ->
                            Json.Decode.succeed PbyNoop

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


publicErrorDecoder : Json.Decode.Decoder PublicError
publicErrorDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map PbeString (Json.Decode.field "PbeString" Json.Decode.string)
        , Json.Decode.map PbeNoteNotFound (Json.Decode.field "PbeNoteNotFound" publicRequestDecoder)
        , Json.Decode.map PbeNoteIsPrivate (Json.Decode.field "PbeNoteIsPrivate" publicRequestDecoder)
        ]


privateRequestDecoder : Json.Decode.Decoder PrivateRequest
privateRequestDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map PvqGetZkNote (Json.Decode.field "PvqGetZkNote" zkNoteIdDecoder)
        , Json.Decode.map PvqGetZkNoteAndLinks (Json.Decode.field "PvqGetZkNoteAndLinks" getZkNoteAndLinksDecoder)
        , Json.Decode.map PvqGetZnlIfChanged (Json.Decode.field "PvqGetZnlIfChanged" getZnlIfChangedDecoder)
        , Json.Decode.map PvqGetZkNoteComments (Json.Decode.field "PvqGetZkNoteComments" getZkNoteCommentsDecoder)
        , Json.Decode.map PvqGetZkNoteArchives (Json.Decode.field "PvqGetZkNoteArchives" getZkNoteArchivesDecoder)
        , Json.Decode.map PvqGetArchiveZkNote (Json.Decode.field "PvqGetArchiveZkNote" getArchiveZkNoteDecoder)
        , Json.Decode.map PvqGetArchiveZklinks (Json.Decode.field "PvqGetArchiveZklinks" getArchiveZkLinksDecoder)
        , Json.Decode.map PvqGetZkLinksSince (Json.Decode.field "PvqGetZkLinksSince" getZkLinksSinceDecoder)
        , Json.Decode.map PvqSearchZkNotes (Json.Decode.field "PvqSearchZkNotes" zkNoteSearchDecoder)
        , Json.Decode.map PvqPowerDelete (Json.Decode.field "PvqPowerDelete" (Json.Decode.list tagSearchDecoder))
        , Json.Decode.map PvqDeleteZkNote (Json.Decode.field "PvqDeleteZkNote" zkNoteIdDecoder)
        , Json.Decode.map PvqSaveZkNote (Json.Decode.field "PvqSaveZkNote" saveZkNoteDecoder)
        , Json.Decode.map PvqSaveZkLinks (Json.Decode.field "PvqSaveZkLinks" zkLinksDecoder)
        , Json.Decode.map PvqSaveZkNoteAndLinks (Json.Decode.field "PvqSaveZkNoteAndLinks" saveZkNoteAndLinksDecoder)
        , Json.Decode.map PvqSaveImportZkNotes (Json.Decode.field "PvqSaveImportZkNotes" (Json.Decode.list importZkNoteDecoder))
        , Json.Decode.map PvqSetHomeNote (Json.Decode.field "PvqSetHomeNote" zkNoteIdDecoder)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PvqSyncRemote" ->
                            Json.Decode.succeed PvqSyncRemote

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.map PvqSyncFiles (Json.Decode.field "PvqSyncFiles" zkNoteSearchDecoder)
        , Json.Decode.map PvqGetJobStatus (Json.Decode.field "PvqGetJobStatus" Json.Decode.int)
        ]


privateReplyDecoder : Json.Decode.Decoder PrivateReply
privateReplyDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map PvyServerError (Json.Decode.field "PvyServerError" privateErrorDecoder)
        , Json.Decode.map PvyZkNote (Json.Decode.field "PvyZkNote" zkNoteDecoder)
        , Json.Decode.map PvyZkNoteAndLinksWhat (Json.Decode.field "PvyZkNoteAndLinksWhat" zkNoteAndLinksWhatDecoder)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PvyNoop" ->
                            Json.Decode.succeed PvyNoop

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.map PvyZkNoteComments (Json.Decode.field "PvyZkNoteComments" (Json.Decode.list zkNoteDecoder))
        , Json.Decode.map PvyArchives (Json.Decode.field "PvyArchives" (Json.Decode.list zkListNoteDecoder))
        , Json.Decode.map PvyZkNoteArchives (Json.Decode.field "PvyZkNoteArchives" zkNoteArchivesDecoder)
        , Json.Decode.map PvyArchiveZkLinks (Json.Decode.field "PvyArchiveZkLinks" (Json.Decode.list archiveZkLinkDecoder))
        , Json.Decode.map PvyZkLinks (Json.Decode.field "PvyZkLinks" (Json.Decode.list uuidZkLinkDecoder))
        , Json.Decode.map PvyZkListNoteSearchResult (Json.Decode.field "PvyZkListNoteSearchResult" zkListNoteSearchResultDecoder)
        , Json.Decode.map PvyZkNoteSearchResult (Json.Decode.field "PvyZkNoteSearchResult" zkNoteSearchResultDecoder)
        , Json.Decode.map PvyZkNoteIdSearchResult (Json.Decode.field "PvyZkNoteIdSearchResult" zkIdSearchResultDecoder)
        , Json.Decode.map PvyZkNoteAndLinksSearchResult (Json.Decode.field "PvyZkNoteAndLinksSearchResult" zkNoteAndLinksSearchResultDecoder)
        , Json.Decode.map PvyPowerDeleteComplete (Json.Decode.field "PvyPowerDeleteComplete" Json.Decode.int)
        , Json.Decode.map PvyDeletedZkNote (Json.Decode.field "PvyDeletedZkNote" zkNoteIdDecoder)
        , Json.Decode.map PvySavedZkNote (Json.Decode.field "PvySavedZkNote" savedZkNoteDecoder)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PvySavedZkLinks" ->
                            Json.Decode.succeed PvySavedZkLinks

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.map PvySavedZkNoteAndLinks (Json.Decode.field "PvySavedZkNoteAndLinks" savedZkNoteDecoder)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PvySavedImportZkNotes" ->
                            Json.Decode.succeed PvySavedImportZkNotes

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.map PvyHomeNoteSet (Json.Decode.field "PvyHomeNoteSet" zkNoteIdDecoder)
        , Json.Decode.map PvyJobStatus (Json.Decode.field "PvyJobStatus" jobStatusDecoder)
        , Json.Decode.map PvyJobNotFound (Json.Decode.field "PvyJobNotFound" Json.Decode.int)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PvyFileSyncComplete" ->
                            Json.Decode.succeed PvyFileSyncComplete

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PvySyncComplete" ->
                            Json.Decode.succeed PvySyncComplete

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


privateErrorDecoder : Json.Decode.Decoder PrivateError
privateErrorDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map PveString (Json.Decode.field "PveString" Json.Decode.string)
        , Json.Decode.map PveNoteNotFound (Json.Decode.field "PveNoteNotFound" zkNoteRqDecoder)
        , Json.Decode.map PveNoteIsPrivate (Json.Decode.field "PveNoteIsPrivate" zkNoteRqDecoder)
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "PveNotLoggedIn" ->
                            Json.Decode.succeed PveNotLoggedIn

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.map PveLoginError (Json.Decode.field "PveLoginError" Json.Decode.string)
        ]


zkNoteRqDecoder : Json.Decode.Decoder ZkNoteRq
zkNoteRqDecoder =
    Json.Decode.succeed ZkNoteRq
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "zknoteid" zkNoteIdDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" (Json.Decode.nullable Json.Decode.string)))


uploadReplyDecoder : Json.Decode.Decoder UploadReply
uploadReplyDecoder =
    Json.Decode.oneOf
        [ Json.Decode.map UrFilesUploaded (Json.Decode.field "UrFilesUploaded" (Json.Decode.list zkListNoteDecoder))
        ]


zkNoteSearchDecoder : Json.Decode.Decoder ZkNoteSearch
zkNoteSearchDecoder =
    Json.Decode.succeed ZkNoteSearch
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "tagsearch" (Json.Decode.list tagSearchDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "limit" (Json.Decode.nullable Json.Decode.int)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "resulttype" resultTypeDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "archives" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "deleted" Json.Decode.bool))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "ordering" (Json.Decode.nullable orderingDecoder)))


orderingDecoder : Json.Decode.Decoder Ordering
orderingDecoder =
    Json.Decode.succeed Ordering
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "field" orderFieldDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "direction" orderDirectionDecoder))


orderDirectionDecoder : Json.Decode.Decoder OrderDirection
orderDirectionDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Ascending" ->
                            Json.Decode.succeed Ascending

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Descending" ->
                            Json.Decode.succeed Descending

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


orderFieldDecoder : Json.Decode.Decoder OrderField
orderFieldDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Title" ->
                            Json.Decode.succeed Title

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Created" ->
                            Json.Decode.succeed Created

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Changed" ->
                            Json.Decode.succeed Changed

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


resultTypeDecoder : Json.Decode.Decoder ResultType
resultTypeDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "RtId" ->
                            Json.Decode.succeed RtId

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "RtListNote" ->
                            Json.Decode.succeed RtListNote

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "RtNote" ->
                            Json.Decode.succeed RtNote

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "RtNoteAndLinks" ->
                            Json.Decode.succeed RtNoteAndLinks

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


tagSearchDecoder : Json.Decode.Decoder TagSearch
tagSearchDecoder =
    let
        elmRsConstructSearchTerm mods term =
            SearchTerm { mods = mods, term = term }

        elmRsConstructNot ts =
            Not { ts = ts }

        elmRsConstructBoolex ts1 ao ts2 =
            Boolex { ts1 = ts1, ao = ao, ts2 = ts2 }
    in
    Json.Decode.oneOf
        [ Json.Decode.field "SearchTerm" (Json.Decode.succeed elmRsConstructSearchTerm |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "mods" (Json.Decode.list searchModDecoder))) |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "term" Json.Decode.string)))
        , Json.Decode.field "Not" (Json.Decode.succeed elmRsConstructNot |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "ts" tagSearchDecoder)))
        , Json.Decode.field "Boolex" (Json.Decode.succeed elmRsConstructBoolex |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "ts1" tagSearchDecoder)) |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "ao" andOrDecoder)) |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "ts2" tagSearchDecoder)))
        ]


searchModDecoder : Json.Decode.Decoder SearchMod
searchModDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "ExactMatch" ->
                            Json.Decode.succeed ExactMatch

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "ZkNoteId" ->
                            Json.Decode.succeed ZkNoteId

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Tag" ->
                            Json.Decode.succeed Tag

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Note" ->
                            Json.Decode.succeed Note

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "User" ->
                            Json.Decode.succeed User

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "File" ->
                            Json.Decode.succeed File

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Before" ->
                            Json.Decode.succeed Before

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "After" ->
                            Json.Decode.succeed After

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Create" ->
                            Json.Decode.succeed Create

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Mod" ->
                            Json.Decode.succeed Mod

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


andOrDecoder : Json.Decode.Decoder AndOr
andOrDecoder =
    Json.Decode.oneOf
        [ Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "And" ->
                            Json.Decode.succeed And

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        , Json.Decode.string
            |> Json.Decode.andThen
                (\x ->
                    case x of
                        "Or" ->
                            Json.Decode.succeed Or

                        unexpected ->
                            Json.Decode.fail <| "Unexpected variant " ++ unexpected
                )
        ]


zkIdSearchResultDecoder : Json.Decode.Decoder ZkIdSearchResult
zkIdSearchResultDecoder =
    Json.Decode.succeed ZkIdSearchResult
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "notes" (Json.Decode.list zkNoteIdDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))


zkListNoteSearchResultDecoder : Json.Decode.Decoder ZkListNoteSearchResult
zkListNoteSearchResultDecoder =
    Json.Decode.succeed ZkListNoteSearchResult
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "notes" (Json.Decode.list zkListNoteDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))


zkNoteSearchResultDecoder : Json.Decode.Decoder ZkNoteSearchResult
zkNoteSearchResultDecoder =
    Json.Decode.succeed ZkNoteSearchResult
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "notes" (Json.Decode.list zkNoteDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))


zkSearchResultHeaderDecoder : Json.Decode.Decoder ZkSearchResultHeader
zkSearchResultHeaderDecoder =
    Json.Decode.succeed ZkSearchResultHeader
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "resulttype" resultTypeDecoder))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))


zkNoteAndLinksSearchResultDecoder : Json.Decode.Decoder ZkNoteAndLinksSearchResult
zkNoteAndLinksSearchResultDecoder =
    Json.Decode.succeed ZkNoteAndLinksSearchResult
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "notes" (Json.Decode.list zkNoteAndLinksDecoder)))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "offset" Json.Decode.int))
        |> Json.Decode.andThen (\x -> Json.Decode.map x (Json.Decode.field "what" Json.Decode.string))
