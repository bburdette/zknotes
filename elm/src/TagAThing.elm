module TagAThing exposing (..)

import Common
import Data exposing (Direction(..), ZkNoteId)
import DataUtil exposing (zklKey, zniCompare)
import Dict exposing (Dict(..))
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Orgauth.Data exposing (UserId(..))
import SearchStackPanel as SP
import TangoColors as TC
import Toop
import Util
import WindowKeys as WK
import ZkCommon as ZC


linkButtonStyle =
    Common.buttonStyle


disabledLinkButtonStyle =
    Common.disabledButtonStyle


type NavChoice
    = NcSearch
    | NcRecent


type SearchOrRecent
    = SearchView
    | RecentView


type alias Thing tmod tmsg tcmd =
    { view : tmod -> Element tmsg
    , update : tmsg -> tmod -> ( tmod, tcmd )
    , model : tmod
    }


type alias Model tmod tmsg tcmd =
    { thing : Thing tmod tmsg tcmd
    , ld : DataUtil.LoginData
    , zklDict : Dict String Data.EditLink
    , searchOrRecent : SearchOrRecent
    , focusSr : Maybe ZkNoteId -- note id in search result.
    , focusLink : Maybe Data.EditLink
    }


type Msg tmsg
    = SearchHistoryPress
    | AddToSearch Data.ZkListNote
    | AddToSearchAsTag String
    | ToLinkPress Data.ZkListNote
    | FromLinkPress Data.ZkListNote
    | SrFocusPress ZkNoteId
    | LinkFocusPress Data.EditLink
    | FlipLink Data.EditLink
    | RemoveLink Data.EditLink
    | SPMsg SP.Msg
    | NavChoiceChanged NavChoice
    | ThingMsg tmsg
    | Noop


type Command tcmd
    = None
    | SearchHistory
    | Search Data.ZkNoteSearch
    | SyncFiles Data.ZkNoteSearch
    | AddToRecent Data.ZkListNote
    | ThingCommand tcmd
    | SPMod (SP.Model -> ( SP.Model, SP.Command ))


init :
    Thing tmod tmsg tcmd
    -> List Data.ZkListNote
    -> List Data.EditLink
    -> DataUtil.LoginData
    -> Model tmod tmsg tcmd
init thing recentZkns links loginData =
    { thing = thing
    , ld = loginData
    , zklDict =
        Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) links)
    , searchOrRecent = SearchView
    , focusSr = Nothing
    , focusLink = Nothing
    }


onWkKeyPress : WK.Key -> Model tmod tmsg tcmd -> ( Model tmod tmsg tcmd, Command tcmd )
onWkKeyPress key model =
    case Toop.T4 key.key key.ctrl key.alt key.shift of
        Toop.T4 "Enter" False False False ->
            ( model, SPMod SP.onEnter )

        _ ->
            ( model, None )


showSr : Model tmod tmsg tcmd -> Data.ZkListNote -> Element (Msg tmsg)
showSr model zkln =
    let
        lnnonme =
            zkln.user /= model.ld.userid

        sysColor =
            ZC.systemColor DataUtil.sysids zkln.sysids

        mbTo =
            Dict.get (zklKey { direction = To, otherid = zkln.id })
                model.zklDict

        mbFrom =
            Dict.get (zklKey { direction = From, otherid = zkln.id })
                model.zklDict

        controlrow =
            E.row [ E.spacing 8, E.width E.fill ]
                [ mbTo
                    |> Maybe.map
                        (\zkl ->
                            EI.button
                                disabledLinkButtonStyle
                                { onPress = Just <| RemoveLink zkl
                                , label = E.el [ E.centerY ] <| E.text "→"
                                }
                        )
                    |> Maybe.withDefault
                        (EI.button linkButtonStyle
                            { onPress = Just <| ToLinkPress zkln
                            , label = E.el [ E.centerY ] <| E.text "→"
                            }
                        )
                , mbFrom
                    |> Maybe.map
                        (\zkl ->
                            EI.button
                                disabledLinkButtonStyle
                                { onPress = Just <| RemoveLink zkl
                                , label = E.el [ E.centerY ] <| E.text "←"
                                }
                        )
                    |> Maybe.withDefault
                        (EI.button linkButtonStyle
                            { onPress = Just <| FromLinkPress zkln
                            , label = E.el [ E.centerY ] <| E.text "←"
                            }
                        )
                , EI.button linkButtonStyle
                    { onPress = Just (AddToSearch zkln)
                    , label = E.text "^"
                    }
                , EI.button linkButtonStyle
                    { onPress = Just (AddToSearchAsTag zkln.title)
                    , label = E.text "t"
                    }
                ]

        listingrow =
            E.el
                ([ E.width E.fill
                 , EE.onClick (SrFocusPress zkln.id)
                 , E.height <| E.px 30
                 , E.clipX
                 ]
                    ++ (sysColor
                            |> Maybe.map (\c -> [ EF.color c ])
                            |> Maybe.withDefault []
                       )
                )
            <|
                E.text zkln.title
    in
    if model.focusSr == Just zkln.id then
        -- focus result!  show controlrow.
        E.column
            [ EBd.width 1
            , E.padding 3
            , EBd.rounded 3
            , EBd.color TC.darkGrey
            , E.width E.fill
            , E.inFront
                (E.row [ E.height E.fill, E.alignRight, EBk.color TC.white ]
                    [ E.el [ E.centerY ] <|
                        ZC.golink 25 zkln.id ZC.otherLinkColor
                    ]
                )
            ]
            [ listingrow, controlrow ]

    else
        listingrow


