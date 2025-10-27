module MdInlineXform exposing (..)

import Data
import DataUtil exposing (zkNoteIdToString)
import EdMarkdown exposing (stringRenderer)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Font as EF
import Element.Input as EI
import GenDialog as GD
import Markdown.Block as MB exposing (Inline(..))
import Markdown.Renderer
import MdGui as MG exposing (findAttrib)
import Orgauth.Data as Data
import Route exposing (Route(..))
import TangoColors as TC
import Toop
import Url
import Util


type alias Model =
    { mobile : Bool
    , inline : MB.Inline
    , transforms : List ( String, XForm )
    , selectedTf : Maybe Int
    , tomsg : MB.Inline -> MG.Msg
    }


type Msg
    = OkClick
    | CancelClick
    | OnSelect Int
    | Noop


type XForm
    = Upd MB.Inline
    | Lb ( String, Data.SavedZkNote -> MB.Inline )


type Command
    = UpdateInline MG.Msg
    | LinkBack String (Data.SavedZkNote -> Command)
    | Close


type alias GDModel =
    GD.Model Model Msg Command


init : MB.Inline -> (MB.Inline -> MG.Msg) -> Bool -> List (E.Attribute Msg) -> Element () -> GDModel
init inline fn mobile buttonStyle underLay =
    let
        tfs =
            transforms inline
    in
    { view = view buttonStyle
    , update = update
    , model =
        { mobile = mobile
        , inline = inline
        , transforms = tfs
        , selectedTf =
            if List.isEmpty tfs then
                Nothing

            else
                Just 0
        , tomsg = fn
        }
    , underLay = underLay
    }


inlineText : List MB.Inline -> Maybe String
inlineText inlines =
    case inlines of
        (MB.Text s) :: _ ->
            Just s

        _ ->
            Nothing


parsePath : String -> Maybe Route
parsePath s =
    (Url.fromString <|
        "https://arbitrary.com"
            ++ s
    )
        |> Maybe.andThen Route.parseUrl


pathZkNoteId : Route -> Maybe Data.ZkNoteId
pathZkNoteId route =
    case route of
        PublicZkNote znid ->
            Just znid

        EditZkNoteR znid _ ->
            Just znid

        LoginR ->
            Nothing

        PublicZkPubId _ ->
            Nothing

        EditZkNoteNew ->
            Nothing

        ArchiveNoteListingR _ ->
            Nothing

        ArchiveNoteR _ _ ->
            Nothing

        ResetPasswordR _ _ ->
            Nothing

        SettingsR ->
            Nothing

        Invite _ ->
            Nothing

        Top ->
            Nothing


