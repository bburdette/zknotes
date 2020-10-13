module SearchPanel exposing (Command(..), Model, Msg(..), initModel, update, view)

import Common exposing (buttonStyle)
import Data
import Element as E exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
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
    , zkid : Int
    }


initModel : Int -> Model
initModel zkid =
    { tagSearchModel = TSP.initModel
    , paginationModel = PP.initModel
    , zkid = zkid
    }


type Msg
    = TSPMsg TSP.Msg
    | PPMsg PP.Msg


type Command
    = None
    | Save
    | Search S.ZkNoteSearch


view : Bool -> Int -> Model -> Element Msg
view narrow nblevel model =
    column []
        [ E.map TSPMsg <| TSP.view narrow nblevel model.tagSearchModel
        , E.map PPMsg <| PP.view model.paginationModel
        ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
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
                    ( { model | tagSearchModel = nm }
                    , Search
                        { tagSearch = ts
                        , zks = [ model.zkid ]
                        , offset = model.paginationModel.offset
                        , limit = Just model.paginationModel.increment
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
                                , zks = [ model.zkid ]
                                , offset = nm.offset
                                , limit = Just nm.increment
                                }
                            )

                        Nothing ->
                            ( { model | paginationModel = nm }, None )
