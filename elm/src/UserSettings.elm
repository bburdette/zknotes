module UserSettings exposing (..)

import Common exposing (buttonStyle)
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import TangoColors as Color


type Msg
    = Noop
    | DonePress
    | LogOutPress


type Command
    = Done
    | LogOut
    | None


type alias Model =
    { login : Data.Login
    }


view : Model -> Element Msg
view model =
    E.column [ E.width E.fill, E.height E.fill ]
        [ EI.button buttonStyle { onPress = Just DonePress, label = E.text "back" }
        , EI.button buttonStyle { onPress = Just LogOutPress, label = E.text "log out" }
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        DonePress ->
            ( model, Done )

        LogOutPress ->
            ( model, LogOut )

        Noop ->
            ( model, None )
