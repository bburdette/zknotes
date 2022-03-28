module SearchStackPanel exposing
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
import Element.Background as EBk
import Element.Border as EBd
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
import TangoColors as TC
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
                { s | tagSearch = model.searchStack ++ s.tagSearch }
            )


setSearch : Model -> List TagSearch -> Model
setSearch model tsl =
    case List.head (List.reverse tsl) of
        Just s ->
            { model
                | spmodel = SP.setSearch model.spmodel s
                , searchStack = List.reverse (List.drop 1 (List.reverse tsl))
            }

        Nothing ->
            { model | spmodel = SP.setSearch model.spmodel (S.SearchTerm [] "") }


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


onEnter : Model -> ( Model, Command )
onEnter model =
    handleSpUpdate model (SP.onEnter model.spmodel)


handleSpUpdate : Model -> ( SP.Model, SP.Command ) -> ( Model, Command )
handleSpUpdate model ( nm, cmd ) =
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
                        model.searchStack ++ ts.tagSearch
                }
            )

        SP.Copy s ->
            ( model, Copy s )

        SP.And s ->
            ( { model
                | searchStack = model.searchStack ++ [ s ]
                , spmodel = SP.initModel
              }
            , None
            )


paginationView : Bool -> Model -> Element Msg
paginationView showCopy model =
    E.map SPMsg (SP.paginationView showCopy model.spmodel)


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
        (if List.isEmpty model.searchStack then
            E.none

         else
            E.column [ E.width E.fill, E.spacing 3, E.padding 3, EBk.color TC.lightGrey ] <|
                List.indexedMap
                    (\i ts ->
                        E.row [ E.width E.fill, E.centerY ]
                            [ E.el [ E.width E.fill, E.clipX, E.height E.fill ] <| E.text <| S.printTagSearch ts
                            , EI.button (buttonStyle ++ [ E.alignRight ])
                                { label =
                                    E.text "-"
                                , onPress = Just <| MinusPress i
                                }
                            ]
                    )
                    model.searchStack
        )
            :: [ E.map SPMsg <| SP.view showCopy narrow nblevel model.spmodel
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
            handleSpUpdate model <|
                SP.update m model.spmodel
