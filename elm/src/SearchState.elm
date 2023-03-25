module SearchState exposing (..)

import Data
import SearchStackPanel as SP


-- type SearchOrRecent
--     = SearchView
--     | RecentView


type alias Model =
    { spmodel : SP.Model
    , zknSearchResult : Data.ZkListNoteSearchResult
    , focusSr : Maybe Int -- note id in search result.
    -- , searchOrRecent : SearchOrRecent
    }
