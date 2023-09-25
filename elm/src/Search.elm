module Search exposing
    ( AndOr(..)
    , FieldText(..)
    , ResultType(..)
    , SearchMod(..)
    , TSText(..)
    , TagSearch(..)
    , ZkNoteSearch
    , andifySearches
    , andor
    , decodeAndOr
    , decodeSearchMod
    , decodeTagSearch
    , decodeTsl
    , defaultSearch
    , defaultSearchLimit
    , encodeAndOr
    , encodeSearchMod
    , encodeTagSearch
    , encodeTsl
    , encodeZkNoteSearch
    , extractTagSearches
    , fieldString
    , fieldText
    , fields
    , getTagSearch
    , oplistParser
    , printAndOr
    , printSearchMod
    , printTagSearch
    , searchMod
    , searchMods
    , searchTerm
    , showAndOr
    , showSearchMod
    , showTagSearch
    , singleTerm
    , spaces
    , tagSearchParser
    )

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
        , loop
        , map
        , oneOf
        , run
        , succeed
        , symbol
        , token
        )
import TDict exposing (TDict)
import Util exposing (first, rest)


type alias ZkNoteSearch =
    { tagSearch : List TagSearch
    , offset : Int
    , limit : Maybe Int
    , what : String
    , resultType : ResultType
    }


type ResultType
    = RtListNote
    | RtNote
    | RtNoteAndLinks


type SearchMod
    = ExactMatch
    | ZkNoteId
    | Tag
    | Note
    | User
    | File


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


getTagSearch : ZkNoteSearch -> TagSearch
getTagSearch zkn =
    andifySearches zkn.tagSearch


andifySearches : List TagSearch -> TagSearch
andifySearches tsl =
    case tsl of
        s :: rest ->
            List.foldr (\sl sr -> Boolex sl And sr) s rest

        [] ->
            SearchTerm [] ""


defaultSearchLimit : Int
defaultSearchLimit =
    25


defaultSearch : ZkNoteSearch
defaultSearch =
    { tagSearch = [ SearchTerm [] "" ]
    , offset = 0
    , limit = Just defaultSearchLimit
    , what = ""
    , resultType = RtListNote
    }


encodeResultType : ResultType -> JE.Value
encodeResultType smod =
    case smod of
        RtListNote ->
            JE.string "RtListNote"

        RtNote ->
            JE.string "RtNote"

        RtNoteAndLinks ->
            JE.string "RtNoteAndLinks"


encodeSearchMod : SearchMod -> JE.Value
encodeSearchMod smod =
    case smod of
        ExactMatch ->
            JE.string "ExactMatch"

        ZkNoteId ->
            JE.string "ZkNoteId"

        Tag ->
            JE.string "Tag"

        Note ->
            JE.string "Note"

        User ->
            JE.string "User"

        File ->
            JE.string "File"


decodeSearchMod : JD.Decoder SearchMod
decodeSearchMod =
    JD.string
        |> JD.andThen
            (\s ->
                case s of
                    "ExactMatch" ->
                        JD.succeed ExactMatch

                    "ZkNoteId" ->
                        JD.succeed ZkNoteId

                    "Tag" ->
                        JD.succeed Tag

                    "Note" ->
                        JD.succeed Note

                    "User" ->
                        JD.succeed User

                    "File" ->
                        JD.succeed File

                    wat ->
                        JD.fail <| "invalid search mod: " ++ wat
            )


decodeTagSearch : JD.Decoder TagSearch
decodeTagSearch =
    JD.oneOf
        [ JD.field "Not" (JD.map Not (JD.field "ts" (JD.lazy (\_ -> decodeTagSearch))))
        , JD.field "Boolex"
            (JD.map3 Boolex
                (JD.field "ts1" (JD.lazy (\_ -> decodeTagSearch)))
                (JD.field "ao" decodeAndOr)
                (JD.field "ts2" (JD.lazy (\_ -> decodeTagSearch)))
            )
        , JD.field "SearchTerm"
            (JD.map2 SearchTerm
                (JD.field "mods" (JD.list decodeSearchMod))
                (JD.field "term" JD.string)
            )
        ]


encodeTsl : List TagSearch -> JE.Value
encodeTsl ts =
    JE.object
        [ ( "searches", JE.list encodeTagSearch ts ) ]


decodeTsl : JD.Decoder (List TagSearch)
decodeTsl =
    JD.oneOf
        [ JD.field "searches" (JD.list decodeTagSearch)
        , decodeTagSearch |> JD.map List.singleton
        ]


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
                          , encodeAndOr ao
                          )
                        , ( "ts2", encodeTagSearch ts2 )
                        ]
                  )
                ]


encodeAndOr : AndOr -> JE.Value
encodeAndOr ao =
    JE.string
        (case ao of
            And ->
                "And"

            Or ->
                "Or"
        )


decodeAndOr : JD.Decoder AndOr
decodeAndOr =
    JD.string
        |> JD.andThen
            (\s ->
                case s of
                    "And" ->
                        JD.succeed And

                    "Or" ->
                        JD.succeed Or

                    wat ->
                        JD.fail <| "invalid and/or: " ++ wat
            )


encodeZkNoteSearch : ZkNoteSearch -> JE.Value
encodeZkNoteSearch zns =
    JE.object <|
        [ ( "tagsearch", encodeTagSearch (andifySearches zns.tagSearch) )
        , ( "offset", JE.int zns.offset )
        , ( "what", JE.string zns.what )
        , ( "resulttype", encodeResultType zns.resultType )
        ]
            ++ (zns.limit
                    |> Maybe.map (\i -> [ ( "limit", JE.int i ) ])
                    |> Maybe.withDefault []
               )


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