transforms : MB.Inline -> List ( String, XForm )
transforms inline =
    case inline of
        HtmlInline htmlBlock ->
            case htmlBlock of
                {-
                   note -> audio
                   note -> video
                -}
                MB.HtmlElement "note" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "id" attribs, findAttrib "show" attribs ) of
                        ( mbtext, Just noteid, show ) ->
                            [ ( "link", Upd <| MB.Link ("/note/" ++ noteid) Nothing [ MB.Text (Maybe.withDefault "" mbtext) ] )
                            , ( "panel"
                              , Upd <|
                                    MB.HtmlInline
                                        (MB.HtmlElement "panel"
                                            [ { name = "noteid", value = noteid } ]
                                            []
                                        )
                              )
                            ]

                        _ ->
                            []

                MB.HtmlElement "yeet" attribs childs ->
                    case Toop.T4 (findAttrib "text" attribs) (findAttrib "id" attribs) (findAttrib "show" attribs) (findAttrib "url" attribs) of
                        Toop.T4 mbtext (Just id) mbshow mburl ->
                            [ ( "note link"
                              , Upd <|
                                    MB.HtmlInline
                                        (MB.HtmlElement "note"
                                            (List.filterMap identity
                                                [ Just
                                                    { name = "id"
                                                    , value = id
                                                    }
                                                , mbtext
                                                    |> Maybe.map (\s -> { name = "text", value = s })
                                                , mbshow
                                                    |> Maybe.map (\s -> { name = "show", value = s })
                                                ]
                                            )
                                            []
                                        )
                              )
                            ]

                        _ ->
                            []

                MB.HtmlElement "image" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "url" attribs, findAttrib "width" attribs ) of
                        ( Just text, Just url, mbwidth ) ->
                            [ ( "md image", Upd <| MB.Image url (Just text) [] )
                            , ( "link", Upd <| MB.Link url (Just text) [] )
                            ]

                        _ ->
                            []

                MB.HtmlElement "video" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "src" attribs ) of
                        ( Just text, Just src ) ->
                            [ ( "link", Upd <| MB.Link src (Just text) [] )
                            ]

                        _ ->
                            []

                MB.HtmlElement "audio" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "src" attribs ) of
                        ( Just text, Just src ) ->
                            [ ( "link", Upd <| MB.Link src (Just text) [] )
                            ]

                        _ ->
                            []

                _ ->
                    []

        Link url mbt inlines ->
            let
                upath =
                    parsePath url |> Maybe.andThen pathZkNoteId
            in
            {-
               audio,
               video
            -}
            List.filterMap identity
                [ Just
                    ( "yeet"
                    , Upd <|
                        MB.HtmlInline
                            (MB.HtmlElement "yeet"
                                (List.filterMap identity
                                    [ Just { name = "url", value = url }
                                    , inlineText inlines
                                        |> Maybe.map (\s -> { name = "text", value = s })
                                    ]
                                )
                                []
                            )
                    )
                , upath
                    |> Maybe.map
                        (\znid ->
                            ( "note link"
                            , Upd <|
                                MB.HtmlInline
                                    (MB.HtmlElement "note"
                                        (List.filterMap identity
                                            [ Just
                                                { name = "id"
                                                , value = zkNoteIdToString znid
                                                }
                                            , inlineText inlines
                                                |> Maybe.map (\s -> { name = "text", value = s })
                                            ]
                                        )
                                        []
                                    )
                            )
                        )
                , Just ( "md image", Upd <| MB.Image url mbt inlines )
                , Just
                    ( "html image"
                    , Upd <|
                        MB.HtmlInline
                            (MB.HtmlElement "image"
                                (List.filterMap identity
                                    [ Just { name = "url", value = url }
                                    , inlineText inlines
                                        |> Maybe.map (\s -> { name = "text", value = s })
                                    ]
                                )
                                []
                            )
                    )
                ]

        Image src mbt inlines ->
            [ ( "link", Upd <| MB.Link src mbt inlines )
            , ( "html image"
              , Upd <|
                    MB.HtmlInline
                        (MB.HtmlElement "image"
                            (List.filterMap identity
                                [ Just { name = "url", value = src }
                                , inlineText inlines
                                    |> Maybe.map (\s -> { name = "text", value = s })
                                ]
                            )
                            []
                        )
              )
            ]

        Emphasis inlines ->
            [ ( "remove"
              , Upd <|
                    case inlines of
                        [ item ] ->
                            item

                        plural ->
                            -- can't return multiple inlines.  TODO fix?
                            Emphasis inlines
              )
            ]

        Strong inlines ->
            [ ( "remove"
              , Upd <|
                    case inlines of
                        [ item ] ->
                            item

                        plural ->
                            -- can't return multiple inlines.  TODO fix?
                            Strong inlines
              )
            ]

        Strikethrough inlines ->
            [ ( "remove"
              , Upd <|
                    case inlines of
                        [ item ] ->
                            item

                        plural ->
                            -- can't return multiple inlines.  TODO fix?
                            Strikethrough inlines
              )
            ]

        CodeSpan s ->
            [ ( "text", Upd <| MB.Text s ) ]

        {-
           text -> search?
        -}
        Text s ->
            [ ( "strong", Upd <| MB.Strong [ MB.Text s ] )
            , ( "emphasis", Upd <| MB.Emphasis [ MB.Text s ] )
            , ( "strikethrough", Upd <| MB.Strikethrough [ MB.Text s ] )
            , ( "codespan", Upd <| MB.CodeSpan s )
            , ( "link", Upd <| MB.Link s Nothing [ MB.Text "" ] )
            , ( "link + text", Upd <| MB.Link s Nothing [ MB.Text s ] )
            , ( "yeet", Upd <| MB.HtmlInline (MB.HtmlElement "yeet" [ { name = "url", value = s } ] []) )
            , ( "linkback - note"
              , Lb
                    ( s
                    , \sn ->
                        MB.HtmlInline
                            (MB.HtmlElement "note"
                                [ { name = "id"
                                  , value = zkNoteIdToString sn.id
                                  }
                                ]
                                []
                            )
                    )
              )
            , ( "linkback - link"
              , Lb
                    ( s
                    , \sn ->
                        MB.Link ("/note/" ++ zkNoteIdToString sn.id) Nothing [ MB.Text s ]
                    )
              )
            ]

        HardLineBreak ->
            []


