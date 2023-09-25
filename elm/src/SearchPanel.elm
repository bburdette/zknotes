module SearchPanel exposing
    ( Command(..)
    , Model
    , Msg(..)
    , addSearchString
    , addToSearch
    , getSearch
    , initModel
    , onEnter
    , paginationView
    , searchResultUpdated
    , setSearch
    , setSearchString
    , update
    , view
    )

import Common exposing (buttonStyle)
import Data
import Element as E exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as EI
import PaginationPanel as PP
import Parser
import Search as S exposing (AndOr(..), SearchMod(..), TSText, TagSearch(..), tagSearchParser)
import SearchHelpPanel
import TDict exposing (TDict)
import TagSearchPanel as TSP
import TangoColors as Color
import Util exposing (Size)


type alias Model =
    { tagSearchModel : TSP.Model
    , paginationModel : PP.Model
    }


initModel : Model
initModel =
    { tagSearchModel = TSP.initModel
    , paginationModel = PP.initModel
    }


searchResultUpdated : Data.ZkListNoteSearchResult -> Model -> Model
searchResultUpdated zsr model =
    { model | paginationModel = PP.searchResultUpdated zsr model.paginationModel }


getSearch : Model -> Maybe S.ZkNoteSearch
getSearch model =
    TSP.getSearch model.tagSearchModel
        |> Maybe.map
            (\s ->
                { tagSearch = [ s ]
                , offset = model.paginationModel.offset
                , limit = Just model.paginationModel.increment
                , what = ""
                , resultType = S.RtListNote
                }
            )


setSearch : Model -> TagSearch -> Model
setSearch model ts =
    { model | tagSearchModel = TSP.setSearch model.tagSearchModel (TSP.TagSearch (Ok ts)) }


setSearchString : Model -> String -> Model
setSearchString model string =
    { model
        | tagSearchModel =
            TSP.updateSearchText model.tagSearchModel string
    }


addSearchString : Model -> String -> Model
addSearchString model string =
    { model
        | tagSearchModel =
            TSP.addSearchText model.tagSearchModel string
    }


addToSearch : Model -> List SearchMod -> String -> Model
addToSearch model searchmods name =
    { model | tagSearchModel = TSP.addToSearchPanel model.tagSearchModel searchmods name }


onEnter : Model -> ( Model, Command )
onEnter model =
    handleTspUpdate model (TSP.onEnter model.tagSearchModel)


type Msg
    = TSPMsg TSP.Msg
    | PPMsg PP.Msg
    | CopyClicked
    | AndClicked


type Command
    = None
    | Save
    | Search S.ZkNoteSearch
    | Copy String
    | And TagSearch


paginationView : Model -> Element Msg
paginationView model =
    E.row [ E.width E.fill, E.spacing 8 ]
        [ E.map PPMsg <| PP.view model.paginationModel
        ]


view : Bool -> Bool -> Int -> Model -> Element Msg
view showCopy narrow nblevel model =
    column [ E.width E.fill, E.spacing 8 ]
        [ E.map TSPMsg <| TSP.view showCopy narrow nblevel model.tagSearchModel
        , paginationView model
        ]


handleTspUpdate : Model -> ( TSP.Model, TSP.Command ) -> ( Model, Command )
handleTspUpdate model ( nm, cmd ) =
    case cmd of
        TSP.None ->
            ( { model | tagSearchModel = nm }, None )

        TSP.Save ->
            ( { model | tagSearchModel = nm }, None )

        TSP.Copy txt ->
            ( model, Copy txt )

        TSP.AddToStack ->
            case model.tagSearchModel.search of
                TSP.TagSearch (Ok ts) ->
                    ( model, And ts )

                _ ->
                    ( model, None )

        TSP.Search ts ->
            ( { model | tagSearchModel = nm, paginationModel = PP.initModel }
            , Search <|
                { tagSearch = [ ts ]
                , offset = 0
                , limit = Just model.paginationModel.increment
                , what = ""
                , resultType = S.RtListNote
                }
            )


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        CopyClicked ->
            ( model, Copy model.tagSearchModel.searchText )

        AndClicked ->
            case model.tagSearchModel.search of
                TSP.TagSearch (Ok ts) ->
                    ( model, And ts )

                _ ->
                    ( model, None )

        TSPMsg m ->
            handleTspUpdate model (TSP.update m model.tagSearchModel)

        PPMsg m ->
            let
                ( nm, cmd ) =
                    PP.update m model.paginationModel
            in
            case cmd of
                PP.None ->
                    ( { model | paginationModel = nm }, None )

                PP.RangeChanged ->
                    case TSP.getSearch model.tagSearchModel of
                        Just ts ->
                            ( { model | paginationModel = nm }
                            , Search
                                { tagSearch = [ ts ]
                                , offset = nm.offset
                                , limit = Just nm.increment
                                , what = ""
                                , resultType = S.RtListNote
                                }
                            )

                        Nothing ->
                            ( { model | paginationModel = nm }, None )
