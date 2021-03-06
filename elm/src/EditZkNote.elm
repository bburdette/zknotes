module EditZkNote exposing (Command(..), EditLink, Model, Msg(..), NavChoice(..), SearchOrRecent(..), WClass(..), addComment, commentsRecieved, commonButtonStyle, compareZklinks, dirty, disabledLinkButtonStyle, elToSzkl, elToSzl, fullSave, gotSelectedText, initFull, initNew, isPublic, isSearch, linkButtonStyle, linksWith, mkButtonStyle, noteLink, onCtrlS, onSaved, onZkNote, pageLink, renderMd, replaceOrAdd, saveZkLinkList, setHomeNote, showSr, showZkl, sznFromModel, sznToZkn, toPubId, toZkListNote, update, updateSearch, updateSearchResult, view, zkLinkName, zklKey, zknview)

import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Common
import Data exposing (Direction(..))
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
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..), inlineFoldl)
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import MdCommon as MC
import Schelme.Show exposing (showTerm)
import Search as S
import SearchPanel as SP
import TagSearchPanel as TSP
import TangoColors as TC
import Url as U
import Url.Builder as UB
import Url.Parser as UP exposing ((</>))
import Util
import ZkCommon as ZC


type Msg
    = OnMarkdownInput String
    | OnCommentInput String
    | AddComment
    | CancelComment
    | NewCommentPress
    | CheckCommentLink EditLink Bool
    | GoHomeNotePress
    | SetHomeNotePress
    | OnSchelmeCodeChanged String String
    | OnTitleChanged String
    | OnPubidChanged String
    | SavePress
    | DonePress
    | RevertPress
    | DeletePress
    | ViewPress
    | NewPress
    | CopyPress
    | SearchHistoryPress
    | SwitchPress Int
    | ToLinkPress Data.ZkListNote
    | FromLinkPress Data.ZkListNote
    | PublicPress Bool
    | EditablePress Bool
    | RemoveLink EditLink
    | MdLink EditLink
    | SPMsg SP.Msg
    | NavChoiceChanged NavChoice
    | SearchOrRecentChanged SearchOrRecent
    | DialogMsg D.Msg
    | RestoreSearch String
    | SrFocusPress Int
    | LinkFocusPress EditLink
    | AddToSearch Data.ZkListNote
    | AddToSearchAsTag String
    | SetSearch String
    | Noop


type NavChoice
    = NcEdit
    | NcView
    | NcSearch
    | NcRecent


type SearchOrRecent
    = SearchView
    | RecentView


type alias EditLink =
    { otherid : Int
    , direction : Direction
    , user : Int
    , zknote : Maybe Int
    , othername : Maybe String
    , sysids : List Int
    , delete : Maybe Bool
    }


type WClass
    = Narrow
    | Medium
    | Wide


type alias NewCommentState =
    { text : String
    , sharelinks : List ( EditLink, Bool )
    }


type alias Model =
    { id : Maybe Int
    , ld : Data.LoginData
    , noteUser : Int
    , noteUserName : String
    , usernote : Int
    , zknSearchResult : Data.ZkListNoteSearchResult
    , focusSr : Maybe Int -- note id in search result.
    , zklDict : Dict String EditLink
    , focusLink : Maybe EditLink
    , comments : List Data.ZkNote
    , newcomment : Maybe NewCommentState
    , pendingcomment : Maybe Data.SaveZkNote
    , editable : Bool
    , editableValue : Bool
    , pubidtxt : String
    , title : String
    , createdate : Maybe Int
    , changeddate : Maybe Int
    , md : String
    , cells : CellDict
    , revert : Maybe Data.SaveZkNote
    , initialZklDict : Dict String EditLink
    , spmodel : SP.Model
    , navchoice : NavChoice
    , searchOrRecent : SearchOrRecent
    , dialog : Maybe D.Model
    , panelNote : Maybe Data.ZkNote
    }


type Command
    = None
    | Save Data.SaveZkNotePlusLinks
    | SaveExit Data.SaveZkNotePlusLinks
    | Revert
    | View Data.SaveZkNote (Maybe Data.ZkNote)
    | Delete Int
    | Switch Int
    | SaveSwitch Data.SaveZkNotePlusLinks Int
    | GetSelectedText (List String)
    | Search S.ZkNoteSearch
    | SearchHistory
    | GetZkNote Int
    | SetHomeNote Int


onZkNote : Data.ZkNote -> Model -> ( Model, Command )
onZkNote zkn model =
    ( { model | panelNote = Just zkn }
    , View (sznFromModel model) (Just zkn)
    )


setHomeNote : Model -> Int -> Model
setHomeNote model id =
    let
        nld =
            model.ld
    in
    { model | ld = { nld | homenote = Just id } }


elToSzl : EditLink -> Data.SaveZkLink
elToSzl el =
    { otherid = el.otherid
    , direction = el.direction
    , user = el.user
    , zknote = el.zknote
    , delete = el.delete
    }


elToSzkl : Int -> EditLink -> Data.ZkLink
elToSzkl this el =
    case el.direction of
        From ->
            { from = this
            , to = el.otherid
            , user = el.user
            , zknote = Nothing
            , fromname = Nothing
            , toname = Nothing
            , delete = Nothing
            }

        To ->
            { from = el.otherid
            , to = this
            , user = el.user
            , zknote = Nothing
            , fromname = Nothing
            , toname = Nothing
            , delete = Nothing
            }


