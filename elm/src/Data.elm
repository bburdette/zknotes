module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE


type alias BlogListEntry =
    { id : Int
    , title : String
    , user : Int
    , createdate : Int
    , changeddate : Int
    }


type alias FullBlogEntry =
    { id : Int
    , title : String
    , content : String
    , user : Int
    , createdate : Int
    , changeddate : Int
    }


type alias SaveBlogEntry =
    { id : Maybe Int
    , title : String
    , content : String
    }


encodeSaveBlogEntry : SaveBlogEntry -> JE.Value
encodeSaveBlogEntry sbe =
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


decodeBlogListEntry : JD.Decoder BlogListEntry
decodeBlogListEntry =
    JD.map5 BlogListEntry
        (JD.field "id" JD.int)
        (JD.field "title" JD.string)
        (JD.field "user" JD.int)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)


decodeFullBlogEntry : JD.Decoder FullBlogEntry
decodeFullBlogEntry =
    JD.map6 FullBlogEntry
        (JD.field "id" JD.int)
        (JD.field "title" JD.string)
        (JD.field "content" JD.string)
        (JD.field "user" JD.int)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)
