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
    }


type Msg
    = NextPress
    | PrevPress
    | ClosePress
    | GotNote ZkNoteId
    | ViewMsg View.Msg


type Command
    = Close
    | GetNote ZkNoteId
    | Noop


viewConfig : View.Config
viewConfig =
    { showLinks = False
    , showTitle = True
    , showContents = True
    , showMedia = True
    , showDates = True
    , showPanel = True
    , loggedin = False
    }


init : FileUrlInfo -> NoteCache -> NlLink -> List NlLink -> ( Model, Command )
init fui nc nl nlls =
    let
        zkn =
            getNote nc nl.id
    in
    ( { nlls = Array.fromList (nl :: nlls)
      , current = 0
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
      }
    , case zkn of
        Just (ZNAL note) ->
            Noop

        Just Private ->
            Noop

        Just NotFound ->
            Noop

        Nothing ->
            GetNote nl.id
    )


view : Time.Zone -> Int -> NoteCache -> Model -> E.Element Msg
view tz maxw nc model =
    E.column []
        [ model.viewModel
            |> Maybe.map (\m -> E.map ViewMsg <| View.view tz maxw nc viewConfig m)
            |> Maybe.withDefault (E.text "loading... ")
        , E.row []
            [ EI.button Common.buttonStyle
                { onPress = Just PrevPress
                , label = E.text "prev"
                }
            , EI.button Common.buttonStyle
                { onPress = Just NextPress
                , label = E.text "next"
                }
            ]
        ]


update : Msg -> NoteCache -> Model -> ( Model, Command )
update msg nc model =
    case msg of
        NextPress ->
            ( { model | current = (model.current + 1) // Array.length model.nlls }, Noop )

        PrevPress ->
            ( { model | current = (model.current - 1) // Array.length model.nlls }, Noop )

        ClosePress ->
            ( model, Close )

        GotNote zkid ->
            ( model, Close )

        ViewMsg vmsg ->
            ( { model
                | viewModel =
                    model.viewModel
                        |> Maybe.map (\vmod -> View.update vmsg vmod |> Tuple.first)
              }
            , Noop
            )
