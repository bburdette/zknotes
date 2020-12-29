module Import exposing
    ( Command(..)
    , Model
    ,  Msg(..)
       -- , addListNote

    , compareZklinks
    , gotId
    , init
    , noteLink
    , pageLink
    , replaceOrAdd
    , saveZkLinkList
    , showZkl
    , sznFromModel
    , toPubId
    , update
    , updateSearchResult
    , view
    , zkLinkName
    , zklKey
    )

import CellCommon exposing (..)
import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Common
import Data
import Dialog as D
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Html exposing (Attribute, Html)
import Html.Attributes
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..), inlineFoldl)
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import Schelme.Show exposing (showTerm)
import Search as S
import SearchPanel as SP
import TangoColors as TC
import Url as U
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))
import Util


type Msg
    = SavePress
    | DonePress
    | CancelPress
    | LinkPress Data.ZkListNote
    | RemoveLink Data.ZkLink
    | SPMsg SP.Msg
    | DialogMsg D.Msg
    | Noop


type alias Model =
    { ld : Data.LoginData
    , noteUser : Int
    , noteUserName : String
    , zknSearchResult : Data.ZkNoteSearchResult
    , zklDict : Dict String Data.ZkLink
    , spmodel : SP.Model
    , dialog : Maybe D.Model
    }


type Command
    = None
    | Save Data.SaveZkNote (List Data.ZkLink)
    | SaveExit Data.SaveZkNote (List Data.ZkLink)
    | Search S.ZkNoteSearch
    | Cancel


updateSearchResult : Data.ZkNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | zknSearchResult = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


zkLinkName : Data.ZkLink -> Int -> String
zkLinkName zklink noteid =
    if noteid == zklink.from then
        zklink.toname |> Maybe.withDefault (String.fromInt zklink.to)

    else if noteid == zklink.to then
        zklink.fromname |> Maybe.withDefault (String.fromInt zklink.from)

    else
        "link error"


showZkl : Int -> Maybe Int -> Data.ZkLink -> Element Msg
showZkl user mbid zkl =
    let
        ( dir, otherid ) =
            case ( Just zkl.from == mbid, Just zkl.to == mbid ) of
                ( True, False ) ->
                    ( E.text "->", Just zkl.to )

                ( False, True ) ->
                    ( E.text "<-", Just zkl.from )

                _ ->
                    ( E.text "", Nothing )
    in
    E.row [ E.spacing 8, E.width E.fill ]
        [ dir
        , mbid
            |> Maybe.map (zkLinkName zkl)
            |> Maybe.withDefault ""
            |> (\s ->
                    E.row
                        [ E.clipX
                        , E.centerY
                        , E.height E.fill
                        , E.width E.fill
                        ]
                        [ E.text s
                        ]
               )
        , if user == zkl.user then
            EI.button (Common.buttonStyle ++ [ E.alignRight ])
                { onPress = Just (RemoveLink zkl)
                , label = E.text "X"
                }

          else
            EI.button (Common.buttonStyle ++ [ E.alignRight, EBk.color TC.darkGray ])
                { onPress = Nothing
                , label = E.text "X"
                }
        ]


type WClass
    = Narrow
    | Medium
    | Wide


view : Util.Size -> Model -> Element Msg
view size model =
    case model.dialog of
        Just dialog ->
            D.view size dialog |> E.map DialogMsg

        Nothing ->
            zknview size model


zknview : Util.Size -> Model -> Element Msg
zknview size model =
    let
        wclass =
            if size.width < 800 then
                Narrow

            else if size.width > 1700 then
                Wide

            else
                Medium

        isdirty =
            True

        showLinks =
            E.row [ EF.bold ] [ E.text "links" ]
                :: List.map
                    (showZkl model.ld.userid model.id)
                    (Dict.values model.zklDict)

        public =
            isPublic model

        searchPanel =
            let
                spwidth =
                    case wclass of
                        Narrow ->
                            E.fill

                        Medium ->
                            E.px 400

                        Wide ->
                            E.px 400
            in
            E.column
                [ E.spacing 8
                , E.alignTop
                , E.alignRight
                , E.width spwidth
                ]
                ((E.map SPMsg <|
                    SP.view (wclass == Narrow) 0 model.spmodel
                 )
                    :: (List.map
                            (\zkln ->
                                let
                                    lnnonme =
                                        zkln.user /= model.ld.userid
                                in
                                E.row [ E.spacing 8, E.width E.fill ]
                                    [ model.id
                                        |> Maybe.andThen
                                            (\id ->
                                                case Dict.get (zklKey { from = id, to = zkln.id }) model.zklDict of
                                                    Just _ ->
                                                        Nothing

                                                    Nothing ->
                                                        Just 1
                                            )
                                        |> Maybe.map
                                            (\_ ->
                                                EI.button Common.buttonStyle
                                                    { onPress = Just <| LinkPress zkln
                                                    , label = E.text "link"
                                                    }
                                            )
                                        |> Maybe.withDefault
                                            (EI.button
                                                Common.disabledButtonStyle
                                                { onPress = Nothing
                                                , label = E.text "link"
                                                }
                                            )
                                    , E.row
                                        [ E.clipX
                                        , E.height E.fill
                                        , E.width E.fill
                                        ]
                                        [ E.text zkln.title
                                        ]
                                    ]
                            )
                        <|
                            case model.id of
                                Just id ->
                                    List.filter (\zkl -> zkl.id /= id) model.zknSearchResult.notes

                                Nothing ->
                                    model.zknSearchResult.notes
                       )
                )
    in
    E.column
        [ E.width E.fill, E.spacing 8, E.padding 8 ]
        [ E.row [ E.width E.fill, E.spacing 8 ]
            [ E.row [ EF.bold ] [ E.text model.ld.name ]
            ]
        , E.row [ E.width E.fill, E.spacing 8 ]
            [ EI.button Common.buttonStyle { onPress = Just CancelPress, label = E.text "cancel" }
            , case isdirty of
                True ->
                    EI.button Common.buttonStyle { onPress = Just SavePress, label = E.text "save" }

                False ->
                    E.none
            ]
        , E.row
            [ E.width E.fill
            , E.spacing 8
            , E.alignTop
            ]
            [ searchPanel ]
        ]


