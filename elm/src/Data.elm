module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE
import Search as S
import UUID exposing (UUID)


type alias LoggedIn =
    { uid : String
    , pwd : String
    , ld : LoginData
    }


type alias Login a =
    { a
        | uid : String
        , pwd : String
    }


type alias LoginData =
    { userid : UUID
    , name : String
    , publicid : UUID
    , shareid : UUID
    , searchid : UUID
    }


type alias ZkListNote =
    { id : UUID
    , user : UUID
    , title : String
    , createdate : Int
    , changeddate : Int
    }


type alias ZkNoteSearchResult =
    { notes : List ZkListNote
    , offset : UUID
    }


type alias SavedZkNote =
    { id : UUID
    , changeddate : Int
    }


type alias ZkNote =
    { id : UUID
    , user : UUID
    , username : String
    , title : String
    , content : String
    , pubid : Maybe String
    , createdate : Int
    , changeddate : Int
    }


type alias SaveZkNote =
    { id : Maybe UUID
    , pubid : Maybe String
    , title : String
    , content : String
    }


type alias ZkLink =
    { from : UUID
    , to : UUID
    , user : UUID
    , fromname : Maybe String
    , toname : Maybe String
    , delete : Maybe Bool
    }


type Direction
    = From
    | To


type alias SaveZkLink =
    { otherid : UUID
    , direction : Direction
    , user : UUID
    , delete : Maybe Bool
    }


type alias SaveZkNotePlusLinks =
    { note : SaveZkNote
    , links : List SaveZkLink
    }


type alias ZkNoteAndAccomplices =
    { zknote : ZkNote
    , links : List ZkLink
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
    { zknote : UUID
    }


type alias GetZkNoteEdit =
    { zknote : UUID
    }


type alias ZkNoteEdit =
    { zknote : ZkNote
    , links : List ZkLink
    }


encodeGetZkLinks : GetZkLinks -> JE.Value
encodeGetZkLinks gzl =
    JE.object
        [ ( "zknote", UUID.toValue gzl.zknote )
        ]


encodeGetZkNoteEdit : GetZkNoteEdit -> JE.Value
encodeGetZkNoteEdit gzl =
    JE.object
        [ ( "zknote", UUID.toValue gzl.zknote )
        ]


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


encodeSaveZkLink : SaveZkLink -> JE.Value
encodeSaveZkLink s =
    [ Just ( "otherid", UUID.toValue s.otherid )
    , Just ( "direction", encodeDirection s.direction )
    , Just ( "user", UUID.toValue s.user )
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
        [ ( "from", UUID.toValue zklink.from )
        , ( "to", UUID.toValue zklink.to )
        , ( "user", UUID.toValue zklink.user )
        ]
            ++ (zklink.delete
                    |> Maybe.map (\b -> [ ( "delete", JE.bool b ) ])
                    |> Maybe.withDefault []
               )


decodeZkLink : JD.Decoder ZkLink
decodeZkLink =
    JD.map6 ZkLink
        (JD.field "from" UUID.jsonDecoder)
        (JD.field "to" UUID.jsonDecoder)
        (JD.field "user" UUID.jsonDecoder)
        (JD.maybe (JD.field "fromname" JD.string))
        (JD.maybe (JD.field "toname" JD.string))
        (JD.succeed Nothing)


saveZkNote : ZkNote -> SaveZkNote
saveZkNote fzn =
    { id = Just fzn.id
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
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
        (Maybe.map (\id -> [ ( "id", UUID.toValue id ) ]) zkn.id
            |> Maybe.withDefault []
        )
            ++ (Maybe.map (\pubid -> [ ( "pubid", JE.string pubid ) ]) zkn.pubid
                    |> Maybe.withDefault []
               )
            ++ [ ( "title", JE.string zkn.title )
               , ( "content", JE.string zkn.content )
               ]


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.map5 ZkListNote
        (JD.field "id" UUID.jsonDecoder)
        (JD.field "user" UUID.jsonDecoder)
        (JD.field "title" JD.string)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


decodeZkNoteSearchResult : JD.Decoder ZkNoteSearchResult
decodeZkNoteSearchResult =
    JD.map2 ZkNoteSearchResult
        (JD.field "notes" (JD.list decodeZkListNote))
        (JD.field "offset" UUID.jsonDecoder)


decodeSavedZkNote : JD.Decoder SavedZkNote
decodeSavedZkNote =
    JD.map2 SavedZkNote
        (JD.field "id" UUID.jsonDecoder)
        (JD.field "changeddate" JD.int)


decodeZkNote : JD.Decoder ZkNote
decodeZkNote =
    JD.map8 ZkNote
        (JD.field "id" UUID.jsonDecoder)
        (JD.field "user" UUID.jsonDecoder)
        (JD.field "username" JD.string)
        (JD.field "title" JD.string)
        (JD.field "content" JD.string)
        (JD.field "pubid" (JD.maybe JD.string))
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


decodeZkNoteAndAccomplices : JD.Decoder ZkNoteAndAccomplices
decodeZkNoteAndAccomplices =
    JD.map2 ZkNoteAndAccomplices
        (JD.field "zknote" decodeZkNote)
        (JD.field "links" (JD.list decodeZkLink))


decodeZkNoteEdit : JD.Decoder ZkNoteEdit
decodeZkNoteEdit =
    JD.map2 ZkNoteEdit
        (JD.field "zknote" decodeZkNote)
        (JD.field "links" (JD.list decodeZkLink))


decodeLoginData : JD.Decoder LoginData
decodeLoginData =
    JD.map5 LoginData
        (JD.field "userid" UUID.jsonDecoder)
        (JD.field "username" JD.string)
        (JD.field "publicid" UUID.jsonDecoder)
        (JD.field "shareid" UUID.jsonDecoder)
        (JD.field "searchid" UUID.jsonDecoder)


encodeImportZkNote : ImportZkNote -> JE.Value
encodeImportZkNote izn =
    JE.object
        [ ( "title", JE.string izn.title )
        , ( "content", JE.string izn.content )
        , ( "fromLinks", JE.list JE.string izn.fromLinks )
        , ( "toLinks", JE.list JE.string izn.toLinks )
        ]
