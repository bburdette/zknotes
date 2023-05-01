module RequestsDialog exposing (..)

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


type alias TRequests =
    { requestCount : Int, requests : Dict String TRequest }


type TRequest
    = FileUpload
        { filenames : List String
        , progress : Maybe Http.Progress
        , files : Maybe (List Data.ZkListNote)
        }
    | Yeet
        { url : String
        , progress : Maybe Http.Progress
        , file : Maybe Data.ZkListNote
        }


type Msg
    = OkClick
    | CancelClick
    | TagClick String
    | ClearClick String
    | Noop


type Command
    = Tag (List Data.ZkListNote)
    | Close


type alias GDModel =
    GD.Model TRequests Msg Command


init : TRequests -> List (E.Attribute Msg) -> Element () -> GDModel
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


view : List (E.Attribute Msg) -> Maybe Util.Size -> TRequests -> Element Msg
view buttonStyle mbsize trqs =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.fill
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "http requests"
        , E.column [ E.width E.fill, E.height E.fill, E.scrollbarY, E.spacing 8 ]
            (trqs.requests
                |> Dict.toList
                |> List.reverse
                |> List.map
                    (\( s, tr ) ->
                        case tr of
                            FileUpload fu ->
                                let
                                    complete =
                                        Util.isJust fu.files
                                in
                                E.column
                                    [ E.width E.fill
                                    , EBk.color (Common.navbarColor 2)
                                    , EBd.rounded 10
                                    , E.padding 10
                                    , E.spacing 8
                                    ]
                                    [ if complete then
                                        E.el [ E.centerX, EF.bold ] <| E.text "file upload complete"

                                      else
                                        E.el [ E.centerX, EF.bold ] <| E.text "uploading..."
                                    , E.row [ E.width E.fill ]
                                        [ E.column
                                            [ EBd.width 3
                                            , EBd.color TC.darkGrey
                                            , E.width E.fill
                                            , E.height <| E.maximum 200 E.fill
                                            , E.scrollbarY
                                            ]
                                            (fu.filenames
                                                |> List.map (\fn -> E.paragraph [] [ E.text fn ])
                                            )
                                        , fu.progress
                                            |> Maybe.map renderProgress
                                            |> Maybe.withDefault E.none
                                        ]
                                    , if complete then
                                        E.row [ E.width E.fill ]
                                            [ EI.button buttonStyle
                                                { onPress = Just (TagClick s), label = E.text "add tags" }
                                            , EI.button (E.alignRight :: buttonStyle)
                                                { onPress = Just (ClearClick s), label = E.text "clear" }
                                            ]

                                      else
                                        E.none
                                    ]

                            Yeet yt ->
                                let
                                    complete =
                                        Util.isJust yt.file
                                in
                                E.column
                                    [ E.width E.fill
                                    , EBk.color (Common.navbarColor 2)
                                    , EBd.rounded 10
                                    , E.padding 10
                                    , E.spacing 8
                                    ]
                                    [ if complete then
                                        E.el [ E.centerX, EF.bold ] <| E.text "yeet complete"

                                      else
                                        E.el [ E.centerX, EF.bold ] <| E.text "yeeting..."
                                    , E.row [ E.width E.fill ]
                                        [ E.column
                                            [ EBd.width 3
                                            , EBd.color TC.darkGrey
                                            , E.width E.fill
                                            , E.height <| E.maximum 200 E.fill
                                            , E.scrollbarY
                                            ]
                                            [ E.paragraph [] [ E.text yt.url ] ]
                                        , yt.progress
                                            |> Maybe.map renderProgress
                                            |> Maybe.withDefault E.none
                                        ]
                                    , if complete then
                                        E.row [ E.width E.fill ]
                                            [ EI.button buttonStyle
                                                { onPress = Just (TagClick s), label = E.text "add tags" }
                                            , EI.button (E.alignRight :: buttonStyle)
                                                { onPress = Just (ClearClick s), label = E.text "clear" }
                                            ]

                                      else
                                        E.none
                                    ]
                    )
            )
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button
                (E.centerX :: buttonStyle)
                { onPress = Just CancelClick, label = E.text "close" }
            ]
        ]


update : Msg -> TRequests -> GD.Transition TRequests Command
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok Close

        TagClick s ->
            case Dict.get s model.requests of
                Just (FileUpload fu) ->
                    case fu.files of
                        Just f ->
                            GD.Ok (Tag f)

                        Nothing ->
                            GD.Dialog model

                Just (Yeet yt) ->
                    case yt.file of
                        Just f ->
                            GD.Ok (Tag [ f ])

                        Nothing ->
                            GD.Dialog model

                Nothing ->
                    GD.Dialog model

        ClearClick s ->
            GD.Dialog { model | requests = Dict.remove s model.requests }

        Noop ->
            GD.Dialog model
