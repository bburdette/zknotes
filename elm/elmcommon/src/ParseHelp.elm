module ParseHelp exposing (listOf, plist)

import Parser as P exposing ((|.), (|=), Parser, Step(..), loop, map, oneOf, succeed)


plist : Parser a -> List a -> Parser (Step (List a) (List a))
plist aparser alist =
    oneOf
        [ succeed (\anA -> Loop (anA :: alist))
            |= aparser
        , succeed ()
            |> map (\_ -> Done (List.reverse alist))
        ]


listOf : Parser a -> Parser (List a)
listOf aparser =
    loop [] <| plist aparser
