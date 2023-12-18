module Route exposing (Route(..), parseUrl, routeTitle, routeUrl)

import UUID exposing (UUID)
import Url exposing (Url)
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))


type Route
    = LoginR
    | PublicZkNote UUID
    | PublicZkPubId String
    | EditZkNoteR UUID
    | EditZkNoteNew
    | ArchiveNoteListingR UUID
    | ArchiveNoteR UUID Int
    | ResetPasswordR String UUID
    | SettingsR
    | Invite String
    | Top


routeTitle : Route -> String
routeTitle route =
    case route of
        LoginR ->
            "login"

        PublicZkNote uuid ->
            "zknote " ++ UUID.toString uuid

        PublicZkPubId id ->
            id ++ " - zknotes"

        EditZkNoteR uuid ->
            "zknote " ++ UUID.toString uuid

        EditZkNoteNew ->
            "new zknote"

        ArchiveNoteListingR uuid ->
            "archives " ++ UUID.toString uuid

        ArchiveNoteR uuid aid ->
            "archive " ++ UUID.toString uuid ++ ": " ++ String.fromInt aid

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
                    </> UP.custom "UUID" (UUID.fromString >> Result.toMaybe)
            , UP.map (\i -> PublicZkPubId (Maybe.withDefault "" (Url.percentDecode i))) <|
                UP.s
                    "page"
                    </> UP.string
            , UP.map ArchiveNoteListingR <|
                UP.s
                    "archivelisting"
                    </> UP.custom "UUID" (UUID.fromString >> Result.toMaybe)
            , UP.map ArchiveNoteR <|
                UP.s
                    "archivenote"
                    </> UP.custom "UUID" (UUID.fromString >> Result.toMaybe)
                    </> UP.int
            , UP.map EditZkNoteR <|
                UP.s
                    "editnote"
                    </> UP.custom "UUID" (UUID.fromString >> Result.toMaybe)
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
            UB.absolute [ "note", UUID.toString uuid ] []

        PublicZkPubId pubid ->
            UB.absolute [ "page", pubid ] []

        EditZkNoteR uuid ->
            UB.absolute [ "editnote", UUID.toString uuid ] []

        EditZkNoteNew ->
            UB.absolute [ "editnote", "new" ] []

        ArchiveNoteListingR uuid ->
            UB.absolute [ "archivelisting", UUID.toString uuid ] []

        ArchiveNoteR uuid aid ->
            UB.absolute [ "archivenote", UUID.toString uuid, String.fromInt aid ] []

        ResetPasswordR user key ->
            UB.absolute [ "reset", user, UUID.toString key ] []

        SettingsR ->
            UB.absolute [ "settings" ] []

        Invite s ->
            UB.absolute [ "invite", s ] []

        Top ->
            UB.absolute [] []
