module EditListing exposing (..)

import Common
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region


type Msg
    = SelectPress Int
    | ViewPress Int
    | NewPress


type alias Model =
    { entries : List Data.BlogListEntry
    }


type Command
    = Selected Int
    | View Int
    | New


view : Model -> Element Msg
view model =
    E.column [] <|
        E.row [ E.spacing 20 ]
            [ E.text "Select an article"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            ]
            :: List.map
                (\e ->
                    E.row []
                        [ E.text e.title
                        , EI.button Common.buttonStyle { onPress = Just (SelectPress e.id), label = E.text "edit" }
                        , EI.button Common.buttonStyle { onPress = Just (ViewPress e.id), label = E.text "view" }
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

        NewPress ->
            ( model, New )
