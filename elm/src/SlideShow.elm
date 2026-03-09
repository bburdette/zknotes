module SlideShow exposing (..)

import Array exposing (Array)
import Common
import Data exposing (ZkNoteId)
import DataUtil exposing (FileUrlInfo, NlLink, ZniSet, emptyZniSet)
import DnDList
import DndPorts exposing (..)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Html.Attributes
import Html.Events as HE
import Json.Decode as JD
import NoteCache exposing (CacheEntry(..), NoteCache, getNote)
import SpecialNotes exposing (Notegraph)
import TSet
import TangoColors as TC
import Time
import Util
import View
import ZkCommon as ZC


type alias Model =
    { nlls : Array NlLink
    , current : Int
    , viewModel : Maybe View.Model
    , fui : FileUrlInfo
    }


type Msg
    = NextPress
    | PrevPress
    | ClosePress
    | ViewMsg View.Msg


type Command
    = Close (Maybe ZkNoteId)
    | GetNote ZkNoteId
    | Noop


viewConfig : View.Config
viewConfig =
    { showLinks = False
    , alwaysShowTitle = True
    , showContents = True
    , showMedia = True
    , showDates = True
    , showPanel = True
    , loggedin = False
    , autoplay = True
    }


init : FileUrlInfo -> NoteCache -> Maybe ZkNoteId -> NlLink -> List NlLink -> ( Model, Command )
init fui nc mbcurrent nl rnlls =
    let
        nlls =
            Array.fromList (nl :: rnlls)

        current =
            Util.findFirstIndex (\nll -> Just nll.id == mbcurrent) nlls
                |> Maybe.withDefault 0

        getid =
            mbcurrent |> Maybe.withDefault nl.id

        zkn =
            getNote nc getid
    in
    ( { nlls = nlls
      , current = current
      , viewModel =
            case zkn of
                Just (ZNAL note) ->
                    Just <| View.initFull fui note

                Just Private ->
                    Nothing

                Just NotFound ->
                    Nothing

                Nothing ->
                    Nothing
      , fui = fui
      }
    , case zkn of
        Just (ZNAL note) ->
            Noop

        Just Private ->
            Noop

        Just NotFound ->
            Noop

        Nothing ->
            GetNote getid
    )


view : Time.Zone -> Int -> NoteCache -> Model -> E.Element Msg
view tz maxw nc model =
    E.column []
        [ E.row []
            [ EI.button Common.buttonStyle
                { onPress = Just PrevPress
                , label = E.text "prev"
                }
            , EI.button Common.buttonStyle
                { onPress = Just NextPress
                , label = E.text "next"
                }
            , EI.button Common.buttonStyle
                { onPress = Just ClosePress
                , label = E.text "close"
                }
            ]
        , model.viewModel
            |> Maybe.map (\m -> E.map ViewMsg <| View.view tz maxw nc viewConfig m)
            |> Maybe.withDefault (E.text "loading... ")
        ]


update : Msg -> NoteCache -> Model -> ( Model, Command )
update msg nc model =
    case msg of
        NextPress ->
            { model
                | current =
                    modBy
                        (Array.length model.nlls)
                        (model.current + 1)
            }
                |> updateNote nc

        PrevPress ->
            { model
                | current = modBy (Array.length model.nlls) (model.current - 1)
            }
                |> updateNote nc

        ClosePress ->
            ( model
            , Close
                (Array.get model.current model.nlls
                    |> Maybe.map .id
                )
            )

        ViewMsg vmsg ->
            case model.viewModel of
                Just vm ->
                    let
                        ( vmod, vcmd ) =
                            View.update vmsg vm
                    in
                    case vcmd of
                        View.None ->
                            ( { model | viewModel = Just vmod }, Noop )

                        View.Done ->
                            ( { model | viewModel = Just vmod }, Noop )

                        View.Switch _ ->
                            ( { model | viewModel = Just vmod }, Noop )

                        View.OnPlaybackEnded ->
                            { model
                                | current =
                                    modBy (Array.length model.nlls)
                                        (model.current + 1)
                            }
                                |> updateNote nc

                Nothing ->
                    ( model, Noop )


updateNote : NoteCache -> Model -> ( Model, Command )
updateNote nc model =
    case Array.get model.current model.nlls of
        Just n ->
            let
                ( nvm, c ) =
                    case getNote nc n.id of
                        Just (ZNAL gotn) ->
                            ( Just <| View.initFull model.fui gotn, Noop )

                        Just Private ->
                            ( Nothing, Noop )

                        Just NotFound ->
                            ( Nothing, Noop )

                        Nothing ->
                            ( Nothing, GetNote n.id )
            in
            case model.viewModel of
                Just vm ->
                    if vm.id == Just n.id then
                        ( model, Noop )

                    else
                        ( { model | viewModel = nvm }, c )

                Nothing ->
                    ( { model | viewModel = nvm }, c )

        Nothing ->
            ( { model | viewModel = Nothing }, Noop )
