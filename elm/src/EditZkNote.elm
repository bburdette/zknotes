module EditZkNote exposing
    ( Command(..)
    , Model
    , Msg(..)
    , addListNote
    , compareZklinks
    , dirty
    , gotId
    , gotSelectedText
    , initExample
    , initFull
    , initNew
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
import Element.Font as Font
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
    = OnMarkdownInput String
    | OnSchelmeCodeChanged String String
    | OnTitleChanged String
    | OnPubidChanged String
    | SavePress
    | DonePress
    | RevertPress
    | DeletePress
    | ViewPress
    | LinksPress
    | NewPress
    | SwitchPress Int
    | LinkPress Data.ZkListNote
    | PublicPress Bool
    | RemoveLink Data.ZkLink
    | MdLink Data.ZkLink
    | SPMsg SP.Msg
    | NavChoiceChanged NavChoice
    | DialogMsg D.Msg


type NavChoice
    = NcEdit
    | NcView
    | NcSearch


type alias Model =
    { id : Maybe Int
    , zk : Data.Zk
    , zknSearchResult : Data.ZkNoteSearchResult
    , zklDict : Dict String Data.ZkLink
    , public : Bool
    , pubidtxt : String
    , title : String
    , md : String
    , cells : CellDict
    , revert : Maybe Data.SaveZkNote
    , initialZklDict : Dict String Data.ZkLink
    , spmodel : SP.Model
    , navchoice : NavChoice
    , dialog : Maybe D.Model
    }


type Command
    = None
    | Save Data.SaveZkNote (List Data.ZkLink)
    | SaveExit Data.SaveZkNote (List Data.ZkLink)
    | Revert
    | View Data.SaveZkNote
    | Delete Int
    | Switch Int
    | SaveSwitch Data.SaveZkNote (List Data.ZkLink) Int
    | GetSelectedText String
    | Search S.ZkNoteSearch


sznFromModel : Model -> Data.SaveZkNote
sznFromModel model =
    { id = model.id
    , zk = model.zk.id
    , title = model.title
    , content = model.md
    , public = model.public
    , pubid = toPubId model.public model.pubidtxt
    }


updateSearchResult : Data.ZkNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | zknSearchResult = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


toPubId : Bool -> String -> Maybe String
toPubId public pubidtxt =
    if public && pubidtxt /= "" then
        Just pubidtxt

    else
        Nothing


zkLinkName : Data.ZkLink -> Int -> String
zkLinkName zklink noteid =
    if noteid == zklink.from then
        zklink.toname |> Maybe.withDefault (String.fromInt zklink.to)

    else if noteid == zklink.to then
        zklink.fromname |> Maybe.withDefault (String.fromInt zklink.from)

    else
        "link error"


dirty : Model -> Bool
dirty model =
    model.revert
        |> Maybe.map
            (\r ->
                not <|
                    (r.id == model.id)
                        && (r.public == model.public)
                        && (r.pubid == toPubId model.public model.pubidtxt)
                        && (r.title == model.title)
                        && (r.content == model.md)
                        && (Dict.keys model.zklDict == Dict.keys model.initialZklDict)
            )
        |> Maybe.withDefault True


showZkl : Int -> List (E.Attribute Msg) -> Maybe Int -> Data.ZkLink -> Element Msg
showZkl width dirtybutton id zkl =
    let
        ( dir, otherid ) =
            case ( Just zkl.from == id, Just zkl.to == id ) of
                ( True, False ) ->
                    ( E.text "->", Just zkl.to )

                ( False, True ) ->
                    ( E.text "<-", Just zkl.from )

                _ ->
                    ( E.text "", Nothing )
    in
    E.row [ E.spacing 8, E.width E.fill ]
        [ dir
        , id
            |> Maybe.map (zkLinkName zkl)
            |> Maybe.withDefault ""
            |> (\s ->
                    E.row
                        [ E.clipX
                        , E.centerY
                        , E.height E.fill
                        , E.width (E.px <| width - 160)
                        ]
                        [ E.text s
                        ]
               )
        , case otherid of
            Just zknoteid ->
                EI.button (dirtybutton ++ [ E.alignRight ]) { onPress = Just (SwitchPress zknoteid), label = E.text "↗" }

            Nothing ->
                E.none
        , EI.button (Common.buttonStyle ++ [ E.alignRight ])
            { onPress = Just (MdLink zkl)
            , label = E.text "^"
            }
        , EI.button (Common.buttonStyle ++ [ E.alignRight ])
            { onPress = Just (RemoveLink zkl)
            , label = E.text "X"
            }
        ]


pageLink : Model -> Maybe String
pageLink model =
    model.id
        |> Maybe.andThen
            (\id ->
                toPubId model.public model.pubidtxt
                    |> Maybe.map
                        (\pubid ->
                            UB.absolute [ "page", pubid ] []
                        )
                    |> Util.mapNothing
                        (UB.absolute [ "note", String.fromInt id ] [])
            )


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
            dirty model

        dirtybutton =
            if isdirty then
                Common.buttonStyle ++ [ EBk.color TC.darkYellow ]

            else
                Common.buttonStyle

        spxwidth =
            case wclass of
                Narrow ->
                    size.width

                Medium ->
                    400

                Wide ->
                    400

        mdedit =
            let
                mdeditwidth =
                    case wclass of
                        Narrow ->
                            size.width

                        Medium ->
                            size.width - spxwidth

                        Wide ->
                            500
            in
            E.column
                [ E.spacing 8
                , E.alignTop
                , E.width
                    (case wclass of
                        Narrow ->
                            E.fill

                        Medium ->
                            E.fill

                        Wide ->
                            E.px 500
                    )
                ]
                (EI.multiline
                    [ E.htmlAttribute (Html.Attributes.id "mdtext")
                    , E.alignTop
                    ]
                    { onChange = OnMarkdownInput
                    , text = model.md
                    , placeholder = Nothing
                    , label = EI.labelHidden "Markdown input"
                    , spellcheck = False
                    }
                    -- show the links.
                    :: E.row [ Font.bold ] [ E.text "links" ]
                    :: List.map
                        (showZkl mdeditwidth dirtybutton model.id)
                        (Dict.values model.zklDict)
                )

        mdview =
            case markdownView (mkRenderer model.cells OnSchelmeCodeChanged) model.md of
                Ok rendered ->
                    E.column
                        [ E.width E.fill
                        , E.centerX
                        , E.alignTop
                        ]
                        [ E.column
                            [ E.centerX
                            , E.paddingXY 30 15
                            , E.spacing 8
                            , EBk.color TC.lightGrey
                            ]
                            [ E.paragraph [] [ E.text model.title ]
                            , E.column
                                [ E.spacing 30
                                , E.padding 20
                                , E.width (E.fill |> E.maximum 1000)
                                , E.centerX
                                , E.alignTop
                                , EBd.width 3
                                , EBd.color TC.darkGrey
                                ]
                                rendered
                            ]
                        ]

                Err errors ->
                    E.text errors

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

                titlemaxconst =
                    135
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
                                E.row [ E.spacing 8 ]
                                    [ case model.id of
                                        Just _ ->
                                            EI.button Common.buttonStyle
                                                { onPress = Just <| LinkPress zkln
                                                , label = E.text "link"
                                                }

                                        Nothing ->
                                            EI.button (Common.buttonStyle ++ [ EBk.color TC.grey ])
                                                { onPress = Nothing
                                                , label = E.text "link"
                                                }
                                    , EI.button dirtybutton
                                        { onPress = Just (SwitchPress zkln.id), label = E.text "edit" }
                                    , E.row
                                        [ E.clipX
                                        , E.centerY
                                        , E.height E.fill
                                        , E.width (E.px <| spxwidth - titlemaxconst)
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
        [ E.width E.fill, E.spacing 8 ]
        [ E.row [ E.width E.fill, E.spacing 8 ]
            [ E.text "edit zk note"
            , EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just DeletePress, label = E.text "delete" }
            ]
        , E.row [ E.width E.fill, E.spacing 8 ]
            [ EI.button
                dirtybutton
                { onPress = Just DonePress, label = E.text "done" }
            , EI.button Common.buttonStyle { onPress = Just RevertPress, label = E.text "cancel" }
            , EI.button Common.buttonStyle { onPress = Just ViewPress, label = E.text "view" }
            , EI.button Common.buttonStyle { onPress = Just LinksPress, label = E.text "links" }
            , case isdirty of
                True ->
                    EI.button dirtybutton { onPress = Just SavePress, label = E.text "save" }

                False ->
                    E.none
            , EI.button dirtybutton { onPress = Just NewPress, label = E.text "new" }
            ]
        , EI.text []
            { onChange = OnTitleChanged
            , text = model.title
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "title")
            }
        , E.row [ E.spacing 8, E.width E.shrink ]
            [ EI.checkbox [ E.width E.shrink ]
                { onChange = PublicPress
                , icon = EI.defaultCheckbox
                , checked = model.public
                , label = EI.labelLeft [] (E.text "public")
                }
            , if model.public then
                EI.text []
                    { onChange = OnPubidChanged
                    , text = model.pubidtxt
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "article id")
                    }

              else
                E.none
            , case pageLink model of
                Just pl ->
                    E.link Common.linkStyle { url = pl, label = E.text pl }

                Nothing ->
                    E.none
            ]
        , case wclass of
            Wide ->
                E.row
                    [ E.width E.fill
                    , E.spacing 8
                    , E.alignTop
                    ]
                    [ mdedit, mdview, searchPanel ]

            Medium ->
                E.row [ E.width E.fill ]
                    [ E.column [ E.width E.fill, E.alignTop ]
                        [ Common.navbar 2
                            (if model.navchoice == NcSearch then
                                NcView

                             else
                                model.navchoice
                            )
                            NavChoiceChanged
                            [ ( NcView, "view" )
                            , ( NcEdit, "edit" )
                            ]
                        , case model.navchoice of
                            NcEdit ->
                                mdedit

                            NcView ->
                                mdview

                            NcSearch ->
                                mdview
                        ]
                    , searchPanel
                    ]

            Narrow ->
                E.column [ E.width E.fill ]
                    [ Common.navbar 2
                        model.navchoice
                        NavChoiceChanged
                        [ ( NcView, "view" )
                        , ( NcEdit, "edit" )
                        , ( NcSearch, "search" )
                        ]
                    , case model.navchoice of
                        NcEdit ->
                            mdedit

                        NcView ->
                            mdview

                        NcSearch ->
                            searchPanel
                    ]
        ]


