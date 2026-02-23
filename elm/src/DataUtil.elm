module DataUtil exposing (..)

import Data exposing (..)
import Http
import Http.Tasks as HT
import Json.Decode as JD
import Json.Encode as JE
import Orgauth.Data exposing (UserId(..), userIdDecoder)
import TDict exposing (TDict)
import TSet exposing (TSet)
import Task
import UUID exposing (UUID)
import Url.Builder as UB
import Util exposing (andMap)


type alias FileUrlInfo =
    { location : String
    , filelocation : String
    , tauri : Bool
    }


getPrNoteInfo : PublicReply -> Maybe ( ZkNoteId, Maybe String )
getPrNoteInfo pr =
    case pr of
        PbyServerError publicError ->
            case publicError of
                PbeString _ ->
                    Nothing

                PbeNoteNotFound publicRequest ->
                    getPrqNoteInfo publicRequest
                        |> Maybe.map (\( l, r ) -> ( l, Just r ))

                PbeNoteIsPrivate publicRequest ->
                    getPrqNoteInfo publicRequest
                        |> Maybe.map (\( l, r ) -> ( l, Just r ))

        PbyZkNoteAndLinks zkNoteAndLinks ->
            Just ( zkNoteAndLinks.zknote.id, Nothing )

        PbyZkNoteAndLinksWhat zkNoteAndLinksWhat ->
            Just ( zkNoteAndLinksWhat.znl.zknote.id, Just zkNoteAndLinksWhat.what )

        PbyNoop ->
            Nothing


getPrqNoteInfo : PublicRequest -> Maybe ( ZkNoteId, String )
getPrqNoteInfo pr =
    case pr of
        PbrGetZkNoteAndLinks getZkNoteAndLinks ->
            Just ( getZkNoteAndLinks.zknote, getZkNoteAndLinks.what )

        PbrGetZnlIfChanged getZnlIfChanged ->
            Just ( getZnlIfChanged.zknote, getZnlIfChanged.what )

        PbrGetZkNotePubId _ ->
            Nothing


zkNoteIdToString : ZkNoteId -> String
zkNoteIdToString id =
    case id of
        Zni zni ->
            zni

        ArchiveZni zni _ ->
            zni


zkNoteIdFromString : String -> Result UUID.Error ZkNoteId
zkNoteIdFromString zni =
    UUID.fromString
        zni
        |> Result.map (\_ -> Zni zni)


trustedZkNoteIdFromString : String -> ZkNoteId
trustedZkNoteIdFromString zni =
    Zni zni


zkNoteIdFromUUID : UUID -> ZkNoteId
zkNoteIdFromUUID zni =
    Zni (UUID.toString zni)


type alias ZniSet =
    TSet ZkNoteId String


emptyZniSet : ZniSet
emptyZniSet =
    TSet.empty zkNoteIdToString trustedZkNoteIdFromString


type alias ZlnDict =
    TDict ZkNoteId String ZkListNote


emptyZlnDict : ZlnDict
emptyZlnDict =
    TDict.empty zkNoteIdToString trustedZkNoteIdFromString


type alias ZniDict =
    TDict ZkNoteId String Data.ZkNote


emptyZniDict : ZniDict
emptyZniDict =
    TDict.empty zkNoteIdToString trustedZkNoteIdFromString


zniEq : ZkNoteId -> ZkNoteId -> Bool
zniEq li ri =
    case ( li, ri ) of
        ( Zni l, Zni r ) ->
            l == r

        ( ArchiveZni l _, ArchiveZni r _ ) ->
            l == r

        _ ->
            False


zniCompare : ZkNoteId -> ZkNoteId -> Order
zniCompare li ri =
    let
        l =
            case li of
                Zni i ->
                    i

                ArchiveZni i _ ->
                    i

        r =
            case ri of
                Zni i ->
                    i

                ArchiveZni i _ ->
                    i
    in
    compare l r


type alias LoginData =
    { userid : UserId
    , uuid : String
    , name : String
    , email : String
    , admin : Bool
    , active : Bool
    , remoteUrl : Maybe String
    , zknote : ZkNoteId
    , homenote : Maybe ZkNoteId
    , server : String
    }


