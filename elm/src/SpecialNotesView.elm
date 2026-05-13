module SpecialNotesView exposing (..)

import ArchiveListing exposing (Command)
import Common
import Data exposing (AndOr(..), LzLink, SaveLzLink, SearchMod(..), TagSearch(..), ZkListNote, ZkNoteId(..))
import DataUtil exposing (NlLink, zkNoteIdToString)
import Dict exposing (Dict)
import Element as E
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import SearchUtil exposing (showTagSearch)
import Set
import SnListView as SLV
import SpecialNotes as SN exposing (CompletedSync, SpecialNote)
import TDict
import Time
import Util


type Msg
    = SlideShowClick
    | SLVMsg SLV.Msg
    | Noop


type Command
    = SlideShow (Maybe ZkNoteId) (List NlLink)
    | SaveLocalData String
    | Batch (List Command)
    | None


type SpecialNoteState
    = SnsSearch (List TagSearch)
    | SnsSync CompletedSync
    | SnsList SLV.Model


saveLocalData : SpecialNoteState -> Maybe String
saveLocalData sns =
    case sns of
        SnsSearch _ ->
            Nothing

        SnsSync _ ->
            Nothing

        SnsList m ->
            m.currentUuid


mbLocalDataId : ZkNoteId -> SpecialNote -> Maybe String
mbLocalDataId zni sn =
    if couldUseLocalState sn then
        Just <| localDataId zni

    else
        Nothing


couldUseLocalState : SpecialNote -> Bool
couldUseLocalState sns =
    case sns of
        SN.SnSearch _ ->
            False

        SN.SnSync _ ->
            False

        SN.SnList ->
            True


localDataId : ZkNoteId -> String
localDataId zni =
    zkNoteIdToString zni ++ "-snstate"


initSpecialNoteStateLz : ZkNoteId -> SN.SpecialNote -> Maybe String -> List LzLink -> SpecialNoteState
initSpecialNoteStateLz znid sn mbsnstate lzls =
    initSpecialNoteState sn mbsnstate (mklzList znid lzls)


initSpecialNoteState : SN.SpecialNote -> Maybe String -> List NlLink -> SpecialNoteState
initSpecialNoteState sn mbsnstate lzls =
    case sn of
        SN.SnSearch tagSearch ->
            SnsSearch tagSearch

        SN.SnSync completedSync ->
            SnsSync completedSync

        SN.SnList ->
            SnsList (SLV.init mbsnstate lzls)


getSpecialNote : SpecialNoteState -> SpecialNote
getSpecialNote sns =
    case sns of
        SnsSearch tagSearch ->
            SN.SnSearch tagSearch

        SnsSync completedSync ->
            SN.SnSync completedSync

        SnsList _ ->
            SN.SnList


guiSn :
    Time.Zone
    -> Int
    -> SpecialNoteState
    -> E.Element Msg
guiSn zone fontsize snote =
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
                ]

        SnsList slem ->
            E.column []
                [ E.row [ E.spacing 3 ]
                    [ EI.button Common.buttonStyle
                        { onPress = Just <| SlideShowClick
                        , label = E.text "slideshow"
                        }
                    ]
                , E.map SLVMsg <| SLV.view fontsize slem
                ]


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
            case slem.currentUuid of
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
                SlideShowClick ->
                    ( SnsSearch tagsearches, None )

                SLVMsg _ ->
                    ( SnsSearch tagsearches, None )

                Noop ->
                    ( SnsSearch tagsearches, None )

        SnsSync completedSync ->
            case msg of
                SlideShowClick ->
                    ( SnsSync completedSync, None )

                SLVMsg _ ->
                    ( SnsSync completedSync, None )

                Noop ->
                    ( SnsSync completedSync, None )

        SnsList slem ->
            case msg of
                SlideShowClick ->
                    ( SnsList slem, SlideShow (Maybe.map Zni slem.currentUuid) slem.nlls )

                SLVMsg m ->
                    let
                        ( nm, c ) =
                            SLV.update m slem
                    in
                    ( SnsList nm
                    , case c of
                        SLV.None ->
                            None

                        SLV.PlayNSave s ->
                            Batch
                                [ SaveLocalData s
                                , SlideShow (Maybe.map Zni nm.currentUuid) nm.nlls
                                ]
                    )

                Noop ->
                    ( SnsList slem, None )
