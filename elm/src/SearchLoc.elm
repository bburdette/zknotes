module SearchLoc exposing (..)

import Search exposing (..)


type TSLoc
    = LNot TSLoc
    | LBT1 TSLoc
    | LBT2 TSLoc
    | LLeaf
    | LThis


getTerm : TSLoc -> TagSearch -> Maybe TagSearch
getTerm tsl ts =
    case ( ts, tsl ) of
        ( SearchTerm _ _, LLeaf ) ->
            Just ts

        ( Not nt, LNot LThis ) ->
            Just ts

        ( Not nt, LNot nts ) ->
            getTerm nts nt

        ( Boolex _ _ _, LThis ) ->
            Just ts

        ( Boolex ts1 _ _, LBT1 bxts ) ->
            getTerm bxts ts1

        ( Boolex _ _ ts2, LBT2 bxts ) ->
            getTerm bxts ts2

        _ ->
            Nothing


setTerm : TSLoc -> TagSearch -> TagSearch -> Maybe TagSearch
setTerm tsl rts ts =
    case ( ts, tsl ) of
        ( SearchTerm _ _, LLeaf ) ->
            Just rts

        ( _, LThis ) ->
            Just rts

        ( Not nt, LNot nts ) ->
            setTerm nts rts ts
                |> Maybe.map (\t -> Not t)

        ( Boolex ts1 andor ts2, LBT1 bxts ) ->
            setTerm bxts rts ts1
                |> Maybe.map
                    (\t1 ->
                        Boolex t1
                            andor
                            ts2
                    )

        ( Boolex ts1 andor ts2, LBT2 bxts ) ->
            setTerm bxts rts ts2
                |> Maybe.map
                    (\t2 ->
                        Boolex ts1
                            andor
                            t2
                    )

        _ ->
            Nothing
