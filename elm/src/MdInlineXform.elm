module MdInlineXform exposing (..)

import Common
import Data
import DataUtil
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import GenDialog as GD
import Html.Attributes as HA
import Http
import Orgauth.Data as Data
import TangoColors as TC
import Util


type alias TJobs =
    { mobile : Bool, jobs : Dict Int Data.JobStatus }


type Msg
    = OkClick
    | CancelClick
    | ClearClick Int
    | Noop


type Command
    = Tag (List Data.ZkListNote)
    | Close


type alias GDModel =
    GD.Model TJobs Msg Command


init : TJobs -> List (E.Attribute Msg) -> Element () -> GDModel
init trqs buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model = trqs
    , underLay = underLay
    }


renderProgress : Http.Progress -> Element Msg
renderProgress p =
    case p of
        Http.Sending ps ->
            Http.fractionSent ps
                |> String.fromFloat
                |> (\s -> s ++ " sent")
                |> E.text

        Http.Receiving pr ->
            Http.fractionReceived pr
                |> String.fromFloat
                |> (\s -> s ++ " received")
                |> E.text


view : List (E.Attribute Msg) -> Maybe Util.Size -> TJobs -> Element Msg
view buttonStyle mbsize trqs =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.map E.px |> Maybe.withDefault E.fill)
        , E.height E.fill
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "server jobs"
        , E.column [ E.width E.fill, E.height E.fill, E.scrollbarY, E.spacing 8 ]
            (trqs.jobs
                |> Dict.toList
                |> List.reverse
                |> List.map
                    (\( jobno, js ) ->
                        E.column
                            [ E.width E.fill
                            , EBk.color (Common.navbarColor 2)
                            , EBd.rounded 10
                            , E.padding 10
                            , E.spacing 8
                            ]
                            [ case js.state of
                                Data.Started ->
                                    E.el [ E.centerX, EF.bold ] <| E.text "started..."

                                Data.Running ->
                                    E.el [ E.centerX, EF.bold ] <| E.text "running..."

                                Data.Completed ->
                                    E.el [ E.centerX, EF.bold ] <| E.text <| "completed"

                                Data.Failed ->
                                    E.el [ E.centerX, EF.bold ] <| E.text "failed"
                            , E.row [ E.width E.fill ]
                                [ E.column
                                    [ EBd.width 3
                                    , EBd.color TC.darkGrey
                                    , E.width E.fill
                                    , E.height <| E.maximum 200 E.fill
                                    , E.scrollbarY
                                    ]
                                    [ E.paragraph
                                        [ E.htmlAttribute (HA.style "overflow-wrap" "break-word")
                                        , E.htmlAttribute (HA.style "word-break" "break-word")
                                        ]
                                        [ E.text js.message ]
                                    ]
                                ]
                            , if DataUtil.jobComplete js.state then
                                E.row [ E.width E.fill ]
                                    [ EI.button (E.alignRight :: buttonStyle)
                                        { onPress = Just (ClearClick jobno), label = E.text "clear" }
                                    ]

                              else
                                E.none
                            ]
                    )
            )
        , if trqs.mobile then
            E.none

          else
            E.row [ E.width E.fill, E.spacing 10 ]
                [ EI.button
                    (E.centerX :: buttonStyle)
                    { onPress = Just CancelClick, label = E.text "close" }
                ]
        ]


update : Msg -> TJobs -> GD.Transition TJobs Command
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok Close

        ClearClick jobno ->
            GD.Dialog { model | jobs = Dict.remove jobno model.jobs }

        Noop ->
            GD.Dialog model