showZkl : Maybe Data.EditLink -> DataUtil.LoginData -> Maybe ZkNoteId -> Maybe E.Color -> Bool -> Data.EditLink -> Element (Msg tmsg)
showZkl focusLink ld id sysColor showflip zkl =
    let
        ( dir, otherid ) =
            case zkl.direction of
                To ->
                    ( E.text "→", Just zkl.otherid )

                From ->
                    ( E.text "←", Just zkl.otherid )

        focus =
            focusLink
                |> Maybe.map
                    (\l ->
                        zkl.direction
                            == l.direction
                            && zkl.otherid
                            == l.otherid
                            && zkl.user
                            == l.user
                    )
                |> Maybe.withDefault False

        display =
            [ E.el
                [ E.height <| E.px 30
                ]
                dir
            , zkl.othername
                |> Maybe.withDefault ""
                |> (\s ->
                        E.el
                            ([ E.clipX
                             , E.height <| E.px 30
                             , E.width E.fill
                             , EE.onClick (LinkFocusPress zkl)
                             ]
                                ++ (sysColor
                                        |> Maybe.map (\c -> [ EF.color c ])
                                        |> Maybe.withDefault []
                                   )
                            )
                            (E.text s)
                   )
            ]
    in
    if focus then
        E.column
            [ E.spacing 8
            , E.width E.fill
            , EBd.width 1
            , E.padding 3
            , EBd.rounded 3
            , EBd.color TC.darkGrey
            ]
            [ E.row [ E.spacing 8, E.width E.fill ] display
            , E.row [ E.spacing 8 ]
                [ if ld.userid == zkl.user then
                    EI.button (linkButtonStyle ++ [ E.alignLeft ])
                        { onPress = Just (RemoveLink zkl)
                        , label = E.text "X"
                        }

                  else
                    EI.button (linkButtonStyle ++ [ E.alignLeft, EBk.color TC.darkGray ])
                        { onPress = Nothing
                        , label = E.text "X"
                        }
                , case zkl.othername of
                    Just name ->
                        EI.button (linkButtonStyle ++ [ E.alignLeft ])
                            { onPress = Just <| AddToSearchAsTag name
                            , label = E.text ">"
                            }

                    Nothing ->
                        E.none
                , if showflip then
                    EI.button (linkButtonStyle ++ [ E.alignLeft ])
                        { onPress = Just (FlipLink zkl)
                        , label = E.text "⇄"
                        }

                  else
                    E.none
                ]
            ]

    else
        E.row [ E.spacing 8, E.width E.fill, E.height <| E.px 30 ] display


view :
    ZC.StylePalette
    -> List Data.ZkListNote
    -> Maybe Util.Size
    -> SP.Model
    -> Data.ZkListNoteSearchResult
    -> Model tmod tmsg tcmd
    -> Element (Msg tmsg)
