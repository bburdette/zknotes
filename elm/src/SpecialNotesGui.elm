module SpecialNotesGui exposing (..)

import ArchiveListing exposing (Command)
import Common
import Data exposing (AndOr(..), LzLink, SaveLzLink, SearchMod(..), TagSearch(..), ZkListNote, ZkNoteId(..))
import DataUtil exposing (lzlKey, zkNoteIdToString, zklKey)
import Dict exposing (Dict)
import Element as E
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import Orgauth.Data exposing (UserId)
import SearchUtil exposing (showTagSearch)
import Set
import SnListEdit as SLE exposing (DragDropWhat(..), NlLink, nllDndSubscriptions)
import SpecialNotes as SN exposing (CompletedSync, Notegraph, SpecialNote)
import TDict
import Time
import Util


type Msg
    = CopySearchPress
    | CopySyncSearchPress Bool
    | GraphFocusClick
    | SLEMsg SLE.Msg
    | Noop


type Command
    = CopySearch (List TagSearch)
    | CopySyncSearch TagSearch
    | GraphFocus
    | None


type SpecialNoteState
    = SnsSearch (List TagSearch)
    | SnsSync CompletedSync
    | SnsList SLE.Model


initSpecialNoteState : ZkNoteId -> SN.SpecialNote -> List LzLink -> SpecialNoteState
initSpecialNoteState znid sn lzls =
    case sn of
        SN.SnSearch tagSearch ->
            SnsSearch tagSearch

        SN.SnSync completedSync ->
            SnsSync completedSync

        SN.SnList notegraph ->
            SnsList (SLE.init notegraph (mklzList znid lzls))


getSpecialNote : SpecialNoteState -> SpecialNote
getSpecialNote sns =
    case sns of
        SnsSearch tagSearch ->
            SN.SnSearch tagSearch

        SnsSync completedSync ->
            SN.SnSync completedSync

        SnsList slem ->
            SN.SnList slem.ng


sngSubscriptions : SpecialNoteState -> List (Sub Msg)
sngSubscriptions sns =
    case sns of
        SnsSearch tagSearch ->
            []

        SnsSync completedSync ->
            []

        SnsList slem ->
            List.map (Sub.map SLEMsg) <| nllDndSubscriptions slem



-- [ nllDndSystem.subscriptions model.nllDnd ]


saveLzLinks : ZkNoteId -> SpecialNoteState -> List SaveLzLink
saveLzLinks this sns =
    case sns of
        SnsSearch _ ->
            []

        SnsSync _ ->
            []

        SnsList slem ->
            List.foldl
                (\nll ( to, lst ) ->
                    ( nll.id
                    , { to = to
                      , from = nll.id
                      , delete = Just False
                      }
                        :: lst
                    )
                )
                ( this, [] )
                (filterNotes this slem.nlls)
                |> Tuple.second


guiSn :
    Time.Zone
    -> SpecialNoteState
    -> E.Element Msg
guiSn zone snote =
    case snote of
        SnsSearch tagsearches ->
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

        SnsSync completedSync ->
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

        SnsList slem ->
            E.column []
                (EI.button Common.buttonStyle
                    { onPress = Just <| GraphFocusClick
                    , label = E.text "add to list"
                    }
                    :: List.indexedMap
                        (\i lzl ->
                            E.map SLEMsg <|
                                SLE.dndRow
                                    SLE.nllId
                                    Drag
                                    i
                                    False
                                    (E.text lzl.title)
                        )
                        slem.nlls
                )



{-
   toLists : Data.ZkNoteId -> Dict String LzLink -> List (List ZkListNote)
   toLists this lzls =
       let
           toDict =
               List.foldl
                   (\lzl todict ->
                       let
                           zkto =
                               zkNoteIdToString lzl.to
                       in
                       case Dict.get zkto todict of
                           Nothing ->
                               Dict.insert zkto [ lzl ] todict

                           Just lzlz ->
                               Dict.insert zkto (lzl :: lzlz) todict
                   )
                   Dict.empty
                   (Dict.values lzls)
       in
       []
-}