view : List (E.Attribute Msg) -> Maybe Util.Size -> Model -> Element Msg
view buttonStyle mbsize model =
    E.column
        [ E.width (mbsize |> Maybe.map .width |> Maybe.map E.px |> Maybe.withDefault E.fill)
        , E.height E.fill
        , E.spacing 15
        ]
        [ E.el [ E.centerX, EF.bold ] <| E.text "inline xform"
        , E.column [ E.width E.fill, E.height E.fill, E.scrollbarY, E.spacing 8 ]
            [ EI.radio []
                { onChange = OnSelect
                , options = model.transforms |> List.indexedMap (\i ( name, _ ) -> EI.option i (E.text name))
                , selected = model.selectedTf
                , label = EI.labelAbove [] (E.text "xform")
                }
            , E.paragraph [ EBk.color TC.lightGray, E.padding 20 ] <|
                [ E.text
                    (model.selectedTf
                        |> Maybe.andThen
                            (\i ->
                                List.head (List.drop i model.transforms)
                            )
                        |> Maybe.andThen
                            (\( _, xform ) ->
                                case xform of
                                    Upd inline ->
                                        Markdown.Renderer.render stringRenderer [ MB.Paragraph [ inline ] ]
                                            |> Result.toMaybe

                                    Lb ( title, f ) ->
                                        let
                                            sn =
                                                { id = Data.Zni "new-note-id"
                                                , changeddate = 0
                                                , server = "local"
                                                , what = Nothing
                                                }
                                        in
                                        Markdown.Renderer.render stringRenderer [ MB.Paragraph [ f sn ] ]
                                            |> Result.toMaybe
                            )
                        |> Maybe.map String.concat
                        |> Maybe.withDefault
                            ""
                    )
                ]
            ]
        , E.row [ E.width E.fill, E.spacing 10 ]
            [ EI.button
                (E.alignLeft :: buttonStyle)
                { onPress = Just CancelClick, label = E.text "cancel" }
            , EI.button
                (E.alignRight :: buttonStyle)
                { onPress = Just OkClick, label = E.text "ok" }
            ]
        ]


update : Msg -> Model -> GD.Transition Model Command
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            model.selectedTf
                |> Maybe.andThen (\i -> List.head (List.drop i model.transforms))
                |> Maybe.map Tuple.second
                |> Maybe.map
                    (\xf ->
                        case xf of
                            Upd inl ->
                                (model.tomsg >> UpdateInline >> GD.Ok) inl

                            Lb ( s, f ) ->
                                GD.Ok <|
                                    LinkBack s
                                        (\sn ->
                                            UpdateInline <|
                                                model.tomsg <|
                                                    f sn
                                        )
                    )
                |> Maybe.withDefault GD.Cancel

        -- ClearClick jobno ->
        --     GD.Dialog { model | jobs = Dict.remove jobno model.jobs }
        OnSelect i ->
            GD.Dialog { model | selectedTf = Just i }

        Noop ->
            GD.Dialog model
