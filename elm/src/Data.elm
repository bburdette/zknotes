module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE
import Orgauth.Data exposing (UserId, decodeUserId, encodeUserId)
import Search as S
import TDict
import TSet exposing (TSet)
import UUID exposing (UUID)
import Url.Builder as UB
import Util exposing (andMap)



------------------------------------------------------------
-- getting, setting text selections in text edit areas.
------------------------------------------------------------


type alias SetSelection =
    { id : String
    , offset : Int
    , length : Int
    }


type alias TASelection =
    -- 'Text Area Selection'
    { text : String
    , offset : Int
    , what : String
    }


type alias TAError =
    { what : String
    }


encodeSetSelection : SetSelection -> JE.Value
encodeSetSelection s =
    JE.object
        [ ( "id", JE.string s.id )
        , ( "offset", JE.int s.offset )
        , ( "length", JE.int s.length )
        ]


decodeTASelection : JD.Decoder TASelection
decodeTASelection =
    JD.succeed TASelection
        |> andMap (JD.field "text" JD.string)
        |> andMap (JD.field "offset" JD.int)
        |> andMap (JD.field "what" JD.string)


decodeTAError : JD.Decoder TAError
decodeTAError =
    JD.succeed TAError
        |> andMap (JD.field "what" JD.string)



----------------------------------------
-- types sent to or from the server.
----------------------------------------


type alias LoginData =
    { userid : UserId
    , uuid : UUID
    , name : String
    , email : String
    , admin : Bool
    , active : Bool
    , zknote : ZkNoteId
    , homenote : Maybe ZkNoteId
    }


type alias Sysids =
    { publicid : ZkNoteId
    , shareid : ZkNoteId
    , searchid : ZkNoteId
    , commentid : ZkNoteId
    }


sysids : Sysids
sysids =
    { publicid = ZkNoteId "f596bc2c-a882-4c1c-b739-8c4e25f34eb2"
    , commentid = ZkNoteId "e82fefee-bcd3-4e2e-b350-9963863e516d"
    , shareid = ZkNoteId "466d39ec-2ea7-4d43-b44c-1d3d083f8a9d"
    , searchid = ZkNoteId "84f72fd0-8836-43a3-ac66-89e0ab49dd87"

    -- , userid = ZkNoteId "4fb37d76-6fc8-4869-8ee4-8e05fa5077f7"
    -- , archiveid = ZkNoteId "ad6a4ca8-0446-4ecc-b047-46282ced0d84"
    -- , systemid = ZkNoteId "0efcc98f-dffd-40e5-af07-90da26b1d469"
    -- , syncid = ZkNoteId "528ccfc2-8488-41e0-a4e1-cbab6406674e"
    }


type alias ZkInviteData =
    List SaveZkLink


type FileStatus
    = NotAFile
    | FileMissing
    | FilePresent


type alias ZkListNote =
    { id : ZkNoteId
    , user : UserId
    , title : String
    , filestatus : FileStatus
    , createdate : Int
    , changeddate : Int
    , sysids : List ZkNoteId
    }


type alias ZkIdSearchResult =
    { notes : List Int
    , offset : Int
    , what : String
    }


type alias ZkListNoteSearchResult =
    { notes : List ZkListNote
    , offset : Int
    , what : String
    }


type alias ZkNoteSearchResult =
    { notes : List ZkNote
    , offset : Int
    , what : String
    }


type alias SavedZkNote =
    { id : ZkNoteId
    , changeddate : Int
    }


type alias ZkNote =
    { id : ZkNoteId
    , user : UserId
    , username : String
    , usernote : ZkNoteId
    , title : String
    , content : String
    , pubid : Maybe String
    , editable : Bool -- whether I'm allowed to edit the note.
    , editableValue : Bool -- whether the user has marked it editable.
    , showtitle : Bool
    , createdate : Int
    , changeddate : Int
    , deleted : Bool
    , filestatus : FileStatus
    , sysids : List ZkNoteId
    }


type ZkNoteId
    = ZkNoteId String


zniEq : ZkNoteId -> ZkNoteId -> Bool
zniEq (ZkNoteId l) (ZkNoteId r) =
    l == r


zniCompare : ZkNoteId -> ZkNoteId -> Order
zniCompare (ZkNoteId l) (ZkNoteId r) =
    compare l r


