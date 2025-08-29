module TagSearchPanel exposing
    ( Command(..)
    , Model
    , Msg(..)
    , Search(..)
    , addSearchText
    , addTagToSearchPrev
    , addToSearch
    , addToSearchPanel
    , getSearch
    , initModel
    , onEnter
    , onOrdering
    , selectPrevSearch
    , setSearch
    , toggleHelpButton
    , update
    , updateSearchText
    , view
    )

-- import Search exposing (AndOr(..), SearchMod(..), TSText, TagSearch(..), showSearchMod, tagSearchParser)

import Common exposing (buttonStyle)
import Data exposing (AndOr(..), ArchivesOrCurrent(..), Ordering, SearchMod(..), TagSearch(..))
import DataUtil exposing (OrderedTagSearch)
import Element as E exposing (..)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events exposing (onClick, onFocus, onLoseFocus)
import Element.Font as EF
import Element.Input as EI
import Element.Keyed as EK
import Html.Attributes as HA
import Parser
import SearchHelpPanel
import SearchLoc as SL exposing (TSLoc(..))
import SearchUtil exposing (printTagSearch, showSearchMod, tagSearchParser)
import TangoColors as TC
import Util


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
    , searchOnEnter : Bool
    , searchTermFocus : Maybe TSLoc
    , ordering : Maybe Ordering
    , archives : ArchivesOrCurrent
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
    , searchOnEnter = False
    , searchTermFocus = Nothing
    , ordering = Nothing
    , archives = Current
    }


type Msg
    = SearchText String
    | STFocus Bool
    | SearchDetails
    | SearchClick
    | ArchiveClick
    | SyncFilesClick
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
    | AddToStackClicked
    | CopyClicked


type Command
    = None
    | Save
    | Search OrderedTagSearch
    | SyncFiles TagSearch
    | AddToStack
    | Copy String


getSearch : Model -> Maybe OrderedTagSearch
getSearch model =
    case model.search of
        TagSearch (Ok s) ->
            Just { ts = s, ordering = model.ordering, archives = model.archives }

        NoSearch ->
            Just { ts = SearchTerm { mods = [], term = "" }, ordering = model.ordering, archives = model.archives }

        TagSearch (Err _) ->
            Nothing


addToSearch : Maybe TSLoc -> List SearchMod -> String -> Search -> Search
addToSearch mbtsloc searchmods name search =
    let
        term =
            SearchTerm
                { mods = searchmods
                , term = String.replace "'" "\\'" name
                }
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
                                SL.setTerm tsloc (Boolex { ts1 = tm, ao = And, ts2 = term }) s
                            )
                        |> Maybe.map (\ts -> TagSearch (Ok ts))
                        |> Maybe.withDefault
                            (TagSearch (Ok (Boolex { ts1 = s, ao = And, ts2 = term })))

                Nothing ->
                    TagSearch (Ok (Boolex { ts1 = s, ao = And, ts2 = term }))


addSearch : Search -> Search -> Search
addSearch ls rs =
    case ls of
        NoSearch ->
            rs

        TagSearch (Err _) ->
            rs

        TagSearch (Ok sl) ->
            case rs of
                NoSearch ->
                    ls

                TagSearch (Err _) ->
                    ls

                TagSearch (Ok sr) ->
                    TagSearch (Ok (Boolex { ts1 = sl, ao = And, ts2 = sr }))


setSearch : Model -> Search -> Model
setSearch model s =
    case s of
        TagSearch (Ok ts) ->
            { model
                | search = s
                , searchText = printTagSearch ts
            }

        NoSearch ->
            { model
                | search = s
                , searchText = ""
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
                , searchText = printTagSearch ts
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
                TagSearch
                    (Ok
                        (txt
                            |> String.split " "
                            |> List.map (\t -> Data.SearchTerm { mods = [], term = t })
                            |> (\terms ->
                                    case terms of
                                        first :: rest ->
                                            List.foldr (\t s -> Data.Boolex { ts1 = t, ao = Data.And, ts2 = s }) first rest

                                        [] ->
                                            Data.SearchTerm { mods = [], term = "" }
                               )
                        )
                    )
    }


