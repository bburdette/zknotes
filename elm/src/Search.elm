module Search exposing (..)

import Array
import ItemStuff exposing (ItemIndexer, ItemStuff)
import Parser
import SearchParser
    exposing
        ( AndOr(..)
        , TSText
        , TagSearch
        )
import TSet exposing (TSet)
import Tag exposing (Tag, TagId, tagByName, tagNames, tagSetParents)
import Util exposing (first, rest)


type TSResult item
    = TsrText String
    | TsrItems (List item)


type Search
    = TagSearch (Result (List Parser.DeadEnd) TagSearch)
    | NoSearch


type ItemSearchMod
    = CaseSensitive
    | ExactMatch
    | Description


type TagSearchMod
    = Parent
    | Ancestor


type ActualTagSearch
    = ItemSearchTerm (List ItemSearchMod) String
    | TagSearchTerm (List TagSearchMod) (List TagId)
    | Not ActualTagSearch
    | Boolex ActualTagSearch AndOr ActualTagSearch


type TSText
    = Text String
    | Search ActualTagSearch


makeTSText : ItemStuff TagId Tag -> (TagId -> Maybe Tag) -> List Tag -> SearchParser.TSText -> TSText
makeTSText tagstuff getTagById tags tst =
    case tst of
        SearchParser.Text txt ->
            Text txt

        SearchParser.Search ts ->
            Search <| makeActualTagSearch tagstuff getTagById tags ts


makeItemSearchMods : List SearchParser.SearchMod -> List ItemSearchMod
makeItemSearchMods sms =
    List.foldl
        (\sm lst ->
            case sm of
                SearchParser.CaseSensitive ->
                    CaseSensitive :: lst

                SearchParser.ExactMatch ->
                    ExactMatch :: lst

                SearchParser.Description ->
                    Description :: lst

                SearchParser.ParentTag ->
                    lst

                SearchParser.AncestorTag ->
                    lst
        )
        []
        sms


makeTagSearchMods : List SearchParser.SearchMod -> List TagSearchMod
makeTagSearchMods sms =
    List.foldl
        (\sm lst ->
            case sm of
                SearchParser.CaseSensitive ->
                    lst

                SearchParser.ExactMatch ->
                    lst

                SearchParser.Description ->
                    lst

                SearchParser.ParentTag ->
                    Parent :: lst

                SearchParser.AncestorTag ->
                    Ancestor :: lst
        )
        []
        sms


makeActualTagSearch : ItemStuff TagId Tag -> (TagId -> Maybe Tag) -> List Tag -> TagSearch -> ActualTagSearch
makeActualTagSearch tagstuff getTagById tags tagsearch =
    case tagsearch of
        SearchParser.SearchTerm mods searchtext ->
            if List.member SearchParser.ParentTag mods || List.member SearchParser.AncestorTag mods then
                let
                    ts =
                        ItemSearchTerm (makeItemSearchMods mods) searchtext

                    tstuff =
                        tagstuff

                    -- get those tag ids!
                    tids =
                        List.map tstuff.getId <|
                            itemSearch ts getTagById tagstuff tags
                in
                TagSearchTerm (makeTagSearchMods mods) tids
            else
                ItemSearchTerm (makeItemSearchMods mods) searchtext

        SearchParser.Not st ->
            Not (makeActualTagSearch tagstuff getTagById tags st)

        SearchParser.Boolex ts1 ao ts2 ->
            Boolex (makeActualTagSearch tagstuff getTagById tags ts1)
                ao
                (makeActualTagSearch tagstuff getTagById tags ts2)


tsTextToTSResult :
    (TagId -> Maybe Tag)
    -> (String -> List Tag)
    -> ItemStuff itemid item
    -> List item
    -> List TSText
    -> List (TSResult item)
tsTextToTSResult getTagById getTagsByName itemstuff items tstx =
    List.map
        (\tst ->
            case tst of
                Text t ->
                    TsrText t

                Search ts ->
                    TsrItems <| itemSearch ts getTagById itemstuff items
        )
        tstx


