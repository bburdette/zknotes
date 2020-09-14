module EditZkNote exposing (Command(..), Model, Msg(..), addListNote, dirty, gotId, gotSelectedText, initExample, initFull, initNew, replaceOrAdd, sznFromModel, update, view)

import CellCommon exposing (..)
import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Common
import Data
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region as ER
import Html exposing (Attribute, Html)
import Html.Attributes
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..))
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import Schelme.Show exposing (showTerm)
import TangoColors as TC
import Url as U
import Url.Parser as UP exposing ((</>))
import Util


type Msg
    = OnMarkdownInput String
    | OnSchelmeCodeChanged String String
    | OnTitleChanged String
    | SavePress
    | DonePress
    | RevertPress
    | DeletePress
    | ViewPress
    | LinksPress
    | NewPress
    | SwitchPress Data.ZkListNote
    | LinkPress Data.ZkListNote
    | PublicPress Bool
    | RemoveLink Data.ZkLink
    | MdLink Data.ZkLink


type alias Model =
    { id : Maybe Int
    , zk : Data.Zk
    , zklist : List Data.ZkListNote
    , zklDict : Dict String Data.ZkLink
    , initialZklDict : Dict String Data.ZkLink
    , public : Bool
    , title : String
    , md : String
    , cells : CellDict
    , revert : Maybe Data.SaveZkNote
    }


type Command
    = None
    | Save Data.SaveZkNote (List Data.ZkLink)
    | SaveExit Data.SaveZkNote (List Data.ZkLink)
    | Revert
    | View Data.SaveZkNote
    | Delete Int
    | Switch Int
    | SaveSwitch Data.SaveZkNote Int
    | GetSelectedText String


sznFromModel : Model -> Data.SaveZkNote
sznFromModel model =
    { id = model.id
    , zk = model.zk.id
    , title = model.title
    , content = model.md
    , public = model.public
    }


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
                    r.id
                        == model.id
                        && r.public
                        == model.public
                        && r.title
                        == model.title
                        && r.content
                        == model.md
                        && Dict.keys model.zklDict
                        == Dict.keys model.initialZklDict
            )
        |> Maybe.withDefault True


showZkl : Maybe Int -> Data.ZkLink -> Element Msg
showZkl id zkl =
    E.row [ E.spacing 8, E.width E.fill ]
        [ case ( Just zkl.from == id, Just zkl.to == id ) of
            ( True, False ) ->
                E.text "->"

            ( False, True ) ->
                E.text "<-"

            _ ->
                E.text ""
        , id
            |> Maybe.map (zkLinkName zkl)
            |> Maybe.withDefault ""
            |> E.text
        , EI.button (Common.buttonStyle ++ [ E.alignRight ])
            { onPress = Just (MdLink zkl)
            , label = E.text "^"
            }
        , EI.button (Common.buttonStyle ++ [ E.alignRight ])
            { onPress = Just (RemoveLink zkl)
            , label = E.text "X"
            }
        ]


view : Model -> Element Msg
view model =
    let
        isdirty =
            dirty model

        dirtybutton =
            if isdirty then
                Common.buttonStyle ++ [ EBk.color TC.darkYellow ]

            else
                Common.buttonStyle
    in
    E.column
        [ E.width E.fill ]
        [ E.text "Edit Zk Note"
        , E.row [ E.width E.fill, E.spacing 8 ]
            [ EI.button
                dirtybutton
                { onPress = Just DonePress, label = E.text "Done" }
            , EI.button Common.buttonStyle { onPress = Just RevertPress, label = E.text "Cancel" }
            , EI.button Common.buttonStyle { onPress = Just ViewPress, label = E.text "View" }
            , EI.button Common.buttonStyle { onPress = Just LinksPress, label = E.text "Links" }
            , case isdirty of
                True ->
                    EI.button dirtybutton { onPress = Just SavePress, label = E.text "Save" }

                False ->
                    E.none
            , EI.button dirtybutton { onPress = Just NewPress, label = E.text "New" }
            , EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just DeletePress, label = E.text "Delete" }
            ]
        , EI.text []
            { onChange = OnTitleChanged
            , text = model.title
            , placeholder = Nothing
            , label = EI.labelLeft [] (E.text "title")
            }
        , EI.checkbox []
            { onChange = PublicPress
            , icon = EI.defaultCheckbox
            , checked = model.public
            , label = EI.labelLeft [] (E.text "public")
            }
        , E.row [ E.width E.fill ]
            [ E.column [ E.spacing 8 ]
                (EI.multiline
                    [ E.width (E.px 400)
                    , E.htmlAttribute (Html.Attributes.id "mdtext")
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
                        (showZkl model.id)
                        (Dict.values model.zklDict)
                )
            , case markdownView (mkRenderer model.cells OnSchelmeCodeChanged) model.md of
                Ok rendered ->
                    E.column
                        [ E.spacing 30
                        , E.padding 80
                        , E.width (E.fill |> E.maximum 1000)
                        , E.centerX
                        , E.alignTop
                        ]
                        rendered

                Err errors ->
                    E.text errors
            , E.column
                [ E.spacing 8
                , E.alignTop
                ]
                (List.map
                    (\zkln ->
                        E.row [ E.spacing 8 ]
                            [ case model.id of
                                Just _ ->
                                    EI.button Common.buttonStyle
                                        { onPress = Just <| LinkPress zkln
                                        , label = E.text "Link"
                                        }

                                Nothing ->
                                    EI.button (Common.buttonStyle ++ [ EBk.color TC.grey ])
                                        { onPress = Nothing
                                        , label = E.text "Link"
                                        }
                            , EI.button dirtybutton { onPress = Just (SwitchPress zkln), label = E.text "Edit" }
                            , E.text zkln.title
                            ]
                    )
                 <|
                    case model.id of
                        Just id ->
                            List.filter (\zkl -> zkl.id /= id) model.zklist

                        Nothing ->
                            model.zklist
                )
            ]
        ]



