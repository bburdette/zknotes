module SearchParser exposing (AndOr(..), FieldText(..), SearchMod(..), TSText(..), TagSearch(..), andor, encodeSearchMod, encodeTagSearch, extractTagSearches, fieldString, fieldText, fields, oplistParser, printAndOr, printSearchMod, printTagSearch, searchMod, searchMods, searchTerm, showAndOr, showSearchMod, showTagSearch, singleTerm, spaces, tagSearchParser)

import Json.Decode as JD
import Json.Encode as JE
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
        , map
        , oneOf
        , run
        , succeed
        , symbol
        )
import TDict exposing (TDict)
import Util exposing (first, rest)


type alias ZkNoteSearch =
    { tagsearch : TagSearch
    , zks : List Int
    }


type SearchMod
    = ExactMatch
    | Tag
    | Note


type TagSearch
    = SearchTerm (List SearchMod) String
    | Not TagSearch
    | Boolex TagSearch AndOr TagSearch


type AndOr
    = And
    | Or


type TSText
    = Text String
    | Search TagSearch


encodeSearchMod : SearchMod -> JE.Value
encodeSearchMod smod =
    case smod of
        ExactMatch ->
            JE.string "ExactMatch"

        Tag ->
            JE.string "Tag"

        Note ->
            JE.string "Note"


encodeTagSearch : TagSearch -> JE.Value
encodeTagSearch ts =
    case ts of
        SearchTerm smods termstr ->
            JE.object
                [ ( "SearchTerm"
                  , JE.object
                        [ ( "mods", JE.list encodeSearchMod smods )
                        , ( "term", JE.string termstr )
                        ]
                  )
                ]

        Not nts ->
            JE.object
                [ ( "Not"
                  , JE.object
                        [ ( "ts", encodeTagSearch nts )
                        ]
                  )
                ]

        Boolex ts1 ao ts2 ->
            JE.object
                [ ( "Boolex"
                  , JE.object
                        [ ( "ts1", encodeTagSearch ts1 )
                        , ( "ao"
                          , JE.string
                                (case ao of
                                    And ->
                                        "And"

                                    Or ->
                                        "Or"
                                )
                          )
                        , ( "ts2", encodeTagSearch ts2 )
                        ]
                  )
                ]


showSearchMod : SearchMod -> String
showSearchMod mod =
    case mod of
        ExactMatch ->
            "ExactMatch"

        Tag ->
            "Tag"

        Note ->
            "Note"


showAndOr : AndOr -> String
showAndOr ao =
    case ao of
        And ->
            "and"

        Or ->
            "or"


showTagSearch : TagSearch -> String
showTagSearch ts =
    case ts of
        SearchTerm modset s ->
            "(searchterm " ++ String.concat (List.intersperse " " (List.map showSearchMod modset)) ++ " '" ++ s ++ "')"

        Not ts1 ->
            "not " ++ showTagSearch ts1

        Boolex ts1 ao ts2 ->
            " ( " ++ showTagSearch ts1 ++ " " ++ showAndOr ao ++ " " ++ showTagSearch ts2 ++ " ) "


printSearchMod : SearchMod -> String
printSearchMod mod =
    case mod of
        ExactMatch ->
            "e"

        Tag ->
            "t"

        Note ->
            "n"


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
        SearchTerm modset s ->
            String.concat (List.map printSearchMod modset) ++ "'" ++ s ++ "'"

        Not ts1 ->
            "!" ++ printTagSearch ts1

        Boolex ts1 ao ts2 ->
            "(" ++ printTagSearch ts1 ++ " " ++ printAndOr ao ++ " " ++ printTagSearch ts2 ++ ")"


searchMod : Parser SearchMod
searchMod =
    oneOf
        [ succeed ExactMatch
            |. symbol "e"
        , succeed Tag
            |. symbol "t"
        , succeed Note
            |. symbol "n"
        ]


searchMods : Parser (List SearchMod)
searchMods =
    listOf searchMod


searchTerm : Parser String
searchTerm =
    succeed identity
        |. symbol "'"
        |= (getChompedString <|
                succeed ()
                    |. chompWhile (\c -> c /= '\'')
           )
        |. symbol "'"


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
                succeed (\opterms -> List.foldl (\( op, term ) exp -> Boolex exp op term) initterm opterms)
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
        [ succeed SearchTerm
            |= searchMods
            |= searchTerm
        , succeed Not
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
                                        Ok (Search (SearchTerm [] "")) :: l

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
