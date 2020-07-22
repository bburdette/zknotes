module Data exposing (..)

import Json.Decode as JD


type alias BlogListEntry =
    { id : Int
    , title : String
    , user : Int
    , createdate : Int
    , changeddate : Int
    }


decodeBlogListEntry : JD.Decoder BlogListEntry
decodeBlogListEntry =
    JD.map5 BlogListEntry
        (JD.field "id" JD.int)
        (JD.field "title" JD.string)
        (JD.field "user" JD.int)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


type alias Login =
    { uid : String
    , pwd : String
    }
