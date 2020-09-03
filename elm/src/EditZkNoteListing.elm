module EditZkNoteListing exposing (..)

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
    | DonePress


type alias Model =
    { zk : Data.Zk
    , notes : List Data.ZkListNote
    }


type Command
    = Selected Int
    | View Int
    | New
    | Example
    | Done


view : Model -> Element Msg
view model =
    E.column [ E.spacing 8, E.padding 8 ]
        [ E.text model.zk.name
        , E.row [ E.spacing 8 ]
            [ E.text "Select a Zk Note"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            , EI.button Common.buttonStyle { onPress = Just ExamplePress, label = E.text "example" }
            , EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "done" }
            ]
        , E.table [ E.spacing 8 ]
            { data = model.notes
            , columns =
                [ { header = E.none
                  , width = E.shrink
                  , view =
                        \n ->
                            E.text n.title
                  }
                , { header = E.none
                  , width = E.shrink
                  , view =
                        \n ->
                            E.row [ E.spacing 8 ]
                                [ EI.button Common.buttonStyle { onPress = Just (SelectPress n.id), label = E.text "edit" }
                                , EI.button Common.buttonStyle { onPress = Just (ViewPress n.id), label = E.text "view" }
                                , E.link [ Font.color TC.darkBlue, Font.underline ] { url = "note/" ++ String.fromInt n.id, label = E.text "link" }
                                ]
                  }
                ]
            }
        ]


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

        DonePress ->
            ( model, Done )
