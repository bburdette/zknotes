module SearchOrRecent exposing (Model, Msg(..), SearchOrRecent(..))

import Data exposing (Direction(..), EditLink)
import Dialog as D
import SearchStackPanel as SP


type Msg
    = SearchHistoryPress
    | SwitchPress Int
    | ToLinkPress Data.ZkListNote
    | FromLinkPress Data.ZkListNote
    | RemoveLink EditLink
    | SPMsg SP.Msg
    | NavChoiceChanged NavChoice
    | DialogMsg D.Msg
    | RestoreSearch String
    | SrFocusPress Int
    | LinkFocusPress EditLink
    | AddToSearch Data.ZkListNote
    | AddToSearchAsTag String
    | SetSearchString String
    | SetSearch (List S.TagSearch)
    | BigSearchPress
    | SettingsPress
    | FlipLink EditLink
    | Noop


type SearchOrRecent
    = SearchView
    | RecentView


type alias Model =
    { zknSearchResult : Data.ZkListNoteSearchResult
    , focusSr : Maybe Int -- note id in search result.
    , spmodel : SP.Model
    , searchOrRecent : SearchOrRecent
    , dialog : Maybe D.Model
    , panelNote : Maybe Data.ZkNote
    }