zklKey : Data.ZkLink -> String
zklKey zkl =
    String.fromInt zkl.from ++ ":" ++ String.fromInt zkl.to


initFull : Data.Zk -> Data.ZkNoteSearchResult -> Data.FullZkNote -> Data.ZkLinks -> SP.Model -> Model
initFull zk zkl zknote zklDict spm =
    let
        cells =
            zknote.content
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Just zknote.id
    , zk = zk
    , zknSearchResult = zkl
    , zklDict = Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) zklDict.links)
    , initialZklDict = Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) zklDict.links)
    , public = zknote.public
    , pubidtxt = zknote.pubid |> Maybe.withDefault ""
    , title = zknote.title
    , md = zknote.content
    , cells = getCd cc
    , revert = Just (Data.saveZkNoteFromFull zknote)
    , spmodel = SP.searchResultUpdated zkl spm
    , navchoice = NcEdit
    , dialog = Nothing
    }


initNew : Data.Zk -> Data.ZkNoteSearchResult -> SP.Model -> Model
initNew zk zkl spm =
    let
        cells =
            ""
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Nothing
    , zk = zk
    , zknSearchResult = zkl
    , zklDict = Dict.empty
    , initialZklDict = Dict.empty
    , public = False
    , pubidtxt = ""
    , title = ""
    , md = ""
    , cells = getCd cc
    , revert = Nothing
    , spmodel = SP.searchResultUpdated zkl spm
    , navchoice = NcEdit
    , dialog = Nothing
    }


