module TangoColors
    exposing
        ( black
        , blue
        , brown
        , charcoal
        , darkBlue
        , darkBrown
        , darkCharcoal
        , darkGray
        , darkGreen
        , darkGrey
        , darkOrange
        , darkPurple
        , darkRed
        , darkYellow
        , gray
        , green
        , grey
        , lightBlue
        , lightBrown
        , lightCharcoal
        , lightGray
        , lightGreen
        , lightGrey
        , lightOrange
        , lightPurple
        , lightRed
        , lightYellow
        , orange
        , purple
        , red
        , white
        , yellow
        )

{-| Library for working with colors. Includes
[RGB](https://en.wikipedia.org/wiki/RGB_color_model) and
[HSL](http://en.wikipedia.org/wiki/HSL_and_HSV) creation, gradients, and
built-in names.


# Built-in Colors

These colors come from the [Tango
palette](http://tango.freedesktop.org/Tango_Icon_Theme_Guidelines)
which provides aesthetically reasonable defaults for colors. Each color also
comes with a light and dark version.


### Standard

@docs red, orange, yellow, green, blue, purple, brown


### Light

@docs lightRed, lightOrange, lightYellow, lightGreen, lightBlue, lightPurple, lightBrown


### Dark

@docs darkRed, darkOrange, darkYellow, darkGreen, darkBlue, darkPurple, darkBrown


### Eight Shades of Grey

These colors are a compatible series of shades of grey, fitting nicely
with the Tango palette.
@docs white, lightGrey, grey, darkGrey, lightCharcoal, charcoal, darkCharcoal, black

These are identical to the _grey_ versions. It seems the spelling is regional, but
that has never helped me remember which one I should be writing.
@docs lightGray, gray, darkGray

-}

import Element exposing (Color, rgb, rgb255, rgba)


-- BUILT-IN COLORS


{-| -}
lightRed : Color
lightRed =
    rgb255 239 41 41


{-| -}
red : Color
red =
    rgb255 204 0 0


{-| -}
darkRed : Color
darkRed =
    rgb255 164 0 0


{-| -}
lightOrange : Color
lightOrange =
    rgb255 252 175 62


{-| -}
orange : Color
orange =
    rgb255 245 121 0


{-| -}
darkOrange : Color
darkOrange =
    rgb255 206 92 0


{-| -}
lightYellow : Color
lightYellow =
    rgb255 255 233 79


{-| -}
yellow : Color
yellow =
    rgb255 237 212 0


{-| -}
darkYellow : Color
darkYellow =
    rgb255 196 160 0


{-| -}
lightGreen : Color
lightGreen =
    rgb255 138 226 52


{-| -}
green : Color
green =
    rgb255 115 210 22


{-| -}
darkGreen : Color
darkGreen =
    rgb255 78 154 6


{-| -}
lightBlue : Color
lightBlue =
    rgb255 114 159 207


{-| -}
blue : Color
blue =
    rgb255 52 101 164


{-| -}
darkBlue : Color
darkBlue =
    rgb255 32 74 135


lightPurple : Color
lightPurple =
    rgb255 173 127 168


{-| -}
purple : Color
purple =
    rgb255 117 80 123


{-| -}
darkPurple : Color
darkPurple =
    rgb255 92 53 102


{-| -}
lightBrown : Color
lightBrown =
    rgb255 233 185 110


{-| -}
brown : Color
brown =
    rgb255 193 125 17


{-| -}
darkBrown : Color
darkBrown =
    rgb255 143 89 2


{-| -}
black : Color
black =
    rgb255 0 0 0


{-| -}
white : Color
white =
    rgb255 255 255 255


{-| -}
lightGrey : Color
lightGrey =
    rgb255 238 238 236


{-| -}
grey : Color
grey =
    rgb255 211 215 207


{-| -}
darkGrey : Color
darkGrey =
    rgb255 186 189 182


{-| -}
lightGray : Color
lightGray =
    rgb255 238 238 236


{-| -}
gray : Color
gray =
    rgb255 211 215 207


{-| -}
darkGray : Color
darkGray =
    rgb255 186 189 182


{-| -}
lightCharcoal : Color
lightCharcoal =
    rgb255 136 138 133


{-| -}
charcoal : Color
charcoal =
    rgb255 85 87 83


{-| -}
darkCharcoal : Color
darkCharcoal =
    rgb255 46 52 54
