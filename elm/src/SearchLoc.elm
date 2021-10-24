module SearchLoc exposing (..)

import Search exposing (..)


type TSLoc
    = LNot TSLoc
    | LBT1 TSLoc
    | LBT2 TSLoc
    | LLeaf
    | LThis



{- toTerm : (TSLoc -> TSLoc) -> TSLoc ->(TSLoc -> TSLoc)
   toTerm tt t =
     case t of
       LLeaf -> tt (always LLeaf)
       LNot TSLoc
       LBT1 TSLoc
       LBT2 TSLoc

-}


getTerm : TagSearch -> TSLoc -> Maybe TagSearch
getTerm ts tsl =
    case ( ts, tsl ) of
        ( SearchTerm _ _, LLeaf ) ->
            Just ts

        ( Not nt, LNot LThis ) ->
            Just ts

        ( Not nt, LNot nts ) ->
            getTerm nt nts

        ( Boolex ts1 _ _, LBT1 LThis ) ->
            Just ts

        ( Boolex ts1 _ _, LBT1 bxts ) ->
            getTerm ts1 bxts

        ( Boolex _ _ ts2, LBT2 LThis ) ->
            Just ts

        ( Boolex _ _ ts2, LBT2 bxts ) ->
            getTerm ts2 bxts

        _ ->
            Nothing


setTerm : TagSearch -> TSLoc -> TagSearch -> Maybe TagSearch
setTerm ts tsl rts =
    case ( ts, tsl ) of
        ( SearchTerm _ _, LLeaf ) ->
            Just rts

        ( Not nt, LNot LThis ) ->
            Just rts

        ( Not nt, LNot nts ) ->
            setTerm nt nts rts
                |> Maybe.map (\t -> Not t)

        ( Boolex ts1 _ _, LBT1 LThis ) ->
            Just rts

        ( Boolex ts1 _ _, LBT1 bxts ) ->
            setTerm ts1 bxts rts

        ( Boolex _ _ ts2, LBT2 LThis ) ->
            Just rts

        ( Boolex _ _ ts2, LBT2 bxts ) ->
            setTerm ts2 bxts rts

        _ ->
            Nothing
