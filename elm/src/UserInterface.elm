module UserInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder)

import Data
import Json.Decode as JD
import Json.Encode as JE


type SendMsg
    = Register String
    | Login
    | GetListing
    | GetBlogEntry Int
    | DeleteBlogEntry Int
    | SaveBlogEntry Data.SaveBlogEntry


type ServerResponse
    = RegistrationSent
    | UserExists
    | UnregisteredUser
    | InvalidUserOrPwd
    | LoggedIn
    | EntryListing (List Data.BlogListEntry)
    | SavedBlogEntry Int
    | DeletedBlogEntry Int
    | BlogEntry Data.FullBlogEntry
    | ServerError String


encodeSendMsg : SendMsg -> String -> String -> JE.Value
encodeSendMsg sm uid pwd =
    case sm of
        Register email ->
            JE.object
                [ ( "what", JE.string "register" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", encodeEmail email )
                ]

        Login ->
            JE.object
                [ ( "what", JE.string "login" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                ]

        GetListing ->
            JE.object
                [ ( "what", JE.string "getlisting" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                ]

        GetBlogEntry id ->
            JE.object
                [ ( "what", JE.string "getblogentry" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.int id )
                ]

        DeleteBlogEntry id ->
            JE.object
                [ ( "what", JE.string "deleteblogentry" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.int id )
                ]

        SaveBlogEntry sbe ->
            JE.object
                [ ( "what", JE.string "saveblogentry" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeSaveBlogEntry sbe )
                ]


encodeEmail : String -> JE.Value
encodeEmail email =
    JE.object
        [ ( "email", JE.string email )
        ]


serverResponseDecoder : JD.Decoder ServerResponse
serverResponseDecoder =
    JD.andThen
        (\what ->
            case what of
                "registration sent" ->
                    JD.succeed RegistrationSent

                "unregistered user" ->
                    JD.succeed UnregisteredUser

                "user exists" ->
                    JD.succeed UserExists

                "logged in" ->
                    JD.succeed LoggedIn

                "invalid user or pwd" ->
                    JD.succeed InvalidUserOrPwd

                "server error" ->
                    JD.map ServerError (JD.at [ "content" ] JD.string)

                "listing" ->
                    JD.map EntryListing (JD.at [ "content" ] <| JD.list Data.decodeBlogListEntry)

                "savedblogentry" ->
                    JD.map SavedBlogEntry (JD.at [ "content" ] <| JD.int)

                "deletedblogentry" ->
                    JD.map DeletedBlogEntry (JD.at [ "content" ] <| JD.int)

                "blogentry" ->
                    JD.map BlogEntry (JD.at [ "content" ] <| Data.decodeFullBlogEntry)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
