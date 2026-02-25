module TagNotes2 exposing (..)

-- import TagAThing exposing (Thing)

import Common
import Data exposing (Direction(..), EditLink)
import DataUtil exposing (ZlnDict, emptyZlnDict, zklKey, zniCompare)
import Dict exposing (Dict(..))
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import SearchStackPanel as SSP
import TDict exposing (TDict)
import TagThings as TT
import TangoColors as TC
import Util
import ZkCommon as ZC


type AddWhich
    = AddNotes
    | AddLinks
    | AddLinksOnly


type Msg
    = OkClick
    | CancelClick
    | NotesClick
    | ToLinkPress
    | FromLinkPress
    | AddNotePress
    | AddToSearch
    | AddToSearchAsTag
    | RemoveClick Data.ZkNoteId
    | SetAddWhich AddWhich
    | LinkFocusPress Data.EditLink
    | FlipLink Data.EditLink
    | RemoveLinks Direction
    | RemoveLink Data.EditLink
    | TTMsg (TT.Msg Msg)
    | Noop


type Command
    = Ok
    | Cancel
    | Which AddWhich
    | SearchHistory
    | AddToRecent (List Data.ZkListNote)
    | SPMod (SSP.Model -> ( SSP.Model, SSP.Command ))
    | None


type alias Model =
    { ld : DataUtil.LoginData
    , notes : List Data.ZkListNote
    , zklDict : Dict String Data.EditLink
    , addWhich : AddWhich
    , focusLink : Maybe Data.EditLink
    , tagThings : TT.Model
    }


linkButtonStyle : List (E.Attribute msg)
linkButtonStyle =
    Common.buttonStyle


disabledLinkButtonStyle : List (E.Attribute msg)
disabledLinkButtonStyle =
    Common.disabledButtonStyle


viewNotes : Model -> Element Msg
viewNotes model =
    E.column [ E.width E.fill, E.height E.fill, EBk.color TC.white, EBd.rounded 10, E.spacing 8, E.padding 10 ]
        [ EI.button
            Common.buttonStyle
            { onPress = Just NotesClick, label = E.text "notes" }
        , E.column [ E.width E.fill, E.height <| E.maximum 200 E.fill, E.scrollbarY, E.centerX, E.spacing 4 ]
            (model.notes
                |> List.map
                    (\fn ->
                        E.row [ E.spacing 8 ]
                            [ EI.button (Common.buttonStyle ++ [ E.paddingXY 2 2 ])
                                { onPress = Just (RemoveClick fn.id), label = E.text "x" }
                            , E.paragraph [] [ E.text fn.title ]
                            ]
                    )
            )
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button
                Common.buttonStyle
                { onPress = Just OkClick, label = E.text "Ok" }
            , EI.button
                (E.alignRight :: Common.buttonStyle)
                { onPress = Just CancelClick, label = E.text "Cancel" }
            ]
        ]