toZkListNote : Model -> Maybe Data.ZkListNote
toZkListNote model =
    case ( model.id, model.createdate, model.changeddate ) of
        ( Just id, Just createdate, Just changeddate ) ->
            Just
                { id = id
                , user = model.noteUser
                , title = model.title
                , createdate = createdate
                , changeddate = changeddate
                , sysids =
                    List.filterMap
                        (\el ->
                            if
                                List.any ((==) el.otherid)
                                    [ model.ld.publicid
                                    , model.ld.shareid
                                    , model.ld.searchid
                                    , model.ld.commentid
                                    ]
                                    && el.direction
                                    == To
                            then
                                Just el.otherid

                            else
                                Nothing
                        )
                        (Dict.values model.zklDict)
                }

        _ ->
            Nothing


sznFromModel : Model -> Data.SaveZkNote
sznFromModel model =
    { id = model.id
    , title = model.title
    , content = model.md
    , pubid = toPubId (isPublic model) model.pubidtxt
    , editable = model.editableValue
    }


fullSave : Model -> Data.SaveZkNotePlusLinks
fullSave model =
    { note = sznFromModel model
    , links = saveZkLinkList model
    }


saveZkLinkList : Model -> List Data.SaveZkLink
saveZkLinkList model =
    List.map
        (\zkl -> { zkl | delete = Nothing })
        (List.map elToSzl (Dict.values (Dict.diff model.zklDict model.initialZklDict)))
        ++ List.map
            (\zkl -> { zkl | delete = Just True })
            (List.map elToSzl (Dict.values (Dict.diff model.initialZklDict model.zklDict)))


commentsRecieved : List Data.ZkNote -> Model -> Model
commentsRecieved comments model =
    { model | comments = comments }


updateSearchResult : Data.ZkListNoteSearchResult -> Model -> Model
updateSearchResult zsr model =
    { model
        | zknSearchResult = zsr
        , spmodel = SP.searchResultUpdated zsr model.spmodel
    }


updateSearch : S.TagSearch -> Model -> ( Model, Command )
updateSearch ts model =
    ( { model
        | spmodel = SP.setSearchString model.spmodel (S.printTagSearch ts)
      }
    , None
    )


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
                        && (r.pubid == toPubId (isPublic model) model.pubidtxt)
                        && (r.title == model.title)
                        && (r.content == model.md)
                        && (r.editable == model.editableValue)
                        && (Dict.keys model.zklDict == Dict.keys model.initialZklDict)
            )
        |> Maybe.withDefault True


showZkl : Bool -> Bool -> Maybe EditLink -> Data.LoginData -> Maybe Int -> Maybe E.Color -> EditLink -> Element Msg
showZkl isDirty editable focusLink ld id sysColor zkl =
    let
        ( dir, otherid ) =
            case zkl.direction of
                To ->
                    ( E.text "→", Just zkl.otherid )

                From ->
                    ( E.text "←", Just zkl.otherid )

        focus =
            focusLink
                |> Maybe.map
                    (\l ->
                        zkl.direction
                            == l.direction
                            && zkl.otherid
                            == l.otherid
                            && zkl.user
                            == l.user
                    )
                |> Maybe.withDefault False

        display =
            [ dir
            , zkl.othername
                |> Maybe.withDefault ""
                |> (\s ->
                        E.el
                            ([ E.clipX
                             , E.height <| E.px 30
                             , E.width E.fill
                             , EE.onClick (LinkFocusPress zkl)
                             ]
                                ++ (sysColor
                                        |> Maybe.map (\c -> [ EF.color c ])
                                        |> Maybe.withDefault []
                                   )
                            )
                            (E.text s)
                   )
            ]
    in
    if focus then
        E.column [ E.spacing 8, E.width E.fill ]
            [ E.row [ E.spacing 8, E.width E.fill ] display
            , E.row [ E.spacing 8 ]
                [ if ld.userid == zkl.user then
                    EI.button (linkButtonStyle ++ [ E.alignLeft ])
                        { onPress = Just (RemoveLink zkl)
                        , label = E.text "X"
                        }

                  else
                    EI.button (linkButtonStyle ++ [ E.alignLeft, EBk.color TC.darkGray ])
                        { onPress = Nothing
                        , label = E.text "X"
                        }
                , if editable then
                    EI.button (linkButtonStyle ++ [ E.alignLeft ])
                        { onPress = Just (MdLink zkl)
                        , label = E.text "^"
                        }

                  else
                    EI.button (disabledLinkButtonStyle ++ [ E.alignLeft ])
                        { onPress = Nothing
                        , label = E.text "^"
                        }
                , case otherid of
                    Just zknoteid ->
                        E.link
                            (if isDirty then
                                ZC.saveLinkStyle

                             else
                                ZC.myLinkStyle
                            )
                            { url = Data.editNoteLink zknoteid
                            , label = E.text "go"
                            }

                    Nothing ->
                        E.none
                ]
            ]

    else
        E.row [ E.spacing 8, E.width E.fill, E.height <| E.px 30 ] display


pageLink : Model -> Maybe String
pageLink model =
    model.id
        |> Maybe.andThen
            (\id ->
                if isPublic model then
                    if model.pubidtxt /= "" then
                        Just <| UB.absolute [ "page", model.pubidtxt ] []

                    else
                        Just <| UB.absolute [ "note", String.fromInt id ] []

                else
                    Just <| UB.absolute [ "editnote", String.fromInt id ] []
            )


view : Util.Size -> List Data.ZkListNote -> Model -> Element Msg
view size recentZkns model =
    case model.dialog of
        Just dialog ->
            D.view size dialog |> E.map DialogMsg

        Nothing ->
            zknview size recentZkns model


commonButtonStyle : Bool -> List (E.Attribute msg)
commonButtonStyle isDirty =
    mkButtonStyle Common.buttonStyle isDirty


