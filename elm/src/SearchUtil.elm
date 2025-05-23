module SearchUtil exposing (..)

import Data exposing (AndOr(..), ResultType(..), SearchMod(..), TagSearch(..), ZkNoteSearch)
import ParseHelp exposing (listOf)
import Parser
    exposing
        ( (|.)
        , (|=)
        , DeadEnd
        , Parser
        , Step(..)
        , andThen
        , chompIf
        , chompWhile
        , getChompedString
        , lazy
        , loop
        , map
        , oneOf
        , succeed
        , symbol
        , token
        )
import Time
import UUID exposing (UUID)
import Util exposing (rest)


andifySearches : List TagSearch -> TagSearch
andifySearches tsl =
    case tsl of
        s :: rest ->
            List.foldr (\sl sr -> Boolex { ts1 = sl, ao = And, ts2 = sr }) s rest

        [] ->
            SearchTerm { mods = [], term = "" }


defaultSearchLimit : Int
defaultSearchLimit =
    25


defaultSearch : ZkNoteSearch
defaultSearch =
    { tagsearch = [ SearchTerm { mods = [], term = "" } ]
    , offset = 0
    , limit = Just defaultSearchLimit
    , what = ""
    , resulttype = RtListNote
    , archives = False
    , deleted = False
    , ordering = Nothing
    }


showSearchMod : SearchMod -> String
showSearchMod mod =
    case mod of
        ExactMatch ->
            "ExactMatch"

        ZkNoteId ->
            "ZkNoteId"

        Tag ->
            "Tag"

        Note ->
            "Note"

        User ->
            "User"

        File ->
            "File"

        Before ->
            "Before"

        After ->
            "After"

        Create ->
            "Create"

        Mod ->
            "Mod"

        Server ->
            "Server"


showAndOr : AndOr -> String
showAndOr ao =
    case ao of
        And ->
            "and"

        Or ->
            "or"


type SemanticError
    = InvalidDateFormat String
    | InvalidDateMods String
    | InvalidServerMods String
    | InvalidUuid String


tagSearchDates : Time.Zone -> TagSearch -> Result SemanticError TagSearch
tagSearchDates tz ts =
    case ts of
        SearchTerm x ->
            termSemanticCheck tz x
                |> Result.map
                    SearchTerm

        Not x ->
            case tagSearchDates tz x.ts of
                Ok tsd ->
                    Ok <| Not { ts = tsd }

                Err de ->
                    Err de

        Boolex x ->
            case ( tagSearchDates tz x.ts1, tagSearchDates tz x.ts2 ) of
                ( Ok t1, Ok t2 ) ->
                    Ok <| Boolex { ts1 = t1, ao = x.ao, ts2 = t2 }

                ( Err e1, _ ) ->
                    Err e1

                ( _, Err e2 ) ->
                    Err e2


type alias ST =
    { mods : List SearchMod, term : String }


termSemanticCheck : Time.Zone -> ST -> Result SemanticError ST
termSemanticCheck tz st =
    let
        isdateterm =
            Util.trueforany
                (\m ->
                    case m of
                        Before ->
                            True

                        After ->
                            True

                        Create ->
                            True

                        Mod ->
                            True

                        _ ->
                            False
                )
                st.mods

        isvaliddateterm =
            isdateterm
                && List.length
                    (List.filter
                        (\m ->
                            case m of
                                Before ->
                                    True

                                After ->
                                    True

                                _ ->
                                    False
                        )
                        st.mods
                    )
                == 1
                && List.length
                    (List.filter
                        (\m ->
                            case m of
                                Create ->
                                    True

                                Mod ->
                                    True

                                _ ->
                                    False
                        )
                        st.mods
                    )
                == 1
    in
    if Util.trueforany ((==) Server) st.mods then
        if List.length st.mods /= 1 then
            Err <| InvalidServerMods <| "no mods other than ! allowed with (s)erver: " ++ (showTagSearch <| SearchTerm st)

        else if st.term == "local" then
            Ok st

        else
            case UUID.fromString st.term of
                Ok _ ->
                    Ok st

                Err _ ->
                    Err <| InvalidUuid st.term

    else if isdateterm then
        if not isvaliddateterm then
            Err <|
                InvalidDateMods <|
                    String.concat (List.map printSearchMod st.mods)
                        ++ "'"
                        ++ st.term
                        ++ "'"

        else
            -- either term should be a number string, or a standard datetime.
            case String.toInt st.term of
                Just _ ->
                    Ok st

                Nothing ->
                    case Util.parseTime tz st.term of
                        Ok (Just t) ->
                            Ok { mods = st.mods, term = String.fromInt (Time.posixToMillis t) }

                        Ok Nothing ->
                            Err <| InvalidDateFormat st.term

                        Err e ->
                            case Util.parseDate tz st.term of
                                Ok (Just t) ->
                                    Ok { mods = st.mods, term = String.fromInt (Time.posixToMillis t) }

                                Ok Nothing ->
                                    Err <| InvalidDateFormat st.term

                                Err _ ->
                                    Err <| InvalidDateFormat st.term

    else
        Ok st


showTagSearch : TagSearch -> String
showTagSearch ts =
    case ts of
        SearchTerm st ->
            "(searchterm " ++ String.concat (List.intersperse " " (List.map showSearchMod st.mods)) ++ " '" ++ st.term ++ "')"

        Not ts1 ->
            "not " ++ showTagSearch ts1.ts

        Boolex { ts1, ao, ts2 } ->
            " ( " ++ showTagSearch ts1 ++ " " ++ showAndOr ao ++ " " ++ showTagSearch ts2 ++ " ) "


