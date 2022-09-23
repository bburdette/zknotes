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
import SearchStackPanel as SP
import TagSearchPanel as TSP
import TangoColors as TC
import Toop
import Util
import WindowKeys as WK
import ZkCommon as ZC


type Msg
    = NewPress
    | DonePress
    | ImportPress
    | PowerDeletePress
    | SPMsg SP.Msg
    | DialogMsg D.Msg
    | SearchHistoryPress


type DWhich
    = DeleteAll
    | DeleteComplete


type alias Model =
    { notes : Data.ZkListNoteSearchResult
    , spmodel : SP.Model
    , dialog : Maybe ( D.Model, DWhich )
    }


type Command
    = New
    | Done
    | Import
    | None
    | Search S.ZkNoteSearch
    | PowerDelete S.TagSearch
    | SearchHistory


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


updateSearchResult : Data.ZkListNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | notes = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


updateSearchStack : List S.TagSearch -> Model -> Model
updateSearchStack tsl model =
    let
        spm =
            model.spmodel
    in
    { model
        | spmodel = { spm | searchStack = tsl }
    }


updateSearch : List S.TagSearch -> Model -> ( Model, Command )
updateSearch ts model =
    ( { model
        | spmodel = SP.setSearch model.spmodel ts
      }
    , None
    )


onWkKeyPress : WK.Key -> Model -> ( Model, Command )
onWkKeyPress key model =
    case Toop.T4 key.key key.ctrl key.alt key.shift of
        Toop.T4 "Enter" False False False ->
            handleSPUpdate model (SP.onEnter model.spmodel)

        _ ->
            ( model, None )


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
    E.el
        [ E.width E.fill
        , EBk.color TC.lightGrey
        ]
    <|
        E.column
            [ E.spacing 8
            , E.padding 8
            , E.width (E.maximum maxwidth E.fill)
            , E.centerX
            , EBk.color TC.lightGrey
            ]
            [ E.row [ E.spacing 8, E.width E.fill ]
                [ ld.homenote
                    |> Maybe.map
                        (\id ->
                            E.link
                                Common.buttonStyle
                                { url = Data.editNoteLink id
                                , label = E.text "⌂"
                                }
                        )
                    |> Maybe.withDefault E.none
                , E.row [ EF.bold ] [ E.text ld.name ]
                , EI.button
                    (E.alignRight :: Common.buttonStyle)
                    { onPress = Just DonePress, label = E.text "settings" }
                ]
            , E.row [ E.spacing 8 ]
                [ EI.button Common.buttonStyle { onPress = Just NewPress, label = E.text "new" }
                , EI.button Common.buttonStyle { onPress = Just ImportPress, label = E.text "import" }
                , EI.button Common.buttonStyle { onPress = Just PowerDeletePress, label = E.text "delete..." }
                ]
            , E.column
                [ E.padding 8
                , EBd.rounded 10
                , EBd.width 1
                , EBd.color TC.darkGrey
                , EBk.color TC.white
                , E.spacing 8
                ]
                [ EI.button Common.buttonStyle
                    { onPress = Just <| SearchHistoryPress
                    , label = E.el [ E.centerY ] <| E.text "history"
                    }
                , E.map SPMsg <| SP.view False (size.width < maxwidth) 0 model.spmodel
                , E.table [ E.spacing 5, E.width E.fill, E.centerX ]
                    { data = model.notes.notes
                    , columns =
                        [ { header = E.none
                          , width =
                                -- E.fill
                                -- clipX doesn't work unless max width is here in px, it seems.
                                -- E.px <| min maxwidth size.width - titlemaxconst
                                E.px <| min maxwidth size.width - 32
                          , view =
                                \n ->
                                    E.row
                                        ([ E.centerY
                                         , E.clipX
                                         , E.width E.fill
                                         ]
                                            ++ (ZC.systemColor ld n.sysids
                                                    |> Maybe.map (\c -> [ EF.color c ])
                                                    |> Maybe.withDefault []
                                               )
                                        )
                                        [ E.link
                                            [ E.height <| E.px 30 ]
                                            { url = Data.editNoteLink n.id
                                            , label = E.text n.title
                                            }
                                        ]
                          }
                        ]
                    }
                ]
            , if List.length model.notes.notes < 15 then
                E.none

              else
                E.map SPMsg <|
                    SP.paginationView model.spmodel
            ]


update : Msg -> Model -> Data.LoginData -> ( Model, Command )
update msg model ld =
    case msg of
        NewPress ->
            ( model, New )

        DonePress ->
            ( model, Done )

        ImportPress ->
            ( model, Import )

        SearchHistoryPress ->
            ( model, SearchHistory )

        PowerDeletePress ->
            case SP.getSearch model.spmodel of
                Nothing ->
                    ( model, None )

                Just s ->
                    ( { model
                        | dialog =
                            Just <|
                                ( D.init
                                    ("delete all notes matching this search?\n"
                                        ++ String.concat (List.map S.showTagSearch s.tagSearch)
                                    )
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
                            case SP.getSearch model.spmodel of
                                Just s ->
                                    ( { model | dialog = Nothing }, PowerDelete (S.getTagSearch s) )

                                Nothing ->
                                    ( { model | dialog = Nothing }, None )

                        ( D.Ok, DeleteComplete ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Dialog dmod2, _ ) ->
                            ( { model | dialog = Just ( dmod2, dw ) }, None )

                Nothing ->
                    ( model, None )

        SPMsg m ->
            handleSPUpdate model (SP.update m model.spmodel)


handleSPUpdate : Model -> ( SP.Model, SP.Command ) -> ( Model, Command )
handleSPUpdate model ( nm, cm ) =
    let
        mod =
            { model | spmodel = nm }
    in
    case cm of
        SP.None ->
            ( mod, None )

        SP.Save ->
            ( mod, None )

        SP.Copy _ ->
            ( mod, None )

        SP.Search ts ->
            ( mod, Search ts )
