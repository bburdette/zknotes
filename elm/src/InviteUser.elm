module InviteUser exposing (Command(..), Model, Msg(..), NavChoice(..), SearchOrRecent(..), disabledLinkButtonStyle, handleSPUpdate, init, linkButtonStyle, showSr, showZkl, update, updateSearchResult, view)

import Common
import Data exposing (Direction(..), zklKey)
import Dict exposing (Dict(..))
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region
import Json.Encode as JE
import Orgauth.Data as OD
import Search as S
import SearchStackPanel as SP
import TangoColors as TC
import Time exposing (Zone)
import Util
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


type alias Model =
    { ld : Data.LoginData
    , email : String
    , spmodel : SP.Model
    , zklDict : Dict String Data.EditLink
    , zknSearchResult : Data.ZkListNoteSearchResult
    , searchOrRecent : SearchOrRecent
    , focusSr : Maybe Int -- note id in search result.
    , focusLink : Maybe Data.EditLink
    }


type Msg
    = EmailChanged String
    | OkClick
    | CancelClick
    | SearchHistoryPress
    | AddToSearch Data.ZkListNote
    | AddToSearchAsTag String
    | ToLinkPress Data.ZkListNote
    | FromLinkPress Data.ZkListNote
    | SrFocusPress Int
    | LinkFocusPress Data.EditLink
    | FlipLink Data.EditLink
    | RemoveLink Data.EditLink
    | SPMsg SP.Msg
    | NavChoiceChanged NavChoice
    | Noop


type Command
    = None
    | GetInvite OD.GetInvite
    | SearchHistory
    | Search S.ZkNoteSearch
    | AddToRecent Data.ZkListNote
    | Cancel


init : SP.Model -> Data.ZkListNoteSearchResult -> List Data.ZkListNote -> List Data.EditLink -> Data.LoginData -> Model
init spmodel spresult recentZkns links loginData =
    { ld = loginData
    , email = ""
    , spmodel = spmodel
    , zklDict =
        Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) links)
    , zknSearchResult = spresult
    , searchOrRecent = SearchView
    , focusSr = Nothing
    , focusLink = Nothing
    }


