module FilesDialog exposing (GDModel, Model, Msg(..), init, update, view)

import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import File
import GenDialog as GD
import Orgauth.Data as Data
import TangoColors as TC
import Time exposing (Zone)
import Util


type alias Model =
    { files : List Data.ZkListNote
    }


type Msg
    = OkClick
    | CancelClick
    | Noop


type alias GDModel =
    GD.Model Model Msg ()


init : List Data.ZkListNote -> List (E.Attribute Msg) -> Element () -> GDModel
init files buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model = { files = files }
    , underLay = underLay
    }


view : List (E.Attribute Msg) -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle mbsize model =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.shrink
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "Files Uploaded"
        , E.column [ E.width E.fill, E.height E.fill ]
            (model.files
                |> List.map (\f -> E.paragraph [] [ E.text f.title ])
            )
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                (E.alignRight :: buttonStyle)
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model ()
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok
                ()

        Noop ->
            GD.Dialog model
