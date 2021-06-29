module ChangeEmail exposing (GDModel, Model, Msg(..), init, update, view)

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
    , pwd : String
    , email : String
    }


type Msg
    = PwdChanged String
    | EmailChanged String
    | OkClick
    | CancelClick
    | Noop


type alias GDModel =
    GD.Model Model Msg Data.ChangeEmail


init : Data.LoginData -> List (E.Attribute Msg) -> Element () -> GDModel
init loginData buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model = { loginData = loginData, pwd = "", email = "" }
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
            { onChange = PwdChanged
            , text = model.pwd
            , placeholder = Nothing
            , show = False
            , label = EI.labelLeft [] (E.text "password")
            }
        , EI.text []
            { onChange = EmailChanged
            , text = model.email
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "new email")
            }
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model Data.ChangeEmail
update msg model =
    case msg of
        PwdChanged s ->
            GD.Dialog { model | pwd = s }

        EmailChanged s ->
            GD.Dialog { model | email = s }

        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok
                { pwd = model.pwd
                , email = model.email
                }

        Noop ->
            GD.Dialog model
