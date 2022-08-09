module InviteUser exposing (GDModel, Model, Msg(..), init, update, view)

import Data exposing (zklKey)
import Dict exposing (Dict(..))
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import GenDialog as GD
import Orgauth.Data as OD
import SearchPanel as SP
import TangoColors as TC
import Time exposing (Zone)
import Util


type alias Model =
    { loginData : OD.LoginData
    , email : String
    , spmodel : SP.Model
    , zklDict : Dict String Data.EditLink
    , zknSearchResult : Data.ZkListNoteSearchResult
    }


type Msg
    = EmailChanged String
    | OkClick
    | CancelClick
    | Noop


type alias GDModel =
    GD.Model Model Msg OD.GetInvite


init : SP.Model -> Data.ZkListNoteSearchResult -> List Data.EditLink -> OD.LoginData -> List (E.Attribute Msg) -> Element () -> GDModel
init spmodel spresult links loginData buttonStyle underLay =
    { view = view buttonStyle
    , update = update
    , model =
        { loginData = loginData
        , email = ""
        , spmodel = spmodel
        , zklDict =
            Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) links)
        , zknSearchResult = spresult
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
        [ EI.text []
            { onChange = EmailChanged
            , text = model.email
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "email")
            }
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model OD.GetInvite
update msg model =
    case msg of
        EmailChanged s ->
            GD.Dialog { model | email = s }

        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok
                { email = Just model.email
                , data = Nothing
                }

        Noop ->
            GD.Dialog model
