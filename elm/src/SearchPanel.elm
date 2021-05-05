module SearchPanel exposing (Command(..), Model, Msg(..), getSearch, initModel, searchResultUpdated, setSearchString, update, view)

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
                { tagSearch = s
                , offset = model.paginationModel.offset
                , limit = Just model.paginationModel.increment
                , what = ""
                , list = True
                }
            )


setSearchString : Model -> String -> Model
setSearchString model string =
    { model | tagSearchModel = TSP.updateSearchText model.tagSearchModel string }


type Msg
    = TSPMsg TSP.Msg
    | PPMsg PP.Msg
    | CopyClicked


type Command
    = None
    | Save
    | Search S.ZkNoteSearch
    | Copy String


view : Bool -> Bool -> Int -> Model -> Element Msg
view showCopy narrow nblevel model =
    column [ E.width E.fill, E.spacing 8 ]
        [ E.map TSPMsg <| TSP.view narrow nblevel model.tagSearchModel
        , E.row [ E.width E.fill ]
            [ E.map PPMsg <| PP.view model.paginationModel
            , if showCopy then
                EI.button (E.alignRight :: buttonStyle)
                    { label = E.text "< Copy"
                    , onPress = Just CopyClicked
                    }

              else
                E.none
            ]
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        CopyClicked ->
            ( model, Copy model.tagSearchModel.searchText )

        TSPMsg m ->
            let
                ( nm, cmd ) =
                    TSP.update m model.tagSearchModel
            in
            case cmd of
                TSP.None ->
                    ( { model | tagSearchModel = nm }, None )

                TSP.Save ->
                    ( { model | tagSearchModel = nm }, None )

                TSP.Search ts ->
                    ( { model | tagSearchModel = nm, paginationModel = PP.initModel }
                    , Search <|
                        { tagSearch = ts
                        , offset = 0
                        , limit = Just model.paginationModel.increment
                        , what = ""
                        , list = True
                        }
                    )

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
                                { tagSearch = ts
                                , offset = nm.offset
                                , limit = Just nm.increment
                                , what = ""
                                , list = True
                                }
                            )

                        Nothing ->
                            ( { model | paginationModel = nm }, None )
