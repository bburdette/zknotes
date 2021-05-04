module PaginationPanel exposing (Command(..), Model, Msg(..), initModel, searchResultUpdated, update, view)

import Common exposing (buttonStyle)
import Data
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
    , end : Bool
    }


initModel : Model
initModel =
    { increment = S.defaultSearchLimit
    , offset = 0
    , end = False
    }


type Msg
    = NextClick
    | PrevClick


type Command
    = None
    | RangeChanged


searchResultUpdated : Data.ZkListNoteSearchResult -> Model -> Model
searchResultUpdated zsr model =
    { model | end = List.length zsr.notes < model.increment }


view : Model -> Element Msg
view model =
    E.row [ E.spacing 8 ]
        [ if model.offset > 0 then
            Input.button buttonStyle
                { onPress = Just PrevClick
                , label = E.text "prev"
                }

          else
            Input.button Common.disabledButtonStyle
                { onPress = Nothing
                , label = E.text "prev"
                }
        , if model.end then
            Input.button Common.disabledButtonStyle
                { onPress = Nothing
                , label = E.text "next"
                }

          else
            Input.button buttonStyle
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
