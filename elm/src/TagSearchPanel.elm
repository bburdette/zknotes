module TagSearchPanel exposing (Command(..), Model, Msg(..), Search(..), addSearchText, addTagToSearchPrev, addToSearch, addToSearchPanel, getSearch, initModel, onEnter, selectPrevSearch, toggleHelpButton, update, updateSearchText, view)

import Common exposing (buttonStyle)
import Element as E exposing (..)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE exposing (onClick, onFocus, onLoseFocus)
import Element.Font as EF
import Element.Input as EI
import Element.Keyed as EK
import Html.Attributes as HA
import Parser
import Search exposing (AndOr(..), SearchMod(..), TSText, TagSearch(..), tagSearchParser)
import SearchHelpPanel
import SearchLoc as SL exposing (TSLoc(..))
import TDict exposing (TDict)
import TangoColors as TC
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
    , searchTermFocus : Maybe TSLoc
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
    , searchTermFocus = Nothing
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
    | ToggleAndOr TSLoc
    | ToggleTermFocus TSLoc
    | DeleteTerm TSLoc
    | NotTerm TSLoc
    | ToggleSearchMod TSLoc SearchMod
    | SetTermText TSLoc String
    | AddEmptyTerm TSLoc


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


addToSearch : Maybe TSLoc -> List SearchMod -> String -> Search -> Search
addToSearch mbtsloc searchmods name search =
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
            case mbtsloc of
                Just tsloc ->
                    s
                        |> SL.getTerm tsloc
                        |> Maybe.andThen
                            (\tm ->
                                SL.setTerm tsloc (Boolex tm And term) s
                            )
                        |> Maybe.map (\ts -> TagSearch (Ok ts))
                        |> Maybe.withDefault search

                Nothing ->
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


setSearch : Search -> Model -> Model
setSearch s model =
    case s of
        TagSearch (Ok ts) ->
            { model
                | search = s
                , searchText = Search.printTagSearch ts
            }

        _ ->
            { model | search = s }


addToSearchPanel : Model -> List SearchMod -> String -> Model
addToSearchPanel model searchmods name =
    let
        s =
            addToSearch model.searchTermFocus searchmods name model.search
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
        , EBd.color TC.black
        , EBd.width 2
        , EBk.color TC.white
        ]
        (List.map
            (\s ->
                row
                    [ mouseOver [ EBk.color TC.lightBlue ]
                    , mouseDown [ EBk.color TC.darkBlue ]
                    , onClick (PrevSelected s)
                    , width fill
                    ]
                    [ text s ]
            )
            searches
        )


viewSearch : Maybe TSLoc -> TagSearch -> Element Msg
viewSearch mbfocusloc ts =
    E.column [ E.width E.fill ] <|
        viewSearchHelper mbfocusloc 0 [] ts


