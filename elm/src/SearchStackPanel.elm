module SearchStackPanel exposing (Command(..), Model, Msg(..), addSearchString, addToSearch, getSearch, initModel, searchResultUpdated, setSearchString, update, view)

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
import SearchPanel as SP
import TDict exposing (TDict)
import TagSearchPanel as TSP
import TangoColors as Color
import Util exposing (Size)



-- looks a lot like the regular search panel, but actually contains a stack of searches
-- and the regular search panel too.


type alias Model =
    { searchStack : List TagSearch
    , spmodel : SP.Model
    }


initModel : Model
initModel =
    { searchStack = []
    , spmodel = SP.initModel
    }


searchResultUpdated : Data.ZkListNoteSearchResult -> Model -> Model
searchResultUpdated zsr model =
    { model | spmodel = SP.searchResultUpdated zsr model.spmodel }


andifySearch : List TagSearch -> TagSearch -> TagSearch
andifySearch searches search =
    List.foldr (\sl sr -> Boolex sl And sr) search searches


getSearch : Model -> Maybe S.ZkNoteSearch
getSearch model =
    SP.getSearch model.spmodel
        |> Maybe.map
            (\s ->
                { s | tagSearch = andifySearch model.searchStack s.tagSearch }
            )


setSearchString : Model -> String -> Model
setSearchString model string =
    { model
        | spmodel = SP.setSearchString model.spmodel string
    }


addSearchString : Model -> String -> Model
addSearchString model string =
    { model
        | spmodel = SP.addSearchString model.spmodel string
    }


addToSearch : Model -> List SearchMod -> String -> Model
addToSearch model searchmods name =
    { model | spmodel = SP.addToSearch model.spmodel searchmods name }


type Msg
    = SPMsg SP.Msg
    | MinusPress Int


type Command
    = None
    | Save
    | Search S.ZkNoteSearch
    | Copy String


view : Bool -> Bool -> Int -> Model -> Element Msg
view showCopy narrow nblevel model =
    E.column [ E.width E.fill, E.spacing 3 ] <|
        List.indexedMap
            (\i ts ->
                E.row [ E.width E.fill, E.centerY ]
                    [ E.el [ E.width E.fill, E.clipX ] <| E.text <| S.printTagSearch ts
                    , EI.button (buttonStyle ++ [ E.alignRight ])
                        { label = E.text "-"
                        , onPress = Just <| MinusPress i
                        }
                    ]
            )
            model.searchStack
            ++ [ E.map SPMsg <| SP.view showCopy narrow nblevel model.spmodel
               ]


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        MinusPress idx ->
            ( { model
                | searchStack =
                    List.take idx model.searchStack
                        ++ List.drop (idx + 1) model.searchStack
              }
            , None
            )

        SPMsg m ->
            let
                ( nm, cmd ) =
                    SP.update m model.spmodel
            in
            case cmd of
                SP.None ->
                    ( { model | spmodel = nm }, None )

                SP.Save ->
                    ( { model | spmodel = nm }, None )

                SP.Search ts ->
                    ( { model | spmodel = nm }
                    , Search <|
                        { ts
                            | tagSearch =
                                andifySearch model.searchStack ts.tagSearch
                        }
                    )

                SP.Copy s ->
                    ( model, Copy s )

                SP.And s ->
                    ( { model | searchStack = model.searchStack ++ [ s ] }, None )
