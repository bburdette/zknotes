module Dialog exposing (..)

import Common exposing (buttonStyle)
import Element as E exposing (Element)
import Element.Background as EBg
import Element.Border as EB
import Element.Events as EE
import Element.Input as EI
import Html exposing (Html)
import Html.Events as HE
import Http
import Json.Decode as JD
import Task
import Time
import Util


type Transition
    = Dialog Model
    | Ok
    | Cancel


type Msg
    = OkClick
    | CancelClick
    | Noop


type alias Model =
    { message : String
    , showCancel : Bool
    , underLay : Util.Size -> Element ()
    }


init : String -> Bool -> (Util.Size -> Element ()) -> Model
init message showcancel underLay =
    { message = message
    , showCancel = showcancel
    , underLay = underLay
    }


update : Msg -> Model -> Transition
update msg model =
    case msg of
        OkClick ->
            Ok

        CancelClick ->
            Cancel

        Noop ->
            Dialog model


view : Util.Size -> Model -> Element Msg
view size model =
    E.column
        [ E.height E.fill
        , E.width E.fill
        , E.inFront (overlay model)
        ]
        [ model.underLay size
            |> E.map (\_ -> Noop)
        ]


overlay : Model -> Element Msg
overlay model =
    E.column
        [ E.height E.fill
        , E.width E.fill
        , EBg.color <| E.rgba 0.5 0.5 0.5 0.5
        , E.inFront (dialogView model)
        , EE.onClick CancelClick
        ]
        []


dialogView : Model -> Element Msg
dialogView model =
    E.column
        [ EB.color <| E.rgb 0 0 0
        , E.centerX
        , E.centerY
        , EB.width 5
        , EBg.color <| E.rgb 1 1 1
        , E.paddingXY 10 10
        , E.spacing 5
        , E.htmlAttribute <|
            HE.custom "click"
                (JD.succeed
                    { message = Noop
                    , stopPropagation = True
                    , preventDefault = True
                    }
                )
        ]
        [ E.row [ E.centerX ] [ E.text model.message ]
        , E.row [ E.width E.fill ] <|
            [ EI.button buttonStyle { label = E.text "ok", onPress = Just OkClick }
            , if model.showCancel then
                EI.button (buttonStyle ++ [ E.alignRight ]) { label = E.text "cancel", onPress = Just CancelClick }

              else
                E.none
            ]
        ]
