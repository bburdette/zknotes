module Route exposing (Route(..), parseUrl, routeTitle, routeUrl)

import Data exposing (ZkNoteId, zkNoteIdFromString, zkNoteIdToString)
import UUID exposing (UUID)
import Url exposing (Url)
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))


type Route
    = LoginR
    | PublicZkNote ZkNoteId
    | PublicZkPubId String
    | EditZkNoteR ZkNoteId
    | EditZkNoteNew
    | ArchiveNoteListingR ZkNoteId
    | ArchiveNoteR ZkNoteId ZkNoteId
    | ResetPasswordR String UUID
    | SettingsR
    | Invite String
    | Top


routeTitle : Route -> String
routeTitle route =
    case route of
        LoginR ->
            "login"

        PublicZkNote id ->
            "zknote " ++ zkNoteIdToString id

        PublicZkPubId id ->
            id ++ " - zknotes"

        EditZkNoteR id ->
            "zknote " ++ zkNoteIdToString id

        EditZkNoteNew ->
            "new zknote"

        ArchiveNoteListingR id ->
            "archives " ++ zkNoteIdToString id

        ArchiveNoteR id aid ->
            "archive " ++ zkNoteIdToString id ++ ": " ++ zkNoteIdToString aid

        ResetPasswordR _ _ ->
            "password reset"

        SettingsR ->
            "user settings"

        Invite _ ->
            "user invite"

        Top ->
            "zknotes"


parseUrl : Url -> Maybe Route
parseUrl url =
    UP.parse
        (UP.oneOf
            [ UP.map LoginR <|
                UP.s
                    "login"
            , UP.map PublicZkNote <|
                UP.s
                    "note"
                    </> UP.custom "ZkNoteId" (zkNoteIdFromString >> Result.toMaybe)
            , UP.map (\i -> PublicZkPubId (Maybe.withDefault "" (Url.percentDecode i))) <|
                UP.s
                    "page"
                    </> UP.string
            , UP.map ArchiveNoteListingR <|
                UP.s
                    "archivelisting"
                    </> UP.custom "ZkNoteId" (zkNoteIdFromString >> Result.toMaybe)
            , UP.map ArchiveNoteR <|
                UP.s
                    "archivenote"
                    </> UP.custom "ZkNoteId" (zkNoteIdFromString >> Result.toMaybe)
                    </> UP.custom "ZkNoteId" (zkNoteIdFromString >> Result.toMaybe)
            , UP.map EditZkNoteR <|
                UP.s
                    "editnote"
                    </> UP.custom "ZkNoteId" (zkNoteIdFromString >> Result.toMaybe)
            , UP.map EditZkNoteNew <|
                UP.s
                    "editnote"
                    </> UP.s "new"
            , UP.map ResetPasswordR <|
                UP.s
                    "reset"
                    </> UP.string
                    </> UP.custom "UUID" (UUID.fromString >> Result.toMaybe)
            , UP.map SettingsR <|
                UP.s
                    "settings"
            , UP.map Invite <|
                UP.s
                    "invite"
                    </> UP.string
            , UP.map Top <| UP.top
            ]
        )
        url


routeUrl : Route -> String
routeUrl route =
    case route of
        LoginR ->
            UB.absolute [ "login" ] []

        PublicZkNote uuid ->
            UB.absolute [ "note", zkNoteIdToString uuid ] []

        PublicZkPubId pubid ->
            UB.absolute [ "page", pubid ] []

        EditZkNoteR uuid ->
            UB.absolute [ "editnote", zkNoteIdToString uuid ] []

        EditZkNoteNew ->
            UB.absolute [ "editnote", "new" ] []

        ArchiveNoteListingR uuid ->
            UB.absolute [ "archivelisting", zkNoteIdToString uuid ] []

        ArchiveNoteR uuid aid ->
            UB.absolute [ "archivenote", zkNoteIdToString uuid, zkNoteIdToString aid ] []

        ResetPasswordR user key ->
            UB.absolute [ "reset", user, UUID.toString key ] []

        SettingsR ->
            UB.absolute [ "settings" ] []

        Invite s ->
            UB.absolute [ "invite", s ] []

        Top ->
            UB.absolute [] []
