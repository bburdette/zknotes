module TagThings exposing (..)

import Common
import Data exposing (Direction(..), ZkListNote)
import DataUtil exposing (ZlnDict, emptyZlnDict, zklKey, zniCompare)
import Dict exposing (Dict(..))
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import Html.Events as HE
import Json.Decode as JD
import Orgauth.Data exposing (UserId(..))
import SearchPanel
import SearchStackPanel as SSP
import TDict
import TagSearchPanel
import TangoColors as TC
import Toop
import Util
import WindowKeys as WK
import ZkCommon as ZC


type NavChoice
    = NcSearch
    | NcRecent


type SearchOrRecent
    = SearchView
    | RecentView



-- type AddWhich
--     = AddNotes
--     | AddLinks
--     | AddLinksOnly
-- Q: can swap out the Thing dynamically??
-- or, make Thing have various modes?
-- type alias Thing tmod tmsg tcmd =
--     { view : tmod -> Element tmsg
--     , update : tmsg -> tmod -> ( tmod, tcmd )
--     , model : tmod
--     , controlRow : tmod -> Element tmsg
--     , addNote : Data.ZkListNote -> tmod -> tmod
--     }


type alias Model =
    { searchOrRecent : SearchOrRecent
    , focusSr : ZlnDict
    }


type Msg tmsg
    = SearchHistoryPress
      -- | AddToSearch Data.ZkListNote
      -- | AddToSearchAsTag String
    | SrFocusPress ZkListNote (List ZkListNote)
    | ClearSelection
    | SPMsg SSP.Msg
    | NavChoiceChanged NavChoice
    | ControlMsg tmsg
    | Noop


type Command tmsg
    = None
    | SearchHistory
    | ControlCommand tmsg
    | SPMod (SSP.Model -> ( SSP.Model, SSP.Command ))


init : Model
init =
    { searchOrRecent = SearchView
    , focusSr = emptyZlnDict
    }


onWkKeyPress : WK.Key -> Model -> ( Model, Command tcmd )
onWkKeyPress key model =
    case Toop.T4 key.key key.ctrl key.alt key.shift of
        Toop.T4 "Enter" False False False ->
            ( model, SPMod SSP.onEnter )

        _ ->
            ( model, None )


showSr : Int -> Model -> Maybe Data.ZkNoteId -> Data.ZkListNoteSearchResult -> Element tmsg -> Data.ZkListNote -> Element (Msg tmsg)
showSr fontsize model lastSelected zlnSearchResult controlRow zkln =
    let
        sysColor =
            ZC.systemColor DataUtil.sysids zkln.sysids

        listingrow =
            \focus ->
                E.el
                    ([ E.width E.fill
                     , E.htmlAttribute <|
                        HE.on "click"
                            -- get the zklistnotes here, since they aren't available in
                            -- the update fn.
                            (JD.map
                                (\shiftkey ->
                                    if shiftkey then
                                        let
                                            sel_range =
                                                Util.foldUntil
                                                    (\i range ->
                                                        if i.id == zkln.id then
                                                            Util.Stop range

                                                        else if TDict.member i.id model.focusSr then
                                                            Util.Go [ i ]

                                                        else if List.isEmpty range then
                                                            Util.Go []

                                                        else
                                                            Util.Go <| i :: range
                                                    )
                                                    []
                                                    zlnSearchResult.notes
                                        in
                                        SrFocusPress zkln sel_range

                                    else
                                        SrFocusPress zkln []
                                )
                                (JD.field "shiftKey" JD.bool)
                            )
                     , E.height <| E.px <| round <| toFloat fontsize * 1.15
                     , E.clipX
                     , E.htmlAttribute <| HA.style "user-select" "none"
                     ]
                        ++ (sysColor
                                |> Maybe.map (\c -> [ EF.color c ])
                                |> Maybe.withDefault []
                           )
                        ++ (if focus then
                                [ EBk.color TC.grey ]

                            else
                                []
                           )
                    )
                <|
                    E.text zkln.title
    in
    if TDict.member zkln.id model.focusSr then
        if lastSelected == Just zkln.id then
            E.column
                [ E.width E.fill
                , E.spacing 3
                ]
                [ listingrow True, E.map ControlMsg <| controlRow ]

        else
            listingrow True

    else
        listingrow False


