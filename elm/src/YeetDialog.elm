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
    , linkback : Bool
    }


type Msg
    = OkClick
    | CancelClick
    | OnUrlChanged String
    | OnLinkbackChanged Bool
      -- | TagClick String
      -- | ClearClick String
    | Noop


type Command
    = Ok
    | Cancel


type alias GDModel =
    GD.Model Model Msg Command


init : String -> Bool -> List (E.Attribute Msg) -> Element () -> GDModel
init url linkback buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model =
        { url = url
        , linkback = linkback
        }
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
            , label =
                EI.labelLeft
                    []
                    (E.text
                        "url"
                    )
            }
        , EI.checkbox
            []
            { checked = model.linkback
            , icon = EI.defaultCheckbox
            , label = EI.labelLeft [] (E.text "linkback")
            , onChange = OnLinkbackChanged
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


update : Msg -> Model -> GD.Transition Model Command
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok Ok

        OnUrlChanged s ->
            GD.Dialog { model | url = s }

        OnLinkbackChanged b ->
            GD.Dialog { model | linkback = b }

        Noop ->
            GD.Dialog model
