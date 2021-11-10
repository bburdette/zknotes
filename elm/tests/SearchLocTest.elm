module SearchLocTest exposing (..)

import Expect exposing (Expectation)
import Fuzz exposing (Fuzzer, int, list, string)
import Search as S
import SearchLoc exposing (RTRes(..), TSLoc(..), getTerm, removeTerm, setTerm, swapLast)
import Test exposing (..)


suite : Test
suite =
    describe "some searchloc tests"
        [ test "swapLast" <|
            \_ ->
                let
                    s1 =
                        LNot (LBT1 (LBT2 LThis))

                    s2 =
                        LNot (LBT1 (LBT2 (LNot LThis)))
                in
                Expect.equal s2 (swapLast s1 (LNot LThis))
        ]



-- todo "Implement our first test. See https://package.elm-lang.org/packages/elm-explorations/test/latest for how to do this!"
