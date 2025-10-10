module MdInlineXform exposing (..)

import Common
import Data exposing (ZkNoteId)
import DataUtil exposing (zkNoteIdToString)
import Dict exposing (Dict)
import EdMarkdown exposing (stringRenderer)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import GenDialog as GD
import Html.Attributes as HA
import Http
import Markdown.Block as MB exposing (Inline(..))
import Markdown.Renderer
import MdGui as MG exposing (findAttrib)
import Orgauth.Data as Data
import Route exposing (Route(..))
import TangoColors as TC
import Toop
import Url exposing (Url)
import Util


type alias Model =
    { mobile : Bool
    , inline : MB.Inline
    , transforms : List ( String, MB.Inline )
    , selectedTf : Maybe Int
    , tomsg : MB.Inline -> MG.Msg
    }


type Msg
    = OkClick
    | CancelClick
    | OnSelect Int
    | Noop


type Command
    = UpdateInline MG.Msg
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


transforms : MB.Inline -> List ( String, MB.Inline )
transforms inline =
    case inline of
        HtmlInline htmlBlock ->
            case htmlBlock of
                MB.HtmlElement "note" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "id" attribs, findAttrib "show" attribs ) of
                        ( mbtext, Just noteid, show ) ->
                            [ ( "none", inline )
                            , ( "link", MB.Link ("/note/" ++ noteid) Nothing [ MB.Text (Maybe.withDefault "" mbtext) ] )
                            , ( "md image", MB.Image ("/file/" ++ noteid) mbtext [] )
                            , ( "zkn image"
                              , MB.HtmlInline
                                    (MB.HtmlElement "image"
                                        (List.filterMap identity
                                            [ Just { name = "url", value = "/file/" ++ noteid }
                                            , mbtext
                                                |> Maybe.map (\s -> { name = "text", value = s })
                                            ]
                                        )
                                        []
                                    )
                              )
                            , ( "zkn video"
                              , MB.HtmlInline
                                    (MB.HtmlElement "video"
                                        (List.filterMap identity
                                            [ Just { name = "src", value = "/file/" ++ noteid }
                                            , mbtext
                                                |> Maybe.map (\s -> { name = "text", value = s })
                                            ]
                                        )
                                        []
                                    )
                              )
                            , ( "zkn audio"
                              , MB.HtmlInline
                                    (MB.HtmlElement "audio"
                                        (List.filterMap identity
                                            [ Just { name = "src", value = "/file/" ++ noteid }
                                            , mbtext
                                                |> Maybe.map (\s -> { name = "text", value = s })
                                            ]
                                        )
                                        []
                                    )
                              )
                            , ( "zkn panel"
                              , MB.HtmlInline
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
                            [ ( "none", inline )
                            , ( "note link"
                              , MB.HtmlInline
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
                            [ ( "none", inline )
                            , ( "md image", MB.Image url (Just text) [] )
                            , ( "link", MB.Link url (Just text) [] )
                            ]

                        _ ->
                            []

                MB.HtmlElement "video" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "src" attribs ) of
                        ( Just text, Just src ) ->
                            [ ( "none", inline )
                            , ( "link", MB.Link src (Just text) [] )
                            ]

                        _ ->
                            []

                MB.HtmlElement "audio" attribs childs ->
                    case ( findAttrib "text" attribs, findAttrib "src" attribs ) of
                        ( Just text, Just src ) ->
                            [ ( "none", inline )
                            , ( "link", MB.Link src (Just text) [] )
                            ]

                        _ ->
                            []

                _ ->
                    []

        {-
           text -> search?
           note -> audio
           note -> video
        -}
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
                [ Just ( "none", inline )
                , Just
                    ( "yeet"
                    , MB.HtmlInline
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
                            , MB.HtmlInline
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
                , Just ( "md image", MB.Image url mbt inlines )
                , Just
                    ( "zkn image"
                    , MB.HtmlInline
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
            [ ( "none", inline )
            , ( "link", MB.Link src mbt inlines )
            , ( "zkn image"
              , MB.HtmlInline
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
              , case inlines of
                    [ item ] ->
                        item

                    plural ->
                        -- can't return multiple inlines.  TODO fix?
                        Emphasis inlines
              )
            ]

        Strong inlines ->
            [ ( "remove"
              , case inlines of
                    [ item ] ->
                        item

                    plural ->
                        -- can't return multiple inlines.  TODO fix?
                        Strong inlines
              )
            ]

        Strikethrough inlines ->
            [ ( "remove"
              , case inlines of
                    [ item ] ->
                        item

                    plural ->
                        -- can't return multiple inlines.  TODO fix?
                        Strikethrough inlines
              )
            ]

        CodeSpan s ->
            [ ( "none", inline )
            , ( "text", MB.Text s )
            ]

        Text s ->
            [ ( "none", inline )
            , ( "strong", MB.Strong [ MB.Text s ] )
            , ( "emphasis", MB.Emphasis [ MB.Text s ] )
            , ( "strikethrough", MB.Strikethrough [ MB.Text s ] )
            , ( "codespan", MB.CodeSpan s )
            , ( "link - url", MB.Link s Nothing [ MB.Text "" ] )
            , ( "link - text", MB.Link "" Nothing [ MB.Text s ] )
            , ( "yeet", MB.HtmlInline (MB.HtmlElement "yeet" [ { name = "url", value = s } ] []) )
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
                            (\( _, inline ) ->
                                Markdown.Renderer.render stringRenderer [ MB.Paragraph [ inline ] ]
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


okResult : GD.Model Model Msg Command -> GD.Transition Model Command
okResult gdm =
    let
        model =
            gdm.model
    in
    okResultInternal model


okResultInternal : Model -> GD.Transition Model Command
okResultInternal model =
    model.selectedTf
        |> Maybe.andThen (\i -> List.head (List.drop i model.transforms))
        |> Maybe.map Tuple.second
        |> Maybe.map (model.tomsg >> UpdateInline >> GD.Ok)
        |> Maybe.withDefault GD.Cancel


update : Msg -> Model -> GD.Transition Model Command
update msg model =
    case msg of
        CancelClick ->
            GD.Cancel

        OkClick ->
            okResultInternal model

        -- ClearClick jobno ->
        --     GD.Dialog { model | jobs = Dict.remove jobno model.jobs }
        OnSelect i ->
            GD.Dialog { model | selectedTf = Just i }

        Noop ->
            GD.Dialog model
