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
                        [ showLine "You can write simple queries by typing text into the box.  Items that contain the text in their titles will be shown." ]

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
                        , thingAndDef 4 "t'string1' =" "match items that link to items with 'string1' in the name."
                        , thingAndDef 4 "e'String1' =" "match items whose names are exactly 'String1', no more, no less."
                        , thingAndDef 4 "n'elements' =" "match items with the string 'elements' in the note body.."
                        ]

                FullMonty ->
                    column [ width fill ]
                        [ showLine "put all the elements together to make complex queries!  For instance "
                        , thingAndDef 2 "t'sheet music' & 'tango' & te'G' =" "match items linked to 'sheet music', with 'tango' in the name, and also linking to an item named 'G'"
                        ]
            ]
        ]
