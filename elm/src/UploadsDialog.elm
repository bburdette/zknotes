module UploadsDialog exposing (..)

import Data
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import File
import GenDialog as GD
import Http
import Orgauth.Data as Data
import TangoColors as TC
import Time exposing (Zone)
import Util


type alias TRequests =
    { requestCount : Int, requests : Dict String TRequest }


type TRequest
    = FileUpload { filenames : List String, progress : Maybe Http.Progress }



-- type alias Model = {}


type Msg
    = OkClick
    | CancelClick
    | Noop


type alias GDModel =
    GD.Model TRequests Msg ()


init : TRequests -> List (E.Attribute Msg) -> Element () -> GDModel
init trqs buttonStyle underLay =
    let
        _ =
            Debug.log "uplaods idalog init " trqs
    in
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


view : List (E.Attribute Msg) -> Maybe Util.Size -> TRequests -> Element Msg
view buttonStyle mbsize trqs =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.fill
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "Files Uploaded"
        , E.column [ E.width E.fill, E.height E.fill, E.scrollbarY ]
            (trqs.requests
                |> Dict.values
                |> List.map
                    (\tr ->
                        case tr of
                            FileUpload fu ->
                                E.row [ E.width E.fill ]
                                    [ E.column [ E.width E.fill ]
                                        (fu.filenames
                                            |> List.map (\fn -> E.paragraph [] [ E.text fn ])
                                        )
                                    , fu.progress
                                        |> Maybe.map renderProgress
                                        |> Maybe.withDefault E.none
                                    ]
                    )
            )
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                (E.alignRight :: buttonStyle)
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> TRequests -> GD.Transition TRequests ()
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok
                ()

        Noop ->
            GD.Dialog model