initExample : Data.Zk -> Data.ZkNoteSearchResult -> SP.Model -> Model
initExample zk zkl spm =
    let
        cells =
            markdownBody
                |> mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Nothing
    , zk = zk
    , zknSearchResult = zkl
    , zklDict = Dict.empty
    , initialZklDict = Dict.empty
    , public = False
    , pubidtxt = ""
    , title = "example"
    , md = markdownBody
    , cells = getCd cc
    , revert = Nothing
    , spmodel = spm
    , navchoice = NcEdit
    , dialog = Nothing
    }


replaceOrAdd : List a -> a -> (a -> a -> Bool) -> (a -> a -> a) -> List a
replaceOrAdd items replacement compare mergef =
    case items of
        l :: r ->
            if compare l replacement then
                mergef l replacement :: r

            else
                l :: replaceOrAdd r replacement compare mergef

        [] ->
            [ replacement ]


addListNote : Model -> Data.SaveZkNote -> Data.SavedZkNote -> Model
addListNote model szn szkn =
    let
        zln =
            { id = szkn.id
            , title = szn.title
            , zk = szn.zk
            , public = szn.public
            , createdate = szkn.changeddate
            , changeddate = szkn.changeddate
            }
    in
    { model
        | zknSearchResult =
            model.zknSearchResult
                |> (\zsr ->
                        { zsr
                            | notes =
                                replaceOrAdd model.zknSearchResult.notes
                                    zln
                                    (\a b -> a.id == b.id)
                                    (\a b -> { b | createdate = a.createdate })
                        }
                   )
    }