showSr : Model -> Data.ZkListNote -> Element Msg
showSr model zkln =
    let
        lnnonme =
            zkln.user /= model.ld.userid

        sysColor =
            ZC.systemColor model.ld zkln.sysids

        controlrow =
            E.row [ E.spacing 8, E.width E.fill ]
                [ (case
                    Dict.get (zklKey { direction = To, otherid = zkln.id })
                        model.zklDict
                   of
                    Just _ ->
                        Nothing

                    Nothing ->
                        Just 1
                  )
                    |> Maybe.map
                        (\_ ->
                            EI.button linkButtonStyle
                                { onPress = Just <| ToLinkPress zkln
                                , label = E.el [ E.centerY ] <| E.text "→"
                                }
                        )
                    |> Maybe.withDefault
                        (EI.button
                            disabledLinkButtonStyle
                            { onPress = Nothing
                            , label = E.el [ E.centerY ] <| E.text "→"
                            }
                        )
                , (case
                    Dict.get (zklKey { direction = From, otherid = zkln.id })
                        model.zklDict
                   of
                    Just _ ->
                        Nothing

                    Nothing ->
                        Just 1
                  )
                    |> Maybe.map
                        (\_ ->
                            EI.button linkButtonStyle
                                { onPress = Just <| FromLinkPress zkln
                                , label = E.el [ E.centerY ] <| E.text "←"
                                }
                        )
                    |> Maybe.withDefault
                        (EI.button
                            disabledLinkButtonStyle
                            { onPress = Nothing
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
                , if lnnonme then
                    E.link
                        ZC.otherLinkStyle
                        { url = Data.editNoteLink zkln.id
                        , label = E.text zkln.title
                        }

                  else
                    E.link
                        ZC.myLinkStyle
                        { url = Data.editNoteLink zkln.id
                        , label = E.text "go"
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
            ]
            [ listingrow, controlrow ]

    else
        listingrow


showZkl : Maybe Data.EditLink -> Data.LoginData -> Maybe Int -> Maybe E.Color -> Bool -> Data.EditLink -> Element Msg
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


handleSPUpdate : Model -> ( SP.Model, SP.Command ) -> ( Model, Command )
handleSPUpdate model ( nm, cmd ) =
    let
        mod =
            { model | spmodel = nm }
    in
    case cmd of
        SP.None ->
            ( mod, None )

        SP.Save ->
            ( mod, None )

        SP.Copy s ->
            ( mod, None )

        SP.Search ts ->
            let
                zsr =
                    mod.zknSearchResult
            in
            ( { mod | zknSearchResult = { zsr | notes = [] } }, Search ts )


updateSearchResult : Data.ZkListNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | zknSearchResult = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


view : ZC.StylePalette -> List Data.ZkListNote -> Maybe Util.Size -> Model -> Element Msg
view stylePalette recentZkns mbsize model =
    let
        sppad =
            [ E.paddingXY 0 5 ]

        spwidth =
            E.px
                400

        showLinks =
            E.row [ EF.bold ] [ E.text "links" ]
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
                                , ZC.systemColor model.ld l.sysids
                                )
                            )
                        |> List.sortWith
                            (\( l, lc ) ( r, rc ) ->
                                case ( lc, rc ) of
                                    ( Nothing, Nothing ) ->
                                        compare r.otherid l.otherid

                                    ( Just _, Nothing ) ->
                                        GT

                                    ( Nothing, Just _ ) ->
                                        LT

                                    ( Just lcolor, Just rcolor ) ->
                                        case Util.compareColor lcolor rcolor of
                                            EQ ->
                                                compare r.otherid l.otherid

                                            a ->
                                                a
                            )
                    )

        searchPanel =
            E.column
                (E.spacing 8 :: E.width E.fill :: sppad)
                (E.row [ E.width E.fill ]
                    [ EI.button Common.buttonStyle
                        { onPress = Just <| SearchHistoryPress
                        , label = E.el [ E.centerY ] <| E.text "search history"
                        }
                    ]
                    :: (E.map SPMsg <|
                            SP.view True True 0 model.spmodel
                       )
                    :: (List.map
                            (showSr model)
                        <|
                            model.zknSearchResult.notes
                       )
                    ++ (if List.length model.zknSearchResult.notes < 15 then
                            []

                        else
                            [ E.map SPMsg <|
                                SP.paginationView True model.spmodel
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
        [ -- E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
          -- , E.height E.fill
          -- , E.height E.shrink
          E.spacing 10
        ]
        [ E.column [] <|
            [ EI.text []
                { onChange = EmailChanged
                , text = model.email
                , placeholder = Nothing
                , label = EI.labelLeft [] (E.text "email")
                }
            , E.row [ E.width E.fill, E.spacing 10 ]
                [ EI.button
                    Common.buttonStyle
                    { onPress = Just OkClick, label = E.text "Ok" }
                , EI.button
                    Common.buttonStyle
                    { onPress = Just CancelClick, label = E.text "Cancel" }
                ]
            ]
                ++ showLinks
        , searchOrRecentPanel
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        EmailChanged s ->
            ( { model | email = s }, None )

        CancelClick ->
            ( model, Cancel )

        OkClick ->
            ( model
            , GetInvite
                { email =
                    if model.email /= "" then
                        Just model.email

                    else
                        Nothing
                , data =
                    Data.encodeZkInviteData (List.map Data.elToSzl (Dict.values model.zklDict))
                        |> JE.encode 2
                        |> Just
                }
            )

        Noop ->
            ( model, None )

        SearchHistoryPress ->
            ( model, SearchHistory )

        AddToSearch zkln ->
            let
                spmod =
                    model.spmodel
            in
            if List.any ((==) model.ld.searchid) zkln.sysids then
                ( { model
                    | spmodel = SP.setSearchString model.spmodel zkln.title
                  }
                , None
                )

            else
                ( { model
                    | spmodel =
                        SP.addToSearch model.spmodel
                            [ S.ExactMatch ]
                            zkln.title
                  }
                , None
                )

        AddToSearchAsTag title ->
            let
                spmod =
                    model.spmodel
            in
            ( { model
                | spmodel =
                    SP.addToSearch model.spmodel
                        [ S.ExactMatch, S.Tag ]
                        title
              }
            , None
            )

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
                    { zkl | direction = Data.flipDirection zkl.direction }
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
            handleSPUpdate model (SP.update m model.spmodel)

        NavChoiceChanged navChoice ->
            ( model, None )
