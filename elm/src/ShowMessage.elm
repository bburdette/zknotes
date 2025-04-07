module ShowMessage exposing (..)

import Element as E exposing (Element)



{- Just displays the message and does nothing.  Good if you're waiting on a server message. -}


type Msg
    = Noop


type alias Model =
    { message : String
    }


view : Model -> Element Msg
view model =
    E.text model.message


update : Msg -> Model -> Model
update msg model =
    case msg of
        Noop ->
            model
