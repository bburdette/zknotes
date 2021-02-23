module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, serverResponseDecoder)

import Data
import Json.Decode as JD
import Json.Encode as JE
import UUID exposing (UUID)
import Util


type SendMsg
    = GetZkNote UUID
    | GetZkNotePubId String


type ServerResponse
    = ServerError String
    | ZkNote Data.ZkNoteAndAccomplices


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "data", UUID.toValue id )
                ]

        GetZkNotePubId pubid ->
            JE.object
                [ ( "what", JE.string "getzknotepubid" )
                , ( "data", JE.string pubid )
                ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "zknote" ->
                    JD.map ZkNote (JD.at [ "content" ] <| Data.decodeZkNoteAndAccomplices)

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
