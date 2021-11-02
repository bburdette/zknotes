module GenDialog exposing (..)

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
import Time exposing (Zone)
import Util



{-

   -- how to use:

   view : Model -> Element Msg
   view model =
       case model.dialog of
           Just dialog ->
               D.view dialog |> E.map DialogMsg

           Nothing ->
               normalview model

   -- or for a dialog centered relative to the window, not the underlying view,
   -- add it at the layout level.  TBD

-}


type Transition model return
    = Dialog model
    | Ok return
    | Cancel


type Msg msg
    = EltMsg msg
    | CancelClick
    | Noop


type alias Model model msg return =
    { view : Maybe Util.Size -> model -> Element msg
    , update : msg -> model -> Transition model return
    , model : model
    , underLay : Element ()
    }


update : Msg msg -> Model model msg return -> Transition (Model model msg return) return
update msg model =
    case msg of
        EltMsg emsg ->
            case model.update emsg model.model of
                Dialog m ->
                    Dialog { model | model = m }

                Ok r ->
                    Ok r

                Cancel ->
                    Cancel

        CancelClick ->
            Cancel

        Noop ->
            Dialog model


view : Maybe Util.Size -> Model model msg return -> Element (Msg msg)
view mbmax model =
    E.column
        [ E.height E.fill
        , E.width E.fill
        , E.inFront (overlay mbmax model)
        ]
        [ model.underLay
            |> E.map (\_ -> Noop)
        ]


layout : Maybe Util.Size -> Model model msg return -> Html (Msg msg)
layout mbmax model =
    E.layout
        [ E.inFront (overlay mbmax model)
        ]
        (model.underLay
            |> E.map (\_ -> Noop)
        )


overlay : Maybe Util.Size -> Model model msg return -> Element (Msg msg)
overlay mbmax model =
    E.column
        [ E.width E.fill
        , E.height E.fill
        , EBg.color <| E.rgba 0.5 0.5 0.5 0.5
        , E.inFront (dialogView mbmax model)
        , EE.onClick CancelClick
        ]
        []


dialogView : Maybe Util.Size -> Model model msg return -> Element (Msg msg)
dialogView mbmax model =
    E.column
        [ E.height (mbmax |> Maybe.map (\x -> E.maximum x.height E.fill) |> Maybe.withDefault E.fill)
        , E.width (mbmax |> Maybe.map (\x -> E.maximum x.width E.fill) |> Maybe.withDefault E.fill)
        , EB.color <| E.rgb 0 0 0
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
        [ E.row
            [ E.centerX
            , E.centerY
            ]
            [ E.map EltMsg
                (model.view
                    (mbmax |> Maybe.map (\s -> { width = s.width - 30, height = s.height - 30 }))
                    model.model
                )
            ]
        ]