{- zkLinkDict : TDict ( Int, Int ) String Data.ZkLink
   zkLinkDict =
       TD.empty (\( a, b ) -> String.fromInt a ++ ":" ++ String.fromInt b)
           (\str ->
               case String.split ":" str of
                   [ a, b ] ->
                       ( String.toInt a, String.toInt b )

                   _ ->
                       ( -1, -1 )
           )


-}


zklKey : Data.ZkLink -> String
zklKey zkl =
    String.fromInt zkl.from ++ ":" ++ String.fromInt zkl.to


initFull : Data.Zk -> List Data.ZkListNote -> Data.FullZkNote -> Data.ZkLinks -> Model
initFull zk zkl zknote zklDict =
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
    , zklist = zkl
    , zklDict = Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) zklDict.links)
    , initialZklDict = Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) zklDict.links)
    , public = zknote.public
    , title = zknote.title
    , md = zknote.content
    , cells = getCd cc
    , revert = Just (Data.saveZkNoteFromFull zknote)
    }


initNew : Data.Zk -> List Data.ZkListNote -> Model
initNew zk zkl =
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
    , zklist = zkl
    , zklDict = Dict.empty
    , initialZklDict = Dict.empty
    , public = False
    , title = ""
    , md = ""
    , cells = getCd cc
    , revert = Nothing
    }


initExample : Data.Zk -> List Data.ZkListNote -> Model
initExample zk zkl =
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
    , zklist = zkl
    , zklDict = Dict.empty
    , initialZklDict = Dict.empty
    , public = False
    , title = "example"
    , md = markdownBody
    , cells = getCd cc
    , revert = Nothing
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
            , createdate = szkn.changeddate
            , changeddate = szkn.changeddate
            }
    in
    { model
        | zklist =
            replaceOrAdd model.zklist
                zln
                (\a b -> a.id == b.id)
                (\a b -> { b | createdate = a.createdate })
    }


gotId : Model -> Int -> Model
gotId model id =
    let
        m1 =
            { model | id = Just (model.id |> Maybe.withDefault id) }
    in
    -- if we already have an ID, keep it.
    { m1 | revert = Just <| sznFromModel m1 }


gotSelectedText : Model -> String -> ( Model, Command )
gotSelectedText model s =
    let
        nmod =
            initNew model.zk model.zklist
    in
    ( { nmod | title = s }
    , if dirty model then
        Save
            (sznFromModel model)
            (Dict.values model.zklDict)

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



-- saveLinks : List Data.ZkLink -> List Data.ZkLink -> List Data.ZkLink
-- saveLinks current initial =
--     let
--         curr =
--             List.sortWith compareZklinks current
--         init =
--             List.sortWith compareZklinks initial
--         duofold left right =
--             case ( left, right ) of
--                 ( [], [] ) ->
--                     []
--                 ( la :: lb, [] ) ->
--                     la :: lb
--                 ( [], ra :: rb ) ->
--                     List.map (\zkl -> { zkl | delete = Just True }) (ra :: rb)
--                 ( la :: lb, ra :: rb ) ->
--                     case compareZklinks la ra of
--                         EQ ->
--                             la :: duofold lb rb
--                         LT ->
--                             la :: duofold lb (ra :: rb)
--                         GT ->
--                             { ra | delete = Just True } :: duofold (la :: lb) rb
--     in
--     curr


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
            , SaveExit
                (sznFromModel model)
                (saveZkLinkList model)
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
                    Debug.log "zklDict" <|
                        case ( blah, model.id ) of
                            ( Err _, _ ) ->
                                Dict.empty

                            ( Ok blocks, Nothing ) ->
                                Dict.empty

                            ( Ok blocks, Just id ) ->
                                CellCommon.inlineFoldl
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

        SwitchPress zkln ->
            if dirty model then
                ( model, SaveSwitch (sznFromModel model) zkln.id )

            else
                ( model, Switch zkln.id )

        PublicPress v ->
            ( { model | public = v }, None )

        RevertPress ->
            ( model, Revert )

        DeletePress ->
            case model.id of
                Just id ->
                    ( model, Delete id )

                Nothing ->
                    ( model, None )

        OnTitleChanged t ->
            ( { model | title = t }, None )

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
