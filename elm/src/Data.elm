module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE


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


type alias ZkListNote =
    { id : Int
    , title : String
    , user : Int
    , createdate : Int
    , changeddate : Int
    }


type alias FullZkNote =
    { id : Int
    , zk : Int
    , title : String
    , content : String
    , user : Int
    , createdate : Int
    , changeddate : Int
    }


type alias SaveZkNote =
    { id : Maybe Int
    , zk : Int
    , title : String
    , content : String
    }


encodeSaveZkNote : SaveZkNote -> JE.Value
encodeSaveZkNote zkn =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", JE.int id ) ]) zkn.id
            |> Maybe.withDefault []
        )
            ++ [ ( "zk", JE.int zkn.zk )
               , ( "title", JE.string zkn.title )
               , ( "content", JE.string zkn.content )
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


type alias Login =
    { uid : String
    , pwd : String
    }


decodeZk : JD.Decoder Zk
decodeZk =
    JD.map5 Zk
        (JD.field "id" JD.int)
        (JD.field "name" JD.string)
        (JD.field "description" JD.string)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.map5 ZkListNote
        (JD.field "id" JD.int)
        (JD.field "title" JD.string)
        (JD.field "zk" JD.int)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


decodeFullZkNote : JD.Decoder FullZkNote
decodeFullZkNote =
    JD.map7 FullZkNote
        (JD.field "id" JD.int)
        (JD.field "zk" JD.int)
        (JD.field "title" JD.string)
        (JD.field "content" JD.string)
        (JD.field "zk" JD.int)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)
