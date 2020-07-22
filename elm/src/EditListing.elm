module EditListing exposing (..)

import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region


type Msg
    = OnSelect Int
    | NewPress


type alias Model =
    { entries : List Data.BlogListEntry
    , login : Data.Login
    }


type Command
    = Selected Int
    | New


view : Model -> Element Msg
view model =
    E.column [] <|
        E.row []
            [ E.text "Select an article"
            , EI.button [] { onPress = Just NewPress, label = E.text "new" }
            ]
            :: List.map (\e -> E.text e.title) model.entries


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        OnSelect id ->
            ( model
            , Selected id
            )

        NewPress ->
            ( model, New )
