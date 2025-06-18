module MdGui exposing (guiBlock)

import Element as E
import Element.Border as EBd
import Element.Font as EF
import Element.Input as EI
import Markdown.Block as MB exposing (..)
import MdCommon as MC
import Toop



{-

   type Block
       = -- Container Blocks
         HtmlBlock (Html Block)
       | UnorderedList ListSpacing (List (ListItem Block))
       | OrderedList ListSpacing Int (List (List Block))
       | BlockQuote (List Block)
         -- Leaf Blocks With Inlines
       | Heading HeadingLevel (List Inline)
       | Paragraph (List Inline)
       | Table (List { label : List Inline, alignment : Maybe Alignment }) (List (List (List Inline)))
         -- Leaf Blocks Without Inlines
       | CodeBlock { body : String, language : Maybe String }
       | ThematicBreak


   type ListSpacing
       = Loose
       | Tight


   type Alignment
       = AlignLeft
       | AlignRight
       | AlignCenter


   type ListItem children
       = ListItem Task (List children)


   type Task
       = NoTask
       | IncompleteTask
       | CompletedTask


   type HeadingLevel
       = H1
       | H2
       | H3
       | H4
       | H5
       | H6

   type Inline
       = HtmlInline (Html Block)
       | Link String (Maybe String) (List Inline)
       | Image String (Maybe String) (List Inline)
       | Emphasis (List Inline)
       | Strong (List Inline)
       | Strikethrough (List Inline)
       | CodeSpan String
       | Text String
       | HardLineBreak

-}


type Msg
    = CbLanguage String
    | CbBody String
    | CellName String
    | CellScript String
    | SearchText String
    | NoteIdText String
    | ImageText String
    | ImageUrl String
    | ImageWidth String
    | VideoSrc String
    | VideoText String
    | VideoWidth String
    | VideoHeight String
    | AudioSrc String
    | AudioText String
    | NoteSrc String
    | NoteShow String
    | NoteText String
    | NoteShowContents Bool
    | NoteShowText Bool
    | NoteShowFile Bool
    | NoteShowCreatedate Bool
    | NoteShowChangedate Bool
    | NoteShowLink Bool


guiBlock : MB.Block -> E.Element Msg
guiBlock block =
    case block of
        HtmlBlock htmlBlock ->
            guiHtml htmlBlock

        UnorderedList listSpacing listItems ->
            E.none

        OrderedList listSpacing startIndex blockLists ->
            E.none

        BlockQuote blocks ->
            E.none

        Heading headingLevel inlines ->
            E.none

        Paragraph inlines ->
            E.none

        Table headings inlines ->
            E.none

        CodeBlock cb ->
            E.column [ E.width E.fill ]
                [ EI.text []
                    { onChange = CbLanguage
                    , text = cb.language |> Maybe.withDefault ""
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "language")
                    }
                , EI.multiline []
                    { onChange = CbBody
                    , text = cb.body
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "body")
                    , spellcheck = False
                    }
                ]

        ThematicBreak ->
            E.text "----------------------"


guiHtml : Html Block -> E.Element Msg
guiHtml block =
    case block of
        HtmlElement tag attribs _ ->
            guiHtmlElement tag attribs

        HtmlComment _ ->
            E.none

        ProcessingInstruction _ ->
            E.none

        HtmlDeclaration _ _ ->
            E.none

        Cdata _ ->
            E.none



-- [ Markdown.Html.tag "cell" hf.schelmeView
--     |> Markdown.Html.withAttribute "name"
--     |> Markdown.Html.withAttribute "schelmecode"
-- , Markdown.Html.tag "search" hf.searchView
--     |> Markdown.Html.withAttribute "query"
-- , Markdown.Html.tag "panel" hf.panelView
--     |> Markdown.Html.withAttribute "noteid"
-- , Markdown.Html.tag "image" hf.imageView
--     |> Markdown.Html.withAttribute "text"
--     |> Markdown.Html.withAttribute "url"
--     |> Markdown.Html.withOptionalAttribute "width"
-- , Markdown.Html.tag "video" hf.videoView
--     |> Markdown.Html.withAttribute "src"
--     |> Markdown.Html.withOptionalAttribute "text"
--     |> Markdown.Html.withOptionalAttribute "width"
--     |> Markdown.Html.withOptionalAttribute "height"
-- , Markdown.Html.tag "audio" hf.audioView
--     |> Markdown.Html.withAttribute "text"
--     |> Markdown.Html.withAttribute "src"
-- , Markdown.Html.tag "note" hf.noteView
--     |> Markdown.Html.withAttribute "id"
--     |> Markdown.Html.withOptionalAttribute "show"
--     |> Markdown.Html.withOptionalAttribute "text"
-- ]


findAttrib : String -> List HtmlAttribute -> Maybe String
findAttrib name attribs =
    attribs
        |> List.filter (\a -> a.name == name)
        |> List.map .value
        |> List.head


