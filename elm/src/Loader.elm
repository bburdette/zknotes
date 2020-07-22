module Loader exposing (..)

import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region



{- Loader displays a message and waits for data from the server.  When the data arrives,
   it calls a function that returns the new state.
-}


type Msg a
    = OnData a


type alias Model a b =
    { message : String
    , ondata : a -> b
    }


type Command b
    = Transition b


view : Model a b -> Element (Msg a)
view model =
    E.text model.message


update : Msg a -> Model a b -> ( Model a b, Command b )
update msg model =
    case msg of
        OnData data ->
            ( model
            , Transition (model.ondata data)
            )