gotId : Model -> Int -> Model
gotId model id =
    let
        -- if we already have an ID, keep it.
        m1 =
            { model | id = Just (model.id |> Maybe.withDefault id) }
    in
    { m1 | revert = Just <| sznFromModel m1 }


gotSelectedText : Model -> String -> ( Model, Command )
gotSelectedText model s =
    let
        nmod =
            initNew model.zk model.zknSearchResult model.spmodel
    in
    ( { nmod | title = s }
    , if dirty model then
        Save
            (sznFromModel model)
            (saveZkLinkList model)

      else
        None
    )


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
                Revert
            )

        ViewPress ->
            ( model
            , View
                (sznFromModel model)
            )

        LinksPress ->
            let
                blah =
                    model.md
                        |> Markdown.Parser.parse
                        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")

                zklDict =
                    case ( blah, model.id ) of
                        ( Err _, _ ) ->
                            Dict.empty

                        ( Ok blocks, Nothing ) ->
                            Dict.empty

                        ( Ok blocks, Just id ) ->
                            inlineFoldl
                                (\inline links ->
                                    case inline of
                                        Block.Link str mbstr moarinlines ->
                                            case noteLink str of
                                                Just rid ->
                                                    let
                                                        zkl =
                                                            { from = id
                                                            , to = rid
                                                            , zknote = Nothing
                                                            , fromname = Nothing
                                                            , toname = mbstr
                                                            , delete = Nothing
                                                            }
                                                    in
                                                    ( zklKey zkl, zkl )
                                                        :: links

                                                Nothing ->
                                                    links

                                        _ ->
                                            links
                                )
                                []
                                blocks
                                |> Dict.fromList
            in
            ( { model | zklDict = Dict.union model.zklDict zklDict }, None )

        NewPress ->
            ( model
            , GetSelectedText "mdtext"
            )

        LinkPress zkln ->
            -- add a zklink, or newlink?
            case model.id of
                Just id ->
                    let
                        nzkl =
                            { from = id
                            , to = zkln.id
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

        MdLink zkln ->
            let
                ( title, id ) =
                    if Just zkln.from == model.id then
                        ( Maybe.withDefault "" zkln.toname, zkln.to )

                    else
                        ( Maybe.withDefault "" zkln.fromname, zkln.from )
            in
            ( { model
                | md =
                    model.md
                        ++ (if model.md == "" then
                                "["

                            else
                                "\n\n["
                           )
                        ++ title
                        ++ "]("
                        ++ "/note/"
                        ++ String.fromInt id
                        ++ ")"
              }
            , None
            )

        SwitchPress id ->
            if dirty model then
                ( model, SaveSwitch (sznFromModel model) (saveZkLinkList model) id )

            else
                ( model, Switch id )

        PublicPress v ->
            ( { model | public = v }, None )

        RevertPress ->
            ( model, Revert )

        DeletePress ->
            ( { model | dialog = Just <| D.init "delete this note?" (\size -> E.map (\_ -> ()) (view size model)) }, None )

        DialogMsg dm ->
            case model.dialog of
                Just dmod ->
                    case ( D.update dm dmod, model.id ) of
                        ( D.Cancel, _ ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Ok, Nothing ) ->
                            ( { model | dialog = Nothing }, None )

                        ( D.Ok, Just id ) ->
                            ( { model | dialog = Nothing }, Delete id )

                        ( D.Dialog dmod2, _ ) ->
                            ( { model | dialog = Just dmod2 }, None )

                Nothing ->
                    ( model, None )

        OnTitleChanged t ->
            ( { model | title = t }, None )

        OnPubidChanged t ->
            ( { model | pubidtxt = t }, None )

        OnMarkdownInput newMarkdown ->
            let
                cells =
                    newMarkdown
                        |> mdCells
                        |> Result.withDefault (CellDict Dict.empty)

                ( cc, result ) =
                    evalCellsFully
                        (mkCc cells)
            in
            ( { model
                | md = newMarkdown
                , cells = getCd cc
              }
            , None
            )

        OnSchelmeCodeChanged name string ->
            let
                (CellDict cd) =
                    model.cells

                ( cc, result ) =
                    evalCellsFully
                        (mkCc
                            (Dict.insert name (defCell string) cd
                                |> CellDict
                            )
                        )
            in
            ( { model
                | cells = getCd cc
              }
            , None
            )

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

        NavChoiceChanged nc ->
            ( { model | navchoice = nc }, None )