decodeLoginData : JD.Decoder LoginData
decodeLoginData =
    JD.succeed LoginData
        |> andMap (JD.field "userid" userIdDecoder)
        |> andMap (JD.field "uuid" JD.string)
        |> andMap (JD.field "name" JD.string)
        |> andMap (JD.field "email" JD.string)
        |> andMap (JD.field "admin" JD.bool)
        |> andMap (JD.field "active" JD.bool)
        |> andMap (JD.field "remote_url" (JD.maybe JD.string))
        |> andMap (JD.field "data" (JD.field "zknote" zkNoteIdDecoder))
        |> andMap (JD.field "data" (JD.field "homenote" (JD.maybe zkNoteIdDecoder)))
        |> andMap (JD.field "data" (JD.field "server" JD.string))


type alias Sysids =
    { publicid : ZkNoteId
    , shareid : ZkNoteId
    , searchid : ZkNoteId
    , commentid : ZkNoteId
    , archiveid : ZkNoteId
    }


sysids : Sysids
sysids =
    { publicid = Zni "f596bc2c-a882-4c1c-b739-8c4e25f34eb2"
    , commentid = Zni "e82fefee-bcd3-4e2e-b350-9963863e516d"
    , shareid = Zni "466d39ec-2ea7-4d43-b44c-1d3d083f8a9d"
    , searchid = Zni "84f72fd0-8836-43a3-ac66-89e0ab49dd87"
    , archiveid = Zni "ad6a4ca8-0446-4ecc-b047-46282ced0d84"

    -- , userid = ZkNoteId "4fb37d76-6fc8-4869-8ee4-8e05fa5077f7"
    -- , systemid = ZkNoteId "0efcc98f-dffd-40e5-af07-90da26b1d469"
    -- , syncid = ZkNoteId "528ccfc2-8488-41e0-a4e1-cbab6406674e"
    }


fromOaLd : Orgauth.Data.LoginData -> Result JD.Error LoginData
fromOaLd oald =
    oald.data
        |> Result.fromMaybe (JD.Failure "no login data" JE.null)
        |> Result.andThen
            (JD.decodeString
                (JD.succeed (LoginData oald.userid oald.uuid oald.name oald.email oald.admin oald.active oald.remoteUrl)
                    |> andMap (JD.field "zknote" Data.zkNoteIdDecoder)
                    |> andMap (JD.field "homenote" (JD.maybe Data.zkNoteIdDecoder))
                    |> andMap (JD.field "server" JD.string)
                )
            )


toOaLd : LoginData -> Orgauth.Data.LoginData
toOaLd ld =
    { userid = ld.userid
    , uuid = ld.uuid
    , name = ld.name
    , email = ld.email
    , admin = ld.admin
    , active = ld.active
    , remoteUrl = ld.remoteUrl
    , data =
        Just <|
            JE.encode 2
                (JE.object
                    [ ( "zknote", zkNoteIdEncoder ld.zknote )
                    , ( "homenote", (Maybe.withDefault JE.null << Maybe.map zkNoteIdEncoder) ld.homenote )
                    ]
                )
    }


decodeSysids : JD.Decoder Sysids
decodeSysids =
    JD.succeed Sysids
        |> andMap (JD.field "publicid" Data.zkNoteIdDecoder)
        |> andMap (JD.field "shareid" Data.zkNoteIdDecoder)
        |> andMap (JD.field "searchid" Data.zkNoteIdDecoder)
        |> andMap (JD.field "commentid" Data.zkNoteIdDecoder)
        |> andMap (JD.field "archiveid" Data.zkNoteIdDecoder)


toZkLink : ZkNoteId -> UserId -> EditLink -> ZkLink
toZkLink noteid user el =
    { from =
        case el.direction of
            From ->
                el.otherid

            To ->
                noteid
    , to =
        case el.direction of
            From ->
                noteid

            To ->
                el.otherid
    , user = user
    , linkzknote = Nothing
    , fromname = Nothing
    , toname = Nothing
    , delete = Nothing
    }


zklKey : { a | otherid : ZkNoteId, direction : Direction } -> String
zklKey zkl =
    zkNoteIdToString zkl.otherid
        ++ ":"
        ++ (case zkl.direction of
                From ->
                    "from"

                To ->
                    "to"
           )


lzlKey : { a | from : ZkNoteId, to : ZkNoteId } -> String
lzlKey lzl =
    zkNoteIdToString lzl.from
        ++ ":"
        ++ zkNoteIdToString lzl.to


elToSzl : EditLink -> SaveZkLink
elToSzl el =
    { otherid = el.otherid
    , direction = el.direction
    , user = el.user
    , zknote = el.zknote
    , delete = el.delete
    }


elToSzl2 : ZkNoteId -> EditLink -> SaveZkLink2
elToSzl2 thisid el =
    let
        ( from, to ) =
            case el.direction of
                From ->
                    ( el.otherid, thisid )

                To ->
                    ( thisid, el.otherid )
    in
    { from = from
    , to = to
    , linkzknote = Nothing
    , delete = el.delete
    }


