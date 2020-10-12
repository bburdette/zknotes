module PaginationPanel exposing (Command(..), Model, Msg(..), addTagToSearch, addTagToSearchPrev, addToSearch, initModel, toggleHelpButton, update, updateSearchText, view)

import Common exposing (buttonStyle)
import Element exposing (..)
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
    , lower : Int
    }


initModel : Model
initModel =
    { increment = 50
    , lower = 0
    }


type Msg
    = NextClick
    | PrevClick


type Command
    = None
    | RangeChanged


view : Model -> Element Msg
view model =
    row []
        [ Input.button buttonStyle
            { onPress = Just PrevClick
            , label = text "prev"
            }
        , Input.button buttonStyle
            { onPress = Just NextClick
            , label = text "next"
            }
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        NextClick ->
            ( { model | lower = lower + increment }
            , RangeChanged
            )

        PrevClick ->
            ( { model | lower = max (lower + increment) 0 }
            , RangeChanged
            )