viewSearchHelper : Maybe TSLoc -> Int -> List (TSLoc -> TSLoc) -> TagSearch -> List (Element Msg)
viewSearchHelper mbfocusloc indent lts ts =
    let
        indentelt =
            \idt -> E.row [ E.width (E.px (8 * indent)) ] []

        toLoc : List (TSLoc -> TSLoc) -> TSLoc -> TSLoc
        toLoc tll tsl =
            List.foldl (\tlf tl -> tlf tl) tsl tll

        hasfocus =
            \term ->
                Just term == mbfocusloc

        color =
            \term ->
                if Just term == mbfocusloc then
                    EF.color TC.blue

                else
                    EF.color TC.black
    in
    case ts of
        SearchTerm searchmods term ->
            let
                tloc =
                    toLoc lts LThis

                downButtonStyle =
                    buttonStyle ++ [ EBk.color TC.grey ]

                modbutton =
                    \mod label ->
                        EI.button
                            (if List.member mod searchmods then
                                downButtonStyle

                             else
                                buttonStyle
                            )
                            { onPress = Just (ToggleSearchMod tloc mod)
                            , label = text label
                            }

                modindicator =
                    \mod ->
                        EI.button
                            (E.alignRight :: downButtonStyle)
                            { onPress = Nothing
                            , label =
                                text
                                    (case mod of
                                        ExactMatch ->
                                            "e"

                                        Tag ->
                                            "t"

                                        Note ->
                                            "n"

                                        User ->
                                            "u"
                                    )
                            }
            in
            [ E.row [ E.width E.fill, E.spacing 8 ]
                [ indentelt indent
                , if hasfocus tloc then
                    E.column
                        [ E.width E.fill
                        , EBk.color TC.lightGrey
                        ]
                        [ E.row
                            [ onClick <| ToggleTermFocus tloc
                            , E.spacing 8
                            , E.width E.fill
                            ]
                            [ E.el
                                [ color tloc
                                ]
                              <|
                                E.text term
                            ]
                        , E.row
                            [ E.padding 8, E.spacing 8, E.centerX ]
                            [ modbutton ExactMatch "e"
                            , modbutton Tag "t"
                            , modbutton Note "n"
                            , modbutton User "u"
                            , EI.button
                                buttonStyle
                                { onPress = Just (NotTerm tloc)
                                , label =
                                    text "!"
                                }
                            , EI.button
                                buttonStyle
                                { onPress = Just (AddEmptyTerm tloc)
                                , label =
                                    text "+"
                                }
                            , EI.button
                                buttonStyle
                                { onPress = Just (DeleteTerm tloc)
                                , label = text "x"
                                }
                            ]
                        , E.row [ E.padding 8, E.spacing 8, E.width E.fill ]
                            [ EI.text
                                [ E.width E.fill ]
                                { onChange = SetTermText tloc
                                , text = term
                                , placeholder = Nothing
                                , label =
                                    EI.labelHidden "search term"
                                }
                            ]
                        ]

                  else
                    E.row
                        [ onClick <| ToggleTermFocus tloc
                        , E.width E.fill
                        , E.spacing 8
                        ]
                        ([ E.el
                            [ color tloc
                            ]
                           <|
                            E.text term
                         ]
                            ++ List.map modindicator searchmods
                        )
                ]
            ]

        Not nts ->
            let
                tloc =
                    toLoc lts LThis
            in
            [ E.row [ E.width E.fill, E.spacing 8 ]
                [ indentelt indent
                , if hasfocus tloc then
                    E.row [ EBk.color TC.lightGrey, E.paddingXY 0 8, E.spacing 8, E.width E.fill ]
                        [ E.el
                            [ onClick <| ToggleTermFocus tloc
                            , color tloc
                            ]
                          <|
                            E.text "not"
                        , EI.button
                            buttonStyle
                            { onPress = Just (AddEmptyTerm tloc)
                            , label =
                                text "+"
                            }
                        , EI.button
                            buttonStyle
                            { onPress = Just (DeleteTerm tloc)
                            , label = text "x"
                            }
                        , E.row [ E.width E.fill, onClick <| ToggleTermFocus tloc ] [ E.text "" ]
                        ]

                  else
                    E.row
                        [ onClick <| ToggleTermFocus tloc
                        , E.width E.fill
                        ]
                        [ E.text "not"
                        ]
                ]
            ]
                ++ viewSearchHelper mbfocusloc (indent + 1) (LNot :: lts) nts

        Boolex ts1 andor ts2 ->
            let
                tloc =
                    toLoc lts LThis
            in
            viewSearchHelper mbfocusloc (indent + 1) (LBT1 :: lts) ts1
                ++ [ E.row [ E.width E.fill, E.spacing 8 ]
                        [ indentelt indent
                        , if hasfocus tloc then
                            E.row [ EBk.color TC.lightGrey, E.paddingXY 0 8, E.spacing 8, E.width E.fill ]
                                [ E.el
                                    [ onClick <| ToggleTermFocus tloc
                                    , color tloc
                                    ]
                                  <|
                                    E.text
                                        (case andor of
                                            And ->
                                                "and"

                                            Or ->
                                                "or"
                                        )
                                , EI.button
                                    buttonStyle
                                    { onPress = Just (ToggleAndOr tloc)
                                    , label =
                                        text
                                            (case andor of
                                                And ->
                                                    "or"

                                                Or ->
                                                    "and"
                                            )
                                    }
                                , EI.button
                                    buttonStyle
                                    { onPress = Just (NotTerm tloc)
                                    , label =
                                        text "!"
                                    }
                                , EI.button
                                    buttonStyle
                                    { onPress = Just (AddEmptyTerm tloc)
                                    , label =
                                        text "+"
                                    }
                                , EI.button
                                    buttonStyle
                                    { onPress = Just (DeleteTerm tloc)
                                    , label = text "x"
                                    }
                                , E.row [ E.width E.fill, onClick <| ToggleTermFocus tloc ] [ E.text "" ]
                                ]

                          else
                            E.row
                                [ onClick <| ToggleTermFocus tloc
                                , E.width E.fill
                                ]
                                [ E.el [ color tloc ] <|
                                    E.text
                                        (case andor of
                                            And ->
                                                "and"

                                            Or ->
                                                "or"
                                        )
                                ]
                        ]
                   ]
                ++ viewSearchHelper mbfocusloc (indent + 1) (LBT2 :: lts) ts2


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
                            [ EBk.color <| rgb255 255 128 128 ]

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
                    EI.button (sbs ++ [ EBk.color TC.grey ]) { onPress = Nothing, label = text "search:" }

                _ ->
                    EI.button sbs { onPress = Just SearchClick, label = text "search:" }

        tinput =
            EI.multiline
                (htmlAttribute (HA.id "searchtext") :: onFocus (STFocus True) :: onLoseFocus (STFocus False) :: tiattribs)
                { onChange = SearchText
                , text = model.searchText
                , placeholder = Nothing
                , spellcheck = False
                , label =
                    EI.labelHidden "search"
                }

        ddbutton =
            none

        {- save and restore search stuff, disabled for now:

           was ddbutton:
           EI.button buttonStyle
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
               EI.button buttonStyle
                   { onPress = Just SaveSearch
                   , label = text "save"
                   }
           ,
        -}
        obs =
            alignRight :: buttonStyle

        buttons =
            [ searchButton
            , EI.button obs
                { onPress = Just SearchDetails
                , label =
                    text <|
                        if model.showParse then
                            "-"

                        else
                            "?"
                }
            , EI.button obs
                { onPress = Just Clear
                , label = text "x"
                }
            ]

        showborder =
            model.showParse || narrow
    in
    -- keyed column retains focus on search field even if other controls are added dynamically
    EK.column
        (if showborder then
            [ padding 2
            , spacing 8
            , width fill
            ]

         else
            [ width fill
            , spacing 8
            ]
        )
        (( "viewsearch"
         , case model.search of
            TagSearch (Ok ts) ->
                viewSearch model.searchTermFocus ts

            _ ->
                E.none
         )
            :: ([ ( "tinput", row [ width fill, spacing 3 ] [ tinput ] )
                , ( "tbuttons", row [ spacing 3, width fill ] buttons )
                ]
                    ++ ( "searchhelp"
                       , if model.showParse then
                            case model.search of
                                TagSearch rts ->
                                    case rts of
                                        Err e ->
                                            column [ width fill ]
                                                [ row [ spacing 3, width fill ]
                                                    [ text "Syntax error:"
                                                    , paragraph [] [ text (Util.deadEndsToString e) ]
                                                    , el [ alignRight ] <| toggleHelpButton model.showHelp
                                                    ]
                                                , if model.showHelp then
                                                    E.map HelpMsg <| SearchHelpPanel.view nblevel model.helpPanel

                                                  else
                                                    E.none
                                                ]

                                        Ok ts ->
                                            column [ width fill ]
                                                [ paragraph [ spacing 3, width fill ]
                                                    [ text "search expression:"
                                                    , paragraph [] [ text <| Search.printTagSearch ts ]
                                                    , el [ alignRight ] <| toggleHelpButton model.showHelp
                                                    ]
                                                , if model.showHelp then
                                                    E.map HelpMsg <| SearchHelpPanel.view nblevel model.helpPanel

                                                  else
                                                    E.none
                                                ]

                                NoSearch ->
                                    E.map HelpMsg <|
                                        SearchHelpPanel.view nblevel model.helpPanel

                         else
                            E.none
                       )
                    :: []
               )
        )


