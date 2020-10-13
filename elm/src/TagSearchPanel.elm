module TagSearchPanel exposing (Command(..), Model, Msg(..), Search(..), addTagToSearch, addTagToSearchPrev, addToSearch, getSearch, initModel, selectPrevSearch, toggleHelpButton, update, updateSearchText, view)

import Common exposing (buttonStyle)
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
import Parser
import Search exposing (AndOr(..), SearchMod(..), TSText, TagSearch(..), tagSearchParser)
import SearchHelpPanel
import TDict exposing (TDict)
import TangoColors as Color
import Util exposing (Size)


type Search
    = TagSearch (Result (List Parser.DeadEnd) TagSearch)
    | NoSearch


type alias Model =
    { searchText : String
    , search : Search
    , showParse : Bool
    , showHelp : Bool
    , helpPanel : SearchHelpPanel.Model
    , showPrevs : Bool
    , prevSearches : List String
    }


initModel : Model
initModel =
    { searchText = ""
    , search = NoSearch
    , showParse = False
    , showHelp = False
    , helpPanel = SearchHelpPanel.init
    , showPrevs = False
    , prevSearches = []
    }


type Msg
    = SearchText String
    | SearchDetails
    | SearchClick
    | ToggleHelp
    | Clear
    | TogglePrev
    | PrevSelected String
    | SaveSearch
    | HelpMsg SearchHelpPanel.Msg


type Command
    = None
    | Save
    | Search TagSearch


getSearch : Model -> Maybe TagSearch
getSearch model =
    case model.search of
        TagSearch (Ok s) ->
            Just s

        NoSearch ->
            Just <| SearchTerm [] ""

        _ ->
            Nothing


addToSearch : String -> Search -> Search
addToSearch name search =
    let
        term =
            SearchTerm
                [ ExactMatch
                , Tag
                ]
                name
    in
    case search of
        NoSearch ->
            TagSearch (Ok term)

        TagSearch (Err e) ->
            TagSearch (Err e)

        TagSearch (Ok s) ->
            TagSearch (Ok (Boolex s And term))


addTagToSearch : Model -> String -> Model
addTagToSearch model name =
    let
        s =
            addToSearch name model.search
    in
    case s of
        TagSearch (Ok ts) ->
            { model
                | search = s
                , searchText = Search.printTagSearch ts
            }

        _ ->
            model


addTagToSearchPrev : Model -> String -> Model
addTagToSearchPrev model name =
    let
        at =
            "t"
    in
    updateSearchText model <|
        if model.searchText == "" then
            at ++ "ec'" ++ name ++ "'"

        else
            model.searchText ++ " & " ++ at ++ "ec'" ++ name ++ "'"


updateSearchText : Model -> String -> Model
updateSearchText model txt =
    { model
        | searchText = txt
        , search =
            if String.contains "'" txt then
                TagSearch <| Parser.run tagSearchParser txt

            else
                TagSearch <| Ok <| Search.SearchTerm [] txt
    }


selectPrevSearch : List String -> Element Msg
selectPrevSearch searches =
    column
        [ width fill
        , centerX
        , centerY
        , Border.color Color.black
        , Border.width 2
        , Background.color Color.white
        ]
        (List.map
            (\s ->
                row
                    [ mouseOver [ Background.color Color.lightBlue ]
                    , mouseDown [ Background.color Color.darkBlue ]
                    , onClick (PrevSelected s)
                    , width fill
                    ]
                    [ text s ]
            )
            searches
        )


