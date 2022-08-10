module InviteUser exposing (GDModel, Model, Msg(..), init, update, view)

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
import GenDialog as GD
import Orgauth.Data as OD
import SearchPanel as SP
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
    { loginData : Data.LoginData
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


type alias GDModel =
    GD.Model Model Msg OD.GetInvite


init : SP.Model -> Data.ZkListNoteSearchResult -> List Data.ZkListNote -> List Data.EditLink -> Data.LoginData -> List (E.Attribute Msg) -> Element () -> GDModel
init spmodel spresult recentZkns links loginData buttonStyle underLay =
    { view = view buttonStyle recentZkns
    , update = update
    , model =
        { loginData = loginData
        , email = ""
        , spmodel = spmodel
        , zklDict =
            Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) links)
        , zknSearchResult = spresult
        , searchOrRecent = SearchView
        , focusSr = Nothing
        , focusLink = Nothing
        }
    , underLay = underLay
    }


showSr : Model -> Data.ZkListNote -> Element Msg
showSr model zkln =
    let
        lnnonme =
            zkln.user /= model.loginData.userid

        sysColor =
            ZC.systemColor model.loginData zkln.sysids

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


view : List (E.Attribute Msg) -> List Data.ZkListNote -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle recentZkns mbsize model =
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
                            model.loginData
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
                                , ZC.systemColor model.loginData l.sysids
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
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.height E.shrink
        , E.spacing 10
        ]
        [ EI.text []
            { onChange = EmailChanged
            , text = model.email
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "email")
            }
        , searchOrRecentPanel
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                buttonStyle
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model OD.GetInvite
update msg model =
    case msg of
        EmailChanged s ->
            GD.Dialog { model | email = s }

        CancelClick ->
            GD.Cancel

        OkClick ->
            GD.Ok
                { email = Just model.email
                , data = Nothing
                }

        Noop ->
            GD.Dialog model

        SearchHistoryPress ->
            GD.Dialog model

        AddToSearch zkListNote ->
            GD.Dialog model

        AddToSearchAsTag string ->
            GD.Dialog model

        ToLinkPress zkListNote ->
            GD.Dialog model

        FromLinkPress zkListNote ->
            GD.Dialog model

        SrFocusPress int ->
            GD.Dialog model

        LinkFocusPress editLink ->
            GD.Dialog model

        FlipLink editLink ->
            GD.Dialog model

        RemoveLink editLink ->
            GD.Dialog model

        SPMsg sPMsg ->
            GD.Dialog model

        NavChoiceChanged navChoice ->
            GD.Dialog model
