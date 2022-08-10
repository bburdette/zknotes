module SearchOrRecent exposing (Model, Msg(..), SearchOrRecent(..))

import Browser.Dom as BD
import Common
import Data exposing (Direction(..), EditLink, zklKey)
import Dialog as D
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Html exposing (Attribute, Html)
import Html.Attributes
import Json.Decode as JD
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..), inlineFoldl)
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import MdCommon as MC
import Schelme.Show exposing (showTerm)
import Search as S
import SearchStackPanel as SP
import TangoColors as TC
import Task
import Time
import Toop
import Url as U
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))
import Util
import WindowKeys as WK
import ZkCommon as ZC


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
