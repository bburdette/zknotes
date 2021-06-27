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
    | ChangePassPress
    | LogOutPress


type Command
    = Done
    | LogOut
    | ChangePassword
    | None


type alias Model =
    { login : Data.LoginData
    }


init : Data.LoginData -> Model
init login =
    { login = login }


view : Model -> Element Msg
view model =
    E.row [ E.width E.fill, E.height E.fill ]
        [ E.column [ E.centerX ]
            [ E.row [ E.width E.fill ]
                [ EI.button buttonStyle { onPress = Just DonePress, label = E.text "back" }
                , EI.button (E.alignRight :: buttonStyle) { onPress = Just LogOutPress, label = E.text "log out" }
                ]
            , E.row []
                [ E.text "user: "
                , E.el [ EF.bold ] <| E.text model.login.name
                ]
            , EI.button buttonStyle { onPress = Just ChangePassPress, label = E.text "change password" }

            -- , EI.button buttonStyle { onPress = Just ChangeEmailPress, label = E.text "change email" }
            ]
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        DonePress ->
            ( model, Done )

        LogOutPress ->
            ( model, LogOut )

        ChangePassPress ->
            ( model, None )

        Noop ->
            ( model, None )
