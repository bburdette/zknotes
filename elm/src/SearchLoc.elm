module SearchLoc exposing (..)

import Search exposing (..)


type TSLoc
    = LNot TSLoc
    | LBT1 TSLoc
    | LBT2 TSLoc
    | LThis


type RTRes
    = Matched
    | Removed TagSearch
    | Unmatched


swapLast : TSLoc -> TSLoc -> TSLoc
swapLast tsl subst =
    case tsl of
        LThis ->
            subst

        LNot l ->
            LNot (swapLast l subst)

        LBT1 l ->
            LBT1 (swapLast l subst)

        LBT2 l ->
            LBT2 (swapLast l subst)


removeTerm : TSLoc -> TagSearch -> RTRes
removeTerm tsl ts =
    case ( ts, tsl ) of
        ( SearchTerm _ _, LThis ) ->
            Matched

        ( Not nt, LThis ) ->
            Removed nt

        ( Not _, LNot LThis ) ->
            Matched

        ( Not nt, LNot nts ) ->
            case removeTerm nts nt of
                Matched ->
                    Matched

                Removed rts ->
                    Removed (Not rts)

                Unmatched ->
                    Unmatched

        ( Boolex _ _ _, LThis ) ->
            Matched

        ( Boolex ts1 ao ts2, LBT1 bxts ) ->
            case removeTerm bxts ts1 of
                Matched ->
                    Removed ts2

                Removed nt1 ->
                    Removed <| Boolex nt1 ao ts2

                Unmatched ->
                    Unmatched

        ( Boolex ts1 ao ts2, LBT2 bxts ) ->
            case removeTerm bxts ts2 of
                Matched ->
                    Removed ts1

                Removed nt2 ->
                    Removed <| Boolex ts1 ao nt2

                Unmatched ->
                    Unmatched

        _ ->
            Unmatched


getTerm : TSLoc -> TagSearch -> Maybe TagSearch
getTerm tsl ts =
    case ( ts, tsl ) of
        ( _, LThis ) ->
            Just ts

        ( Not nt, LNot nts ) ->
            getTerm nts nt

        ( Boolex ts1 _ _, LBT1 bxts ) ->
            getTerm bxts ts1

        ( Boolex _ _ ts2, LBT2 bxts ) ->
            getTerm bxts ts2

        _ ->
            Nothing


setTerm : TSLoc -> TagSearch -> TagSearch -> Maybe TagSearch
setTerm tsl rts ts =
    case ( ts, tsl ) of
        ( SearchTerm _ _, LThis ) ->
            Just rts

        ( _, LThis ) ->
            Just rts

        ( Not nt, LNot nts ) ->
            setTerm nts rts nt
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
