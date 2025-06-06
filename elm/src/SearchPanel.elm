module SearchPanel exposing
    ( Command(..)
    , Model
    , Msg(..)
    , addSearchString
    , addToSearch
    , getSearch
    , initModel
    , onEnter
    , onOrdering
    , paginationView
    , searchResultUpdated
    , setSearch
    , setSearchString
    , update
    , view
    )

import Data exposing (AndOr(..), SearchMod(..), TagSearch(..))
import Element as E exposing (..)
import PaginationPanel as PP
import TagSearchPanel as TSP


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


showDeleted : Bool
showDeleted =
    False


getSearch : Model -> Maybe Data.ZkNoteSearch
getSearch model =
    TSP.getSearch model.tagSearchModel
        |> Maybe.map
            (\s ->
                { tagsearch = [ s.ts ]
                , offset = model.paginationModel.offset
                , limit = Just model.paginationModel.increment
                , what = ""
                , resulttype = Data.RtListNote
                , archives = False
                , deleted = showDeleted
                , ordering = s.ordering
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


onOrdering : Maybe Data.Ordering -> Model -> ( Model, Command )
onOrdering ordering model =
    handleTspUpdate model (TSP.onOrdering ordering model.tagSearchModel)


type Msg
    = TSPMsg TSP.Msg
    | PPMsg PP.Msg
    | CopyClicked
    | AndClicked


type Command
    = None
    | Save
    | Search Data.ZkNoteSearch
    | SyncFiles Data.ZkNoteSearch
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

        TSP.Search s ->
            ( { model | tagSearchModel = nm, paginationModel = PP.initModel }
            , Search <|
                { tagsearch = [ s.ts ]
                , offset = 0
                , limit = Just model.paginationModel.increment
                , what = ""
                , resulttype = Data.RtListNote
                , archives = False
                , deleted = showDeleted
                , ordering = s.ordering
                }
            )

        TSP.SyncFiles ts ->
            ( { model | tagSearchModel = nm, paginationModel = PP.initModel }
            , SyncFiles <|
                { tagsearch = [ ts ]
                , offset = 0
                , limit = Nothing
                , what = ""
                , resulttype = Data.RtListNote
                , archives = False
                , deleted = showDeleted
                , ordering = Nothing
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
                        Just s ->
                            ( { model | paginationModel = nm }
                            , Search
                                { tagsearch = [ s.ts ]
                                , offset = nm.offset
                                , limit = Just nm.increment
                                , what = ""
                                , resulttype = Data.RtListNote
                                , archives = False
                                , deleted = showDeleted
                                , ordering = s.ordering
                                }
                            )

                        Nothing ->
                            ( { model | paginationModel = nm }, None )
