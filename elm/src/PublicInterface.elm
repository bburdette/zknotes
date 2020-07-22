module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, serverResponseDecoder)

import Json.Decode as JD
import Json.Encode as JE
import Util


type SendMsg
    = GetBloag String Int


type ServerResponse
    = ReceiveFail
    | ServerError String


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


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
