module TagSearchPanel exposing (Command(..), Model, Msg(..), Search(..), addSearchText, addTagToSearchPrev, addToSearch, addToSearchPanel, getSearch, initModel, onEnter, selectPrevSearch, toggleHelpButton, update, updateSearchText, view)

import Common exposing (buttonStyle)
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick, onFocus, onLoseFocus)
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
    , searchFocus : Bool
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
    , searchFocus = False
    }


type Msg
    = SearchText String
    | STFocus Bool
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

        TagSearch (Err _) ->
            Nothing


addToSearch : List SearchMod -> String -> Search -> Search
addToSearch searchmods name search =
    let
        term =
            SearchTerm
                searchmods
                -- escape single quotes
                (String.replace "'" "\\'" name)
    in
    case search of
        NoSearch ->
            TagSearch (Ok term)

        TagSearch (Err e) ->
            TagSearch (Err e)

        TagSearch (Ok s) ->
            TagSearch (Ok (Boolex s And term))


addSearch : Search -> Search -> Search
addSearch ls rs =
    case ls of
        NoSearch ->
            rs

        TagSearch (Err e) ->
            rs

        TagSearch (Ok sl) ->
            case rs of
                NoSearch ->
                    ls

                TagSearch (Err e) ->
                    ls

                TagSearch (Ok sr) ->
                    TagSearch (Ok (Boolex sl And sr))


addToSearchPanel : Model -> List SearchMod -> String -> Model
addToSearchPanel model searchmods name =
    let
        s =
            addToSearch searchmods name model.search
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


addSearchText : Model -> String -> Model
addSearchText model txt =
    let
        addsearch =
            if String.contains "'" txt then
                TagSearch <| Parser.run tagSearchParser txt

            else
                TagSearch <| Ok <| Search.SearchTerm [] txt

        nsearch =
            addSearch addsearch model.search
    in
    case nsearch of
        TagSearch (Ok ts) ->
            { model
                | searchText = Search.printTagSearch ts
                , search = nsearch
            }

        _ ->
            model


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

        sbs =
            if narrow then
                buttonStyle ++ [ alignLeft ]

            else
                buttonStyle

        searchButton =
            case model.search of
                TagSearch (Err _) ->
                    Input.button (sbs ++ [ Background.color Color.grey ]) { onPress = Nothing, label = text "search:" }

                _ ->
                    Input.button sbs { onPress = Just SearchClick, label = text "search:" }

        tinput =
            Input.text
                (onFocus (STFocus True) :: onLoseFocus (STFocus False) :: tiattribs)
                { onChange = SearchText
                , text = model.searchText
                , placeholder = Nothing
                , label =
                    if narrow then
                        Input.labelHidden "search"

                    else
                        Input.labelLeft [] <|
                            row [ centerY ]
                                [ searchButton
                                ]
                }

        ddbutton =
            none

        {- save and restore search stuff, disabled for now:

           was ddbutton:
           Input.button buttonStyle
               { onPress = Just TogglePrev
               , label =
                   text <|
                       if model.showPrevs then
                           "∧"
                       else
                           "∨"
               }

               from 'buttons', below.
           if List.any (\elt -> elt == model.searchText) model.prevSearches then
               none
             else
               Input.button buttonStyle
                   { onPress = Just SaveSearch
                   , label = text "save"
                   }
           ,
        -}
        obs =
            alignRight :: buttonStyle

        buttons =
            [ if narrow then
                searchButton

              else
                none
            , Input.button obs
                { onPress = Just SearchDetails
                , label =
                    text <|
                        if model.showParse then
                            "-"

                        else
                            "?"
                }
            , Input.button obs
                { onPress = Just Clear
                , label = text "x"
                }
            ]

        showborder =
            model.showParse || narrow
    in
    column
        (if showborder then
            [ padding 2
            , spacing 8
            , width fill
            ]

         else
            [ width fill ]
        )
        ((if narrow then
            [ row [ width fill, spacing 3 ] [ tinput, ddbutton ]
            , row [ spacing 3, width fill ] buttons
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


onEnter : Model -> ( Model, Command )
onEnter model =
    if model.searchFocus then
        doSearchClick model

    else
        ( model, None )


doSearchClick : Model -> ( Model, Command )
doSearchClick model =
    ( model
    , case getSearch model of
        Just s ->
            Search s

        Nothing ->
            None
    )


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SearchText txt ->
            ( updateSearchText model txt
            , None
            )

        STFocus focused ->
            ( { model | searchFocus = focused }, None )

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
            doSearchClick model

        ToggleHelp ->
            ( { model | showHelp = not model.showHelp }, None )

        HelpMsg hmsg ->
            ( { model | helpPanel = SearchHelpPanel.update model.helpPanel hmsg }, None )
