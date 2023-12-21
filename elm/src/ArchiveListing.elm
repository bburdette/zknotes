module ArchiveListing exposing (..)

import Common
import Data exposing (ZkNoteId(..))
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
import Route as R
import Search as S exposing (TagSearch(..))
import SearchStackPanel as SP
import TDict exposing (TDict)
import TagSearchPanel as TSP
import TangoColors as TC
import Time
import Toop
import UUID exposing (UUID)
import Util
import WindowKeys as WK
import ZkCommon as ZC


type Msg
    = SelectPress ZkNoteId
    | PPMsg PP.Msg
    | DonePress


type alias Model =
    { noteid : ZkNoteId
    , notes : List Data.ZkListNote
    , selected : Maybe ZkNoteId
    , fullnotes : TDict ZkNoteId String Data.ZkNote
    , ppmodel : PP.Model
    }


type Command
    = Selected ZkNoteId
    | GetArchives Data.GetZkNoteArchives
    | Done
    | None


init : Data.ZkNoteArchives -> Model
init zna =
    { noteid = zna.zknote
    , notes = zna.results.notes
    , selected = Nothing
    , fullnotes = Data.emptyZniDict
    , ppmodel = PP.searchResultUpdated zna.results PP.initModel
    }


onZkNote : Data.ZkNote -> Model -> ( Model, Command )
onZkNote zkn model =
    ( { model | fullnotes = TDict.insert zkn.id zkn model.fullnotes, selected = Just zkn.id }
    , None
    )


updateSearchResult : Data.ZkListNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | notes = zsr.notes
        , ppmodel = PP.searchResultUpdated zsr model.ppmodel
    }


view : Data.Sysids -> Data.LoginData -> Time.Zone -> Util.Size -> Model -> Element Msg
view si ld zone size model =
    (if size.width > 1400 then
        E.row [ E.centerX ]

     else
        E.column [ E.centerX ]
    )
        [ listview si ld zone size model
        , model.selected
            |> Maybe.andThen (\id -> TDict.get id model.fullnotes)
            |> Maybe.map (\zkn -> E.column [ E.width (E.maximum 700 E.fill), E.scrollbarX ] <| [ E.text zkn.content ])
            |> Maybe.withDefault E.none
        ]


listview : Data.Sysids -> Data.LoginData -> Time.Zone -> Util.Size -> Model -> Element Msg
listview si ld zone size model =
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
                [ E.row [ E.width E.fill ]
                    [ E.link
                        Common.linkStyle
                        { url = R.routeUrl <| R.EditZkNoteR model.noteid
                        , label = E.text "back"
                        }
                    , E.el [ E.centerX ] <|
                        E.map
                            PPMsg
                        <|
                            PP.view model.ppmodel
                    ]
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
                                            ++ (ZC.systemColor si n.sysids
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
