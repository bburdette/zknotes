module UserSettings exposing (..)

import Common exposing (buttonStyle)
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import TangoColors as TC


type Msg
    = Noop
    | DonePress
    | ChangePassPress
    | ChangeEmailPress
    | LogOutPress


type Command
    = Done
    | LogOut
    | ChangePassword
    | ChangeEmail
    | None


type alias Model =
    { login : Data.LoginData
    }


init : Data.LoginData -> Model
init login =
    { login = login }


view : Model -> Element Msg
view model =
    E.row [ E.width E.fill, E.height E.fill, EBk.color TC.lightGrey ]
        [ E.column
            [ E.centerX
            , E.width (E.maximum 400 E.fill)
            , E.spacing 8
            ]
            [ E.row [ E.width E.fill ]
                [ EI.button buttonStyle { onPress = Just DonePress, label = E.text "done" }
                ]
            , E.column
                [ E.centerX
                , E.width (E.maximum 400 E.fill)
                , EBd.width 1
                , EBd.color TC.darkGrey
                , EBd.rounded 10
                , E.padding 5
                , EBk.color TC.white
                , E.spacing 8
                ]
                [ E.el [ EF.bold, E.centerX ] <| E.text "user settings"
                , E.row [ E.width E.fill ]
                    [ E.text "logged in as user: "
                    , E.el [ EF.bold ] <| E.text model.login.name
                    , EI.button (E.alignRight :: buttonStyle) { onPress = Just LogOutPress, label = E.text "log out" }
                    ]
                , EI.button (E.centerX :: buttonStyle) { onPress = Just ChangePassPress, label = E.text "change password" }
                , EI.button (E.centerX :: buttonStyle) { onPress = Just ChangeEmailPress, label = E.text "change email" }
                ]
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
            ( model, ChangePassword )

        ChangeEmailPress ->
            ( model, ChangeEmail )

        Noop ->
            ( model, None )
