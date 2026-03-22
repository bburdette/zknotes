module PickColorDialog exposing (GDModel, Model, Msg(..), init, update, view)

import Color exposing (Color(..))
import ColorPicker
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import GenDialog as GD
import Orgauth.Data as Data
import TangoColors as TC
import Time exposing (Zone)
import Util


type alias Model =
    { colorPicker : ColorPicker.State
    , color : Color
    }


type Msg
    = ColorPickerMsg ColorPicker.Msg
    | OkClick
    | CancelClick
    | Noop


type alias GDModel =
    GD.Model Model Msg Color


init : Color -> List (E.Attribute Msg) -> Element () -> GDModel
init initColor buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model =
        { colorPicker = ColorPicker.empty
        , color = initColor
        }
    , underLay = underLay
    }


view : List (E.Attribute Msg) -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle mbsize model =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.shrink
        , E.spacing 10
        ]
        [ ColorPicker.view model.color model.colorPicker
            |> E.html
            |> E.map ColorPickerMsg
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model Color
update msg model =
    case msg of
        ColorPickerMsg cpm ->
            let
                ( m, colour ) =
                    ColorPicker.update cpm model.color model.colorPicker
            in
            GD.Dialog
                { model
                    | colorPicker = m
                    , color = colour |> Maybe.withDefault model.color
                }

        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok model.color

        Noop ->
            GD.Dialog model
