module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE


type alias ZkList =
    { id : Int
    , name : String
    , description : String
    , createdate : Int
    , changeddate : Int
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
    , title : String
    , content : String
    , user : Int
    , createdate : Int
    , changeddate : Int
    }


type alias SaveZkNote =
    { id : Maybe Int
    , title : String
    , content : String
    }


encodeSaveZkNote : SaveZkNote -> JE.Value
encodeSaveZkNote sbe =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", JE.int id ) ]) sbe.id
            |> Maybe.withDefault []
        )
            ++ [ ( "title", JE.string sbe.title )
               , ( "content", JE.string sbe.content )
               ]


type alias Login =
    { uid : String
    , pwd : String
    }


decodeZkList : JD.Decoder ZkList
decodeZkList =
    JD.map5 ZkList
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
    JD.map6 FullZkNote
        (JD.field "id" JD.int)
        (JD.field "title" JD.string)
        (JD.field "content" JD.string)
        (JD.field "zk" JD.int)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)