view :
    ZC.StylePalette
    -> Maybe Util.Size
    -> List Data.ZkListNote
    -> SSP.Model
    -> Data.ZkListNoteSearchResult
    -> Model
    -> Element tmsg
    -> Element (Msg tmsg)
view stylePalette mbsize recentZkns spmodel zknSearchResult model controlRow =
    let
        sppad =
            [ E.padding 5 ]

        spwidth =
            E.px
                400

        lastSelected =
            List.foldl
                (\n mbn ->
                    if TDict.member n.id model.focusSr then
                        Just n

                    else
                        mbn
                )
                Nothing
                zknSearchResult.notes
                |> Maybe.map .id

        pagView =
            E.row [ E.width E.fill ]
                (if List.length zknSearchResult.notes < 15 then
                    []

                 else
                    [ E.map SPMsg <|
                        SSP.paginationView spmodel
                    ]
                )

        searchPanel =
            E.column
                (E.spacing 8
                    :: E.width E.fill
                    :: sppad
                )
                (E.row [ E.width E.fill ]
                    [ EI.button Common.buttonStyle
                        { onPress = Just <| SearchHistoryPress
                        , label = E.el [ E.centerY ] <| E.text "history"
                        }
                    ]
                    :: (E.map SPMsg <|
                            SSP.view True True 0 spmodel
                       )
                    :: pagView
                    :: (List.map
                            (showSr stylePalette.fontSize model lastSelected zknSearchResult controlRow)
                        <|
                            zknSearchResult.notes
                       )
                    ++ [ pagView ]
                )

        recentPanel =
            E.column
                (E.spacing 8
                    :: sppad
                )
                (List.map
                    (showSr stylePalette.fontSize model lastSelected zknSearchResult controlRow)
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
                [ Common.navbar 2
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
                , case model.searchOrRecent of
                    SearchView ->
                        searchPanel

                    RecentView ->
                        recentPanel
                ]
    in
    searchOrRecentPanel


update : Msg tmsg -> Model -> ( Model, Command tmsg )
update msg model =
    case msg of
        Noop ->
            ( model, None )

        SearchHistoryPress ->
            ( model, SearchHistory )

        SrFocusPress zln range ->
            case range of
                [] ->
                    ( { model
                        | focusSr =
                            if TDict.member zln.id model.focusSr then
                                TDict.remove zln.id model.focusSr

                            else
                                TDict.insert zln.id zln model.focusSr
                      }
                    , None
                    )

                nonempty ->
                    ( { model
                        | focusSr =
                            List.foldl (\zn d -> TDict.insert zn.id zn d)
                                model.focusSr
                                (zln :: nonempty)
                      }
                    , None
                    )

        ClearSelection ->
            ( { model | focusSr = emptyZlnDict }, None )

        SPMsg m ->
            -- clicking Search button clears focusSr.  next/prev don't
            -- nor do other search panel msgs.  bit of a hack.
            case m of
                SSP.SPMsg (SearchPanel.TSPMsg TagSearchPanel.SearchClick) ->
                    ( { model | focusSr = emptyZlnDict }, SPMod (SSP.update m) )

                _ ->
                    ( model, SPMod (SSP.update m) )

        ControlMsg tmsg ->
            ( model, ControlCommand tmsg )

        -- let
        --     ( tmod, tcmd ) =
        --         model.thing.update tmsg model.thing.model
        --     thing =
        --         model.thing
        -- in
        -- ( { model | thing = { thing | model = tmod } }, ThingCommand tcmd )
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


addToSearch : List ZkListNote -> (SSP.Model -> ( SSP.Model, SSP.Command ))
addToSearch notes =
    let
        f =
            List.foldl
                (\zkln g ->
                    if List.any ((==) DataUtil.sysids.searchid) zkln.sysids then
                        \m -> SSP.setSearchString (g m) zkln.title

                    else
                        \m -> SSP.addToSearch (g m) [ Data.ExactMatch ] zkln.title
                )
                Basics.identity
                notes
    in
    \m -> ( f m, SSP.None )


addToSearchAsTag : List ZkListNote -> (SSP.Model -> ( SSP.Model, SSP.Command ))
addToSearchAsTag notes =
    let
        f =
            List.foldl
                (\zkln g ->
                    \m -> SSP.addToSearch (g m) [ Data.ExactMatch, Data.Tag ] zkln.title
                )
                Basics.identity
                notes
    in
    \m -> ( f m, SSP.None )
