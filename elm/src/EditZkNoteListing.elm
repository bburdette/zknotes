module EditZkNoteListing exposing (..)

import Common
import Data
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region
import Search as S exposing (TagSearch(..))
import SearchPanel as SP
import TangoColors as TC
import Util


type Msg
    = SelectPress Int
    | ViewPress Int
    | NewPress
    | ExamplePress
    | DonePress
    | SPMsg SP.Msg


type alias Model =
    { zk : Data.Zk
    , notes : Data.ZkNoteSearchResult
    , spmodel : SP.Model
    }


type Command
    = Selected Int
    | View Int
    | New
    | Example
    | Done
    | None
    | Search S.ZkNoteSearch


updateSearchResult : Data.ZkNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | notes = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


view : Util.Size -> Model -> Element Msg
view size model =
    let
        maxwidth =
            700

        titlemaxconst =
            245
    in
    E.column [ E.spacing 8, E.padding 8, E.width (E.maximum maxwidth E.fill), E.centerX ]
        [ E.row [] [ E.text "zettelkasten: ", E.row [ Font.bold ] [ E.text model.zk.name ] ]
        , E.row [ E.spacing 8 ]
            [ E.text "select a zk note"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            , EI.button Common.buttonStyle { onPress = Just ExamplePress, label = E.text "example" }
            , EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "done" }
            ]
        , E.map SPMsg <| SP.view (size.width < maxwidth) 0 model.spmodel
        , E.table [ E.spacing 10, E.width (E.maximum maxwidth E.fill), E.centerX ]
            { data = model.notes.notes
            , columns =
                [ { header = E.none
                  , width =
                        E.px <| min maxwidth size.width - titlemaxconst
                  , view =
                        \n ->
                            E.row
                                [ E.clipX
                                , E.centerY
                                , E.height E.fill
                                , E.width E.fill
                                ]
                                [ E.text n.title
                                ]
                  }
                , { header = E.none
                  , width = E.shrink
                  , view =
                        \n ->
                            E.row [ E.spacing 8 ]
                                [ EI.button Common.buttonStyle { onPress = Just (SelectPress n.id), label = E.text "edit" }
                                , EI.button Common.buttonStyle { onPress = Just (ViewPress n.id), label = E.text "view" }
                                , E.link [ Font.color TC.darkBlue, Font.underline ] { url = "note/" ++ String.fromInt n.id, label = E.text "link" }
                                , if n.public then
                                    E.text "public"

                                  else
                                    E.text "      "
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

        SPMsg m ->
            let
                ( nm, cm ) =
                    SP.update m model.spmodel

                mod =
                    { model | spmodel = nm }
            in
            case cm of
                SP.None ->
                    ( mod, None )

                SP.Save ->
                    ( mod, None )

                SP.Search ts ->
                    ( mod, Search ts )
