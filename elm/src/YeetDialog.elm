module YeetDialog exposing (..)

import Common
import Data
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import GenDialog as GD
import Http
import Orgauth.Data as Data
import TangoColors as TC
import Util


type alias Model =
    { url : String
    }


type Msg
    = OkClick
    | CancelClick
    | OnUrlChanged String
    | Noop




type alias GDModel =
    GD.Model Model Msg Data.Yeet


init : String -> List (E.Attribute Msg) -> Element () -> GDModel
init url buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model = { url = url }
    , underLay = underLay
    }


view : List (E.Attribute Msg) -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle mbsize model =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.fill
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "http requests"
        , EI.text [ E.width E.fill ]
            { onChange = OnUrlChanged
            , text = model.url
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "url")
            }
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button
                (E.centerX :: buttonStyle)
                { onPress = Just OkClick, label = E.text "ok" }
            , EI.button
                (E.centerX :: buttonStyle)
                { onPress = Just CancelClick, label = E.text "cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model Data.Yeet
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok { url = model.url, audio = True }

        OnUrlChanged s ->
            GD.Dialog { model | url = s }

        Noop ->
            GD.Dialog model
