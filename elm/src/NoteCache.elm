module NoteCache exposing (NoteCache, addNote, empty, getNote, purgeNotes, setKeeps)

import Data exposing (ZkNoteEdit)
import Dict exposing (Dict)
import Set exposing (Set)
import Time
import Util


type alias ZneEntry =
    { receivetime : Int, zne : ZkNoteEdit }


type alias NoteCache =
    { byId : Dict Int ZneEntry
    , byReceipt : Dict Int (Set Int)
    , keep : Set Int
    , max : Int
    }


setKeeps : Set Int -> NoteCache -> NoteCache
setKeeps keep nc =
    { nc | keep = keep }


addNote : Time.Posix -> ZkNoteEdit -> NoteCache -> NoteCache
addNote pt zne nc =
    let
        ms =
            Time.posixToMillis pt

        id =
            zne.zknote.id
    in
    { byId =
        Dict.insert id
            { receivetime = ms, zne = zne }
            nc.byId
    , byReceipt =
        case Dict.get ms nc.byReceipt of
            Just set ->
                Dict.insert ms (Set.insert id set) nc.byReceipt

            Nothing ->
                Dict.insert ms (Set.insert id Set.empty) nc.byReceipt

    -- add new notes to keeps!  assuming they belong in the current note.
    , keep = Set.insert id nc.keep
    , max = nc.max
    }


getNote : Int -> NoteCache -> Maybe ZkNoteEdit
getNote id nc =
    Dict.get id nc.byId
        |> Maybe.map .zne


removeNote : Int -> NoteCache -> NoteCache
removeNote id nc =
    case Dict.get id nc.byId of
        Just ze ->
            { byId = Dict.remove id nc.byId
            , byReceipt =
                case Dict.get ze.receivetime nc.byReceipt of
                    Just set ->
                        let
                            ns =
                                Set.remove id set
                        in
                        if Set.isEmpty ns then
                            Dict.remove ze.receivetime nc.byReceipt

                        else
                            Dict.insert ze.receivetime ns nc.byReceipt

                    Nothing ->
                        nc.byReceipt
            , keep = Set.remove id nc.keep
            , max = nc.max
            }

        Nothing ->
            nc


purgeNotes : NoteCache -> NoteCache
purgeNotes nc =
    let
        ncount =
            Dict.size nc.byId

        toremove =
            ncount - nc.max
    in
    if toremove <= 0 then
        nc

    else
        let
            br =
                nc.byReceipt |> Dict.toList |> List.map (Tuple.second >> Set.toList) |> List.concat

            ( rcount, nnnc ) =
                Util.foldUntil
                    (\id ( rmv, nnc ) ->
                        if rmv <= 0 then
                            Util.Stop ( rmv, nnc )

                        else if Set.member id nc.keep then
                            Util.Go ( rmv, nnc )

                        else
                            Util.Go ( rmv - 1, removeNote id nnc )
                    )
                    ( toremove, nc )
                    br
        in
        nnnc


empty : Int -> NoteCache
empty max =
    { byId = Dict.empty
    , byReceipt = Dict.empty
    , keep = Set.empty
    , max = max
    }