mkButtonStyle : List (E.Attribute msg) -> Bool -> List (E.Attribute msg)
mkButtonStyle style isdirty =
    if isdirty then
        style ++ [ EBk.color TC.darkYellow ]

    else
        style


linkButtonStyle =
    Common.buttonStyle


disabledLinkButtonStyle =
    Common.disabledButtonStyle


showSr : Model -> Bool -> Data.ZkListNote -> Element Msg
showSr model isdirty zkln =
    let
        lnnonme =
            zkln.user /= model.ld.userid

        sysColor =
            ZC.systemColor model.ld zkln.sysids

        controlrow =
            E.row [ E.spacing 8, E.width E.fill ]
                [ (case
                    Dict.get (zklKey { direction = To, otherid = zkln.id })
                        model.zklDict
                   of
                    Just _ ->
                        Nothing

                    Nothing ->
                        Just 1
                  )
                    |> Maybe.map
                        (\_ ->
                            EI.button linkButtonStyle
                                { onPress = Just <| ToLinkPress zkln
                                , label = E.el [ E.centerY ] <| E.text "→"
                                }
                        )
                    |> Maybe.withDefault
                        (EI.button
                            disabledLinkButtonStyle
                            { onPress = Nothing
                            , label = E.el [ E.centerY ] <| E.text "→"
                            }
                        )
                , (case
                    Dict.get (zklKey { direction = From, otherid = zkln.id })
                        model.zklDict
                   of
                    Just _ ->
                        Nothing

                    Nothing ->
                        Just 1
                  )
                    |> Maybe.map
                        (\_ ->
                            EI.button linkButtonStyle
                                { onPress = Just <| FromLinkPress zkln
                                , label = E.el [ E.centerY ] <| E.text "←"
                                }
                        )
                    |> Maybe.withDefault
                        (EI.button
                            disabledLinkButtonStyle
                            { onPress = Nothing
                            , label = E.el [ E.centerY ] <| E.text "←"
                            }
                        )
                , EI.button linkButtonStyle
                    { onPress = Just (AddToSearch zkln)
                    , label = E.text "^"
                    }
                , EI.button linkButtonStyle
                    { onPress = Just (AddToSearchAsTag zkln.title)
                    , label = E.text "t"
                    }
                , if lnnonme then
                    E.link
                        (if isdirty then
                            ZC.saveLinkStyle

                         else
                            ZC.otherLinkStyle
                        )
                        { url = Data.editNoteLink zkln.id
                        , label = E.text zkln.title
                        }

                  else
                    E.link
                        (if isdirty then
                            ZC.saveLinkStyle

                         else
                            ZC.myLinkStyle
                        )
                        { url = Data.editNoteLink zkln.id
                        , label = E.text "go"
                        }
                ]

        listingrow =
            E.el
                ([ E.width E.fill
                 , EE.onClick (SrFocusPress zkln.id)
                 , E.height <| E.px 30
                 , E.clipX
                 ]
                    ++ (sysColor
                            |> Maybe.map (\c -> [ EF.color c ])
                            |> Maybe.withDefault []
                       )
                )
            <|
                E.text zkln.title
    in
    if model.focusSr == Just zkln.id then
        -- focus result!  show controlrow.
        E.column [ E.spacing 8 ] [ listingrow, controlrow ]

    else
        listingrow


makeNewCommentState : Model -> NewCommentState
makeNewCommentState model =
    let
        sharelinks =
            model.zklDict
                |> Dict.values
                |> List.filterMap
                    (\l ->
                        if
                            List.any ((==) model.ld.shareid) l.sysids
                                || (l.otherid == model.ld.publicid)
                        then
                            Just
                                ( { otherid = l.otherid
                                  , direction = l.direction
                                  , user = model.ld.userid
                                  , zknote = Nothing
                                  , delete = Nothing
                                  , othername = l.othername
                                  , sysids = l.sysids
                                  }
                                , True
                                )

                        else
                            Nothing
                    )

        userlink =
            if model.ld.userid == model.noteUser then
                []

            else
                [ ( { otherid = model.usernote
                    , direction = Data.To
                    , user = model.ld.userid
                    , zknote = Nothing
                    , delete = Nothing
                    , othername = Just model.noteUserName
                    , sysids = []
                    }
                  , True
                  )
                ]
    in
    { text = "", sharelinks = userlink ++ sharelinks }


addComment : NewCommentState -> Element Msg
addComment ncs =
    E.column
        [ E.width E.fill
        , E.spacing 8
        , EBk.color TC.lightCharcoal
        , E.padding 8
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "new comment"
        , EI.multiline
            [ EF.color TC.black
            , E.width E.fill
            , E.alignTop
            ]
            { onChange = OnCommentInput
            , text = ncs.text
            , placeholder = Nothing
            , label = EI.labelHidden "Comment"
            , spellcheck = False
            }
        , if List.isEmpty ncs.sharelinks then
            E.none

          else
            E.row [ E.width E.fill, E.spacing 10 ]
                [ E.el [ E.alignTop ] <| E.text "sharing: "
                , E.column []
                    (ncs.sharelinks
                        |> List.map
                            (\( l, b ) ->
                                EI.checkbox []
                                    { checked = b
                                    , icon = EI.defaultCheckbox
                                    , label = EI.labelRight [] (E.text (l.othername |> Maybe.withDefault ""))
                                    , onChange = CheckCommentLink l
                                    }
                            )
                    )
                ]
        , E.row [ E.width E.fill ]
            [ EI.button (E.alignLeft :: Common.buttonStyle)
                { onPress = Just AddComment, label = E.text "reply" }
            , EI.button (E.alignRight :: Common.buttonStyle)
                { onPress = Just CancelComment, label = E.text "cancel" }
            ]
        ]


