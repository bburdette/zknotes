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


type Transition prevmodel
    = Dialog (Model prevmodel)
    | Ok
    | Cancel


type Msg
    = OkClick
    | CancelClick
    | Noop


type alias Model prevmodel =
    { message : String
    , prevModel : prevmodel
    , prevRender : prevmodel -> Element ()
    }


init : String -> a -> (a -> Element ()) -> Model a
init message prevmod render =
    { message = message
    , prevModel = prevmod
    , prevRender = render
    }


update : Msg -> Model a -> Transition a
update msg model =
    case msg of
        OkClick ->
            Ok

        CancelClick ->
            Cancel

        Noop ->
            Dialog model


view : Model a -> Html Msg
view model =
    E.layout
        [ E.height E.fill
        , E.width E.fill
        , E.inFront (overlay model)
        ]
        (model.prevRender model.prevModel
            |> E.map (\_ -> Noop)
        )


overlay : Model a -> Element Msg
overlay model =
    E.column
        [ E.height E.fill
        , E.width E.fill
        , EBg.color <| E.rgba 0.5 0.5 0.5 0.5
        , E.inFront (dialogView model)
        , EE.onClick CancelClick
        ]
        []


dialogView : Model a -> Element Msg
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
        , E.row [ E.width E.fill ]
            [ EI.button buttonStyle { label = E.text "Ok", onPress = Just OkClick }
            , EI.button (buttonStyle ++ [ E.alignRight ]) { label = E.text "Cancel", onPress = Just CancelClick }
            ]
        ]
