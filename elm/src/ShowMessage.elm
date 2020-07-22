module ShowMessage exposing (..)

import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region



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