encodeZkNoteId : ZkNoteId -> JE.Value
encodeZkNoteId (ZkNoteId zni) =
    JE.string zni


zkNoteIdToString : ZkNoteId -> String
zkNoteIdToString (ZkNoteId zni) =
    zni


zkNoteIdFromString : String -> Result UUID.Error ZkNoteId
zkNoteIdFromString zni =
    UUID.fromString
        zni
        |> Result.map (\_ -> ZkNoteId zni)


trustedZkNoteIdFromString : String -> ZkNoteId
trustedZkNoteIdFromString zni =
    ZkNoteId zni


zkNoteIdFromUUID : UUID -> ZkNoteId
zkNoteIdFromUUID zni =
    ZkNoteId (UUID.toString zni)


decodeZkNoteId : JD.Decoder ZkNoteId
decodeZkNoteId =
    UUID.jsonDecoder
        |> JD.map zkNoteIdFromUUID


type alias ZniSet =
    TSet ZkNoteId String


emptyZniSet : ZniSet
emptyZniSet =
    TSet.empty zkNoteIdToString trustedZkNoteIdFromString


emptyZniDict =
    TDict.empty zkNoteIdToString trustedZkNoteIdFromString


type alias ZkLink =
    { from : ZkNoteId
    , to : ZkNoteId
    , user : UserId
    , zknote : Maybe ZkNoteId
    , fromname : Maybe String
    , toname : Maybe String
    , delete : Maybe Bool
    }


type Direction
    = From
    | To


type alias EditLink =
    { otherid : ZkNoteId
    , direction : Direction
    , user : UserId
    , zknote : Maybe ZkNoteId
    , othername : Maybe String
    , sysids : List ZkNoteId
    , delete : Maybe Bool
    }


type alias SaveZkNote =
    { id : Maybe ZkNoteId
    , pubid : Maybe String
    , title : String
    , content : String
    , editable : Bool
    , showtitle : Bool
    , deleted : Bool
    }


type alias SaveZkLink =
    { otherid : ZkNoteId
    , direction : Direction
    , user : UserId
    , zknote : Maybe ZkNoteId
    , delete : Maybe Bool
    }


type alias SaveZkNoteAndLinks =
    { note : SaveZkNote
    , links : List SaveZkLink
    }


type alias ZkLinks =
    { links : List ZkLink
    }


type alias SaveZkLinks =
    { saveLinks : List ZkLink
    , deleteLinks : List ZkLink
    }


type alias ImportZkNote =
    { title : String, content : String, fromLinks : List String, toLinks : List String }


type alias GetZkLinks =
    { zknote : ZkNoteId
    }


type alias GetZkNoteComments =
    { zknote : ZkNoteId
    , offset : Int
    }


type alias GetZkNoteAndLinks =
    { zknote : ZkNoteId
    , what : String
    }


type alias GetZnlIfChanged =
    { zknote : ZkNoteId
    , changeddate : Int
    , what : String
    }


type alias ZkNoteAndLinksWhat =
    { what : String
    , znl : ZkNoteAndLinks
    }


type alias ZkNoteAndLinks =
    { zknote : ZkNote
    , links : List EditLink
    }


type alias GetZkNoteArchives =
    { zknote : ZkNoteId
    , offset : Int
    , limit : Maybe Int
    }


type alias ZkNoteArchives =
    { zknote : ZkNoteId
    , results : ZkListNoteSearchResult
    }


type alias GetArchiveZkNote =
    { parentnote : ZkNoteId
    , noteid : ZkNoteId
    }



----------------------------------------
-- Utility ftns
----------------------------------------


fromOaLd : Orgauth.Data.LoginData -> Result JD.Error LoginData
fromOaLd oald =
    JD.decodeValue
        (JD.succeed (LoginData oald.userid oald.uuid oald.name oald.email oald.admin oald.active)
            |> andMap (JD.field "zknote" decodeZkNoteId)
            |> andMap (JD.field "homenote" (JD.maybe decodeZkNoteId))
        )
        oald.data


