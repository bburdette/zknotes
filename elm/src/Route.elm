module Route exposing (Route(..), parseUrl, routeTitle, routeUrl)

import UUID exposing (UUID)
import Url exposing (Url)
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))


type Route
    = LoginR
    | PublicZkNote Int
    | PublicZkPubId String
    | EditZkNoteR Int
    | EditZkNoteNew
    | ArchiveNoteListingR Int
    | ArchiveNoteR Int Int
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
            "zknote " ++ String.fromInt id

        PublicZkPubId id ->
            id ++ " - zknotes"

        EditZkNoteR id ->
            "zknote " ++ String.fromInt id

        EditZkNoteNew ->
            "new zknote"

        ArchiveNoteListingR id ->
            "archives " ++ String.fromInt id

        ArchiveNoteR id aid ->
            "archive " ++ String.fromInt id ++ ": " ++ String.fromInt aid

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
                    </> UP.int
            , UP.map (\i -> PublicZkPubId (Maybe.withDefault "" (Url.percentDecode i))) <|
                UP.s
                    "page"
                    </> UP.string
            , UP.map ArchiveNoteListingR <|
                UP.s
                    "archivelisting"
                    </> UP.int
            , UP.map ArchiveNoteR <|
                UP.s
                    "archivenote"
                    </> UP.int
                    </> UP.int
            , UP.map EditZkNoteR <|
                UP.s
                    "editnote"
                    </> UP.int
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

        PublicZkNote id ->
            UB.absolute [ "note", String.fromInt id ] []

        PublicZkPubId pubid ->
            UB.absolute [ "page", pubid ] []

        EditZkNoteR id ->
            UB.absolute [ "editnote", String.fromInt id ] []

        EditZkNoteNew ->
            UB.absolute [ "editnote", "new" ] []

        ArchiveNoteListingR id ->
            UB.absolute [ "archivelisting", String.fromInt id ] []

        ArchiveNoteR id aid ->
            UB.absolute [ "archivenote", String.fromInt id, String.fromInt aid ] []

        ResetPasswordR user key ->
            UB.absolute [ "reset", user, UUID.toString key ] []

        SettingsR ->
            UB.absolute [ "settings" ] []

        Invite s ->
            UB.absolute [ "invite", s ] []

        Top ->
            UB.absolute [] []