renderMd : CellDict -> String -> Int -> Element Msg
renderMd cd md mdw =
    case MC.markdownView (MC.mkRenderer RestoreSearch mdw cd True OnSchelmeCodeChanged) md of
        Ok rendered ->
            E.column
                [ E.spacing 30
                , E.padding 20
                , E.width (E.fill |> E.maximum 1000)
                , E.centerX
                , E.alignTop
                , EBd.width 2
                , EBd.color TC.darkGrey
                , EBk.color TC.lightGrey
                ]
                rendered

        Err errors ->
            E.text errors


zknview : Util.Size -> List Data.ZkListNote -> Model -> Element Msg
zknview size recentZkns model =
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

        perhapsdirtybutton =
            commonButtonStyle isdirty

        mine =
            model.noteUser == model.ld.userid

        public =
            isPublic model

        search =
            isSearch
                model

        editable =
            model.editable
                && not search

        -- super lame math because images suck in html/elm-ui
        mdw =
            min 1000
                (case wclass of
                    Narrow ->
                        size.width

                    Medium ->
                        size.width - 400 - 8

                    Wide ->
                        size.width - 400 - 500 - 16
                )
                - (60 * 2 + 6)

        showComments =
            E.el [ EF.bold, E.width E.fill ] (E.text "comments")
                :: List.map
                    (\zkn ->
                        E.row [ E.width E.fill, E.spacing 8 ]
                            [ renderMd model.cells zkn.content mdw
                            , E.el [ E.alignRight ] <| E.text zkn.username
                            , E.link
                                (E.alignRight
                                    :: (if isdirty then
                                            ZC.saveLinkStyle

                                        else
                                            ZC.myLinkStyle
                                       )
                                )
                                { url = Data.editNoteLink zkn.id
                                , label = E.text "go"
                                }
                            ]
                    )
                    model.comments
                ++ (case model.newcomment of
                        Just s ->
                            [ addComment s ]

                        Nothing ->
                            [ EI.button Common.buttonStyle
                                { label = E.text "new comment", onPress = Just NewCommentPress }
                            ]
                   )

        showLinks =
            E.row [ EF.bold ] [ E.text "links" ]
                :: List.map
                    (\( l, c ) -> showZkl isdirty editable model.focusLink model.ld model.id c l)
                    (Dict.values model.zklDict
                        |> List.map
                            (\l ->
                                ( l
                                , ZC.systemColor model.ld l.sysids
                                )
                            )
                        |> List.sortWith
                            (\( l, lc ) ( r, rc ) ->
                                case ( lc, rc ) of
                                    ( Nothing, Nothing ) ->
                                        compare r.otherid l.otherid

                                    ( Just _, Nothing ) ->
                                        GT

                                    ( Nothing, Just _ ) ->
                                        LT

                                    ( Just lcolor, Just rcolor ) ->
                                        case Util.compareColor lcolor rcolor of
                                            EQ ->
                                                compare r.otherid l.otherid

                                            a ->
                                                a
                            )
                    )

        editview =
            let
                titleed =
                    EI.text
                        (if editable then
                            [ E.htmlAttribute (Html.Attributes.id "title")
                            ]

                         else
                            [ EF.color TC.darkGrey, E.htmlAttribute (Html.Attributes.id "title") ]
                        )
                        { onChange =
                            if editable then
                                OnTitleChanged

                            else
                                always Noop
                        , text = model.title
                        , placeholder = Nothing
                        , label = EI.labelLeft [] (E.text "title")
                        }
            in
            E.column
                [ E.spacing 8
                , E.alignTop
                , E.width E.fill
                , E.paddingXY 5 0
                ]
                ([ titleed
                 , if mine then
                    EI.checkbox [ E.width E.shrink ]
                        { onChange =
                            if editable then
                                EditablePress

                            else
                                always Noop
                        , icon = EI.defaultCheckbox
                        , checked = model.editableValue
                        , label = EI.labelLeft [] (E.text "editable")
                        }

                   else
                    E.row [ E.spacing 8 ]
                        [ EI.checkbox [ E.width E.shrink ]
                            { onChange = always Noop -- can't change editable unless you're the owner.
                            , icon = EI.defaultCheckbox
                            , checked = model.editableValue
                            , label = EI.labelLeft [] (E.text "editable")
                            }
                        , E.text "creator"
                        , E.row [ EF.bold ] [ E.text model.noteUserName ]
                        ]
                 , E.row [ E.spacing 8, E.width E.fill ]
                    [ EI.checkbox [ E.width E.shrink ]
                        { onChange =
                            if editable then
                                PublicPress

                            else
                                always Noop
                        , icon = EI.defaultCheckbox
                        , checked = public
                        , label = EI.labelLeft [] (E.text "public")
                        }
                    , if public then
                        EI.text [ E.width E.fill ]
                            { onChange =
                                if editable then
                                    OnPubidChanged

                                else
                                    always Noop
                            , text = model.pubidtxt
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "article id")
                            }

                      else
                        E.none
                    , if wclass /= Narrow then
                        showpagelink

                      else
                        E.none
                    ]
                 , if wclass == Narrow then
                    showpagelink

                   else
                    E.none
                 , EI.multiline
                    [ if editable then
                        EF.color TC.black

                      else
                        EF.color TC.darkGrey
                    , E.htmlAttribute (Html.Attributes.id "mdtext")
                    , E.alignTop
                    ]
                    { onChange =
                        if editable then
                            OnMarkdownInput

                        else
                            always Noop
                    , text = model.md
                    , placeholder = Nothing
                    , label = EI.labelHidden "Markdown input"
                    , spellcheck = False
                    }
                 ]
                    ++ showComments
                    -- show the links.
                    ++ showLinks
                )

        mdview =
            E.column
                [ E.width E.fill
                , E.centerX
                , E.alignTop
                , E.spacing 8
                , E.paddingXY 5 0
                ]
            <|
                [ E.column
                    [ E.centerX
                    , E.paddingXY 30 10
                    , E.spacing 8
                    ]
                    [ E.row [ E.width E.fill, E.spacing 8 ]
                        [ E.paragraph [ EF.bold ] [ E.text model.title ]
                        , if search then
                            EI.button (E.alignRight :: Common.buttonStyle)
                                { label = E.text ">", onPress = Just <| SetSearch model.title }

                          else
                            EI.button (E.alignRight :: Common.buttonStyle)
                                { label = E.text ">", onPress = Just <| AddToSearchAsTag model.title }
                        ]
                    , renderMd model.cells model.md mdw
                    ]
                ]
                    ++ (if wclass == Wide then
                            []

                        else
                            showComments ++ showLinks
                       )

        parabuttonstyle =
            Common.buttonStyle ++ [ E.paddingXY 10 0 ]

        disabledparabuttonstyle =
            Common.disabledButtonStyle ++ [ E.paddingXY 10 0 ]

        perhapsdirtyparabuttonstyle =
            perhapsdirtybutton ++ [ E.paddingXY 10 0 ]

        ( spwidth, sppad ) =
            case wclass of
                Narrow ->
                    ( E.fill, [ E.paddingXY 0 5 ] )

                Medium ->
                    ( E.px 400, [ E.padding 5 ] )

                Wide ->
                    ( E.px 400, [ E.padding 5 ] )

        searchOrRecentPanel =
            E.column
                [ E.spacing 8
                , E.alignTop
                , E.alignRight
                , E.width spwidth
                , EBd.width 1
                , EBd.color TC.darkGrey
                , EBd.rounded 10
                , EBk.color TC.white
                , E.clip
                ]
                (Common.navbar 2
                    model.searchOrRecent
                    SearchOrRecentChanged
                    [ ( SearchView, "search" )
                    , ( RecentView, "recent" )
                    ]
                    :: [ case model.searchOrRecent of
                            SearchView ->
                                searchPanel

                            RecentView ->
                                recentPanel
                       ]
                )

        searchPanel =
            E.column
                (E.spacing 8 :: E.width E.fill :: sppad)
                (EI.button Common.buttonStyle
                    { onPress = Just <| SearchHistoryPress
                    , label = E.el [ E.centerY ] <| E.text "search history"
                    }
                    :: (E.map SPMsg <|
                            SP.view True (size.width < 400 || wclass /= Narrow) 0 model.spmodel
                       )
                    :: (List.map
                            (showSr model isdirty)
                        <|
                            case model.id of
                                Just id ->
                                    List.filter (\zkl -> zkl.id /= id) model.zknSearchResult.notes

                                Nothing ->
                                    model.zknSearchResult.notes
                       )
                )

        recentPanel =
            E.column (E.spacing 8 :: sppad)
                (List.map
                    (showSr model isdirty)
                 <|
                    recentZkns
                )

        showpagelink =
            case pageLink model of
                Just pl ->
                    E.link Common.linkStyle { url = pl, label = E.text pl }

                Nothing ->
                    E.none

        headingPanel : String -> List (E.Attribute Msg) -> Element Msg -> Element Msg
        headingPanel name attribs elt =
            E.column
                ([ E.spacing 12
                 , E.alignTop
                 , EBd.width 1
                 , EBd.color TC.darkGrey
                 , EBd.rounded 10
                 , E.clip
                 , E.height E.fill
                 , EBk.color TC.white
                 ]
                    ++ attribs
                )
                [ Common.navbar 2
                    ()
                    (\_ -> Noop)
                    [ ( (), name )
                    ]
                , elt
                ]
    in
    E.column
        [ E.width E.fill
        , E.height E.fill
        , E.spacing 8
        , E.padding 8
        , EBk.color TC.lightGray
        ]
        [ E.row [ E.width E.fill, E.spacing 8 ]
            [ model.ld.homenote
                |> Maybe.map
                    (\id ->
                        if Just id == model.id then
                            EI.button Common.disabledButtonStyle
                                { onPress = Nothing
                                , label = E.text "⌂"
                                }

                        else
                            EI.button perhapsdirtybutton
                                { onPress = Just (SwitchPress id)
                                , label = E.text "⌂"
                                }
                    )
                |> Maybe.withDefault E.none
            , E.el [ EF.bold ] (E.text model.ld.name)
            , if mine then
                EI.button (E.alignRight :: Common.buttonStyle) { onPress = Just DeletePress, label = E.text "delete" }

              else
                EI.button (E.alignRight :: Common.disabledButtonStyle) { onPress = Nothing, label = E.text "delete" }
            ]
        , E.paragraph
            [ E.width E.fill, E.spacingXY 3 17 ]
          <|
            List.intersperse (E.text " ")
                [ EI.button
                    perhapsdirtyparabuttonstyle
                    { onPress = Just DonePress, label = E.text "done" }
                , EI.button parabuttonstyle { onPress = Just RevertPress, label = E.text "cancel" }
                , EI.button parabuttonstyle { onPress = Just ViewPress, label = E.text "view" }
                , EI.button parabuttonstyle { onPress = Just CopyPress, label = E.text "copy" }
                , let
                    disb =
                        EI.button disabledparabuttonstyle
                            { onPress = Nothing
                            , label = E.text "→⌂"
                            }

                    enb =
                        EI.button parabuttonstyle
                            { onPress = Just SetHomeNotePress
                            , label = E.text "→⌂"
                            }
                  in
                  case ( model.ld.homenote, model.id ) of
                    ( _, Nothing ) ->
                        disb

                    ( Just x, Just y ) ->
                        if x == y then
                            disb

                        else
                            enb

                    ( Nothing, Just _ ) ->
                        enb

                -- , EI.button parabuttonstyle { onPress = Just LinksPress, label = E.text"links" }
                , case isdirty of
                    True ->
                        EI.button perhapsdirtyparabuttonstyle { onPress = Just SavePress, label = E.text "save" }

                    False ->
                        E.none
                , EI.button perhapsdirtyparabuttonstyle { onPress = Just NewPress, label = E.text "new" }
                ]
        , case wclass of
            Wide ->
                E.row
                    [ E.width E.fill
                    , E.alignTop
                    , E.spacing 8
                    ]
                    [ headingPanel "edit" [ E.width E.fill ] editview
                    , headingPanel "view" [ E.width E.fill ] mdview
                    , searchOrRecentPanel
                    ]

            Medium ->
                E.row
                    [ E.width E.fill
                    , E.spacing 8
                    ]
                    [ E.column
                        [ E.spacing 12
                        , E.alignTop
                        , EBd.width 1
                        , EBd.color TC.darkGrey
                        , EBd.rounded 10
                        , E.clip
                        , E.width E.fill
                        , E.height E.fill
                        , EBk.color TC.white
                        ]
                        [ Common.navbar 2
                            (if model.navchoice == NcSearch then
                                NcView

                             else
                                model.navchoice
                            )
                            NavChoiceChanged
                            [ ( NcView, "view" )
                            , ( NcEdit
                              , if editable then
                                    "edit"

                                else
                                    "markdown"
                              )
                            ]
                        , case model.navchoice of
                            NcEdit ->
                                editview

                            NcView ->
                                mdview

                            NcSearch ->
                                mdview

                            NcRecent ->
                                mdview
                        ]
                    , searchOrRecentPanel
                    ]

            Narrow ->
                E.column [ E.width E.fill ]
                    [ Common.navbar 2
                        model.navchoice
                        NavChoiceChanged
                        [ ( NcView, "view" )
                        , ( NcEdit
                          , if editable then
                                "edit"

                            else
                                "markdown"
                          )
                        , ( NcSearch, "search" )
                        , ( NcRecent, "recent" )
                        ]
                    , case model.navchoice of
                        NcEdit ->
                            editview

                        NcView ->
                            mdview

                        NcSearch ->
                            searchPanel

                        NcRecent ->
                            recentPanel
                    ]
        ]


