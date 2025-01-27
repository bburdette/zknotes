module SearchUtil exposing (..)

import Data exposing (AndOr(..), ResultType(..), SearchMod(..), TagSearch(..), ZkNoteSearch)
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
        , succeed
        , symbol
        , token
        )
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


encodeResultType : ResultType -> JE.Value
encodeResultType smod =
    case smod of
        RtId ->
            JE.string "RtId"

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

        Before ->
            JE.string "Before"

        After ->
            JE.string "After"

        Create ->
            JE.string "Create"

        Mod ->
            JE.string "Mod"


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

                    "Before" ->
                        JD.succeed Before

                    "After" ->
                        JD.succeed After

                    "Create" ->
                        JD.succeed Create

                    "Mod" ->
                        JD.succeed Mod

                    wat ->
                        JD.fail <| "invalid search mod: " ++ wat
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

        Before ->
            "Before"

        After ->
            "After"

        Create ->
            "Create"

        Mod ->
            "Mod"


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
