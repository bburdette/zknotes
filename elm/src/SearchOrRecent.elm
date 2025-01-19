module SearchOrRecent exposing (Model, Msg(..), SearchOrRecent(..))

import Data exposing (Direction(..), EditLink, zklKey)
import Dialog as D
import Dict exposing (Dict)
import Element as E exposing (Element)
import Html exposing (Attribute, Html)
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..), inlineFoldl)
import Schelme.Show exposing (showTerm)
import SearchStackPanel as SP
import Url.Parser as UP exposing ((</>))


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
