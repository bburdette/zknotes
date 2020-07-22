module UserInterface exposing (SendMsg(..), ServerResponse(..), encodeEmail, encodeSendMsg, serverResponseDecoder)

import Data
import Json.Decode as JD
import Json.Encode as JE



-- import Activity exposing (Activity)
-- import LogEntry exposing (LogEntry)
-- import Tag exposing (Tag)
-- import TagBase
--     exposing
--         ( PubTagBase
--         , TagBaseId(..)
--         , encodePtb
--         )
-- import TagBaseData exposing (TagBaseData, decodeTagBaseData, encodeTagBaseData)
-- import Thing exposing (Thing)
-- type Mtb
--     = User
--     | Public
-- type TbFor
--     = Open
--     | Merge


type SendMsg
    = Register String
    | Login



-- | SaveTagBase (List Activity) (List Thing) (List Tag) (List LogEntry) (List LogEntry) TagBaseId Int
-- | SaveMtbData JE.Value
-- | GetTagBase TagBaseId TbFor
-- | PublishTagBase PubTagBase
-- | UnpublishTagBase TagBaseId
-- | ImportTagBase PubTagBase
-- | CopyTagBase
--     { from : TagBaseId
--     , to : TagBaseId
--     }
-- | GetMtb Mtb


type ServerResponse
    = RegistrationSent
    | UserExists
    | UnregisteredUser
    | InvalidUserOrPwd
    | LoggedIn
    | EntryListing (List Data.BlogListEntry)
    | ServerError String



-- | TagBaseReceived TagBaseData TagBaseId TbFor (Maybe Int)
-- | MetaTagBaseReceived Mtb TagBase.TagBaseReceived
-- | MetaTagBaseNotFound
-- | MetaTagBaseSaved Int
-- | NoTagBaseReceived TagBaseId
-- | TagBaseSaved Int
-- | TagBaseCopied
-- | TagBasePublished
-- | TagBaseUnpublished
-- | TagBaseImported
--     { userTbId : TagBaseId
--     , publicTbId : TagBaseId
--     }
-- | BadSaveId
-- | ReceiveFail


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



-- SaveTagBase la lth lt ll lq tbid saveid ->
--     JE.object
--         [ ( "what", JE.string "savetagbase" )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         , ( "data", encodeTagBase la lth lt ll lq tbid saveid )
--         ]
-- SaveMtbData value ->
--     JE.object
--         [ ( "what", JE.string "saveusermtb" )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         , ( "data", value )
--         ]
-- GetTagBase (TagBaseId tbid) for ->
--     JE.object
--         [ ( "what", JE.string "gettagbase" )
--         , ( "data"
--           , JE.object
--                 [ ( "tbid", JE.int tbid )
--                 , ( "tbfor"
--                   , JE.string <|
--                         case for of
--                             Open ->
--                                 "open"
--                             Merge ->
--                                 "merge"
--                   )
--                 ]
--           )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         ]
-- CopyTagBase tbinfo ->
--     let
--         (TagBaseId from) =
--             tbinfo.from
--         (TagBaseId to) =
--             tbinfo.to
--     in
--     JE.object
--         [ ( "what", JE.string "copytagbase" )
--         , ( "data"
--           , JE.object
--                 [ ( "fromid", JE.int from )
--                 , ( "toid", JE.int to )
--                 ]
--           )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         ]
-- PublishTagBase ptb ->
--     JE.object
--         [ ( "what", JE.string "publishtagbase" )
--         , ( "data", encodePtb ptb )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         ]
-- UnpublishTagBase tbid ->
--     let
--         (TagBaseId id) =
--             tbid
--     in
--     JE.object
--         [ ( "what", JE.string "unpublishtagbase" )
--         , ( "data", JE.int id )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         ]
-- ImportTagBase ptb ->
--     JE.object
--         [ ( "what", JE.string "importtagbase" )
--         , ( "data", encodePtb ptb )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         ]
-- GetMtb mtb ->
--     JE.object
--         [ ( "what", JE.string "getmtb" )
--         , ( "data"
--           , JE.string
--                 (case mtb of
--                     User ->
--                         "user"
--                     Public ->
--                         "public"
--                 )
--           )
--         , ( "uid", JE.string uid )
--         , ( "pwd", JE.string pwd )
--         ]
{- encodeTagBase : List Activity -> List Thing -> List Tag -> List LogEntry -> List LogEntry -> TagBaseId -> Int -> JE.Value
   encodeTagBase a thgs t l q (TagBaseId tbid) saveid =
       JE.object
           [ ( "tagbase", encodeTagBaseData a thgs t l q )
           , ( "tbid", JE.int tbid )
           , ( "tbfor", JE.string "" )
           , ( "saveid", JE.int saveid )
           ]


   decodeTagBase : JD.Decoder ServerResponse
   decodeTagBase =
       JD.map4 TagBaseReceived
           (JD.at [ "content", "tagbase" ] decodeTagBaseData)
           (JD.at [ "content", "tbid" ] (JD.map TagBaseId JD.int))
           (JD.at [ "content", "tbfor" ]
               (JD.string
                   |> JD.andThen
                       (\tbfor ->
                           case tbfor of
                               "open" ->
                                   JD.succeed Open

                               "merge" ->
                                   JD.succeed Merge

                               _ ->
                                   JD.fail "invalid tbfor code"
                       )
               )
           )
           (JD.field "content" (JD.maybe (JD.field "saveid" JD.int)))

-}


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

                -- "tagbase" ->
                --     decodeTagBase
                -- "usermtbnotfound" ->
                --     JD.succeed MetaTagBaseNotFound
                -- "usermtb" ->
                --     JD.map (MetaTagBaseReceived User) TagBase.decodeTbr
                -- "publicmtb" ->
                --     JD.map (MetaTagBaseReceived Public) TagBase.decodeTbr
                -- "nodata" ->
                --     JD.map NoTagBaseReceived
                --         (JD.map TagBaseId
                --             (JD.at [ "content", "tbid" ] JD.int)
                --         )
                -- "tagbase saved" ->
                --     JD.map TagBaseSaved
                --         (JD.field "content" JD.int)
                -- "mtb saved" ->
                --     JD.map MetaTagBaseSaved
                --         (JD.field "content" JD.int)
                -- "tagbase copied" ->
                --     JD.succeed TagBaseCopied
                -- "tagbase published" ->
                --     JD.succeed TagBasePublished
                -- "tagbase unpublished" ->
                --     JD.succeed TagBaseUnpublished
                -- "tagbase imported" ->
                --     JD.map2 (\u p -> TagBaseImported { userTbId = u, publicTbId = p })
                --         (JD.at [ "content", "usertbid" ] (JD.map TagBaseId JD.int))
                --         (JD.at [ "content", "publictbid" ] (JD.map TagBaseId JD.int))
                -- "save failed - bad save id!" ->
                --     JD.succeed BadSaveId
                wat ->
                    JD.succeed
                        (ServerError ("invalid 'what' from server: " ++ wat))
        )
        (JD.at [ "what" ]
            JD.string
        )
