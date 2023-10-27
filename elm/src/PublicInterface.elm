module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, getErrorIndexNote, serverResponseDecoder)

import Data
import Http
import Http.Tasks as HT
import Json.Decode as JD
import Json.Encode as JE
import MdCommon as MC
import Task exposing (Task)
import Util


type SendMsg
    = GetZkNoteAndLinks Data.GetZkNoteAndLinks
    | GetZkNotePubId String
    | GetZnlIfChanged Data.GetZnlIfChanged


type ServerResponse
    = ServerError String
    | ZkNoteAndLinks Data.ZkNoteAndLinksWhat
    | Noop


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetZkNoteAndLinks x ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "data", Data.encodeGetZkNoteEdit x )
                ]

        GetZkNotePubId pubid ->
            JE.object
                [ ( "what", JE.string "getzknotepubid" )
                , ( "data", JE.string pubid )
                ]

        GetZnlIfChanged x ->
            JE.object
                [ ( "what", JE.string "getzneifchanged" )
                , ( "data", Data.encodeGetZneIfChanged x )
                ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "zknote" ->
                    JD.map ZkNoteAndLinks
                        (JD.at [ "content" ] <| Data.decodeZkNoteEditWhat)

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                "noop" ->
                    JD.succeed Noop

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )


getErrorIndexNote : String -> Int -> (Result Http.Error ServerResponse -> msg) -> Cmd msg
getErrorIndexNote location noteid tomsg =
    HT.post
        { url = location ++ "/public"
        , body =
            Http.jsonBody <|
                encodeSendMsg
                    (GetZkNoteAndLinks
                        { zknote = noteid
                        , what = ""
                        }
                    )
        , resolver =
            HT.resolveJson
                serverResponseDecoder
        }
        |> Task.attempt tomsg