zklKey : { a | from : Int, to : Int } -> String
zklKey zkl =
    String.fromInt zkl.from ++ ":" ++ String.fromInt zkl.to


linksWith : List Data.ZkLink -> Int -> Bool
linksWith links pubid =
    Util.trueforany (\l -> l.from == pubid || l.to == pubid) links


isPublic : Model -> Bool
isPublic model =
    linksWith (Dict.values model.zklDict) model.ld.publicid


init : Data.LoginData -> Data.ZkNoteSearchResult -> SP.Model -> Model
init ld zkl spm =
    { ld = ld
    , zknSearchResult = zkl
    , zklDict = Dict.empty -- Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) zklDict.links)
    , spmodel = SP.searchResultUpdated zkl spm
    , dialog = Nothing
    }


noteLink : String -> Maybe Int
noteLink str =
    -- hack allows parsing /note/<N>
    -- other urls will be invalid which is fine.
    U.fromString ("http://wat" ++ str)
        |> Maybe.andThen
            (UP.parse (UP.s "note" </> UP.int))


compareZklinks : Data.ZkLink -> Data.ZkLink -> Order
compareZklinks left right =
    case compare left.from right.from of
        EQ ->
            compare left.to right.to

        ltgt ->
            ltgt


saveZkLinkList : Model -> List Data.ZkLink
saveZkLinkList model =
    List.map
        (\zkl -> { zkl | delete = Nothing })
        (Dict.values (Dict.diff model.zklDict model.initialZklDict))
        ++ List.map
            (\zkl -> { zkl | delete = Just True })
            (Dict.values (Dict.diff model.initialZklDict model.zklDict))


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SavePress ->
            -- TODO more reliability.  What if the save fails?
            let
                saveZkn =
                    sznFromModel model
            in
            ( { model
                | revert = Just saveZkn
                , initialZklDict = model.zklDict
              }
            , Save
                saveZkn
                (saveZkLinkList model)
            )

        DonePress ->
            ( model
            , if dirty model then
                SaveExit
                    (sznFromModel model)
                    (saveZkLinkList model)

              else
                Cancel
            )

        LinkPress zkln ->
            -- add a zklink, or newlink?
            case model.id of
                Just id ->
                    let
                        nzkl =
                            { from = id
                            , to = zkln.id
                            , user = model.ld.userid
                            , zknote = Nothing
                            , fromname = Nothing
                            , toname = Just zkln.title
                            , delete = Nothing
                            }
                    in
                    ( { model
                        | zklDict = Dict.insert (zklKey nzkl) nzkl model.zklDict
                      }
                    , None
                    )

                Nothing ->
                    ( model, None )

        RemoveLink zkln ->
            ( { model
                | zklDict = Dict.remove (zklKey zkln) model.zklDict
              }
            , None
            )

        DialogMsg dm ->
            case model.dialog of
                Just dmod ->
                    case ( D.update dm dmod, model.id ) of
                        ( D.Cancel, _ ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Ok, Nothing ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Dialog dmod2, _ ) ->
                            ( { model | dialog = Just dmod2 }, None )

                Nothing ->
                    ( model, None )

        SPMsg m ->
            let
                ( nm, cm ) =
                    SP.update m model.spmodel

                mod =
                    { model | spmodel = nm }
            in
            case cm of
                SP.None ->
                    ( mod, None )

                SP.Save ->
                    ( mod, None )

                SP.Search ts ->
                    ( mod, Search ts )

        Noop ->
            ( model, None )
