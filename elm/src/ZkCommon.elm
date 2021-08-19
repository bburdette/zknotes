module ZkCommon exposing (..)

import Data as D
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import TangoColors as TC
import Util as U


searchColor : E.Color
searchColor =
    TC.darkRed


commentColor : E.Color
commentColor =
    TC.darkGreen


publicColor : E.Color
publicColor =
    TC.darkBlue


shareColor : E.Color
shareColor =
    TC.darkBrown


systemColor : D.LoginData -> List Int -> Maybe E.Color
systemColor ld ids =
    let
        sysColor : Int -> Maybe E.Color
        sysColor color =
            if color == ld.publicid then
                Just publicColor

            else if color == ld.shareid then
                Just shareColor

            else if color == ld.searchid then
                Just searchColor

            else if color == ld.commentid then
                Just commentColor

            else
                Nothing
    in
    ids
        |> U.first
            sysColor


myLinkStylePlain =
    [ EF.color TC.darkBlue ]


otherLinkStylePlain =
    [ EF.color TC.lightBlue ]


myLinkStyle =
    [ EF.color TC.black, EF.underline ]


otherLinkStyle =
    [ EF.color TC.darkBlue, EF.underline ]


saveLinkStyle =
    [ EF.color TC.darkYellow, EF.underline ]


fullScreen =
    E.row [ EF.size 10 ] [ E.column [] [ E.text "↖", E.text "↙" ], E.column [] [ E.text "↗", E.text "↘" ] ]
