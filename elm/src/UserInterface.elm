module UserInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder)

import Data
import Json.Decode as JD
import Json.Encode as JE


type SendMsg
    = Register String
    | Login
    | GetListing
    | GetZkNote Int
    | DeleteZkNote Int
    | SaveZkNote Data.SaveZkNote
    | SaveZk Data.SaveZk


type ServerResponse
    = RegistrationSent
    | UserExists
    | UnregisteredUser
    | InvalidUserOrPwd
    | LoggedIn
    | ZkNoteListing (List Data.ZkListNote)
    | ZkListing (List Data.Zk)
    | SavedZkNote Int
    | SavedZk Int
    | DeletedZkNote Int
    | ZkNote Data.FullZkNote
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
                [ ( "what", JE.string "getzklisting" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                ]

        GetZkNote id ->
            JE.object
                [ ( "what", JE.string "getzknote" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.int id )
                ]

        DeleteZkNote id ->
            JE.object
                [ ( "what", JE.string "deletezknote" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", JE.int id )
                ]

        SaveZkNote sbe ->
            JE.object
                [ ( "what", JE.string "savezknote" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeSaveZkNote sbe )
                ]

        SaveZk sbe ->
            JE.object
                [ ( "what", JE.string "savezk" )
                , ( "uid", JE.string uid )
                , ( "pwd", JE.string pwd )
                , ( "data", Data.encodeSaveZk sbe )
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

                "zklisting" ->
                    JD.map ZkListing (JD.at [ "content" ] <| JD.list Data.decodeZk)

                "zknotelisting" ->
                    JD.map ZkNoteListing (JD.at [ "content" ] <| JD.list Data.decodeZkListNote)

                "savedzk" ->
                    JD.map SavedZk (JD.at [ "content" ] <| JD.int)

                "savedzknote" ->
                    JD.map SavedZkNote (JD.at [ "content" ] <| JD.int)

                "deletedzknote" ->
                    JD.map DeletedZkNote (JD.at [ "content" ] <| JD.int)

                "zknote" ->
                    JD.map ZkNote (JD.at [ "content" ] <| Data.decodeFullZkNote)

                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