toggleHelpButton : Bool -> Element Msg
toggleHelpButton showHelp =
    EI.button buttonStyle
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
            ( updateSearchText model (String.replace "\n" "" txt)
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

        ToggleAndOr tsl ->
            let
                ns =
                    case model.search of
                        TagSearch (Ok search) ->
                            search
                                |> SL.getTerm tsl
                                |> Maybe.andThen
                                    (\term ->
                                        case term of
                                            Boolex ts1 andor ts2 ->
                                                SL.setTerm tsl
                                                    (Boolex ts1
                                                        (case andor of
                                                            And ->
                                                                Or

                                                            Or ->
                                                                And
                                                        )
                                                        ts2
                                                    )
                                                    search
                                                    |> Maybe.map (TagSearch << Ok)

                                            _ ->
                                                Nothing
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch ns model
            , None
            )

        NotTerm tsl ->
            let
                ns =
                    case model.search of
                        TagSearch (Ok search) ->
                            search
                                |> SL.getTerm tsl
                                |> Maybe.andThen
                                    (\term ->
                                        SL.setTerm tsl (Not term) search
                                            |> Maybe.map (TagSearch << Ok)
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch ns model
            , None
            )

        ToggleTermFocus tsl ->
            ( { model
                | searchTermFocus =
                    if model.searchTermFocus == Just tsl then
                        Nothing

                    else
                        Just tsl
              }
            , None
            )

        DeleteTerm tsl ->
            let
                ns =
                    case model.search of
                        TagSearch (Ok search) ->
                            case SL.removeTerm tsl search of
                                SL.Matched ->
                                    NoSearch

                                SL.Removed s ->
                                    TagSearch (Ok s)

                                SL.Unmatched ->
                                    model.search

                        _ ->
                            model.search
            in
            ( setSearch ns model
            , None
            )

        ToggleSearchMod tsl mod ->
            let
                ns =
                    case model.search of
                        TagSearch (Ok search) ->
                            search
                                |> SL.getTerm tsl
                                |> Maybe.andThen
                                    (\term ->
                                        case term of
                                            SearchTerm mods str ->
                                                let
                                                    nmods =
                                                        if List.member mod mods then
                                                            List.filter (\i -> i /= mod) mods

                                                        else
                                                            mod :: mods
                                                in
                                                SL.setTerm tsl (SearchTerm nmods str) search
                                                    |> Maybe.map (\s -> TagSearch (Ok s))

                                            _ ->
                                                Nothing
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch ns model
            , None
            )

        SetTermText tsl nstr ->
            let
                ns =
                    case model.search of
                        TagSearch (Ok search) ->
                            search
                                |> SL.getTerm tsl
                                |> Maybe.andThen
                                    (\term ->
                                        case term of
                                            SearchTerm mods str ->
                                                SL.setTerm tsl (SearchTerm mods nstr) search
                                                    |> Maybe.map (\s -> TagSearch (Ok s))

                                            _ ->
                                                Nothing
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch ns model
            , None
            )

        AddEmptyTerm tsl ->
            ( { model
                | search = addToSearch (Just tsl) [] "" model.search
                , searchTermFocus = Just <| SL.swapLast tsl (LBT2 LThis)
              }
            , None
            )