itemMatch : ItemStuff itemid item -> item -> TSet TagId Int -> ActualTagSearch -> Bool
itemMatch itemstuff item ids search =
    let
        istuff =
            itemstuff
    in
    case search of
        ItemSearchTerm mods searchtext ->
            let
                comptxt =
                    if List.member Description mods then
                        istuff.getDescription item
                    else
                        istuff.getName item

                ( st, ctxt ) =
                    if List.member CaseSensitive mods then
                        ( searchtext, comptxt )
                    else
                        ( String.toLower searchtext, String.toLower comptxt )
            in
            if List.member ExactMatch mods then
                st == ctxt
            else
                String.contains st ctxt

        TagSearchTerm mods idlst ->
            if List.member Ancestor mods then
                Util.trueforany (\id -> TSet.member id ids) idlst
            else
                let
                    parentids =
                        istuff.getTags item
                in
                Util.trueforany (\id -> TSet.member id parentids) idlst

        Not ts ->
            not <| itemMatch itemstuff item ids ts

        Boolex tsl op tsr ->
            case op of
                And ->
                    (&&) (itemMatch itemstuff item ids tsl) (itemMatch itemstuff item ids tsr)

                Or ->
                    (||) (itemMatch itemstuff item ids tsl) (itemMatch itemstuff item ids tsr)


itemSearch :
    ActualTagSearch
    -> (TagId -> Maybe Tag)
    -> ItemStuff itemid item
    -> List item
    -> List item
itemSearch ts getTagById istuff items =
    -- more efficient to find childen of tags, then search all items for those?
    -- currently finding parents of all tags in all items.
    -- alternative:  find parents of all tags in all items, and store in the ItemIndex.
    -- or, in tag itemindex, find children of all tags, and keep same updated.
    -- for now just reimplement the current algo.
    let
        taggeditems =
            List.map
                (\i -> ( i, Tag.tagSetAndParents (istuff.getTags i) getTagById ))
                items
    in
    List.map Tuple.first <| List.filter (\( i, tids ) -> itemMatch istuff i tids ts) taggeditems


type alias SearchResulter itemid item iidxs tidxs searchargs result results =
    { buildSearchResults :
        searchargs
        -> ItemIndexer itemid item iidxs
        -> iidxs
        -> ItemIndexer TagId Tag tidxs
        -> tidxs
        -> SearchResults item
        -> SearchResults item
    , resultCount : results -> Int
    , resultRange : results -> Int -> Int -> List result
    , resultList : results -> List result
    }


type alias SearchResult item =
    { item : item
    , tagnames : List String
    , tagparentnames : List String
    }


type alias SearchResults item =
    { results : Array.Array (SearchResult item)
    , tidxsChangeId : Int
    , iidxsChangeId : Int
    , searchArgs : SearchArgs
    , searchMatches : List item
    , searchMatchCount : Int
    }


clearSearchResults : SearchResults item -> SearchResults item
clearSearchResults sr =
    { results = Array.fromList []
    , iidxsChangeId = -1
    , tidxsChangeId = -1
    , searchArgs = sr.searchArgs
    , searchMatches = []
    , searchMatchCount = 0
    }


textMatch : ItemStuff itemid item -> item -> String -> Bool
textMatch istuff item lowercasetext =
    -- text arg should be lower cased already!
    String.contains lowercasetext (String.toLower (istuff.getName item))
        || String.contains lowercasetext (String.toLower (istuff.getDescription item))


type alias SearchArgs =
    { showCount : Maybe Int
    , search : Search
    }


type ResultDifference
    = RsSearch -- search is different.
    | RsCount -- count is different
    | RsSame -- searches are the same.


