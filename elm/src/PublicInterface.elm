module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, getErrorIndexNote, serverResponseDecoder)

import Data exposing (ZkNoteId)
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
                [ ( "what", JE.string "GetZkNoteAndLinks" )
                , ( "data", Data.encodeGetZkNoteEdit x )
                ]

        GetZkNotePubId pubid ->
            JE.object
                [ ( "what", JE.string "GetZkNotePubId" )
                , ( "data", JE.string pubid )
                ]

        GetZnlIfChanged x ->
            JE.object
                [ ( "what", JE.string "GetZnlIfChanged" )
                , ( "data", Data.encodeGetZnlIfChanged x )
                ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "ZkNoteAndLinks" ->
                    JD.map ZkNoteAndLinks
                        (JD.at [ "content" ] <| Data.decodeZkNoteEditWhat)

                "ServerError" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                "Noop" ->
                    JD.succeed Noop

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )


getErrorIndexNote : String -> ZkNoteId -> (Result Http.Error ServerResponse -> msg) -> Cmd msg
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
