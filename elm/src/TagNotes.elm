module TagNotes exposing (..)

import Common
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Input as EI
import TagAThing exposing (Thing)
import TangoColors as TC


type Msg
    = OkClick
    | CancelClick
    | NotesClick
    | Noop


type Command
    = Ok
    | Cancel
    | Which TagAThing.AddWhich
    | None


type alias Model =
    { notes : List Data.ZkListNote }


view : Model -> Element Msg
view model =
    E.column [ E.width E.fill, E.height E.fill, EBk.color TC.white, EBd.rounded 10, E.spacing 8, E.padding 10 ]
        [ EI.button
            Common.buttonStyle
            { onPress = Just NotesClick, label = E.text "notes" }
        , E.column [ E.width E.fill, E.height <| E.maximum 200 E.fill, E.scrollbarY, E.centerX ]
            (model.notes
                |> List.map (\fn -> E.paragraph [] [ E.text fn.title ])
            )
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button
                Common.buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                (E.alignRight :: Common.buttonStyle)
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

        NotesClick ->
            ( model, Which TagAThing.AddNotes )

        Noop ->
            ( model, None )


addNote : Data.ZkListNote -> Model -> Model
addNote zln model =
    { model | notes = model.notes ++ [ zln ] }


initThing : List Data.ZkListNote -> Thing Model Msg Command
initThing notes =
    { model = { notes = notes }
    , view = view
    , update = update
    , addNote = addNote
    }
