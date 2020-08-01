module EditListing exposing (..)

import Common
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region
import TangoColors as TC


type Msg
    = SelectPress Int
    | ViewPress Int
    | NewPress
    | ExamplePress


type alias Model =
    { entries : List Data.Zk
    }


type Command
    = Selected Int
    | View Int
    | New
    | Example


view : Model -> Element Msg
view model =
    E.column [ E.spacing 8, E.padding 8 ] <|
        E.row [ E.spacing 20 ]
            [ E.text "Select an article"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            , EI.button Common.buttonStyle { onPress = Just ExamplePress, label = E.text "example" }
            ]
            :: List.map
                (\e ->
                    E.row [ E.spacing 8 ]
                        [ E.text e.name
                        , EI.button Common.buttonStyle { onPress = Just (SelectPress e.id), label = E.text "edit" }
                        , EI.button Common.buttonStyle { onPress = Just (ViewPress e.id), label = E.text "view" }
                        , E.link [ Font.color TC.darkBlue, Font.underline ] { url = "blog/" ++ String.fromInt e.id, label = E.text "link" }
                        ]
                )
                model.entries


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SelectPress id ->
            ( model
            , Selected id
            )

        ViewPress id ->
            ( model
            , View id
            )

        ExamplePress ->
            ( model, Example )

        NewPress ->
            ( model, New )