zklKey : { a | otherid : Int, direction : Direction } -> String
zklKey zkl =
    String.fromInt zkl.otherid
        ++ ":"
        ++ (case zkl.direction of
                From ->
                    "from"

                To ->
                    "to"
           )


linksWith : List EditLink -> Int -> Bool
linksWith links linkid =
    Util.trueforany (\l -> l.otherid == linkid) links


isPublic : Model -> Bool
isPublic model =
    linksWith (Dict.values model.zklDict) model.ld.publicid


isSearch : Model -> Bool
isSearch model =
    linksWith (Dict.values model.zklDict) model.ld.searchid


initFull : Data.LoginData -> SearchOrRecent -> Data.ZkListNoteSearchResult -> Data.ZkNote -> List Data.EditLink -> SP.Model -> ( Model, Data.GetZkNoteComments )
initFull ld searchOrRecent zkl zknote dtlinks spm =
    let
        cells =
            zknote.content
                |> MC.mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)

        links =
            List.map
                (\dl ->
                    { otherid = dl.otherid
                    , direction = dl.direction
                    , user = dl.user
                    , zknote = dl.zknote
                    , othername = dl.othername
                    , sysids = dl.sysids
                    , delete = Nothing
                    }
                )
                dtlinks
    in
    ( { id = Just zknote.id
      , ld = ld
      , noteUser = zknote.user
      , noteUserName = zknote.username
      , usernote = zknote.usernote
      , zknSearchResult = zkl
      , focusSr = Nothing
      , zklDict = Dict.fromList (List.map (\zl -> ( zklKey zl, zl )) links)
      , initialZklDict =
            Dict.fromList
                (List.map
                    (\zl -> ( zklKey zl, zl ))
                    links
                )
      , focusLink = Nothing
      , pubidtxt = zknote.pubid |> Maybe.withDefault ""
      , title = zknote.title
      , md = zknote.content
      , comments = []
      , newcomment = Nothing
      , pendingcomment = Nothing
      , editable = zknote.editable
      , editableValue = zknote.editableValue
      , createdate = Just zknote.createdate
      , changeddate = Just zknote.changeddate
      , cells = getCd cc
      , revert = Just (Data.saveZkNote zknote)
      , spmodel = SP.searchResultUpdated zkl spm
      , navchoice = NcView
      , searchOrRecent = searchOrRecent
      , dialog = Nothing
      , panelNote = Nothing
      }
    , { zknote = zknote.id, offset = 0, limit = Nothing }
    )


