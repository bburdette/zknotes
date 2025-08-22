module SpecialNotesGui exposing (..)

-- import SearchPanel exposing (Command(..), Msg(..))

import ArchiveListing exposing (Command)
import Common
import Data exposing (AndOr(..), SearchMod(..), TagSearch(..), ZkNoteSearch)
import Either exposing (andMapLeft)
import Element as E
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import SearchUtil exposing (showTagSearch)
import Set
import SpecialNotes as SN
import TangoColors as TC
import Time
import Toop
import Util


type Msg
    = CopySearchPress
    | CopySyncSearhPress
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
            E.row [ E.alignTop, E.width E.fill ]
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
                    ]
                , EI.button (E.alignRight :: Common.buttonStyle)
                    { onPress = Just CopySyncSearhPress
                    , label = E.text ">"
                    }
                ]

        SN.SnPlaylist _ ->
            E.none


syncSearch : SN.CompletedSync -> TagSearch
syncSearch csync =
    case csync.after of
        Just a ->
            Boolex
                { ts1 =
                    Not
                        { ts =
                            SearchTerm
                                { mods = [ Server ]
                                , term = "local"
                                }
                        }
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
                    Not
                        { ts =
                            SearchTerm
                                { mods = [ Server ]
                                , term = "local"
                                }
                        }
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

                CopySyncSearhPress ->
                    ( SN.SnSearch tagsearches, None )

                Noop ->
                    ( SN.SnSearch tagsearches, None )

        SN.SnSync completedSync ->
            case msg of
                CopySearchPress ->
                    ( SN.SnSync completedSync, None )

                CopySyncSearhPress ->
                    ( SN.SnSync completedSync, CopySyncSearch (syncSearch completedSync) )

                Noop ->
                    ( SN.SnSync completedSync, None )

        SN.SnPlaylist notelist ->
            ( SN.SnPlaylist notelist, None )