lzlToSll : Bool -> LzLink -> SaveLzLink
lzlToSll delete lzl =
    { from = lzl.from
    , to = lzl.to
    , delete = Just delete
    }


saveZkNote : ZkNote -> SaveZkNote
saveZkNote fzn =
    { id = Just fzn.id
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
    , editable = fzn.editableValue
    , showtitle = fzn.showtitle
    , deleted = fzn.deleted
    , what = Nothing
    }


jobComplete : JobState -> Bool
jobComplete js =
    case js of
        Started ->
            False

        Running ->
            False

        -- TODO: fix.  should contain a count or something?
        Completed ->
            True

        Failed ->
            True


editNoteLink : ZkNoteId -> String
editNoteLink noteid =
    case noteid of
        Zni uuid ->
            UB.absolute [ "editnote", uuid ] []

        ArchiveZni uuid parentnoteid ->
            UB.absolute [ "archivenote", parentnoteid, uuid ] []


archiveNoteLink : ZkNoteId -> ZkNoteId -> String
archiveNoteLink parentnoteid noteid =
    UB.absolute [ "archivenote", zkNoteIdToString parentnoteid, zkNoteIdToString noteid ] []


flipDirection : Direction -> Direction
flipDirection direction =
    case direction of
        To ->
            From

        From ->
            To


flipOrderDirection : OrderDirection -> OrderDirection
flipOrderDirection od =
    case od of
        Ascending ->
            Descending

        Descending ->
            Ascending


type alias OrderedTagSearch =
    { ts : TagSearch
    , ordering : Maybe Ordering
    , archives : ArchivesOrCurrent
    }



------------------------------------------------------------
-- getting, setting text selections in text edit areas.
------------------------------------------------------------


type alias SetSelection =
    { id : String
    , offset : Int
    , length : Int
    }


type alias TASelection =
    -- 'Text Area Selection'
    { text : String
    , offset : Int
    , what : String
    }


type alias TAError =
    { what : String
    }


encodeSetSelection : SetSelection -> JE.Value
encodeSetSelection s =
    JE.object
        [ ( "id", JE.string s.id )
        , ( "offset", JE.int s.offset )
        , ( "length", JE.int s.length )
        ]


decodeTASelection : JD.Decoder TASelection
decodeTASelection =
    JD.succeed TASelection
        |> andMap (JD.field "text" JD.string)
        |> andMap (JD.field "offset" JD.int)
        |> andMap (JD.field "what" JD.string)


decodeTAError : JD.Decoder TAError
decodeTAError =
    JD.succeed TAError
        |> andMap (JD.field "what" JD.string)


type alias ZkInviteData =
    List SaveZkLink


encodeZkInviteData : ZkInviteData -> JE.Value
encodeZkInviteData zid =
    JE.list saveZkLinkEncoder zid


showPrivateReply : PrivateReply -> String
showPrivateReply pr =
    privateReplyEncoder pr
        |> JE.encode 2


showPrivateClosureReply : PrivateClosureReply -> String
showPrivateClosureReply pr =
    privateClosureReplyEncoder pr
        |> JE.encode 2


showAdminResponse : Orgauth.Data.AdminResponse -> String
showAdminResponse pr =
    Orgauth.Data.adminResponseEncoder pr
        |> JE.encode 2


showPrivateError : PrivateError -> String
showPrivateError pe =
    case pe of
        PveString s ->
            s

        PveNoteNotFound _ ->
            "note not found"

        PveNoteIsPrivate _ ->
            "note is private"

        PveNotLoggedIn ->
            "not logged in"

        PveLoginError e ->
            "login error: " ++ e


showPublicError : PublicError -> String
showPublicError pe =
    case pe of
        PbeString s ->
            s

        PbeNoteNotFound _ ->
            "note not found"

        PbeNoteIsPrivate _ ->
            "note is private"


getErrorIndexNote : String -> ZkNoteId -> (Result Http.Error PublicReply -> msg) -> Cmd msg
getErrorIndexNote location noteid tomsg =
    HT.post
        { url = location ++ "/public"
        , body =
            Http.jsonBody <|
                Data.publicRequestEncoder
                    (PbrGetZkNoteAndLinks
                        { zknote = noteid
                        , what = ""
                        , edittab = Nothing
                        }
                    )
        , resolver =
            HT.resolveJson
                publicReplyDecoder
        }
        |> Task.attempt tomsg