lzToDict : Dict String LzLink -> DataUtil.LzlDict
lzToDict lzls =
    lzls
        |> Dict.values
        |> List.foldl
            (\lzl lzll ->
                TDict.insert lzl.to lzl lzll
            )
            DataUtil.emptyLzlDict


lzToDict2 : List LzLink -> DataUtil.LzlDict
lzToDict2 lzls =
    lzls
        |> List.foldl
            (\lzl lzll ->
                TDict.insert lzl.to lzl lzll
            )
            DataUtil.emptyLzlDict


addNotes :
    ZkNoteId
    -> List ZkListNote
    -> SpecialNoteState
    -> SpecialNoteState
addNotes this zlns sns =
    case sns of
        SnsList slem ->
            -- disallow linking 'this' and disallow multiple occurances
            -- of notes.
            let
                notes =
                    List.map (\zln -> { id = zln.id, title = zln.title }) zlns
            in
            case slem.ng.currentUuid of
                Nothing ->
                    SnsList
                        { slem | nlls = filterNotes this <| notes ++ slem.nlls }

                Just uuid ->
                    let
                        zni =
                            Zni uuid

                        nlnks =
                            List.foldr
                                (\n lst ->
                                    if n.id == zni then
                                        n :: notes ++ lst

                                    else
                                        n :: lst
                                )
                                []
                                slem.nlls

                        flnks =
                            filterNotes this nlnks
                    in
                    SnsList { slem | nlls = flnks }

        SnsSearch s ->
            SnsSearch s

        SnsSync s ->
            SnsSync s


filterNotes : ZkNoteId -> List NlLink -> List NlLink
filterNotes this nlls =
    -- filter out duplicates and 'this'
    List.foldr
        (\nll ( nls, nlst ) ->
            let
                nllid =
                    zkNoteIdToString nll.id
            in
            if Set.member nllid nls then
                ( nls, nlst )

            else
                ( Set.insert nllid nls, nll :: nlst )
        )
        ( Set.singleton (zkNoteIdToString this), [] )
        nlls
        |> Tuple.second


mklzList : ZkNoteId -> List Data.LzLink -> List { id : ZkNoteId, title : String }
mklzList this links =
    List.reverse <| dolst this (lzToDict2 links) []


dolst : ZkNoteId -> DataUtil.LzlDict -> List { id : ZkNoteId, title : String } -> List { id : ZkNoteId, title : String }
dolst toid lz2d lst =
    case TDict.get toid lz2d of
        Nothing ->
            lst

        Just l ->
            dolst l.from
                (TDict.remove toid lz2d)
                ({ id = l.from, title = l.fromname } :: lst)


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


updateSn : Msg -> SpecialNoteState -> ( SpecialNoteState, Command )
updateSn msg snote =
    case snote of
        SnsSearch tagsearches ->
            case msg of
                GraphFocusClick ->
                    ( SnsSearch tagsearches, None )

                CopySearchPress ->
                    ( SnsSearch tagsearches, CopySearch tagsearches )

                CopySyncSearchPress _ ->
                    ( SnsSearch tagsearches, None )

                SLEMsg _ ->
                    ( SnsSearch tagsearches, None )

                Noop ->
                    ( SnsSearch tagsearches, None )

        SnsSync completedSync ->
            case msg of
                GraphFocusClick ->
                    ( SnsSync completedSync, None )

                CopySearchPress ->
                    ( SnsSync completedSync, None )

                CopySyncSearchPress fromremote ->
                    ( SnsSync completedSync, CopySyncSearch (syncSearch fromremote completedSync) )

                SLEMsg _ ->
                    ( SnsSync completedSync, None )

                Noop ->
                    ( SnsSync completedSync, None )

        SnsList slem ->
            case msg of
                GraphFocusClick ->
                    ( SnsList slem, GraphFocus )

                CopySearchPress ->
                    ( SnsList slem, None )

                CopySyncSearchPress _ ->
                    ( SnsList slem, None )

                SLEMsg _ ->
                    ( SnsList slem, None )

                Noop ->
                    ( SnsList slem, None )