view : Bool -> Int -> Model -> Element Msg
view narrow nblevel model =
    let
        inputcolor =
            case model.search of
                TagSearch ts ->
                    case ts of
                        Ok _ ->
                            []

                        Err _ ->
                            [ Background.color <| rgb255 255 128 128 ]

                NoSearch ->
                    []

        attribs =
            width fill :: inputcolor

        tiattribs =
            if model.showPrevs then
                below (selectPrevSearch model.prevSearches) :: attribs

            else
                attribs

        tinput =
            Input.text
                tiattribs
                { onChange = SearchText
                , text = model.searchText
                , placeholder = Nothing
                , label =
                    Input.labelLeft [] <|
                        row [ centerY ]
                            [ case model.search of
                                TagSearch (Err _) ->
                                    Input.button (buttonStyle ++ [ Background.color Color.grey ]) { onPress = Nothing, label = text "search:" }

                                _ ->
                                    Input.button buttonStyle { onPress = Just SearchClick, label = text "search:" }
                            ]
                }

        ddbutton =
            Input.button buttonStyle
                { onPress = Just TogglePrev
                , label =
                    text <|
                        if model.showPrevs then
                            "∧"

                        else
                            "∨"
                }

        buttons =
            [ if List.any (\elt -> elt == model.searchText) model.prevSearches then
                none

              else
                Input.button buttonStyle
                    { onPress = Just SaveSearch
                    , label = text "save"
                    }
            , Input.button buttonStyle
                { onPress = Just SearchDetails
                , label =
                    text <|
                        if model.showParse then
                            "-"

                        else
                            "?"
                }
            , Input.button buttonStyle
                { onPress = Just Clear
                , label = text "x"
                }
            ]

        showborder =
            model.showParse || narrow
    in
    column
        (if showborder then
            [ Border.color Color.darkBlue
            , Border.width 3
            , padding 2
            , width fill
            ]

         else
            [ width fill ]
        )
        ((if narrow then
            [ row [ width fill, spacing 3 ] [ tinput, ddbutton ]
            , row [ spacing 3, alignRight ] buttons
            ]

          else
            [ row [ width fill, spacing 3 ]
                (tinput :: ddbutton :: buttons)
            ]
         )
            ++ (if model.showParse then
                    case model.search of
                        TagSearch rts ->
                            case rts of
                                Err e ->
                                    [ column [ width fill ]
                                        [ row [ spacing 3, width fill ]
                                            [ text "Syntax error:"
                                            , paragraph [] [ text (Util.deadEndsToString e) ]
                                            , el [ alignRight ] <| toggleHelpButton model.showHelp
                                            ]
                                        , if model.showHelp then
                                            Element.map HelpMsg <| SearchHelpPanel.view nblevel model.helpPanel

                                          else
                                            Element.none
                                        ]
                                    ]

                                Ok ts ->
                                    [ column [ width fill ]
                                        [ paragraph [ spacing 3, width fill ]
                                            [ text "search expression:"
                                            , paragraph [] [ text <| Search.printTagSearch ts ]
                                            , el [ alignRight ] <| toggleHelpButton model.showHelp
                                            ]
                                        , if model.showHelp then
                                            Element.map HelpMsg <| SearchHelpPanel.view nblevel model.helpPanel

                                          else
                                            Element.none
                                        ]
                                    ]

                        NoSearch ->
                            [ Element.map HelpMsg <|
                                SearchHelpPanel.view nblevel model.helpPanel
                            ]

                else
                    []
               )
        )


toggleHelpButton : Bool -> Element Msg
toggleHelpButton showHelp =
    Input.button buttonStyle
        { onPress = Just ToggleHelp
        , label =
            case showHelp of
                True ->
                    text "hide search help"

                False ->
                    text "show search help"
        }


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SearchText txt ->
            ( updateSearchText model txt
            , None
            )

        Clear ->
            ( { model
                | searchText = ""
                , search = NoSearch
              }
            , None
            )

        TogglePrev ->
            ( { model | showPrevs = not model.showPrevs }
            , None
            )

        PrevSelected ps ->
            ( updateSearchText
                { model
                    | showPrevs = False
                }
                ps
            , None
            )

        SaveSearch ->
            ( { model | prevSearches = model.searchText :: model.prevSearches }, Save )

        SearchDetails ->
            ( { model | showParse = not model.showParse }, None )

        SearchClick ->
            ( model
            , case getSearch model of
                Just s ->
                    Search s

                Nothing ->
                    None
            )

        ToggleHelp ->
            ( { model | showHelp = not model.showHelp }, None )

        HelpMsg hmsg ->
            ( { model | helpPanel = SearchHelpPanel.update model.helpPanel hmsg }, None )