rdgte : ResultDifference -> ResultDifference -> Bool
rdgte rsa rsb =
    if rsa == rsb then
        True
    else
        let
            order =
                [ RsSearch, RsCount, RsSame ]
        in
        case
            Util.first
                (\a ->
                    if a == rsa then
                        Just True
                    else if a == rsb then
                        Just False
                    else
                        Nothing
                )
                order
        of
            Just r ->
                r

            Nothing ->
                False


diffSearches : SearchArgs -> ItemIndexer itemid item iidxs -> iidxs -> ItemIndexer TagId Tag tidxs -> tidxs -> SearchResults item -> ResultDifference
diffSearches sa iidxr iidxs tidxr tidxs sr =
    let
        osa =
            sr.searchArgs

        icid =
            iidxr.getChangeId iidxs

        tcid =
            tidxr.getChangeId tidxs
    in
    if
        not
            (osa.search == sa.search && icid == sr.iidxsChangeId && tcid == sr.tidxsChangeId)
    then
        RsSearch
    else if osa.showCount /= sa.showCount then
        RsCount
    else
        RsSame


perhapsBuildSearchResults :
    SearchArgs
    -> ItemIndexer itemid item iidxs
    -> iidxs
    -> ItemIndexer TagId Tag tidxs
    -> tidxs
    -> SearchResults item
    -> SearchResults item
perhapsBuildSearchResults sa iidxr iidxs tidxr tidxs osr =
    let
        searchdifference =
            diffSearches
                sa
                iidxr
                iidxs
                tidxr
                tidxs
                osr

        istuff =
            iidxr.itemStuff

        searchMatches =
            if rdgte searchdifference RsSearch then
                case sa.search of
                    TagSearch ts ->
                        case ts of
                            Ok s ->
                                itemSearch
                                    (makeActualTagSearch
                                        tidxr.itemStuff
                                        (tidxr.getItemById tidxs)
                                        (tidxr.getItemList tidxs)
                                        s
                                    )
                                    (tidxr.getItemById tidxs)
                                    iidxr.itemStuff
                                    (iidxr.getItemList iidxs)

                            Err e ->
                                iidxr.getItemList iidxs

                    NoSearch ->
                        iidxr.getItemList iidxs
            else
                osr.searchMatches

        results =
            if rdgte searchdifference RsCount then
                Array.fromList <|
                    List.map
                        (\i ->
                            { item = i
                            , tagnames = tagNames (tidxr.getItemById tidxs) (istuff.getTags i)
                            , tagparentnames =
                                tagNames (tidxr.getItemById tidxs)
                                    (tagSetParents (istuff.getTags i) (tidxr.getItemById tidxs))
                            }
                        )
                    <|
                        case sa.showCount of
                            Just count ->
                                List.take count searchMatches

                            Nothing ->
                                searchMatches
            else
                osr.results
    in
    { results = results
    , iidxsChangeId = iidxr.getChangeId iidxs
    , tidxsChangeId = tidxr.getChangeId tidxs
    , searchArgs = sa
    , searchMatches = searchMatches
    , searchMatchCount = List.length searchMatches
    }


makeSearchResulter =
    { buildSearchResults = perhapsBuildSearchResults
    , resultCount = \rslts -> Array.length rslts.results
    , resultRange = \rslts from to -> Array.toList <| Array.slice from to rslts.results
    , resultList = \rslts -> Array.toList rslts.results
    }


type alias SearchState itemid item idxs tidxs =
    { searchArgs : SearchArgs
    , searchResults : SearchResults item
    , searchResulter : SearchResulter itemid item idxs tidxs SearchArgs (SearchResult item) (SearchResults item)
    }


initSearchState : SearchState itemid item iidxs tidxs
initSearchState =
    let
        sa =
            { showCount = Just 50
            , search = NoSearch
            }
    in
    { searchArgs = sa
    , searchResults =
        { results = Array.fromList []
        , iidxsChangeId = -1
        , tidxsChangeId = -1
        , searchArgs = sa
        , searchMatches = []
        , searchMatchCount = 0
        }
    , searchResulter = makeSearchResulter
    }
