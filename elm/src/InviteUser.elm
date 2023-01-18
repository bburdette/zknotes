module InviteUser exposing (..)

import Common
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import TagAThing exposing (Thing)
import TangoColors as TC


type Msg
    = OkClick
    | CancelClick
    | EmailChanged String
    | Noop


type Command
    = Ok
    | Cancel
    | None


type alias Model =
    { email : String
    }


view : Model -> Element Msg
view model =
    E.column [ E.width E.fill, E.height E.fill, EBk.color TC.white, EBd.rounded 10, E.spacing 8, E.padding 10 ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "invite new user"
        , EI.text []
            { onChange = EmailChanged
            , text = model.email
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "email (optional)")
            }
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button
                Common.buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                Common.buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        OkClick ->
            ( model, Ok )

        CancelClick ->
            ( model, Cancel )

        EmailChanged s ->
            ( { model | email = s }, None )

        Noop ->
            ( model, None )


initThing : String -> Thing Model Msg Command
initThing email =
    { model = { email = email }
    , view = view
    , update = update
    }