guiHtmlElement : String -> List HtmlAttribute -> E.Element Msg
guiHtmlElement tag attribs =
    case tag of
        "cell" ->
            case ( findAttrib "name" attribs, findAttrib "script" attribs ) of
                ( Just name, Just script ) ->
                    E.column []
                        [ EI.text []
                            { onChange = CellName
                            , text = name
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "name")
                            }
                        , EI.multiline []
                            { onChange = CellScript
                            , text = script
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "script")
                            , spellcheck = False
                            }
                        ]

                _ ->
                    E.none

        "search" ->
            case findAttrib "search" attribs of
                Just search ->
                    E.column []
                        [ EI.text []
                            { onChange = SearchText
                            , text = search
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "search")
                            }
                        ]

                _ ->
                    E.none

        "panel" ->
            case findAttrib "noteid" attribs of
                Just noteid ->
                    E.column []
                        [ EI.text []
                            { onChange = NoteIdText
                            , text = noteid
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "noteid")
                            }
                        ]

                _ ->
                    E.none

        "image" ->
            case ( findAttrib "text" attribs, findAttrib "url" attribs, findAttrib "width" attribs ) of
                ( Just name, Just url, mbwidth ) ->
                    E.column []
                        [ EI.text []
                            { onChange = ImageText
                            , text = name
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "name")
                            }
                        , EI.text []
                            { onChange = ImageUrl
                            , text = url
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "url")
                            }
                        , EI.text []
                            { onChange = ImageWidth
                            , text = mbwidth |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbwidth")
                            }
                        ]

                _ ->
                    E.none

        "video" ->
            case Toop.T4 (findAttrib "src" attribs) (findAttrib "text" attribs) (findAttrib "width" attribs) (findAttrib "height" attribs) of
                Toop.T4 (Just src) mbtext mbwidth mbheight ->
                    E.column []
                        [ EI.text []
                            { onChange = VideoSrc
                            , text = src
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "src")
                            }
                        , EI.text []
                            { onChange = VideoText
                            , text = mbtext |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbtext")
                            }
                        , EI.text []
                            { onChange = VideoWidth
                            , text = mbwidth |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbwidth")
                            }
                        , EI.text []
                            { onChange = VideoHeight
                            , text = mbheight |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbheight")
                            }
                        ]

                _ ->
                    E.none

        "audio" ->
            case Toop.T2 (findAttrib "src" attribs) (findAttrib "text" attribs) of
                Toop.T2 (Just src) (Just text) ->
                    E.column []
                        [ EI.text []
                            { onChange = AudioSrc
                            , text = src
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "src")
                            }
                        , EI.text []
                            { onChange = AudioText
                            , text = text
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "text")
                            }
                        ]

                _ ->
                    E.none

        "note" ->
            case ( findAttrib "id" attribs, findAttrib "show" attribs, findAttrib "text" attribs ) of
                ( Just noteid, mbshow, mbtext ) ->
                    let
                        ns =
                            mbshow
                                |> Maybe.map MC.parseNoteShow
                                |> Maybe.withDefault
                                    { title = False
                                    , contents = False
                                    , text = False
                                    , file = False
                                    , createdate = False
                                    , changedate = False
                                    , link = False
                                    }
                    in
                    E.column []
                        [ EI.text []
                            { onChange = NoteSrc
                            , text = noteid
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "noteid")
                            }
                        , E.column [ EBd.width 1, E.padding 8, E.spacing 3 ]
                            [ E.el [ EF.bold ] <| E.text "show"
                            , EI.checkbox []
                                { onChange = NoteShowContents
                                , icon = EI.defaultCheckbox
                                , checked = ns.contents
                                , label = EI.labelRight [] (E.text "contents")
                                }
                            , EI.checkbox []
                                { onChange = NoteShowText
                                , icon = EI.defaultCheckbox
                                , checked = ns.text
                                , label = EI.labelRight [] (E.text "text")
                                }
                            , EI.checkbox []
                                { onChange = NoteShowFile
                                , icon = EI.defaultCheckbox
                                , checked = ns.file
                                , label = EI.labelRight [] (E.text "file")
                                }
                            , EI.checkbox []
                                { onChange = NoteShowCreatedate
                                , icon = EI.defaultCheckbox
                                , checked = ns.createdate
                                , label = EI.labelRight [] (E.text "createdate")
                                }
                            , EI.checkbox []
                                { onChange = NoteShowChangedate
                                , icon = EI.defaultCheckbox
                                , checked = ns.changedate
                                , label = EI.labelRight [] (E.text "changedate")
                                }
                            , EI.checkbox []
                                { onChange = NoteShowLink
                                , icon = EI.defaultCheckbox
                                , checked = ns.link
                                , label = EI.labelRight [] (E.text "link")
                                }
                            ]
                        , EI.text []
                            { onChange = NoteText
                            , text = mbtext |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "text")
                            }
                        ]

                _ ->
                    E.none

        _ ->
            E.none


guiInline : MB.Inline -> E.Element Msg
guiInline inline =
    case inline of
        HtmlInline block ->
            guiHtml block

        Link url mbtitle inlines ->
            E.none

        Image src mbtitle inlines ->
            E.none

        Emphasis inlines ->
            E.none

        Strong inlines ->
            E.none

        Strikethrough inlines ->
            E.none

        CodeSpan s ->
            E.none

        Text s ->
            E.none

        HardLineBreak ->
            E.none