initNew : Data.LoginData -> Data.ZkListNoteSearchResult -> SP.Model -> Model
initNew ld zkl spm =
    let
        cells =
            ""
                |> MC.mdCells
                |> Result.withDefault (CellDict Dict.empty)

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { id = Nothing
    , ld = ld
    , noteUser = ld.userid
    , noteUserName = ld.name
    , usernote = ld.zknote
    , zknSearchResult = zkl
    , focusSr = Nothing
    , zklDict = Dict.empty
    , initialZklDict = Dict.empty
    , focusLink = Nothing
    , pubidtxt = ""
    , title = ""
    , comments = []
    , newcomment = Nothing
    , pendingcomment = Nothing
    , editable = True
    , editableValue = True
    , createdate = Nothing
    , changeddate = Nothing
    , md = ""
    , cells = getCd cc
    , revert = Nothing
    , spmodel = SP.searchResultUpdated zkl spm
    , navchoice = NcEdit
    , searchOrRecent = SearchView
    , dialog = Nothing
    , panelNote = Nothing
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



{- addListNote : Model -> Int -> Data.SaveZkNote -> Data.SavedZkNote -> Model
   addListNote model uid szn szkn =
       let
           zln =
               { id = szkn.id
               , user = uid
               , title = szn.title
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

-}


sznToZkn : Int -> String -> Int -> List Int -> Data.SavedZkNote -> Data.SaveZkNote -> Data.ZkNote
sznToZkn uid uname unote sysids sdzn szn =
    { id = sdzn.id
    , user = uid
    , username = uname
    , usernote = unote
    , title = szn.title
    , content = szn.content
    , pubid = Nothing
    , editable = False
    , editableValue = False
    , createdate = sdzn.changeddate
    , changeddate = sdzn.changeddate
    , sysids = sysids
    }


onSaved : Model -> Data.SavedZkNote -> Model
onSaved model szn =
    case model.pendingcomment of
        Just pc ->
            { model
                | comments =
                    model.comments
                        ++ [ sznToZkn
                                model.ld.userid
                                model.ld.name
                                model.ld.zknote
                                [ model.ld.commentid ]
                                szn
                                pc
                           ]
            }

        Nothing ->
            let
                -- if we already have an ID, keep it.
                m1 =
                    { model
                        | id = Just (model.id |> Maybe.withDefault szn.id)
                        , createdate = model.createdate |> Util.mapNothing szn.changeddate
                        , changeddate = Just szn.changeddate
                    }
            in
            { m1 | revert = Just <| sznFromModel m1 }


gotSelectedText : Model -> String -> ( Model, Command )
gotSelectedText model s =
    let
        nmod =
            initNew model.ld model.zknSearchResult model.spmodel
    in
    ( { nmod | title = s }
    , if dirty model then
        Save
            (fullSave model)

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


onCtrlS : Model -> ( Model, Command )
onCtrlS model =
    if dirty model then
        update SavePress model

    else
        ( model, None )


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        RestoreSearch s ->
            let
                spmodel =
                    SP.addSearchString model.spmodel s
            in
            ( { model | spmodel = spmodel }
              -- DON'T automatically search.  To allow building searches
              -- from components.
              -- , SP.getSearch spmodel
              --     |> Maybe.map Search
              --     |> Maybe.withDefault None
            , None
            )

        SavePress ->
            let
                saveZkn =
                    sznFromModel model
            in
            ( { model
                | revert = Just saveZkn
                , initialZklDict = model.zklDict
              }
            , Save
                (fullSave model)
            )

        DonePress ->
            ( model
            , if dirty model then
                SaveExit
                    (fullSave model)

              else
                Revert
            )

        CopyPress ->
            ( { model
                | id = Nothing
                , title = "Copy of " ++ model.title
                , noteUser = model.ld.userid
                , noteUserName = model.ld.name
                , usernote = model.ld.zknote
                , editable = True
                , pubidtxt = "" -- otherwise we get a conflict on save.
                , zklDict =
                    model.zklDict
                        |> Dict.remove (zklKey { otherid = model.ld.publicid, direction = To })
                        |> Dict.remove (zklKey { otherid = model.ld.publicid, direction = From })
                , initialZklDict = Dict.empty
              }
            , None
            )

        ViewPress ->
            case MC.mdPanel model.md of
                Just panel ->
                    if Maybe.map .id model.panelNote == Just panel.noteid then
                        ( model
                        , View
                            (sznFromModel model)
                            model.panelNote
                        )

                    else
                        ( model
                        , GetZkNote panel.noteid
                        )

                Nothing ->
                    ( model
                    , View
                        (sznFromModel model)
                        Nothing
                    )

        {- LinksPress ->
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
                                                           , user = model.ld.userid
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

        -}
        NewPress ->
            ( model
            , GetSelectedText [ "title", "mdtext" ]
            )

        ToLinkPress zkln ->
            let
                nzkl =
                    { direction = To
                    , otherid = zkln.id
                    , user = model.ld.userid
                    , zknote = Nothing
                    , othername = Just zkln.title
                    , sysids = zkln.sysids
                    , delete = Nothing
                    }
            in
            ( { model
                | zklDict = Dict.insert (zklKey nzkl) nzkl model.zklDict
              }
            , None
            )

        FromLinkPress zkln ->
            let
                nzkl =
                    { direction = From
                    , otherid = zkln.id
                    , user = model.ld.userid
                    , zknote = Nothing
                    , othername = Just zkln.title
                    , sysids = zkln.sysids
                    , delete = Nothing
                    }
            in
            ( { model
                | zklDict = Dict.insert (zklKey nzkl) nzkl model.zklDict
              }
            , None
            )

        RemoveLink zkln ->
            ( { model
                | zklDict = Dict.remove (zklKey zkln) model.zklDict
              }
            , None
            )

        MdLink zkln ->
            let
                ( title, id ) =
                    ( zkln.othername |> Maybe.withDefault "", zkln.otherid )
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
                ( model, SaveSwitch (fullSave model) id )

            else
                ( model, Switch id )

        EditablePress _ ->
            -- if we're getting this event, we should be allowed to change the value.
            ( { model | editableValue = not model.editableValue }, None )

        PublicPress _ ->
            case model.id of
                Nothing ->
                    ( model, None )

                Just id ->
                    if isPublic model then
                        ( { model
                            | zklDict =
                                model.zklDict
                                    |> Dict.remove (zklKey { direction = From, otherid = model.ld.publicid })
                                    |> Dict.remove (zklKey { direction = To, otherid = model.ld.publicid })
                          }
                        , None
                        )

                    else
                        let
                            nzkl =
                                { direction = To
                                , otherid = model.ld.publicid
                                , user = model.ld.userid
                                , zknote = Nothing
                                , othername = Just "public"
                                , sysids = []
                                , delete = Nothing
                                }
                        in
                        ( { model
                            | zklDict = Dict.insert (zklKey nzkl) nzkl model.zklDict
                          }
                        , None
                        )

        RevertPress ->
            ( model, Revert )

        DeletePress ->
            ( { model
                | dialog =
                    Just <|
                        D.init "delete this note?"
                            True
                            (\size -> E.map (\_ -> ()) (view size [] model))
              }
            , None
            )

        SearchHistoryPress ->
            ( model, SearchHistory )

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

        OnCommentInput s ->
            let
                ncs =
                    model.newcomment
                        |> Maybe.map (\state -> { state | text = s })
                        |> Util.mapNothing (makeNewCommentState model)
            in
            ( { model | newcomment = ncs }, None )

        NewCommentPress ->
            ( { model | newcomment = model.newcomment |> Util.mapNothing (makeNewCommentState model) }, None )

        CheckCommentLink editlink checked ->
            ( { model
                | newcomment =
                    model.newcomment
                        |> Maybe.map
                            (\ncs ->
                                { ncs
                                    | sharelinks =
                                        ncs.sharelinks
                                            |> List.map
                                                (\( el, ch ) ->
                                                    if el == editlink then
                                                        ( el, checked )

                                                    else
                                                        ( el, ch )
                                                )
                                }
                            )
              }
            , None
            )

        CancelComment ->
            ( { model | newcomment = Nothing }, None )

        AddComment ->
            case ( model.id, model.newcomment ) of
                ( Just id, Just newcomment ) ->
                    let
                        nc =
                            { id = Nothing
                            , pubid = Nothing
                            , title = "comment"
                            , content = newcomment.text
                            , editable = False
                            }
                    in
                    ( { model | newcomment = Nothing, pendingcomment = Just nc }
                    , Save
                        { note =
                            nc
                        , links =
                            [ { otherid = model.ld.commentid
                              , direction = To
                              , user = model.ld.userid
                              , zknote = Nothing
                              , delete = Nothing
                              }
                            , { otherid = id
                              , direction = To
                              , user = model.ld.userid
                              , zknote = Nothing
                              , delete = Nothing
                              }
                            ]
                                ++ List.filterMap
                                    (\( l, b ) ->
                                        if b then
                                            Just
                                                { otherid = l.otherid
                                                , direction = l.direction
                                                , user = model.ld.userid
                                                , zknote = Nothing
                                                , delete = Nothing
                                                }

                                        else
                                            Nothing
                                    )
                                    newcomment.sharelinks
                        }
                    )

                _ ->
                    ( model, None )

        OnMarkdownInput newMarkdown ->
            let
                cells =
                    newMarkdown
                        |> MC.mdCells
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
                            (Dict.insert name (MC.defCell string) cd
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

                SP.Copy s ->
                    ( { mod
                        | md =
                            model.md
                                ++ (if model.md == "" then
                                        "<search query=\""

                                    else
                                        "\n\n<search query=\""
                                   )
                                ++ String.replace "&" "&amp;" s
                                ++ "\"/>"
                      }
                    , None
                    )

                SP.Search ts ->
                    let
                        zsr =
                            mod.zknSearchResult
                    in
                    ( { mod | zknSearchResult = { zsr | notes = [] } }, Search ts )

        NavChoiceChanged nc ->
            ( { model | navchoice = nc }, None )

        SearchOrRecentChanged x ->
            ( { model | searchOrRecent = x }, None )

        SrFocusPress id ->
            ( { model
                | focusSr =
                    if model.focusSr == Just id then
                        Nothing

                    else
                        Just id
              }
            , None
            )

        LinkFocusPress link ->
            ( { model
                | focusLink =
                    if model.focusLink == Just link then
                        Nothing

                    else
                        Just link
              }
            , None
            )

        AddToSearch zkln ->
            let
                spmod =
                    model.spmodel
            in
            if List.any ((==) model.ld.searchid) zkln.sysids then
                ( { model
                    | spmodel =
                        { spmod
                            | tagSearchModel =
                                TSP.updateSearchText spmod.tagSearchModel zkln.title
                        }
                  }
                , None
                )

            else
                ( { model
                    | spmodel =
                        { spmod
                            | tagSearchModel =
                                TSP.addToSearchPanel spmod.tagSearchModel [ S.ExactMatch ] zkln.title
                        }
                  }
                , None
                )

        SetSearch text ->
            let
                spmod =
                    model.spmodel
            in
            ( { model
                | spmodel =
                    { spmod
                        | tagSearchModel =
                            TSP.updateSearchText spmod.tagSearchModel text
                    }
              }
            , None
            )

        AddToSearchAsTag title ->
            let
                spmod =
                    model.spmodel
            in
            ( { model
                | spmodel =
                    { spmod
                        | tagSearchModel =
                            TSP.addToSearchPanel spmod.tagSearchModel
                                [ S.ExactMatch, S.Tag ]
                                title
                    }
              }
            , None
            )

        GoHomeNotePress ->
            ( model, None )

        SetHomeNotePress ->
            ( model, model.id |> Maybe.map (\id -> SetHomeNote id) |> Maybe.withDefault None )

        Noop ->
            ( model, None )
