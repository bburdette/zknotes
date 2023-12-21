module NoteCache exposing (NoteCache, addNote, empty, getNote, purgeNotes, setKeeps)

import Data exposing (ZkNoteAndLinks, ZkNoteId, ZniSet) 
import Dict exposing (Dict)
import Set exposing (Set)
import TSet exposing (TSet)
import TDict exposing (TDict)
import Time
import Util


type alias ZneEntry =
    { receivetime : Int, zne : ZkNoteAndLinks }


type alias NoteCache =
    { byId : ZneDict
    , byReceipt : Dict Int ZniSet
    , keep : ZniSet
    , max : Int
    }

type alias ZneDict =
    TDict ZkNoteId String ZneEntry


emptyZneDict : ZneDict
emptyZneDict =
    TDict.empty Data.zkNoteIdToString Data.trustedZkNoteIdFromString



setKeeps : ZniSet -> NoteCache -> NoteCache
setKeeps keep nc =
    { nc | keep = keep }


addNote : Time.Posix -> ZkNoteAndLinks -> NoteCache -> NoteCache
addNote pt zne nc =
    let
        ms =
            Time.posixToMillis pt

        id =
            zne.zknote.id
    in
    { byId =
        TDict.insert id
            { receivetime = ms, zne = zne }
            nc.byId
    , byReceipt =
        case Dict.get ms nc.byReceipt of
            Just set ->
                Dict.insert ms (TSet.insert id set) nc.byReceipt

            Nothing ->
                Dict.insert ms (TSet.insert id Data.emptyZniSet) nc.byReceipt

    -- (TODO?) add new notes to keeps!  assuming they belong in the current note.
    , keep = TSet.insert id nc.keep
    , max = nc.max
    }


getNote : NoteCache -> ZkNoteId -> Maybe ZkNoteAndLinks
getNote nc id =
    TDict.get id nc.byId
        |> Maybe.map .zne


removeNote : NoteCache -> ZkNoteId -> NoteCache
removeNote nc id =
    case TDict.get id nc.byId of
        Just ze ->
            { byId = TDict.remove id nc.byId
            , byReceipt =
                case Dict.get ze.receivetime nc.byReceipt of
                    Just set ->
                        let
                            ns =
                                TSet.remove id set
                        in
                        if TSet.isEmpty ns then
                            Dict.remove ze.receivetime nc.byReceipt

                        else
                            Dict.insert ze.receivetime ns nc.byReceipt

                    Nothing ->
                        nc.byReceipt
            , keep = TSet.remove id nc.keep
            , max = nc.max
            }

        Nothing ->
            nc


purgeNotes : NoteCache -> NoteCache
purgeNotes nc =
    let
        ncount =
            TDict.size nc.byId

        toremove =
            ncount - nc.max
    in
    if toremove <= 0 then
        nc

    else
        let
            br =
                nc.byReceipt |> Dict.toList |> List.map (Tuple.second >> TSet.toList) |> List.concat

            ( rcount, nnnc ) =
                Util.foldUntil
                    (\id ( rmv, nnc ) ->
                        if rmv <= 0 then
                            Util.Stop ( rmv, nnc )

                        else if TSet.member id nc.keep then
                            Util.Go ( rmv, nnc )

                        else
                            Util.Go ( rmv - 1, removeNote nnc id )
                    )
                    ( toremove, nc )
                    br
        in
        nnnc


empty : Int -> NoteCache
empty max =
    { byId = emptyZneDict
    , byReceipt = Dict.empty
    , keep = Data.emptyZniSet
    , max = max
    }
