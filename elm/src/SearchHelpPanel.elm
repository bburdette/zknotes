module SearchHelpPanel exposing (..)

import Common
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
import TangoColors as Color


type alias Model =
    { tab : HelpTab }


type HelpTab
    = Basic
    | Compound
    | Modifiers
    | FullMonty


type Msg
    = SetTab HelpTab


init : Model
init =
    { tab = Basic }


tabChoice : HelpTab -> HelpTab -> String -> Element Msg
tabChoice currenttab thistab txt =
    if currenttab == thistab then
        el [ Font.bold, onClick (SetTab thistab) ] (text txt)

    else
        el [ onClick (SetTab thistab) ] (text txt)


navbar : Int -> Model -> Element Msg
navbar level m =
    Common.navbar level
        m.tab
        SetTab
        [ ( Basic, "Basic" )
        , ( Compound, "Compound" )
        , ( Modifiers, "Modifiers" )
        , ( FullMonty, "Full Monty" )
        ]


update : Model -> Msg -> Model
update model msg =
    case msg of
        SetTab tab ->
            { model | tab = tab }


showLine : String -> Element Msg
showLine s =
    paragraph [ width fill ] [ text s ]


indent : Element a -> Element a
indent elt =
    row [ width fill ] [ el [ width (px 15) ] none, elt ]


thingAndDef : Int -> String -> String -> Element Msg
thingAndDef dportion th def =
    indent <|
        row [ width fill ]
            [ paragraph [ width <| fillPortion 1 ] [ text th ]
            , paragraph [ width <| fillPortion dportion ] [ text def ]
            ]


view : Int -> Model -> Element Msg
view nblevel hmod =
    column
        [ width fill

        -- , Background.color <| Common.navbarColor (nblevel + 1)
        , spacing 2
        ]
        [ el [ Font.bold, centerX ] <| text "search help"
        , navbar nblevel hmod
        , paragraph [ width fill ]
            [ case hmod.tab of
                Basic ->
                    column []
                        [ showLine "You can write simple queries by typing text into the box.  Items that contain the text in their names will be shown.  If there are many items you can see more with the 'Show More' and 'Show All' buttons at the bottom of the listing" ]

                Compound ->
                    column [ width fill ]
                        [ showLine "You can match on multiple strings with single quotes. Link the strings together with & or | ('and' or 'or')"
                        , thingAndDef 2 "'string1' & 'string2' =" "match items that contain both string1 and string2 in their name."
                        , thingAndDef 2 "'string1' | 'string2' =" "match items that contain either string1 or string2 in their name."
                        , thingAndDef 2 "!'string1' =" "match items that do not have string1 in their name."
                        ]

                Modifiers ->
                    column [ width fill ]
                        [ showLine "the quoted strings can take a number of modifiers:"
                        , thingAndDef 4 "t'string1' =" "match items with a parent tag that contains 'string1' in its name."
                        , thingAndDef 4 "c'String1' =" "match items whose name contains String1, with that specific case (capital S, lower case tring)"
                        , thingAndDef 4 "e'String1' =" "match items whose names are exactly 'String1', no more, no less."
                        , thingAndDef 4 "a'string1' =" "match items with an 'ancestor' tag containing string1.  That could be a parent tag of the item, or grandparent tag, or great-grandparent tag, etc."
                        , thingAndDef 4 "d'elements' =" "match items with the string 'elements' in their description."
                        , thingAndDef 4 "aec'Music' =" "match items with an ancestor with name exactly 'Music'.  'music' is not matched, nor is 'music stuff'."
                        ]

                FullMonty ->
                    column [ width fill ]
                        [ showLine "put all the elements together to make complex queries!  For instance "
                        , thingAndDef 2 "a'sheet music' & t'tango' & ea'G' =" "match items with an ancestor tag 'sheet music', a parent tag 'tango', and an exact ancestor tag of 'G'."
                        ]
            ]
        ]