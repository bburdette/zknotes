module EditZkNoteListing exposing (..)

import Common
import Data
import Dialog as D
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Element.Region
import Import
import Search as S exposing (TagSearch(..))
import SearchPanel as SP
import TagSearchPanel as TSP
import TangoColors as TC
import Util


type Msg
    = SelectPress Int
    | NewPress
    | DonePress
    | ImportPress
    | PowerDeletePress
    | SPMsg SP.Msg
    | DialogMsg D.Msg


type DWhich
    = DeleteAll
    | DeleteComplete


type alias Model =
    { notes : Data.ZkNoteSearchResult
    , spmodel : SP.Model
    , dialog : Maybe ( D.Model, DWhich )
    }


type Command
    = Selected Int
    | New
    | Done
    | Import
    | None
    | Search S.ZkNoteSearch
    | PowerDelete S.TagSearch


onPowerDeleteComplete : Int -> Data.LoginData -> Model -> Model
onPowerDeleteComplete count ld model =
    { model
        | dialog =
            Just <|
                ( D.init
                    ("deleted " ++ String.fromInt count ++ " notes")
                    False
                    (\size -> E.map (\_ -> ()) (listview ld size model))
                , DeleteComplete
                )
    }


updateSearchResult : Data.ZkNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | notes = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


view : Data.LoginData -> Util.Size -> Model -> Element Msg
view ld size model =
    case model.dialog of
        Just ( dialog, _ ) ->
            D.view size dialog |> E.map DialogMsg

        Nothing ->
            listview ld size model


listview : Data.LoginData -> Util.Size -> Model -> Element Msg
listview ld size model =
    let
        maxwidth =
            700

        titlemaxconst =
            85
    in
    E.column [ E.spacing 8, E.padding 8, E.width (E.maximum maxwidth E.fill), E.centerX ]
        [ E.row [ E.spacing 8, E.width E.fill ]
            [ E.row [ EF.bold ] [ E.text ld.name ]
            , EI.button
                (E.alignRight :: Common.buttonStyle)
                { onPress = Just DonePress, label = E.text "logout" }
            ]
        , E.row [ E.spacing 8 ]
            [ E.text "select a zk note"
            , EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
            , EI.button Common.buttonStyle { onPress = Just ImportPress, label = E.text "import" }
            , EI.button Common.buttonStyle { onPress = Just PowerDeletePress, label = E.text "delete all" }
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
                                [ if n.user == ld.userid then
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


update : Msg -> Model -> Data.LoginData -> ( Model, Command )
update msg model ld =
    case msg of
        SelectPress id ->
            ( model
            , Selected id
            )

        NewPress ->
            ( model, New )

        DonePress ->
            ( model, Done )

        ImportPress ->
            ( model, Import )

        PowerDeletePress ->
            case TSP.getSearch model.spmodel.tagSearchModel of
                Nothing ->
                    ( model, None )

                Just s ->
                    ( { model
                        | dialog =
                            Just <|
                                ( D.init
                                    ("delete all notes matching this search?\n" ++ S.showTagSearch s)
                                    True
                                    (\size -> E.map (\_ -> ()) (listview ld size model))
                                , DeleteAll
                                )
                      }
                    , None
                    )

        DialogMsg dm ->
            case model.dialog of
                Just ( dmod, dw ) ->
                    case ( D.update dm dmod, dw ) of
                        ( D.Cancel, _ ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Ok, DeleteAll ) ->
                            case TSP.getSearch model.spmodel.tagSearchModel of
                                Just s ->
                                    ( { model | dialog = Nothing }, PowerDelete s )

                                Nothing ->
                                    ( { model | dialog = Nothing }, None )

                        ( D.Ok, DeleteComplete ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Dialog dmod2, _ ) ->
                            ( { model | dialog = Just ( dmod2, dw ) }, None )

                Nothing ->
                    ( model, None )

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
