module SpecialNotesGui exposing (..)

import ArchiveListing exposing (Command)
import Common
import Data exposing (AndOr(..), SearchMod(..), TagSearch(..))
import Element as E
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import SearchUtil exposing (showTagSearch)
import SpecialNotes as SN
import Time
import Util


type Msg
    = CopySearchPress
    | CopySyncSearchPress Bool
    | Noop


type Command
    = CopySearch (List TagSearch)
    | CopySyncSearch TagSearch
    | None


guiSn : Time.Zone -> SN.SpecialNote -> E.Element Msg
guiSn zone snote =
    case snote of
        SN.SnSearch tagsearches ->
            E.row
                [ E.alignTop
                , E.width E.fill
                ]
                [ E.paragraph
                    [ E.htmlAttribute (HA.style "overflow-wrap" "break-word")
                    , E.htmlAttribute (HA.style "word-break" "break-word")
                    ]
                    (tagsearches
                        |> List.map (showTagSearch >> E.text)
                    )
                , EI.button (E.alignRight :: Common.buttonStyle)
                    { onPress = Just CopySearchPress
                    , label = E.text ">"
                    }
                ]

        SN.SnSync completedSync ->
            E.wrappedRow [ E.alignTop, E.width E.fill ]
                [ E.column []
                    [ E.text "sync"
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "start:"
                        , case completedSync.after of
                            Just s ->
                                E.text (Util.showDateTime zone (Time.millisToPosix s))

                            Nothing ->
                                E.text "-∞"
                        ]
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "end:"
                        , E.text (Util.showDateTime zone (Time.millisToPosix completedSync.now))
                        ]
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "local server id:"
                        , completedSync.local |> Maybe.withDefault "" |> E.text
                        ]
                    , E.row [ E.spacing 3 ]
                        [ E.el [ EF.bold ] <| E.text "remote server id:"
                        , completedSync.remote |> Maybe.withDefault "" |> E.text
                        ]
                    ]
                , E.column [ E.alignRight, E.spacing 3 ]
                    [ EI.button (E.alignRight :: Common.buttonStyle)
                        { onPress = Just <| CopySyncSearchPress True
                        , label = E.text "search notes synced from remote >"
                        }
                    , EI.button (E.alignRight :: Common.buttonStyle)
                        { onPress = Just <| CopySyncSearchPress False
                        , label = E.text "search notes synced to remote >"
                        }
                    ]
                ]

        SN.SnGraph _ ->
            E.none


syncSearch : Bool -> SN.CompletedSync -> TagSearch
syncSearch fromremote csync =
    case csync.after of
        Just a ->
            Boolex
                { ts1 =
                    let
                        st =
                            SearchTerm
                                { mods = [ Server ]
                                , term = "local"
                                }
                    in
                    if fromremote then
                        Not
                            { ts = st
                            }

                    else
                        st
                , ao = And
                , ts2 =
                    Boolex
                        { ts1 =
                            SearchTerm
                                { mods = [ After, Mod ]
                                , term = String.fromInt a
                                }
                        , ao = And
                        , ts2 =
                            SearchTerm
                                { mods = [ Before, Mod ]
                                , term = String.fromInt csync.now
                                }
                        }
                }

        Nothing ->
            Boolex
                { ts1 =
                    let
                        st =
                            SearchTerm
                                { mods = [ Server ]
                                , term = "local"
                                }
                    in
                    if fromremote then
                        Not
                            { ts = st
                            }

                    else
                        st
                , ao = And
                , ts2 =
                    SearchTerm
                        { mods = [ Before, Mod ]
                        , term = String.fromInt csync.now
                        }
                }


updateSn : Msg -> SN.SpecialNote -> ( SN.SpecialNote, Command )
updateSn msg snote =
    case snote of
        SN.SnSearch tagsearches ->
            case msg of
                CopySearchPress ->
                    ( SN.SnSearch tagsearches, CopySearch tagsearches )

                CopySyncSearchPress _ ->
                    ( SN.SnSearch tagsearches, None )

                Noop ->
                    ( SN.SnSearch tagsearches, None )

        SN.SnSync completedSync ->
            case msg of
                CopySearchPress ->
                    ( SN.SnSync completedSync, None )

                CopySyncSearchPress fromremote ->
                    ( SN.SnSync completedSync, CopySyncSearch (syncSearch fromremote completedSync) )

                Noop ->
                    ( SN.SnSync completedSync, None )

        SN.SnGraph g ->
            ( SN.SnGraph g, None )