printSearchMod : SearchMod -> String
printSearchMod mod =
    case mod of
        ExactMatch ->
            "e"

        ZkNoteId ->
            "z"

        Tag ->
            "t"

        Note ->
            "n"

        User ->
            "u"

        File ->
            "f"

        Before ->
            "b"

        After ->
            "a"

        Create ->
            "c"

        Mod ->
            "m"

        Server ->
            "s"


printAndOr : AndOr -> String
printAndOr ao =
    case ao of
        And ->
            "&"

        Or ->
            "|"


printTagSearch : TagSearch -> String
printTagSearch ts =
    case ts of
        SearchTerm { mods, term } ->
            String.concat (List.map printSearchMod mods) ++ "'" ++ term ++ "'"

        Not ts1 ->
            "!" ++ printTagSearch ts1.ts

        Boolex { ts1, ao, ts2 } ->
            "(" ++ printTagSearch ts1 ++ " " ++ printAndOr ao ++ " " ++ printTagSearch ts2 ++ ")"


searchMod : Parser SearchMod
searchMod =
    oneOf
        [ succeed ExactMatch
            |. symbol "e"
        , succeed ZkNoteId
            |. symbol "z"
        , succeed Tag
            |. symbol "t"
        , succeed Note
            |. symbol "n"
        , succeed User
            |. symbol "u"
        , succeed File
            |. symbol "f"
        , succeed File
            |. symbol "f"
        , succeed Before
            |. symbol "b"
        , succeed After
            |. symbol "a"
        , succeed Create
            |. symbol "c"
        , succeed Mod
            |. symbol "m"
        , succeed Server
            |. symbol "s"
        ]


searchMods : Parser (List SearchMod)
searchMods =
    listOf searchMod


searchTerm : Parser String
searchTerm =
    succeed identity
        |. symbol "'"
        |= loop [] termHelp


termHelp : List String -> Parser (Step (List String) String)
termHelp revChunks =
    oneOf
        [ succeed (Loop ("'" :: revChunks))
            |. token "\\'"
        , token "'"
            |> map (\_ -> Done (String.join "" (List.reverse revChunks)))
        , chompWhile isUninteresting
            |> getChompedString
            |> map
                (\chunk ->
                    case chunk of
                        "" ->
                            -- prevent infinite loop!
                            Done (String.join "" (List.reverse revChunks))

                        _ ->
                            Loop (chunk :: revChunks)
                )
        ]


isUninteresting : Char -> Bool
isUninteresting char =
    char /= '\\' && char /= '\''


spaces : Parser ()
spaces =
    succeed ()
        |. chompWhile (\char -> char == ' ')


andor : Parser AndOr
andor =
    oneOf
        [ succeed And |. symbol "&"
        , succeed Or |. symbol "|"
        ]


tagSearchParser : Parser TagSearch
tagSearchParser =
    singleTerm
        |> andThen
            (\initterm ->
                succeed (\opterms -> List.foldl (\( op, term ) exp -> Boolex { ts1 = exp, ao = op, ts2 = term }) initterm opterms)
                    |= oplistParser
            )


oplistParser : Parser (List ( AndOr, TagSearch ))
oplistParser =
    listOf <|
        succeed (\a b -> ( a, b ))
            |. spaces
            |= andor
            |. spaces
            |= lazy (\_ -> singleTerm)


singleTerm : Parser TagSearch
singleTerm =
    oneOf
        [ succeed (\m t -> SearchTerm { mods = m, term = t })
            |= searchMods
            |= searchTerm
        , succeed (\t -> Not { ts = t })
            |. symbol "!"
            |. spaces
            |= lazy (\_ -> singleTerm)
        , succeed identity
            |. symbol "("
            |. spaces
            |= lazy (\_ -> tagSearchParser)
            |. spaces
            |. symbol ")"
        ]


type FieldText
    = FText String
    | Field String


fieldString : Parser FieldText
fieldString =
    succeed Field
        |. symbol "<"
        |= (getChompedString <|
                succeed ()
                    |. chompWhile (\c -> c /= '>')
           )
        |. symbol ">"


fieldText : Parser FieldText
fieldText =
    succeed FText
        |= (getChompedString <|
                succeed ()
                    |. chompIf (\c -> c /= '<')
                    |. chompWhile (\c -> c /= '<')
           )


fields : Parser (List FieldText)
fields =
    listOf (oneOf [ fieldText, fieldString ])


type TSText
    = Text String
    | Search TagSearch


extractTagSearches : String -> Result (List DeadEnd) (List TSText)
extractTagSearches text =
    case Parser.run fields text of
        Err meh ->
            Err meh

        Ok fieldtext ->
            let
                searchntext : List (Result (List DeadEnd) TSText)
                searchntext =
                    List.foldr
                        (\i l ->
                            case i of
                                Field s ->
                                    if s == "" then
                                        Ok (Search (SearchTerm { mods = [], term = "" })) :: l

                                    else
                                        case Parser.run tagSearchParser s of
                                            Err e ->
                                                Err e :: l

                                            Ok srch ->
                                                Ok (Search srch) :: l

                                FText t ->
                                    Ok (Text t) :: l
                        )
                        []
                        fieldtext
            in
            List.foldr
                (\i res ->
                    case res of
                        Err e ->
                            Err e

                        Ok l ->
                            case i of
                                Err e ->
                                    Err e

                                Ok s_or_t ->
                                    Ok <| s_or_t :: l
                )
                (Ok [])
                searchntext
