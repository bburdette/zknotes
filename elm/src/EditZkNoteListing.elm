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
    | NewPress
    | DonePress
    | SPMsg SP.Msg


type alias Model =
    { uid : Int
    , notes : Data.ZkNoteSearchResult
    , spmodel : SP.Model
    }


type Command
    = Selected Int
    | New
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
            85
    in
    E.column [ E.spacing 8, E.padding 8, E.width (E.maximum maxwidth E.fill), E.centerX ]
        [ E.row [ E.spacing 8 ]
            [ E.text "select a zk note"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            , EI.button Common.buttonStyle { onPress = Just DonePress, label = E.text "logout" }
            ]
        , E.map SPMsg <| SP.view (size.width < maxwidth) 0 model.spmodel
        , E.table [ E.spacing 10, E.width E.fill, E.centerX ]
            { data = model.notes.notes
            , columns =
                [ { header = E.none
                  , width =
                        -- E.fill
                        -- clipX doesn't work unless max width is here in px, it seems.
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
                                [ if n.user == model.uid then
                                    EI.button
                                        Common.buttonStyle
                                        { onPress = Just (SelectPress n.id), label = E.text "edit" }

                                  else
                                    EI.button
                                        (Common.buttonStyle
                                            ++ [ EBk.color TC.lightBlue
                                               ]
                                        )
                                        { onPress = Just (SelectPress n.id), label = E.text "show" }
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
