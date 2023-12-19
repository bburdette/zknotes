module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE
import Orgauth.Data exposing (UserId, decodeUserId, encodeUserId)
import Search as S
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
    , zknote : Int
    , homenote : Maybe UUID
    }


type alias Sysids =
    { publicid : Int
    , shareid : Int
    , searchid : Int
    , commentid : Int
    }


type alias ZkInviteData =
    List SaveZkLink


type alias ZkListNote =
    { id : Int
    , uuid : UUID
    , user : UserId
    , title : String
    , isFile : Bool
    , createdate : Int
    , changeddate : Int
    , sysids : List Int
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
    { id : Int
    , uuid : UUID
    , changeddate : Int
    }


type alias ZkNote =
    { id : Int
    , uuid : UUID
    , user : UserId
    , username : String
    , usernote : Int
    , title : String
    , content : String
    , pubid : Maybe String
    , editable : Bool -- whether I'm allowed to edit the note.
    , editableValue : Bool -- whether the user has marked it editable.
    , showtitle : Bool
    , createdate : Int
    , changeddate : Int
    , deleted : Bool
    , isFile : Bool
    , sysids : List Int
    }


type ZkNoteId
    = ZkInt Int
    | ZkUUID UUID


encodeZkNoteId zni =
    case zni of
        ZkInt id ->
            JE.object [ ( "ZkInt", JE.int id ) ]

        ZkUUID id ->
            JE.object [ ( "ZkUUID", UUID.toValue id ) ]


decodeZkNoteId : JD.Decoder ZkNoteId
decodeZkNoteId =
    JD.oneOf
        [ JD.map ZkInt (JD.field "ZkInt" JD.int)
        , JD.map ZkUUID (JD.field "ZkUUID" UUID.jsonDecoder)
        ]


type alias ZkLink =
    { from : Int
    , to : Int
    , user : UserId
    , zknote : Maybe Int
    , fromname : Maybe String
    , toname : Maybe String
    , delete : Maybe Bool
    }


type Direction
    = From
    | To


type alias EditLink =
    { otherid : Int
    , direction : Direction
    , user : UserId
    , zknote : Maybe Int
    , othername : Maybe String
    , sysids : List Int
    , delete : Maybe Bool
    }


type alias SaveZkNote =
    { id : Maybe Int
    , uuid : Maybe UUID
    , pubid : Maybe String
    , title : String
    , content : String
    , editable : Bool
    , showtitle : Bool
    , deleted : Bool
    }


type alias SaveZkLink =
    { otherid : Int
    , direction : Direction
    , user : UserId
    , zknote : Maybe Int
    , delete : Maybe Bool
    }


type alias SaveZkNotePlusLinks =
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
    , limit : Maybe Int
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
    { zknote : Int
    , zkuuid : UUID
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
            |> andMap (JD.field "zknote" JD.int)
            |> andMap (JD.field "homenote" (JD.maybe UUID.jsonDecoder))
        )
        oald.data


decodeSysids : JD.Decoder Sysids
decodeSysids =
    JD.succeed Sysids
        |> andMap (JD.field "publicid" JD.int)
        |> andMap (JD.field "shareid" JD.int)
        |> andMap (JD.field "searchid" JD.int)
        |> andMap (JD.field "commentid" JD.int)


toZkLink : Int -> UserId -> EditLink -> ZkLink
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


zklKey : { a | otherid : Int, direction : Direction } -> String
zklKey zkl =
    String.fromInt zkl.otherid
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
            ++ (case x.limit of
                    Just l ->
                        [ ( "limit", JE.int l )
                        ]

                    Nothing ->
                        []
               )


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
    [ Just ( "otherid", JE.int s.otherid )
    , Just ( "direction", encodeDirection s.direction )
    , Just ( "user", encodeUserId s.user )
    , s.zknote |> Maybe.map (\n -> ( "zknote", JE.int n ))
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
        [ ( "from", JE.int zklink.from )
        , ( "to", JE.int zklink.to )
        , ( "user", encodeUserId zklink.user )
        ]
            ++ (zklink.delete
                    |> Maybe.map (\b -> [ ( "delete", JE.bool b ) ])
                    |> Maybe.withDefault []
               )
            ++ (zklink.zknote
                    |> Maybe.map
                        (\id ->
                            [ ( "linkzknote", JE.int id ) ]
                        )
                    |> Maybe.withDefault
                        []
               )


decodeZkLink : JD.Decoder ZkLink
decodeZkLink =
    JD.map7 ZkLink
        (JD.field "from" JD.int)
        (JD.field "to" JD.int)
        (JD.field "user" decodeUserId)
        (JD.maybe (JD.field "linkzknote" JD.int))
        (JD.maybe (JD.field "fromname" JD.string))
        (JD.maybe (JD.field "toname" JD.string))
        (JD.succeed Nothing)


decodeEditLink : JD.Decoder EditLink
decodeEditLink =
    JD.map6 (\a b c d e f -> EditLink a b c d e f Nothing)
        (JD.field "otherid" JD.int)
        (JD.field "direction" decodeDirection)
        (JD.field "user" decodeUserId)
        (JD.maybe (JD.field "zknote" JD.int))
        (JD.maybe (JD.field "othername" JD.string))
        (JD.field "sysids" (JD.list JD.int))


saveZkNote : ZkNote -> SaveZkNote
saveZkNote fzn =
    { id = Just fzn.id
    , uuid = Just fzn.uuid
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
    , editable = fzn.editableValue
    , showtitle = fzn.showtitle
    , deleted = fzn.deleted
    }


encodeSaveZkNotePlusLinks : SaveZkNotePlusLinks -> JE.Value
encodeSaveZkNotePlusLinks s =
    JE.object
        [ ( "note", encodeSaveZkNote s.note )
        , ( "links", JE.list encodeSaveZkLink s.links )
        ]


encodeSaveZkNote : SaveZkNote -> JE.Value
encodeSaveZkNote zkn =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", JE.int id ) ]) zkn.id
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


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.succeed ZkListNote
        |> andMap (JD.field "id" JD.int)
        |> andMap (JD.field "uuid" UUID.jsonDecoder)
        |> andMap (JD.field "user" decodeUserId)
        |> andMap (JD.field "title" JD.string)
        |> andMap (JD.field "is_file" JD.bool)
        |> andMap (JD.field "createdate" JD.int)
        |> andMap (JD.field "changeddate" JD.int)
        |> andMap (JD.field "sysids" (JD.list JD.int))


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
    JD.map3 SavedZkNote
        (JD.field "id" JD.int)
        (JD.field "uuid" UUID.jsonDecoder)
        (JD.field "changeddate" JD.int)


decodeZkNote : JD.Decoder ZkNote
decodeZkNote =
    JD.succeed ZkNote
        |> andMap (JD.field "id" JD.int)
        |> andMap (JD.field "uuid" UUID.jsonDecoder)
        |> andMap (JD.field "user" decodeUserId)
        |> andMap (JD.field "username" JD.string)
        |> andMap (JD.field "usernote" JD.int)
        |> andMap (JD.field "title" JD.string)
        |> andMap (JD.field "content" JD.string)
        |> andMap (JD.field "pubid" (JD.maybe JD.string))
        |> andMap (JD.field "editable" JD.bool)
        |> andMap (JD.field "editableValue" JD.bool)
        |> andMap (JD.field "showtitle" JD.bool)
        |> andMap (JD.field "createdate" JD.int)
        |> andMap (JD.field "changeddate" JD.int)
        |> andMap (JD.field "deleted" JD.bool)
        |> andMap (JD.field "is_file" JD.bool)
        |> andMap (JD.field "sysids" <| JD.list JD.int)


decodeZkNoteArchives : JD.Decoder ZkNoteArchives
decodeZkNoteArchives =
    JD.map3 ZkNoteArchives
        (JD.field "zknote" JD.int)
        (JD.field "zkuuid" UUID.jsonDecoder)
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
        |> andMap (JD.field "data" (JD.field "zknote" JD.int))
        |> andMap (JD.field "data" (JD.field "homenote" (JD.maybe UUID.jsonDecoder)))


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


editNoteLink : UUID -> String
editNoteLink noteid =
    UB.absolute [ "editnote", UUID.toString noteid ] []


archiveNoteLink : Int -> Int -> String
archiveNoteLink parentnoteid noteid =
    UB.absolute [ "archivenote", String.fromInt parentnoteid, String.fromInt noteid ] []


flipDirection : Direction -> Direction
flipDirection direction =
    case direction of
        To ->
            From

        From ->
            To
