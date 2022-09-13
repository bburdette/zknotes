module ArchiveListing exposing (..)

import Common
import Data
import Dialog as D
import Dict exposing (Dict(..))
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Element.Region
import Import
import PaginationPanel as PP
import Search as S exposing (TagSearch(..))
import SearchStackPanel as SP
import TagSearchPanel as TSP
import TangoColors as TC
import Time
import Toop
import Util
import WindowKeys as WK
import ZkCommon as ZC


type Msg
    = SelectPress Int
    | PPMsg PP.Msg
    | DonePress


type alias Model =
    { noteid : Int
    , notes : List Data.ZkListNote
    , selected : Maybe Int
    , fullnotes : Dict Int Data.ZkNote
    , ppmodel : PP.Model
    }


type Command
    = Selected Int
    | GetArchives Data.GetZkNoteArchives
    | Done
    | None


init : Data.ZkNoteArchives -> Model
init zna =
    { noteid = zna.zknote
    , notes = zna.results.notes
    , selected = Nothing
    , fullnotes = Dict.empty
    , ppmodel = PP.searchResultUpdated zna.results PP.initModel
    }


onZkNote : Data.ZkNote -> Model -> ( Model, Command )
onZkNote zkn model =
    ( { model | fullnotes = Dict.insert zkn.id zkn model.fullnotes, selected = Just zkn.id }
    , None
    )


updateSearchResult : Data.ZkListNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | notes = zsr.notes
        , ppmodel = PP.searchResultUpdated zsr model.ppmodel
    }


view : Data.LoginData -> Time.Zone -> Util.Size -> Model -> Element Msg
view ld zone size model =
    (if size.width > 1400 then
        E.row [ E.centerX ]

     else
        E.column [ E.centerX ]
    )
        [ listview ld zone size model
        , model.selected
            |> Maybe.andThen (\id -> Dict.get id model.fullnotes)
            |> Maybe.map (\zkn -> E.text zkn.content)
            |> Maybe.withDefault E.none
        ]


listview : Data.LoginData -> Time.Zone -> Util.Size -> Model -> Element Msg
listview ld zone size model =
    let
        maxwidth =
            700

        titlemaxconst =
            85
    in
    E.el
        [ E.width E.fill
        , EBk.color TC.lightGrey
        , E.alignTop
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
            , E.column
                [ E.padding 8
                , EBd.rounded 10
                , EBd.width 1
                , EBd.color TC.darkGrey
                , EBk.color TC.white
                , E.spacing 8
                ]
                [ E.map PPMsg <| PP.view model.ppmodel
                , E.table [ E.spacing 5, E.width E.fill, E.centerX ]
                    { data = model.notes
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
                                            ++ (if Just n.id == model.selected then
                                                    [ EBk.color TC.lightBlue ]

                                                else
                                                    []
                                               )
                                        )
                                        [ E.link
                                            [ E.height <| E.px 30 ]
                                            { url = Data.archiveNoteLink model.noteid n.id
                                            , label =
                                                E.row [ E.spacing 10 ]
                                                    [ E.text (Util.showDateTime zone (Time.millisToPosix n.changeddate))
                                                    , E.text n.title
                                                    ]
                                            }
                                        ]
                          }
                        ]
                    }
                ]

            -- , if List.length model.notes.notes < 15 then
            --     E.none
            --   else
            --     E.map SPMsg <|
            --         SP.paginationView True model.spmodel
            ]


update : Msg -> Model -> Data.LoginData -> ( Model, Command )
update msg model ld =
    case msg of
        SelectPress id ->
            ( model
            , Selected id
            )

        PPMsg pms ->
            let
                ( nm, cmd ) =
                    PP.update pms model.ppmodel
            in
            ( { model | ppmodel = nm }
            , case cmd of
                PP.RangeChanged ->
                    GetArchives { zknote = model.noteid, offset = nm.offset, limit = Just nm.increment }

                PP.None ->
                    None
            )

        DonePress ->
            ( model, Done )
