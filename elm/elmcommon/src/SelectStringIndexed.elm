module SelectString exposing (GDModel, Model, Msg(..), init, update, view)

import Array as A exposing (Array)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import GenDialog as GD
import Html.Attributes
import TangoColors as TC
import Time exposing (Zone)
import Util


type alias GDModel =
    GD.Model Model Msg Int


type alias Model =
    { choices : Array String
    , selected : Maybe Int
    , search : String
    , buttonStyle : List (E.Attribute Msg)
    }


type alias Palette =
    {}


type Msg
    = RowClick Int
    | OkClick
    | CancelClick
    | SearchChanged String
    | Noop


selectedrow : List (E.Attribute Msg)
selectedrow =
    [ EBk.color TC.lightBlue ]


view : Maybe Util.Size -> Model -> Element Msg
view mbmax model =
    let
        ls =
            String.toLower model.search
    in
    E.column
        [ E.spacing 10
        , E.height E.fill
        , E.width E.fill
        ]
        [ EI.text []
            { onChange = SearchChanged
            , text = model.search
            , placeholder = Nothing
            , label = EI.labelLeft [] <| E.text "search"
            }
        , E.column
            [ E.scrollbars
            , E.height (mbmax |> Maybe.map (\m -> E.maximum m.height E.fill) |> Maybe.withDefault E.fill)
            , E.width (mbmax |> Maybe.map (\m -> E.maximum m.width E.fill) |> Maybe.withDefault E.fill)
            ]
          <|
            (A.indexedMap
                (\i s ->
                    if String.contains ls (String.toLower s) then
                        let
                            style =
                                if Just i == model.selected then
                                    selectedrow

                                else
                                    []
                        in
                        E.row
                            ((EE.onClick <| RowClick i)
                                :: E.width E.fill
                                :: (E.height <| E.px 30)
                                :: style
                            )
                        <|
                            [ E.text s ]

                    else
                        E.none
                )
                model.choices
                |> A.toList
            )
        , E.row [ E.width E.shrink, E.spacing 10 ]
            [ EI.button model.buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                model.buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model Int
update msg model =
    case msg of
        RowClick i ->
            if model.selected == Just i then
                -- double click selection
                GD.Ok i

            else
                GD.Dialog { model | selected = Just i }

        SearchChanged s ->
            GD.Dialog { model | search = s }

        CancelClick ->
            GD.Cancel

        OkClick ->
            model.selected
                |> Maybe.map GD.Ok
                |> Maybe.withDefault GD.Cancel

        Noop ->
            GD.Dialog model


init : Model -> Element () -> GDModel
init model underLay =
    { view = view
    , update = update
    , model = model
    , underLay = underLay
    }
