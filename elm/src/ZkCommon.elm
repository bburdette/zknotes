module ZkCommon exposing (..)

import Data as D
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import TangoColors as TC
import Util as U


type alias StylePalette =
    { defaultSpacing : Int
    }



---------------------------------------------------
-- system note colors.
---------------------------------------------------


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


systemColor : D.Sysids -> List Int -> Maybe E.Color
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



---------------------------------------------------


saveColor =
    TC.darkYellow


myLinkColor =
    TC.black


disabledLinkColor =
    TC.darkGrey


otherLinkColor =
    TC.darkBlue


myLinkStyle =
    [ EF.color TC.black, EF.underline ]


disabledLinkStyle =
    [ EF.color TC.darkGrey, EF.underline ]


otherLinkStyle =
    [ EF.color TC.darkBlue, EF.underline ]


saveLinkStyle =
    [ EF.color saveColor, EF.underline ]


fullScreen =
    E.column []
        [ E.row
            [ E.inFront
                (E.row []
                    [ E.el [] <| E.text "⌞"
                    , E.el [] <| E.text "⌟"
                    ]
                )
            ]
            [ E.el [] <| E.text "⌜"
            , E.el [] <| E.text "⌝"
            ]
        ]


golink zknid color =
    E.link
        [ EF.color color ]
        { url = D.editNoteLink zknid
        , label =
            E.el
                [ E.inFront (E.el [ EF.size 50 ] <| E.text "↗")
                , EF.size 50
                ]
            <|
                E.text "☐"
        }
