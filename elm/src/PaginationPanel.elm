module PaginationPanel exposing (Command(..), Model, Msg(..), initModel, update, view)

import Common exposing (buttonStyle)
import Element as E exposing (Element)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
import Parser
import Search as S exposing (AndOr(..), SearchMod(..), TSText, TagSearch(..), tagSearchParser)
import SearchHelpPanel
import TDict exposing (TDict)
import TangoColors as Color
import Util exposing (Size)


type alias Model =
    { increment : Int
    , offset : Int
    }


initModel : Model
initModel =
    { increment = S.defaultSearchLimit
    , offset = 0
    }


type Msg
    = NextClick
    | PrevClick


type Command
    = None
    | RangeChanged


view : Model -> Element Msg
view model =
    E.row [ E.spacing 8 ]
        [ Input.button buttonStyle
            { onPress = Just PrevClick
            , label = E.text "prev"
            }
        , Input.button buttonStyle
            { onPress = Just NextClick
            , label = E.text "next"
            }
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        PrevClick ->
            ( { model | offset = max (model.offset - model.increment) 0 }
            , RangeChanged
            )

        NextClick ->
            ( { model | offset = model.offset + model.increment }
            , RangeChanged
            )