decodeSysids : JD.Decoder Sysids
decodeSysids =
    JD.succeed Sysids
        |> andMap (JD.field "publicid" decodeZkNoteId)
        |> andMap (JD.field "shareid" decodeZkNoteId)
        |> andMap (JD.field "searchid" decodeZkNoteId)
        |> andMap (JD.field "commentid" decodeZkNoteId)


toZkLink : ZkNoteId -> UserId -> EditLink -> ZkLink
toZkLink noteid user el =
    { from =
        case el.direction of
            From ->
                el.otherid

            To ->
                noteid
    , to =
        case el.direction of
            From ->
                noteid

            To ->
                el.otherid
    , user = user
    , zknote = Nothing
    , fromname = Nothing
    , toname = Nothing
    , delete = Nothing
    }


zklKey : { a | otherid : ZkNoteId, direction : Direction } -> String
zklKey zkl =
    zkNoteIdToString zkl.otherid
        ++ ":"
        ++ (case zkl.direction of
                From ->
                    "from"

                To ->
                    "to"
           )


elToSzl : EditLink -> SaveZkLink
elToSzl el =
    { otherid = el.otherid
    , direction = el.direction
    , user = el.user
    , zknote = el.zknote
    , delete = el.delete
    }



----------------------------------------
-- Json encoders/decoders
----------------------------------------


encodeZkInviteData : ZkInviteData -> JE.Value
encodeZkInviteData zid =
    JE.list encodeSaveZkLink zid


encodeGetZkLinks : GetZkLinks -> JE.Value
encodeGetZkLinks gzl =
    JE.object
        [ ( "zknote", encodeZkNoteId gzl.zknote )
        ]


encodeGetZkNoteEdit : GetZkNoteAndLinks -> JE.Value
encodeGetZkNoteEdit gzl =
    JE.object
        [ ( "zknote", encodeZkNoteId gzl.zknote )
        , ( "what", JE.string gzl.what )
        ]


encodeGetZnlIfChanged : GetZnlIfChanged -> JE.Value
encodeGetZnlIfChanged x =
    JE.object
        [ ( "zknote", encodeZkNoteId x.zknote )
        , ( "changeddate", JE.int x.changeddate )
        , ( "what", JE.string x.what )
        ]


encodeGetZkNoteComments : GetZkNoteComments -> JE.Value
encodeGetZkNoteComments x =
    JE.object <|
        [ ( "zknote", encodeZkNoteId x.zknote )
        , ( "offset", JE.int x.offset )
        ]



-- ++ (case x.limit of
--         Just l ->
--             [ ( "limit", JE.int l )
--             ]
--         Nothing ->
--             []
--    )


encodeZkLinks : ZkLinks -> JE.Value
encodeZkLinks zklinks =
    JE.object
        [ ( "links", JE.list encodeZkLink zklinks.links )
        ]


encodeDirection : Direction -> JE.Value
encodeDirection direction =
    case direction of
        To ->
            JE.string "To"

        From ->
            JE.string "From"


decodeDirection : JD.Decoder Direction
decodeDirection =
    JD.string
        |> JD.andThen
            (\s ->
                case s of
                    "From" ->
                        JD.succeed From

                    "To" ->
                        JD.succeed To

                    wat ->
                        JD.fail ("not a direction: " ++ wat)
            )


encodeSaveZkLink : SaveZkLink -> JE.Value
encodeSaveZkLink s =
    [ Just ( "otherid", encodeZkNoteId s.otherid )
    , Just ( "direction", encodeDirection s.direction )
    , Just ( "user", encodeUserId s.user )
    , s.zknote |> Maybe.map (\n -> ( "zknote", encodeZkNoteId n ))
    , s.delete |> Maybe.map (\n -> ( "delete", JE.bool n ))
    ]
        |> List.filterMap identity
        |> JE.object


decodeZkLinks : JD.Decoder ZkLinks
decodeZkLinks =
    JD.map ZkLinks
        (JD.field "links" (JD.list decodeZkLink))


encodeZkLink : ZkLink -> JE.Value
encodeZkLink zklink =
    JE.object <|
        [ ( "from", encodeZkNoteId zklink.from )
        , ( "to", encodeZkNoteId zklink.to )
        , ( "user", encodeUserId zklink.user )
        ]
            ++ (zklink.delete
                    |> Maybe.map (\b -> [ ( "delete", JE.bool b ) ])
                    |> Maybe.withDefault []
               )
            ++ (zklink.zknote
                    |> Maybe.map
                        (\id ->
                            [ ( "linkzknote", encodeZkNoteId id ) ]
                        )
                    |> Maybe.withDefault
                        []
               )


