module ChangePassword exposing (GDModel, Model, Msg(..), init, update, view)

import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import GenDialog as GD
import TangoColors as TC
import Time exposing (Zone)
import Util


type alias Model =
    { loginData : Data.LoginData
    , oldpwd : String
    , newpwd : String
    }


type Msg
    = OldPwdChanged String
    | NewPwdChanged String
    | OkClick
    | CancelClick
    | Noop


type alias GDModel =
    GD.Model Model Msg Data.ChangePassword


init : Data.LoginData -> List (E.Attribute Msg) -> Element () -> GDModel
init loginData buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model = { loginData = loginData, oldpwd = "", newpwd = "" }
    , underLay = underLay
    }


view : List (E.Attribute Msg) -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle mbsize model =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.shrink
        , E.spacing 10
        ]
        [ EI.currentPassword []
            { onChange = OldPwdChanged
            , text = model.oldpwd
            , placeholder = Nothing
            , show = False
            , label = EI.labelLeft [] (E.text "old password")
            }
        , EI.newPassword []
            { onChange = NewPwdChanged
            , text = model.newpwd
            , placeholder = Nothing
            , show = False
            , label = EI.labelLeft [] (E.text "new password")
            }
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model Data.ChangePassword
update msg model =
    case msg of
        OldPwdChanged s ->
            GD.Dialog { model | oldpwd = s }

        NewPwdChanged s ->
            GD.Dialog { model | newpwd = s }

        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok
                { oldpwd = model.oldpwd
                , newpwd = model.newpwd
                }

        Noop ->
            GD.Dialog model
