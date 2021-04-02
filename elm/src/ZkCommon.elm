module ZkCommon exposing (..)

import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import TangoColors as TC


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