decodeZkLink : JD.Decoder ZkLink
decodeZkLink =
    JD.map7 ZkLink
        (JD.field "from" decodeZkNoteId)
        (JD.field "to" decodeZkNoteId)
        (JD.field "user" decodeUserId)
        (JD.maybe (JD.field "linkzknote" decodeZkNoteId))
        (JD.maybe (JD.field "fromname" JD.string))
        (JD.maybe (JD.field "toname" JD.string))
        (JD.succeed Nothing)


decodeEditLink : JD.Decoder EditLink
decodeEditLink =
    JD.map6 (\a b c d e f -> EditLink a b c d e f Nothing)
        (JD.field "otherid" decodeZkNoteId)
        (JD.field "direction" decodeDirection)
        (JD.field "user" decodeUserId)
        (JD.maybe (JD.field "zknote" decodeZkNoteId))
        (JD.maybe (JD.field "othername" JD.string))
        (JD.field "sysids" (JD.list decodeZkNoteId))


saveZkNote : ZkNote -> SaveZkNote
saveZkNote fzn =
    { id = Just fzn.id
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
    , editable = fzn.editableValue
    , showtitle = fzn.showtitle
    , deleted = fzn.deleted
    }


encodeSaveZkNoteAndLinks : SaveZkNoteAndLinks -> JE.Value
encodeSaveZkNoteAndLinks s =
    JE.object
        [ ( "note", encodeSaveZkNote s.note )
        , ( "links", JE.list encodeSaveZkLink s.links )
        ]


encodeSaveZkNote : SaveZkNote -> JE.Value
encodeSaveZkNote zkn =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", encodeZkNoteId id ) ]) zkn.id
            |> Maybe.withDefault []
        )
            ++ (Maybe.map (\pubid -> [ ( "pubid", JE.string pubid ) ]) zkn.pubid
                    |> Maybe.withDefault []
               )
            ++ [ ( "title", JE.string zkn.title )
               , ( "content", JE.string zkn.content )
               , ( "editable", JE.bool zkn.editable )
               , ( "showtitle", JE.bool zkn.showtitle )
               , ( "deleted", JE.bool zkn.deleted )
               ]


decodeFileStatus : JD.Decoder FileStatus
decodeFileStatus =
    JD.string
        |> JD.andThen
            (\s ->
                case s of
                    "NotAFile" ->
                        JD.succeed NotAFile

                    "FileMissing" ->
                        JD.succeed FileMissing

                    "FilePresent" ->
                        JD.succeed FilePresent

                    wup ->
                        JD.fail <| "invalid filestatus: " ++ wup
            )


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.succeed ZkListNote
        |> andMap (JD.field "id" decodeZkNoteId)
        |> andMap (JD.field "user" decodeUserId)
        |> andMap (JD.field "title" JD.string)
        |> andMap (JD.field "filestatus" decodeFileStatus)
        |> andMap (JD.field "createdate" JD.int)
        |> andMap (JD.field "changeddate" JD.int)
        |> andMap (JD.field "sysids" (JD.list decodeZkNoteId))


decodeZkIdSearchResult : JD.Decoder ZkIdSearchResult
decodeZkIdSearchResult =
    JD.map3 ZkIdSearchResult
        (JD.field "notes" (JD.list JD.int))
        (JD.field "offset" JD.int)
        (JD.field "what" JD.string)


decodeZkListNoteSearchResult : JD.Decoder ZkListNoteSearchResult
decodeZkListNoteSearchResult =
    JD.map3 ZkListNoteSearchResult
        (JD.field "notes" (JD.list decodeZkListNote))
        (JD.field "offset" JD.int)
        (JD.field "what" JD.string)


decodeZkNoteSearchResult : JD.Decoder ZkNoteSearchResult
decodeZkNoteSearchResult =
    JD.map3 ZkNoteSearchResult
        (JD.field "notes" (JD.list decodeZkNote))
        (JD.field "offset" JD.int)
        (JD.field "what" JD.string)


