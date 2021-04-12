module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE
import Search as S
import Url.Builder as UB
import Util exposing (andMap)



----------------------------------------
-- types sent to or from the server.
----------------------------------------


type alias Registration =
    { uid : String
    , pwd : String
    , email : String
    }


type alias Login =
    { uid : String
    , pwd : String
    }


type alias LoginData =
    { userid : Int
    , name : String
    , publicid : Int
    , shareid : Int
    , searchid : Int
    }


type alias ZkListNote =
    { id : Int
    , user : Int
    , title : String
    , createdate : Int
    , changeddate : Int
    }


type alias ZkNoteSearchResult =
    { notes : List ZkListNote
    , offset : Int
    }


type alias SavedZkNote =
    { id : Int
    , changeddate : Int
    }


type alias ZkNote =
    { id : Int
    , user : Int
    , username : String
    , title : String
    , content : String
    , pubid : Maybe String
    , editable : Bool -- whether I'm allowed to edit the note.
    , editableValue : Bool -- whether the user has marked it editable.
    , createdate : Int
    , changeddate : Int
    }


type alias SaveZkNote =
    { id : Maybe Int
    , pubid : Maybe String
    , title : String
    , content : String
    , editable : Bool
    }


type alias ZkLink =
    { from : Int
    , to : Int
    , user : Int
    , zknote : Maybe Int
    , fromname : Maybe String
    , toname : Maybe String
    , delete : Maybe Bool
    }


type Direction
    = From
    | To


type alias SaveZkLink =
    { otherid : Int
    , direction : Direction
    , user : Int
    , zknote : Maybe Int
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
    { zknote : Int
    }


type alias GetZkNoteComments =
    { zknote : Int
    , offset : Int
    , limit : Maybe Int
    }


type alias GetZkNoteEdit =
    { zknote : Int
    }


type alias ZkNoteEdit =
    { zknote : ZkNote
    , links : List ZkLink
    }



----------------------------------------
-- Json encoders/decoders
----------------------------------------


encodeRegistration : Registration -> JE.Value
encodeRegistration l =
    JE.object
        [ ( "uid", JE.string l.uid )
        , ( "pwd", JE.string l.pwd )
        , ( "email", JE.string l.email )
        ]


encodeLogin : Login -> JE.Value
encodeLogin l =
    JE.object
        [ ( "uid", JE.string l.uid )
        , ( "pwd", JE.string l.pwd )
        ]


encodeGetZkLinks : GetZkLinks -> JE.Value
encodeGetZkLinks gzl =
    JE.object
        [ ( "zknote", JE.int gzl.zknote )
        ]


encodeGetZkNoteEdit : GetZkNoteEdit -> JE.Value
encodeGetZkNoteEdit gzl =
    JE.object
        [ ( "zknote", JE.int gzl.zknote )
        ]


encodeGetZkNoteComments : GetZkNoteComments -> JE.Value
encodeGetZkNoteComments x =
    JE.object <|
        [ ( "zknote", JE.int x.zknote )
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


encodeSaveZkLink : SaveZkLink -> JE.Value
encodeSaveZkLink s =
    [ Just ( "otherid", JE.int s.otherid )
    , Just ( "direction", encodeDirection s.direction )
    , Just ( "user", JE.int s.user )
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
        , ( "user", JE.int zklink.user )
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
        (JD.field "user" JD.int)
        (JD.maybe (JD.field "linkzknote" JD.int))
        (JD.maybe (JD.field "fromname" JD.string))
        (JD.maybe (JD.field "toname" JD.string))
        (JD.succeed Nothing)


saveZkNote : ZkNote -> SaveZkNote
saveZkNote fzn =
    { id = Just fzn.id
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
    , editable = fzn.editableValue
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
               ]


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.map5 ZkListNote
        (JD.field "id" JD.int)
        (JD.field "user" JD.int)
        (JD.field "title" JD.string)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


decodeZkNoteSearchResult : JD.Decoder ZkNoteSearchResult
decodeZkNoteSearchResult =
    JD.map2 ZkNoteSearchResult
        (JD.field "notes" (JD.list decodeZkListNote))
        (JD.field "offset" JD.int)


decodeSavedZkNote : JD.Decoder SavedZkNote
decodeSavedZkNote =
    JD.map2 SavedZkNote
        (JD.field "id" JD.int)
        (JD.field "changeddate" JD.int)


decodeZkNote : JD.Decoder ZkNote
decodeZkNote =
    JD.succeed ZkNote
        |> andMap (JD.field "id" JD.int)
        |> andMap (JD.field "user" JD.int)
        |> andMap (JD.field "username" JD.string)
        |> andMap (JD.field "title" JD.string)
        |> andMap (JD.field "content" JD.string)
        |> andMap (JD.field "pubid" (JD.maybe JD.string))
        |> andMap (JD.field "editable" JD.bool)
        |> andMap (JD.field "editableValue" JD.bool)
        |> andMap (JD.field "createdate" JD.int)
        |> andMap (JD.field "changeddate" JD.int)


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
        (JD.field "userid" JD.int)
        (JD.field "name" JD.string)
        (JD.field "publicid" JD.int)
        (JD.field "shareid" JD.int)
        (JD.field "searchid" JD.int)


encodeImportZkNote : ImportZkNote -> JE.Value
encodeImportZkNote izn =
    JE.object
        [ ( "title", JE.string izn.title )
        , ( "content", JE.string izn.content )
        , ( "fromLinks", JE.list JE.string izn.fromLinks )
        , ( "toLinks", JE.list JE.string izn.toLinks )
        ]



----------------------------------------
-- misc functions
----------------------------------------


editNoteLink : Int -> String
editNoteLink noteid =
    UB.absolute [ "editnote", String.fromInt noteid ] []
