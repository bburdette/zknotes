module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, serverResponseDecoder)

import Data
import Json.Decode as JD
import Json.Encode as JE
import Util


type SendMsg
    = GetZkNote Int


type ServerResponse
    = ServerError String
    | ZkNote Data.FullZkNote


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetZkNote beid ->
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
                    JD.map ZkNote (JD.at [ "content" ] <| Data.decodeFullZkNote)

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
