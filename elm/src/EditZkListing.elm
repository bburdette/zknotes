module EditZkListing exposing (..)

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
    = SelectPress Data.Zk
    | ViewPress Int
    | NotesPress Data.Zk
    | NewPress


type alias Model =
    { zks : List Data.Zk
    }


type Command
    = Selected Data.Zk
    | Notes Data.Zk
    | View Int
    | New


view : Model -> Element Msg
view model =
    E.column [ E.spacing 8, E.padding 8, E.centerX ] <|
        [ E.row [ E.spacing 20 ]
            [ E.text "select a zettelkasten"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            ]
        , E.table [ E.spacing 8 ]
            { data = model.zks
            , columns =
                [ { header = E.none
                  , width = E.shrink
                  , view =
                        \n ->
                            E.text n.name
                  }
                , { header = E.none
                  , width = E.shrink
                  , view =
                        \n ->
                            E.row [ E.spacing 8 ]
                                [ EI.button Common.buttonStyle { onPress = Just (NotesPress n), label = E.text "notes" }
                                , EI.button Common.buttonStyle { onPress = Just (SelectPress n), label = E.text "edit" }
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

        NotesPress id ->
            ( model
            , Notes id
            )

        ViewPress id ->
            ( model
            , View id
            )

        NewPress ->
            ( model, New )
