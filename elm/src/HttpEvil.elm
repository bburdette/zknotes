module HttpEvil exposing (getJsonTask, jsonResolver, jsonTask, postJsonTask, resolve)

import Http exposing (Error(..), Resolver, Response(..))
import Json.Decode as JD
import Json.Encode as JE
import Task exposing (Task)


jsonResolver : JD.Decoder a -> Resolver Error a
jsonResolver decoder =
    Http.stringResolver <|
        resolve
            (\string ->
                Result.mapError JD.errorToString (JD.decodeString decoder string)
            )


postJsonTask : JsonArgs a -> Task Error a
postJsonTask args =
    jsonTask "POST" args


getJsonTask : JsonArgs a -> Task Error a
getJsonTask args =
    jsonTask "GET" args


type alias JsonArgs a =
    { url : String, body : Http.Body, decoder : JD.Decoder a }


jsonTask : String -> JsonArgs a -> Task Error a
jsonTask method args =
    Http.task
        { method = method
        , headers = []
        , url = args.url
        , body = args.body
        , resolver = jsonResolver args.decoder
        , timeout = Nothing
        }


resolve : (body -> Result String a) -> Response body -> Result Error a
resolve toResult response =
    case response of
        BadUrl_ url ->
            Err (BadUrl url)

        Timeout_ ->
            Err Timeout

        NetworkError_ ->
            Err NetworkError

        BadStatus_ metadata _ ->
            Err (BadStatus metadata.statusCode)

        GoodStatus_ _ body ->
            Result.mapError BadBody (toResult body)