decodeSavedZkNote : JD.Decoder SavedZkNote
decodeSavedZkNote =
    JD.map2 SavedZkNote
        (JD.field "id" decodeZkNoteId)
        (JD.field "changeddate" JD.int)


decodeZkNote : JD.Decoder ZkNote
decodeZkNote =
    JD.succeed ZkNote
        |> andMap (JD.field "id" decodeZkNoteId)
        |> andMap (JD.field "user" decodeUserId)
        |> andMap (JD.field "username" JD.string)
        |> andMap (JD.field "usernote" decodeZkNoteId)
        |> andMap (JD.field "title" JD.string)
        |> andMap (JD.field "content" JD.string)
        |> andMap (JD.field "pubid" (JD.maybe JD.string))
        |> andMap (JD.field "editable" JD.bool)
        |> andMap (JD.field "editableValue" JD.bool)
        |> andMap (JD.field "showtitle" JD.bool)
        |> andMap (JD.field "createdate" JD.int)
        |> andMap (JD.field "changeddate" JD.int)
        |> andMap (JD.field "deleted" JD.bool)
        |> andMap (JD.field "filestatus" decodeFileStatus)
        |> andMap (JD.field "sysids" <| JD.list decodeZkNoteId)


decodeZkNoteArchives : JD.Decoder ZkNoteArchives
decodeZkNoteArchives =
    JD.map2 ZkNoteArchives
        (JD.field "zknote" decodeZkNoteId)
        (JD.field "results" decodeZkListNoteSearchResult)


decodeZkNoteEdit : JD.Decoder ZkNoteAndLinks
decodeZkNoteEdit =
    JD.map2 ZkNoteAndLinks
        (JD.field "zknote" decodeZkNote)
        (JD.field "links" (JD.list decodeEditLink))


decodeZkNoteEditWhat : JD.Decoder ZkNoteAndLinksWhat
decodeZkNoteEditWhat =
    JD.map2 ZkNoteAndLinksWhat
        (JD.field "what" JD.string)
        (JD.field "znl" decodeZkNoteEdit)


decodeLoginData : JD.Decoder LoginData
decodeLoginData =
    JD.succeed LoginData
        |> andMap (JD.field "userid" decodeUserId)
        |> andMap (JD.field "uuid" UUID.jsonDecoder)
        |> andMap (JD.field "name" JD.string)
        |> andMap (JD.field "email" JD.string)
        |> andMap (JD.field "admin" JD.bool)
        |> andMap (JD.field "active" JD.bool)
        |> andMap (JD.field "data" (JD.field "zknote" decodeZkNoteId))
        |> andMap (JD.field "data" (JD.field "homenote" (JD.maybe decodeZkNoteId)))


encodeImportZkNote : ImportZkNote -> JE.Value
encodeImportZkNote izn =
    JE.object
        [ ( "title", JE.string izn.title )
        , ( "content", JE.string izn.content )
        , ( "fromLinks", JE.list JE.string izn.fromLinks )
        , ( "toLinks", JE.list JE.string izn.toLinks )
        ]


encodeGetZkNoteArchives : GetZkNoteArchives -> JE.Value
encodeGetZkNoteArchives x =
    JE.object <|
        [ ( "zknote", encodeZkNoteId x.zknote )
        , ( "offset", JE.int x.offset )
        ]
            ++ (case x.limit of
                    Just l ->
                        [ ( "limit", JE.int l )
                        ]

                    Nothing ->
                        []
               )


encodeGetArchiveZkNote : GetArchiveZkNote -> JE.Value
encodeGetArchiveZkNote x =
    JE.object <|
        [ ( "parentnote", encodeZkNoteId x.parentnote )
        , ( "noteid", encodeZkNoteId x.noteid )
        ]



----------------------------------------
-- misc functions
----------------------------------------


editNoteLink : ZkNoteId -> String
editNoteLink noteid =
    UB.absolute [ "editnote", zkNoteIdToString noteid ] []


archiveNoteLink : ZkNoteId -> ZkNoteId -> String
archiveNoteLink parentnoteid noteid =
    UB.absolute [ "archivenote", zkNoteIdToString parentnoteid, zkNoteIdToString noteid ] []


flipDirection : Direction -> Direction
flipDirection direction =
    case direction of
        To ->
            From

        From ->
            To
