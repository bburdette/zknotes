module MdGui exposing (guiBlock)

import Element as E
import Element.Input as EI
import Markdown.Block as MB exposing (..)



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


guiBlock : MB.Block -> E.Element Msg
guiBlock block =
    case block of
        HtmlBlock htmlBlock ->
            E.none

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
            E.none

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

guiHtmlElement : String -> List HtmlAttribute -> E.Element Msg
guiHtmlElement tag attribs =
    case tag of
        "cell" ->
            E.column []
                [ EI.text []
                    { onChange = CellName
                    , text =  |> Maybe.withDefault ""
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "name")
                    },
            EI.multiline []
                    { onChange = CellScript
                    , text = cb.body
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "script")
                    , spellcheck = False
                    }
                    ]
                    
                
            E.none

        "search" ->
            E.none

        "panel" ->
            E.none

        "image" ->
            E.none

        "video" ->
            E.none

        "audio" ->
            E.none

        "note" ->
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
