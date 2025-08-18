module ZkCommon exposing (..)

import Data exposing (ZkNoteId)
import DataUtil as DU exposing (zniEq)
import Element as E
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
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


archiveColor : E.Color
archiveColor =
    TC.darkYellow


shareColor : E.Color
shareColor =
    TC.darkBrown


systemColor : DU.Sysids -> List ZkNoteId -> Maybe E.Color
systemColor ld ids =
    let
        sysColor : ZkNoteId -> Maybe E.Color
        sysColor color =
            if zniEq color ld.archiveid then
                Just archiveColor

            else if zniEq color ld.publicid then
                Just publicColor

            else if zniEq color ld.shareid then
                Just shareColor

            else if zniEq color ld.searchid then
                Just searchColor

            else if zniEq color ld.commentid then
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


golink size zknid color =
    E.link
        [ EF.color color ]
        { url = DU.editNoteLink zknid
        , label =
            E.el
                [ E.inFront (E.el [ EF.size size ] <| E.text "↗")
                , EF.size size
                ]
            <|
                E.text "☐"
        }


stringToolTip : String -> E.Element msg
stringToolTip str =
    E.el
        [ EBk.color (E.rgb 0.6 0.6 0.6)
        , EF.color (E.rgb 1 1 1)
        , E.padding 4
        , EBd.rounded 5
        , EF.size 14
        , EBd.shadow
            { offset = ( 0, 3 ), blur = 6, size = 0, color = E.rgba 0 0 0 0.32 }
        ]
        (E.text str)
