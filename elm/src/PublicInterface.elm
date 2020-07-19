module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, serverResponseDecoder)

-- import TagBase exposing (TagBaseId(..))
-- import TagBaseData exposing (TagBaseData, decodeTagBaseData)

import Json.Decode as JD
import Json.Encode as JE
import Util


type SendMsg
    = GetBloag String Int



-- = GetTagBase TagBaseId
-- | GetMtb


type ServerResponse
    = ReceiveFail
    | ServerError String



-- | TagBaseReceived TagBaseData TagBaseId String
-- | MetaTagBaseReceived TagBase.TagBaseReceived
-- | MetaTagBaseNotFound
-- | NoTagBaseReceived TagBaseId


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetBloag uid beid ->
            JE.object
                [ ( "what", JE.string "getbloag" )
                , ( "data"
                  , JE.object
                        [ ( "uid", JE.string uid )
                        , ( "beid", JE.int beid )
                        ]
                  )
                ]



-- GetMtb ->
--     JE.object
--         [ ( "what", JE.string "getmtb" )
--         ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                -- "tagbase" ->
                --     JD.map3 TagBaseReceived
                --         (JD.at [ "content", "tagbase" ] decodeTagBaseData)
                --         (JD.at [ "content", "tbid" ] (JD.map TagBaseId JD.int))
                --         (JD.at [ "content", "tbname" ] JD.string)
                -- "publicmtb" ->
                --     JD.map MetaTagBaseReceived TagBase.decodeTbr
                -- "nodata" ->
                --     JD.map NoTagBaseReceived
                --         (JD.map TagBaseId
                --             (JD.at [ "content", "tbid" ] JD.int)
                --         )
                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
