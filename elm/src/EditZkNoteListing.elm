module EditZkNoteListing exposing (..)

import Common
import Data
import DataUtil exposing (LoginData)
import Dialog as D
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import SearchStackPanel as SP
import SearchUtil exposing (showTagSearch)
import TangoColors as TC
import Time
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
    { dialog : Maybe ( D.Model, DWhich )
    , zone : Time.Zone
    }


type Command
    = New
    | Done
    | Import
    | None
    | PowerDelete (List Data.TagSearch)
    | SPMod (SP.Model -> ( SP.Model, SP.Command ))
    | SearchHistory


onPowerDeleteComplete : Int -> LoginData -> Model -> SP.Model -> Data.ZkListNoteSearchResult -> Model
onPowerDeleteComplete count ld model spmodel notes =
    { model
        | dialog =
            Just <|
                ( D.init
                    ("deleted " ++ String.fromInt count ++ " notes")
                    False
                    (\size -> E.map (\_ -> ()) (listview ld size model spmodel notes))
                , DeleteComplete
                )
    }


onWkKeyPress : WK.Key -> Model -> ( Model, Command )
onWkKeyPress key model =
    case Toop.T4 key.key key.ctrl key.alt key.shift of
        Toop.T4 "Enter" False False False ->
            ( model
            , SPMod SP.onEnter
            )

        _ ->
            ( model, None )


view : LoginData -> Util.Size -> Model -> SP.Model -> Data.ZkListNoteSearchResult -> Element Msg
view ld size model spmodel notes =
    case model.dialog of
        Just ( dialog, _ ) ->
            D.view size dialog |> E.map DialogMsg

        Nothing ->
            listview ld size model spmodel notes


listview : LoginData -> Util.Size -> Model -> SP.Model -> Data.ZkListNoteSearchResult -> Element Msg
listview ld size model spmodel notes =
    let
        maxwidth =
            700
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
                                { url = DataUtil.editNoteLink id
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
                , E.map SPMsg <| SP.view False (size.width < maxwidth) 0 spmodel
                , E.table [ E.spacing 5, E.width E.fill, E.centerX ]
                    { data = notes.notes
                    , columns =
                        [ { header = E.el [ EF.underline ] <| E.text "title"
                          , width =
                                -- E.fill
                                -- clipX doesn't work unless max width is here in px, it seems.
                                -- E.px <| min maxwidth size.width - titlemaxconst
                                E.px <| min maxwidth size.width - 32
                          , view =
                                \n ->
                                    let
                                        lnnonme =
                                            n.user /= ld.userid
                                    in
                                    -- E.row
                                    --     ([ E.centerY
                                    --      , E.clipX
                                    --      , E.width E.fill
                                    --      ]
                                    --         ++ (ZC.systemColor DataUtil.sysids n.sysids
                                    --                 |> Maybe.map (\c -> [ EF.color c ])
                                    --                 |> Maybe.withDefault []
                                    --            )
                                    --     )
                                    --     [
                                    E.column []
                                        [ if lnnonme then
                                            ZC.golink n.id
                                                ZC.otherLinkColor

                                          else
                                            ZC.golink n.id
                                                ZC.myLinkColor
                                        , E.el
                                            -- [ E.height <| E.px 30, E.width (E.minimum (maxwidth - 32) E.fill) ]
                                            [ E.height <| E.px 30 ]
                                            (E.text n.title)
                                        ]

                          -- { id : ZkNoteId
                          -- , title : String
                          -- , filestatus : FileStatus
                          -- , user : UserId
                          -- , createdate : Int
                          -- , changeddate : Int
                          -- , sysids : List (ZkNoteId)
                          -- }
                          }
                        , { header = E.el [ EF.underline ] <| E.text "file"
                          , width = E.shrink
                          , view =
                                \n ->
                                    case n.filestatus of
                                        Data.NotAFile ->
                                            E.none

                                        Data.FileMissing ->
                                            E.text "missing"

                                        Data.FilePresent ->
                                            E.text "file"
                          }
                        , { header = E.el [ EF.underline ] <| E.text "create"
                          , width = E.shrink
                          , view =
                                \n ->
                                    E.text <| Util.showDate model.zone <| Time.millisToPosix n.createdate
                          }
                        , { header = E.el [ EF.underline ] <| E.text "mod"
                          , width = E.shrink
                          , view =
                                \n ->
                                    E.text <| Util.showDate model.zone <| Time.millisToPosix n.changeddate
                          }
                        , { header = E.el [ EF.underline ] <| E.text "mine"
                          , width = E.shrink
                          , view =
                                \n ->
                                    if n.user == ld.userid then
                                        E.text "mine"

                                    else
                                        E.text ""
                          }
                        ]
                    }
                ]
            , if List.length notes.notes < 15 then
                E.none

              else
                E.map SPMsg <|
                    SP.paginationView spmodel
            ]


update : Msg -> Model -> SP.Model -> Data.ZkListNoteSearchResult -> LoginData -> ( Model, Command )
update msg model spmodel notes ld =
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
            case SP.getSearch spmodel of
                Nothing ->
                    ( model, None )

                Just s ->
                    ( { model
                        | dialog =
                            Just <|
                                ( D.init
                                    ("delete all notes matching this search?\n"
                                        ++ String.concat (List.map showTagSearch s.tagsearch)
                                    )
                                    True
                                    (\size -> E.map (\_ -> ()) (listview ld size model spmodel notes))
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
                            case SP.getSearch spmodel of
                                Just s ->
                                    ( { model | dialog = Nothing }, PowerDelete s.tagsearch )

                                Nothing ->
                                    ( { model | dialog = Nothing }, None )

                        ( D.Ok, DeleteComplete ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Dialog dmod2, _ ) ->
                            ( { model | dialog = Just ( dmod2, dw ) }, None )

                Nothing ->
                    ( model, None )

        SPMsg m ->
            ( model
            , SPMod (SP.update m)
            )
