module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, serverResponseDecoder)

import Data
import Json.Decode as JD
import Json.Encode as JE
import Util


type SendMsg
    = GetBlogEntry Int


type ServerResponse
    = ServerError String
    | BlogEntry Data.FullBlogEntry


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetBlogEntry beid ->
            JE.object
                [ ( "what", JE.string "getblogentry" )
                , ( "data", JE.int beid )
                ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "blogentry" ->
                    JD.map BlogEntry (JD.at [ "content" ] <| Data.decodeFullBlogEntry)

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