view stylePalette recentZkns mbsize spmodel zknSearchResult model =
    let
        sppad =
            [ E.padding 5 ]

        spwidth =
            E.px
                400

        showLinks =
            E.column [ E.height E.fill, EBk.color TC.white, EBd.rounded 10, E.padding 10, E.width E.fill ]
                (E.row [ EF.bold, E.width E.fill, E.centerX ] [ E.text "add links" ]
                    :: List.map
                        (\( l, c ) ->
                            showZkl
                                model.focusLink
                                model.ld
                                Nothing
                                c
                                (Dict.get
                                    (zklKey
                                        { otherid = l.otherid
                                        , direction =
                                            case l.direction of
                                                To ->
                                                    From

                                                From ->
                                                    To
                                        }
                                    )
                                    model.zklDict
                                    |> Util.isJust
                                    |> not
                                )
                                l
                        )
                        (Dict.values model.zklDict
                            |> List.map
                                (\l ->
                                    ( l
                                    , ZC.systemColor DataUtil.sysids l.sysids
                                    )
                                )
                            |> List.sortWith
                                (\( l, lc ) ( r, rc ) ->
                                    case ( lc, rc ) of
                                        ( Nothing, Nothing ) ->
                                            zniCompare r.otherid l.otherid

                                        ( Just _, Nothing ) ->
                                            GT

                                        ( Nothing, Just _ ) ->
                                            LT

                                        ( Just lcolor, Just rcolor ) ->
                                            case Util.compareColor lcolor rcolor of
                                                EQ ->
                                                    zniCompare r.otherid l.otherid

                                                a ->
                                                    a
                                )
                        )
                )

        searchPanel =
            E.column
                (E.spacing 8 :: E.width E.fill :: sppad)
                (E.row [ E.width E.fill ]
                    [ EI.button Common.buttonStyle
                        { onPress = Just <| SearchHistoryPress
                        , label = E.el [ E.centerY ] <| E.text "history"
                        }
                    ]
                    :: (E.map SPMsg <|
                            SP.view True True 0 spmodel
                       )
                    :: (List.map
                            (showSr model)
                        <|
                            zknSearchResult.notes
                       )
                    ++ (if List.length zknSearchResult.notes < 15 then
                            []

                        else
                            [ E.map SPMsg <|
                                SP.paginationView spmodel
                            ]
                       )
                )

        recentPanel =
            E.column (E.spacing 8 :: sppad)
                (List.map
                    (showSr model)
                 <|
                    recentZkns
                )

        searchOrRecentPanel =
            E.column
                [ E.spacing 8
                , E.alignTop
                , E.alignRight
                , E.width spwidth
                , EBd.width 1
                , EBd.color TC.darkGrey
                , EBd.rounded 10
                , EBk.color TC.white
                , E.clip
                ]
                (Common.navbar 2
                    (case model.searchOrRecent of
                        SearchView ->
                            NcSearch

                        RecentView ->
                            NcRecent
                    )
                    NavChoiceChanged
                    [ ( NcSearch, "search" )
                    , ( NcRecent, "recent" )
                    ]
                    :: [ case model.searchOrRecent of
                            SearchView ->
                                searchPanel

                            RecentView ->
                                recentPanel
                       ]
                )
    in
    E.row
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.spacing 10
        ]
        [ E.row [ E.centerX, E.width <| E.maximum 1000 E.fill, E.spacing 10, E.alignTop, E.height E.fill ] <|
            [ E.map ThingMsg <| model.thing.view model.thing.model
            , showLinks
            ]
        , searchOrRecentPanel
        ]


update : Msg tmsg -> Model tmod tmsg tcmd -> ( Model tmod tmsg tcmd, Command tcmd )
update msg model =
    case msg of
        Noop ->
            ( model, None )

        SearchHistoryPress ->
            ( model, SearchHistory )

        AddToSearch zkln ->
            if List.any ((==) DataUtil.sysids.searchid) zkln.sysids then
                ( model, SPMod (\m -> ( SP.setSearchString m zkln.title, SP.None )) )

            else
                ( model, SPMod (\m -> ( SP.addToSearch m [ Data.ExactMatch ] zkln.title, SP.None )) )

        AddToSearchAsTag title ->
            ( model, SPMod (\m -> ( SP.addToSearch m [ Data.ExactMatch, Data.Tag ] title, SP.None )) )

        ToLinkPress zkln ->
            let
                nzkl =
                    { direction = To
                    , otherid = zkln.id
                    , user = model.ld.userid
                    , zknote = Nothing
                    , othername = Just zkln.title
                    , sysids = zkln.sysids
                    , delete = Nothing
                    }
            in
            ( { model
                | zklDict = Dict.insert (zklKey nzkl) nzkl model.zklDict
              }
            , AddToRecent zkln
            )

        FromLinkPress zkln ->
            let
                nzkl =
                    { direction = From
                    , otherid = zkln.id
                    , user = model.ld.userid
                    , zknote = Nothing
                    , othername = Just zkln.title
                    , sysids = zkln.sysids
                    , delete = Nothing
                    }
            in
            ( { model
                | zklDict = Dict.insert (zklKey nzkl) nzkl model.zklDict
              }
            , AddToRecent zkln
            )

        SrFocusPress id ->
            ( { model
                | focusSr =
                    if model.focusSr == Just id then
                        Nothing

                    else
                        Just id
              }
            , None
            )

        LinkFocusPress link ->
            ( { model
                | focusLink =
                    if model.focusLink == Just link then
                        Nothing

                    else
                        Just link
              }
            , None
            )

        FlipLink zkl ->
            let
                zklf =
                    { zkl | direction = DataUtil.flipDirection zkl.direction }
            in
            ( { model
                | zklDict =
                    model.zklDict
                        |> Dict.remove (zklKey zkl)
                        |> Dict.insert
                            (zklKey zklf)
                            zklf
                , focusLink =
                    if model.focusLink == Just zkl then
                        Just zklf

                    else
                        model.focusLink
              }
            , None
            )

        RemoveLink zkln ->
            ( { model
                | zklDict = Dict.remove (zklKey zkln) model.zklDict
              }
            , None
            )

        SPMsg m ->
            ( model, SPMod (SP.update m) )

        ThingMsg tmsg ->
            let
                ( tmod, tcmd ) =
                    model.thing.update tmsg model.thing.model

                thing =
                    model.thing
            in
            ( { model | thing = { thing | model = tmod } }, ThingCommand tcmd )

        NavChoiceChanged nc ->
            ( { model
                | searchOrRecent =
                    case nc of
                        NcSearch ->
                            SearchView

                        NcRecent ->
                            RecentView
              }
            , None
            )
