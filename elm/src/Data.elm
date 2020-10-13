module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE
import Search as S


type alias Login =
    { uid : String
    , pwd : String
    }


type alias Zk =
    { id : Int
    , name : String
    , description : String
    , createdate : Int
    , changeddate : Int
    }


type alias SaveZk =
    { id : Maybe Int
    , name : String
    , description : String
    }


type alias ZkMember =
    { name : String
    , zkid : Int
    }


type alias ZkListNote =
    { id : Int
    , title : String
    , zk : Int
    , public : Bool
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


type alias FullZkNote =
    { id : Int
    , zk : Int
    , title : String
    , content : String
    , public : Bool
    , pubid : Maybe String
    , createdate : Int
    , changeddate : Int
    }


type alias SaveZkNote =
    { id : Maybe Int
    , zk : Int
    , public : Bool
    , pubid : Maybe String
    , title : String
    , content : String
    }


type alias ZkLink =
    { from : Int
    , to : Int
    , zknote : Maybe Int
    , fromname : Maybe String
    , toname : Maybe String
    , delete : Maybe Bool
    }


type alias ZkLinks =
    { zk : Int
    , links : List ZkLink
    }


type alias SaveZkLinks =
    { zk : Int
    , saveLinks : List ZkLink
    , deleteLinks : List ZkLink
    }


type alias GetZkLinks =
    { zknote : Int
    , zk : Int
    }


encodeGetZkLinks : GetZkLinks -> JE.Value
encodeGetZkLinks gzl =
    JE.object
        [ ( "zknote", JE.int gzl.zknote )
        , ( "zk", JE.int gzl.zk )
        ]


encodeZkLinks : ZkLinks -> JE.Value
encodeZkLinks zklinks =
    JE.object
        [ ( "zk", JE.int zklinks.zk )
        , ( "links", JE.list encodeZkLink zklinks.links )
        ]


decodeZkLinks : JD.Decoder ZkLinks
decodeZkLinks =
    JD.map2 ZkLinks
        (JD.field "zk" JD.int)
        (JD.field "links" (JD.list decodeZkLink))


encodeZkLink : ZkLink -> JE.Value
encodeZkLink zklink =
    JE.object <|
        [ ( "from", JE.int zklink.from )
        , ( "to", JE.int zklink.to )
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
    JD.map6 ZkLink
        (JD.field "from" JD.int)
        (JD.field "to" JD.int)
        (JD.maybe (JD.field "linkzknote" JD.int))
        (JD.maybe (JD.field "fromname" JD.string))
        (JD.maybe (JD.field "toname" JD.string))
        (JD.succeed Nothing)


saveZkNoteFromFull : FullZkNote -> SaveZkNote
saveZkNoteFromFull fzn =
    { id = Just fzn.id
    , zk = fzn.zk
    , public = fzn.public
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
    }


encodeSaveZkNote : SaveZkNote -> JE.Value
encodeSaveZkNote zkn =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", JE.int id ) ]) zkn.id
            |> Maybe.withDefault []
        )
            ++ (Maybe.map (\pubid -> [ ( "pubid", JE.string pubid ) ]) zkn.pubid
                    |> Maybe.withDefault []
               )
            ++ [ ( "zk", JE.int zkn.zk )
               , ( "title", JE.string zkn.title )
               , ( "content", JE.string zkn.content )
               , ( "public", JE.bool zkn.public )
               ]


encodeSaveZk : SaveZk -> JE.Value
encodeSaveZk sbe =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", JE.int id ) ]) sbe.id
            |> Maybe.withDefault []
        )
            ++ [ ( "name", JE.string sbe.name )
               , ( "description", JE.string sbe.description )
               ]


decodeZk : JD.Decoder Zk
decodeZk =
    JD.map5 Zk
        (JD.field "id" JD.int)
        (JD.field "name" JD.string)
        (JD.field "description" JD.string)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


encodeZkMember : ZkMember -> JE.Value
encodeZkMember zkm =
    JE.object
        [ ( "name", JE.string zkm.name )
        , ( "zkid", JE.int zkm.zkid )
        ]


decodeZkMember : JD.Decoder ZkMember
decodeZkMember =
    JD.map2 ZkMember
        (JD.field "name" JD.string)
        (JD.field "zkid" JD.int)


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.map6 ZkListNote
        (JD.field "id" JD.int)
        (JD.field "title" JD.string)
        (JD.field "zk" JD.int)
        (JD.field "public" JD.bool)
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


decodeFullZkNote : JD.Decoder FullZkNote
decodeFullZkNote =
    JD.map8 FullZkNote
        (JD.field "id" JD.int)
        (JD.field "zk" JD.int)
        (JD.field "title" JD.string)
        (JD.field "content" JD.string)
        (JD.field "public" JD.bool)
        (JD.field "pubid" (JD.maybe JD.string))
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)
