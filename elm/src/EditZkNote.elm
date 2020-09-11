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


type alias Model =
    { id : Maybe Int
    , zk : Data.Zk
    , zklist : List Data.ZkListNote
    , zklinks : List Data.ZkLink
    , public : Bool
    , title : String
    , md : String
    , cells : CellDict
    , revert : Maybe Data.SaveZkNote
    }


type Command
    = None
    | Save Data.SaveZkNote
    | SaveExit Data.SaveZkNote
    | Revert
    | View Data.SaveZkNote
    | Delete Int
    | Switch Int
    | SaveSwitch Data.SaveZkNote Int
    | GetSelectedText String
    | SaveLinks Data.ZkLinks


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
    if noteid == zklink.left then
        zklink.rightname |> Maybe.withDefault (String.fromInt zklink.right)

    else if noteid == zklink.right then
        zklink.leftname |> Maybe.withDefault (String.fromInt zklink.left)

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
            )
        |> Maybe.withDefault True


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
            [ E.column []
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
                    :: List.map
                        (\zkl ->
                            E.row [ E.spacing 8 ]
                                [ model.id
                                    |> Maybe.map (zkLinkName zkl)
                                    |> Maybe.withDefault ""
                                    |> E.text
                                ]
                        )
                        model.zklinks
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
                            [ EI.button Common.buttonStyle { onPress = Just (LinkPress zkln), label = E.text "Link" }
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


initFull : Data.Zk -> List Data.ZkListNote -> Data.FullZkNote -> Data.ZkLinks -> Model
initFull zk zkl zknote zklinks =
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
    , zklinks = zklinks.links
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
    , zklinks = []
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
    , zklinks = []
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


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SavePress ->
            -- TODO more reliability.  What if the save fails?
            let
                saveZkn =
                    sznFromModel model
            in
            ( { model | revert = Just saveZkn }
            , Save
                saveZkn
            )

        DonePress ->
            ( model
            , SaveExit
                (sznFromModel model)
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

                zklinks =
                    case ( blah, model.id ) of
                        ( Err _, _ ) ->
                            Debug.log "linkserr:" <|
                                []

                        ( Ok blocks, Nothing ) ->
                            Debug.log "linksnothing:" <|
                                []

                        ( Ok blocks, Just id ) ->
                            Debug.log "links:" <|
                                CellCommon.inlineFoldl
                                    (\inline links ->
                                        case inline of
                                            Block.Link str mbstr moarinlines ->
                                                case noteLink str of
                                                    Just rid ->
                                                        { left = id
                                                        , right = rid
                                                        , zknote = Nothing
                                                        , leftname = Nothing
                                                        , rightname = Nothing
                                                        }
                                                            :: links

                                                    Nothing ->
                                                        links

                                            _ ->
                                                links
                                    )
                                    []
                                    blocks
            in
            ( model
            , case zklinks of
                [] ->
                    None

                a :: b ->
                    SaveLinks
                        { zk = model.zk.id
                        , links = a :: b
                        }
            )

        NewPress ->
            ( model
            , GetSelectedText "mdtext"
            )

        LinkPress zkln ->
            ( { model | md = model.md ++ "\n[" ++ zkln.title ++ "](/note/" ++ String.fromInt zkln.id ++ ")" }, None )

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
