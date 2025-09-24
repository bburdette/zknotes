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
import Markdown.Block as MB exposing (Inline(..))
import MdGui as MG
import Orgauth.Data as Data
import TangoColors as TC
import Util


type alias Model =
    { mobile : Bool
    , inline : MB.Inline
    , transforms : List ( String, MB.Inline )
    , selectedTf : Maybe Int
    , tomsg : MB.Inline -> MG.Msg
    }


type Msg
    = OkClick
    | CancelClick
    | OnSelect Int
    | Noop


type Command
    = UpdateInline MG.Msg
    | Close


type alias GDModel =
    GD.Model Model Msg Command


init : MB.Inline -> (MB.Inline -> MG.Msg) -> Bool -> List (E.Attribute Msg) -> Element () -> GDModel
init inline fn mobile buttonStyle underLay =
    let
        tfs =
            transforms inline
    in
    { view = view buttonStyle
    , update = update
    , model =
        { mobile = mobile
        , inline = inline
        , transforms = tfs
        , selectedTf =
            if List.isEmpty tfs then
                Nothing

            else
                Just 0
        , tomsg = fn
        }
    , underLay = underLay
    }


transforms : MB.Inline -> List ( String, MB.Inline )
transforms inline =
    case inline of
        HtmlInline htmlBlock ->
            []

        Link url _ inlines ->
            [ ( "yeet", MB.HtmlInline (MB.HtmlElement "yeet" [ { name = "url", value = url } ] []) ) ]

        Image src _ inlines ->
            []

        Emphasis inlines ->
            []

        Strong inlines ->
            []

        Strikethrough inlines ->
            []

        CodeSpan s ->
            []

        Text s ->
            []

        HardLineBreak ->
            []


view : List (E.Attribute Msg) -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle mbsize model =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.map E.px |> Maybe.withDefault E.fill)
        , E.height E.fill
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "inline xform"

        -- , E.row [ E.width E.fill, E.height E.fill, E.scrollbarY, E.spacing 8 ] [
        , E.row [ E.width E.fill, E.height E.fill, E.scrollbarY, E.spacing 8 ]
            [ EI.radio []
                { onChange = OnSelect
                , options = model.transforms |> List.indexedMap (\i ( name, _ ) -> EI.option i (E.text name))
                , selected = model.selectedTf
                , label = EI.labelAbove [] (E.text "xform")
                }
            , E.text "blah"
            ]
        , if model.mobile then
            E.none

          else
            E.row [ E.width E.fill, E.spacing 10 ]
                [ EI.button
                    (E.alignLeft :: buttonStyle)
                    { onPress = Just CancelClick, label = E.text "cancel" }
                , EI.button
                    (E.alignRight :: buttonStyle)
                    { onPress = Just OkClick, label = E.text "ok" }
                ]
        ]


update : Msg -> Model -> GD.Transition Model Command
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            model.selectedTf
                |> Maybe.andThen (\i -> List.head (List.drop i model.transforms))
                |> Maybe.map Tuple.second
                |> Maybe.map (model.tomsg >> UpdateInline >> GD.Ok)
                |> Maybe.withDefault GD.Cancel

        -- ClearClick jobno ->
        --     GD.Dialog { model | jobs = Dict.remove jobno model.jobs }
        OnSelect i ->
            GD.Dialog { model | selectedTf = Just i }

        Noop ->
            GD.Dialog model
