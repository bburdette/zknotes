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


type alias Model =
    { entries : List Data.BlogListEntry
    }


type Command
    = Selected Int


view : Model -> Element Msg
view model =
    E.column [] <|
        E.text "Select an article"
            :: List.map (\e -> E.text e.title) model.entries


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        OnSelect id ->
            ( model
            , Selected id
            )
