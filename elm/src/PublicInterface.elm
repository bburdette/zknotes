module PublicInterface exposing (SendMsg(..), ServerResponse(..), encodeSendMsg, getPublicZkNote, serverResponseDecoder)

import MdCommon as MC
import Data
import Http
import Http.Tasks as HT
import Json.Decode as JD
import Json.Encode as JE
import Task exposing (Task)
import Util


type SendMsg
    = GetZkNote Int
    | GetZkNotePubId String


type ServerResponse
    = ServerError String
    | ZkNote Data.ZkNoteEdit


encodeSendMsg : SendMsg -> JE.Value
encodeSendMsg sm =
    case sm of
        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "data", JE.int id )
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
                    JD.map ZkNote (JD.at [ "content" ] <| Data.decodeZkNoteEdit)

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )


firstTask : String -> JE.Value -> Task Http.Error ServerResponse
firstTask location jsonBody =
    HT.post
        { url = location ++ "/public"
        , body = Http.jsonBody jsonBody
        , resolver = HT.resolveJson serverResponseDecoder
        }


secondTask : String -> ServerResponse -> Task Http.Error ServerResponse
secondTask location sr =
    case sr of
        ZkNote zknoteedit ->
            zknoteedit.zknote.content
                |> MC.mdPanel
                |> Maybe.map
                    (\panel ->
                        HT.post
                            { url = location ++ "/public"
                            , body = Http.jsonBody <| encodeSendMsg (GetZkNote panel.noteid)
                            , resolver =
                                HT.resolveJson
                                    (serverResponseDecoder
                                        |> JD.andThen
                                            (\sr2 ->
                                                case sr2 of
                                                    ZkNote panelnoteedit ->
                                                        JD.succeed (ZkNote { zknoteedit | panelNote = Just panelnoteedit.zknote })

                                                    _ ->
                                                        JD.succeed sr2
                                            )
                                    )
                            }
                    )
                |> Maybe.withDefault
                    (Task.succeed <|
                        ZkNote zknoteedit
                    )

        _ ->
            Task.succeed sr


getPublicZkNote : String -> JE.Value -> (Result Http.Error ServerResponse -> msg) -> Cmd msg
getPublicZkNote location jsonBody tomsg =
    firstTask location jsonBody
        |> Task.andThen (\sr -> secondTask location sr)
        |> Task.attempt tomsg