showLinks : Model -> Element Msg
showLinks model =
    let
        albutton =
            [ EI.button
                Common.buttonStyle
                { onPress = Just (SetAddWhich AddLinks), label = E.text "add links" }
            ]
    in
    E.column [ E.height E.fill, EBk.color TC.white, EBd.rounded 10, E.padding 10, E.width E.fill ]
        (E.row [ EF.bold, E.width E.fill, E.centerX ]
            (case model.addWhich of
                AddNotes ->
                    albutton

                AddLinks ->
                    albutton

                AddLinksOnly ->
                    [ E.text "add links" ]
            )
            :: List.map
                (\( l, c ) ->
                    showZkl
                        model.focusLink
                        model.ld
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


showZkl : Maybe Data.EditLink -> DataUtil.LoginData -> Maybe E.Color -> Bool -> Data.EditLink -> Element Msg
showZkl focusLink ld sysColor showflip zkl =
    let
        dir =
            case zkl.direction of
                To ->
                    E.text "→"

                From ->
                    E.text "←"

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
                            { onPress = Just <| AddToSearchAsTag
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
    -> Maybe Util.Size
    -> List Data.ZkListNote
    -> SSP.Model
    -> Data.ZkListNoteSearchResult
    -> Model
    -> Element Msg
view stylePalette mbsize recentZkns spmodel zknSearchResult model =
    let
        focusStyle =
            [ EBd.width 3, EBd.color TC.blue, E.width E.fill, E.height E.fill ]

        nonFocusStyle =
            [ E.width E.fill, E.height E.fill ]
    in
    E.row
        [ E.width (mbsize |> Maybe.map .width |> Maybe.withDefault 500 |> E.px)
        , E.spacing 10
        ]
        [ E.row
            [ E.centerX
            , E.width <| E.maximum 1000 E.fill
            , E.spacing 10
            , E.alignTop
            , E.height E.fill
            ]
          <|
            [ E.text "addnotes2"
            , E.el
                (case model.addWhich of
                    AddLinks ->
                        nonFocusStyle

                    AddLinksOnly ->
                        nonFocusStyle

                    AddNotes ->
                        focusStyle
                )
              <|
                viewNotes model
            , E.el
                (case model.addWhich of
                    AddLinks ->
                        focusStyle

                    AddLinksOnly ->
                        nonFocusStyle

                    AddNotes ->
                        nonFocusStyle
                )
                (showLinks model)
            ]
        , E.map TTMsg <|
            TT.view
                stylePalette
                mbsize
                recentZkns
                spmodel
                zknSearchResult
                model.tagThings
                (controlrow model)
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        OkClick ->
            ( model, Ok )

        CancelClick ->
            ( model, Cancel )

        NotesClick ->
            ( { model | addWhich = AddNotes }, None )

        RemoveClick zni ->
            ( { model | notes = List.filter (\n -> n.id /= zni) model.notes }, None )

        ToLinkPress ->
            let
                focusNotes =
                    TDict.toList model.tagThings.focusSr |> List.map Tuple.second

                zklDict =
                    List.foldr
                        (\zkln zkld ->
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
                            Dict.insert (zklKey nzkl) nzkl zkld
                        )
                        model.zklDict
                        focusNotes
            in
            ( { model
                | zklDict = zklDict
              }
            , AddToRecent focusNotes
            )

        FromLinkPress ->
            let
                focusNotes =
                    TDict.toList model.tagThings.focusSr |> List.map Tuple.second

                zklDict =
                    List.foldr
                        (\zkln zkld ->
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
                            Dict.insert (zklKey nzkl) nzkl zkld
                        )
                        model.zklDict
                        focusNotes
            in
            ( { model
                | zklDict = zklDict
              }
            , AddToRecent focusNotes
            )

        AddNotePress ->
            let
                tmod =
                    List.foldl
                        (\n mod ->
                            addNote n mod
                        )
                        model
                        (TDict.toList model.tagThings.focusSr |> List.map Tuple.second)
            in
            ( tmod, None )

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

        RemoveLinks direction ->
            ( { model
                | zklDict =
                    List.foldl (\zkln zkld -> Dict.remove (zklKey { direction = direction, otherid = zkln.id }) zkld)
                        model.zklDict
                        (TDict.toList model.tagThings.focusSr |> List.map Tuple.second)
              }
            , None
            )

        AddToSearch ->
            ( model, SPMod (TT.addToSearch (model.tagThings.focusSr |> TDict.values)) )

        AddToSearchAsTag ->
            ( model, SPMod (TT.addToSearchAsTag (model.tagThings.focusSr |> TDict.values)) )

        SetAddWhich addWhich ->
            ( { model | addWhich = addWhich }, None )

        TTMsg ttm ->
            let
                ( nm, cmd ) =
                    TT.update ttm model.tagThings
            in
            case cmd of
                TT.None ->
                    ( { model | tagThings = nm }, None )

                TT.SearchHistory ->
                    ( { model | tagThings = nm }, SearchHistory )

                TT.ControlCommand cc ->
                    update cc { model | tagThings = nm }

                TT.SPMod f ->
                    ( { model | tagThings = nm }, SPMod f )

        -- None
        -- SearchHistory
        -- Search Data.ZkNoteSearch
        -- SyncFiles Data.ZkNoteSearch
        -- ControlCommand tmsg
        -- SPMod (SSP.Model -> ( SSP.Model, SSP.Command ))
        Noop ->
            ( model, None )


addNote : Data.ZkListNote -> Model -> Model
addNote zln model =
    { model | notes = model.notes ++ [ zln ] }



-- type alias Thing tmod tmsg tcmd =
--     { view : tmod -> Element tmsg
--     , update : tmsg -> tmod -> ( tmod, tcmd )
--     , model : tmod
--     , controlRow : tmod -> Element tmsg
--     , addNote : Data.ZkListNote -> tmod -> tmod
--     }


controlrow : Model -> Element Msg
controlrow model =
    let
        focusNotes =
            TDict.toList model.tagThings.focusSr |> List.map Tuple.second

        clearButton =
            if TDict.isEmpty model.tagThings.focusSr then
                E.none

            else
                EI.button Common.buttonStyle
                    { onPress = Just <| TTMsg TT.ClearSelection
                    , label = E.el [ E.centerY ] <| E.text "clear"
                    }

        calcAll =
            \direction ->
                List.all
                    (\n ->
                        Dict.member
                            (zklKey { direction = direction, otherid = n.id })
                            model.zklDict
                    )

        tflinks =
            [ if calcAll To focusNotes then
                EI.button
                    disabledLinkButtonStyle
                    { onPress = Just <| RemoveLinks To
                    , label = E.el [ E.centerY ] <| E.text "→"
                    }

              else
                EI.button linkButtonStyle
                    { onPress = Just <| ToLinkPress
                    , label = E.el [ E.centerY ] <| E.text "→"
                    }
            , if calcAll From focusNotes then
                EI.button
                    disabledLinkButtonStyle
                    { onPress = Just <| RemoveLinks From
                    , label = E.el [ E.centerY ] <| E.text "←"
                    }

              else
                EI.button linkButtonStyle
                    { onPress = Just <| FromLinkPress
                    , label = E.el [ E.centerY ] <| E.text "←"
                    }
            ]
    in
    E.row [ E.spacing 8, E.width E.fill ]
        ((case model.addWhich of
            AddLinks ->
                tflinks

            AddLinksOnly ->
                tflinks

            AddNotes ->
                [ EI.button linkButtonStyle
                    { onPress = Just <| AddNotePress
                    , label = E.el [ E.centerY ] <| E.text "+"
                    }
                ]
         )
            ++ [ EI.button linkButtonStyle
                    { onPress = Just AddToSearch
                    , label = E.text "^"
                    }
               , EI.button linkButtonStyle
                    { onPress = Just AddToSearchAsTag
                    , label = E.text "t"
                    }
               , clearButton
               ]
        )


init : DataUtil.LoginData -> List Data.ZkListNote -> List EditLink -> AddWhich -> Model
init ld notes links addwhich =
    { ld = ld
    , notes = notes
    , zklDict =
        Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) links)
    , addWhich = addwhich
    , focusLink = Nothing
    , tagThings = TT.init
    }



-- type alias Model =
--     { notes : List Data.ZkListNote
--     , zklDict : Dict String Data.EditLink
--     , addwhich : AddWhich
--     , tagThings : TT.Model
--     }