addSearchText : Model -> String -> Model
addSearchText model txt =
    let
        addsearch =
            if String.contains "'" txt then
                TagSearch <| Parser.run tagSearchParser txt

            else
                TagSearch <| Ok <| Data.SearchTerm { mods = [], term = txt }

        nsearch =
            addSearch addsearch model.search
    in
    case nsearch of
        TagSearch (Ok ts) ->
            { model
                | searchText = printTagSearch ts
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
            E.row [ E.width (E.px (8 * indent)) ] []

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
        SearchTerm { mods, term } ->
            let
                tloc =
                    toLoc lts LThis

                downButtonStyle =
                    buttonStyle ++ [ EBk.color TC.grey ]

                modbutton =
                    \mod label ->
                        EI.button
                            (if List.member mod mods then
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
                                    (showSearchMod mod)
                            }
            in
            [ E.row [ E.width E.fill, E.spacing 8 ]
                [ indentelt
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
                            , modbutton File "f"
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
                                [ onFocus (STFocus True)
                                , onLoseFocus (STFocus False)
                                , E.width E.fill
                                ]
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
                        ((E.el
                            [ color tloc
                            ]
                          <|
                            E.text term
                         )
                            :: List.map modindicator mods
                        )
                ]
            ]

        Not nts ->
            let
                tloc =
                    toLoc lts LThis
            in
            E.row [ E.width E.fill, E.spacing 8 ]
                [ indentelt
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
                :: viewSearchHelper mbfocusloc (indent + 1) (LNot :: lts) nts.ts

        Boolex { ts1, ao, ts2 } ->
            let
                tloc =
                    toLoc lts LThis
            in
            viewSearchHelper mbfocusloc (indent + 1) (LBT1 :: lts) ts1
                ++ E.row [ E.width E.fill, E.spacing 8 ]
                    [ indentelt
                    , if hasfocus tloc then
                        E.row [ EBk.color TC.lightGrey, E.paddingXY 0 8, E.spacing 8, E.width E.fill ]
                            [ E.el
                                [ onClick <| ToggleTermFocus tloc
                                , color tloc
                                ]
                              <|
                                E.text
                                    (case ao of
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
                                        (case ao of
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
                                    (case ao of
                                        And ->
                                            "and"

                                        Or ->
                                            "or"
                                    )
                            ]
                    ]
                :: viewSearchHelper mbfocusloc (indent + 1) (LBT2 :: lts) ts2


view : Bool -> Bool -> Int -> Model -> Element Msg
view showCopy narrow nblevel model =
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
                    EI.button (sbs ++ [ EBk.color TC.grey ]) { onPress = Nothing, label = text "search" }

                _ ->
                    EI.button sbs { onPress = Just SearchClick, label = text "search" }

        archiveButton =
            case model.search of
                TagSearch (Err _) ->
                    E.none

                _ ->
                    EI.button (sbs ++ [ E.alignRight ]) { onPress = Just ArchiveClick, label = text "A" }

        fileSyncButton =
            case model.search of
                TagSearch (Err _) ->
                    EI.button (sbs ++ [ EBk.color TC.grey, E.alignRight ]) { onPress = Nothing, label = text "search" }

                _ ->
                    EI.button (sbs ++ [ E.alignRight ]) { onPress = Just SyncFilesClick, label = text "FS" }

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

        orderingRow =
            let
                ord =
                    model.ordering
                        |> Maybe.map
                            (\o ->
                                E.text <|
                                    (case o.field of
                                        Data.Title ->
                                            "title"

                                        Data.Created ->
                                            "created"

                                        Data.Changed ->
                                            "changed"
                                    )
                                        ++ " "
                                        ++ (case o.direction of
                                                Data.Ascending ->
                                                    "ascending"

                                                Data.Descending ->
                                                    "descending"
                                           )
                            )

                arch =
                    case model.archives of
                        Current ->
                            Nothing

                        Archives ->
                            Just <| E.text "archives"

                        CurrentAndArchives ->
                            Just <| E.text "current + archives"
            in
            case ( ord, arch ) of
                ( Nothing, Nothing ) ->
                    E.none

                ( l, r ) ->
                    E.row [ E.width E.fill ]
                        [ Maybe.withDefault E.none l
                        , r |> Maybe.map (\t -> E.el [ E.alignRight ] t) |> Maybe.withDefault E.none
                        ]

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
            , archiveButton
            , fileSyncButton
            , if showCopy then
                EI.button (E.alignRight :: buttonStyle)
                    { label = E.text "<"
                    , onPress = Just CopyClicked
                    }

              else
                E.none
            , EI.button obs
                { onPress = Just SearchDetails
                , label =
                    E.el [ EF.family [ EF.monospace ] ] <|
                        text <|
                            if model.showParse then
                                "-"

                            else
                                "?"
                }
            , EI.button obs
                { onPress = Just Clear
                , label =
                    E.el [ EF.family [ EF.monospace ] ] <|
                        text "x"
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
        [ ( "addbutton"
          , row [ width fill ]
                [ EI.button (height (px 19) :: E.centerX :: buttonStyle)
                    { label =
                        E.el [ EF.family [ EF.monospace ] ] <|
                            E.text "^"
                    , onPress = Just AddToStackClicked
                    }
                ]
          )
        , ( "viewsearch"
          , case model.search of
                TagSearch (Ok ts) ->
                    viewSearch model.searchTermFocus ts

                _ ->
                    E.none
          )
        , ( "tinput"
          , row [ width fill, spacing 3 ]
                [ tinput
                ]
          )
        , ( "orderingRow", orderingRow )
        , ( "tbuttons", row [ spacing 3, width fill ] buttons )
        , ( "searchhelp"
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
                                        , paragraph [] [ text <| printTagSearch ts ]
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
        ]


toggleHelpButton : Bool -> Element Msg
toggleHelpButton showHelp =
    EI.button buttonStyle
        { onPress = Just ToggleHelp
        , label =
            if showHelp then
                text "hide search help"

            else
                text "show search help"
        }


onEnter : Model -> ( Model, Command )
onEnter model =
    if model.searchOnEnter then
        doSearchClick model

    else
        ( model, None )


onOrdering : Maybe Ordering -> Model -> ( Model, Command )
onOrdering ordering model =
    ( { model | ordering = ordering }, None )


doSearchClick : Model -> ( Model, Command )
doSearchClick model =
    ( model
    , case getSearch model of
        Just s ->
            Search s

        Nothing ->
            None
    )


doFileSyncClick : Model -> ( Model, Command )
doFileSyncClick model =
    ( model
    , case getSearch model of
        Just s ->
            SyncFiles s.ts

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
            ( { model | searchOnEnter = focused }, None )

        Clear ->
            ( { model
                | searchText = ""
                , search = NoSearch
                , ordering = Nothing
                , archives = Current
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

        SyncFilesClick ->
            doFileSyncClick model

        ArchiveClick ->
            ( { model
                | archives =
                    case model.archives of
                        Current ->
                            Archives

                        Archives ->
                            CurrentAndArchives

                        CurrentAndArchives ->
                            Current
              }
            , None
            )

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
                                            Boolex { ts1, ao, ts2 } ->
                                                SL.setTerm tsl
                                                    (Boolex
                                                        { ts1 = ts1
                                                        , ao =
                                                            case ao of
                                                                And ->
                                                                    Or

                                                                Or ->
                                                                    And
                                                        , ts2 =
                                                            ts2
                                                        }
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
            ( setSearch model ns
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
                                        SL.setTerm tsl (Not { ts = term }) search
                                            |> Maybe.map (TagSearch << Ok)
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch model ns
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
            ( setSearch model ns
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
                                            SearchTerm st ->
                                                let
                                                    nmods =
                                                        if List.member mod st.mods then
                                                            List.filter (\i -> i /= mod) st.mods

                                                        else
                                                            mod :: st.mods
                                                in
                                                SL.setTerm tsl (SearchTerm { mods = nmods, term = st.term }) search
                                                    |> Maybe.map (\s -> TagSearch (Ok s))

                                            _ ->
                                                Nothing
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch model ns
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
                                            SearchTerm { mods } ->
                                                SL.setTerm tsl (SearchTerm { mods = mods, term = nstr }) search
                                                    |> Maybe.map (\s -> TagSearch (Ok s))

                                            _ ->
                                                Nothing
                                    )
                                |> Maybe.withDefault model.search

                        _ ->
                            model.search
            in
            ( setSearch model ns
            , None
            )

        AddEmptyTerm tsl ->
            ( { model
                | search = addToSearch (Just tsl) [] "" model.search
                , searchTermFocus = Just <| SL.swapLast tsl (LBT2 LThis)
              }
            , None
            )

        AddToStackClicked ->
            ( model, AddToStack )

        CopyClicked ->
            ( model, Copy model.searchText )
