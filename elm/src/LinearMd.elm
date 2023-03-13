module LinearMd exposing (MdElement(..), lpad, mbToMe, miToMe, viewMdElement, viewhtml)

import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Markdown.Block as MB


type MdElement
    = -- Container Blocks
      HtmlBlock (MB.Html MB.Block)
    | UnorderedListStart MB.ListSpacing -- (List (ListItem Block))
    | UnorderedListItem MB.Task (List MB.Block)
    | UnorderedListEnd
    | OrderedListStart MB.ListSpacing Int -- (List (List Block))
    | OrderedListItem (List MB.Block)
    | OrderedListEnd
    | BlockQuoteStart -- list blocks
    | BlockQuoteEnd
      -- Leaf Blocks With Inlines
    | HeadingStart MB.HeadingLevel
    | HeadingEnd
    | ParagraphStart
    | ParagraphEnd
    | Table (List { label : List MB.Inline, alignment : Maybe MB.Alignment }) (List (List (List MB.Inline)))
      -- Leaf Blocks Without Inlines
    | CodeBlock { body : String, language : Maybe String }
    | ThematicBreak
      -- Inlines
    | HtmlInline (MB.Html MB.Block)
    | Link String (Maybe String) (List MB.Inline)
    | Image String (Maybe String) (List MB.Inline)
    | EmphasisBegin
    | EmphasisEnd
    | StrongBegin
    | StrongEnd
    | StrikethroughBegin
    | StrikethroughEnd
    | CodeSpan String
    | Text String
    | HardLineBreak


viewMdElement : MdElement -> Element msg
viewMdElement b =
    case b of
        HtmlBlock hblock ->
            viewhtml hblock

        UnorderedListStart listSpacing ->
            E.text <| "UnorderedListStart"

        UnorderedListItem task blocks ->
            E.text "UnorderedListItem"

        UnorderedListEnd ->
            E.text "UnorderedListEnd"

        OrderedListStart listSpacing offset ->
            E.text "OrderedListStart"

        OrderedListItem blocks ->
            E.text "OrderedListItem"

        OrderedListEnd ->
            E.text "OrderedListEnd"

        BlockQuoteStart ->
            E.text "BlockQuoteStart"

        BlockQuoteEnd ->
            E.text "BlockQuoteEnd"

        HeadingStart headingLevel ->
            E.text "HeadingStart"

        HeadingEnd ->
            E.text "HeadingEnd"

        ParagraphStart ->
            E.text "ParagraphStart"

        ParagraphEnd ->
            E.text "ParagraphEnd"

        Table headings items ->
            E.text "Table"

        -- Leaf Blocks Without Inlines -> E.text "--"
        CodeBlock code ->
            E.text "CodeBlock"

        ThematicBreak ->
            E.text "ThematicBreak"

        -- Inlines
        HtmlInline hblock ->
            viewhtml hblock

        Link url maybeTitle inlines ->
            E.text "Link"

        Image url maybeTitle inlines ->
            E.text "Image"

        EmphasisBegin ->
            E.text "EmphasisBegin"

        EmphasisEnd ->
            E.text "EmphasisEnd"

        StrongBegin ->
            E.text "StrongBegin"

        StrongEnd ->
            E.text "StrongEnd"

        StrikethroughBegin ->
            E.text "StrikethroughBegin"

        StrikethroughEnd ->
            E.text "StrikethroughEnd"

        CodeSpan string ->
            E.text "CodeSpan"

        Text string ->
            E.column []
                [ E.text <| "Text"
                , E.text string
                ]

        HardLineBreak ->
            E.text "HardLineBreak"


lpad : E.Attribute msg
lpad =
    E.paddingEach
        { top = 0, right = 0, bottom = 0, left = 10 }


viewhtml : MB.Html MB.Block -> Element msg
viewhtml htmlb =
    E.column []
        [ E.text "HtmlBlock"
        , case htmlb of
            MB.HtmlElement string listHtmlAttribute listChildren ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "HtmlElement "
                            ++ string

            MB.HtmlComment string ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "HtmlComment "
                            ++ string

            MB.ProcessingInstruction string ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "ProcessingInstruction "
                            ++ string

            MB.HtmlDeclaration string1 string2 ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "HtmlDeclaration "
                            ++ string1
                            ++ " "
                            ++ string2

            MB.Cdata string ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "Cdata "
                            ++ string
        ]


mbToMe : MB.Block -> List MdElement
mbToMe block =
    case block of
        MB.HtmlBlock hblock ->
            [ HtmlBlock hblock ]

        MB.UnorderedList listSpacing listitems ->
            UnorderedListStart listSpacing
                :: List.map (\(MB.ListItem task blocks) -> UnorderedListItem task blocks) listitems
                ++ [ UnorderedListEnd ]

        MB.OrderedList listSpacing offset items ->
            OrderedListStart listSpacing offset
                :: List.map OrderedListItem items
                ++ [ OrderedListEnd ]

        MB.BlockQuote blocks ->
            BlockQuoteStart
                :: (List.map mbToMe blocks
                        |> List.concat
                   )
                ++ [ BlockQuoteEnd ]

        MB.Heading headingLevel inlines ->
            HeadingStart headingLevel
                :: (List.map miToMe inlines
                        |> List.concat
                   )
                ++ [ HeadingEnd ]

        MB.Paragraph inlines ->
            ParagraphStart
                :: (List.map miToMe inlines
                        |> List.concat
                   )
                ++ [ ParagraphEnd ]

        MB.Table headings items ->
            [ Table headings items ]

        MB.CodeBlock code ->
            [ CodeBlock code ]

        MB.ThematicBreak ->
            [ ThematicBreak ]


miToMe : MB.Inline -> List MdElement
miToMe inline =
    case inline of
        MB.HtmlInline hblock ->
            [ HtmlInline hblock ]

        MB.Link url maybeTitle inlines ->
            [ Link url maybeTitle inlines ]

        MB.Image url maybeTitle inlines ->
            [ Image url maybeTitle inlines ]

        MB.Emphasis inlines ->
            EmphasisBegin
                :: (List.map miToMe inlines |> List.concat)
                ++ [ EmphasisEnd ]

        MB.Strong inlines ->
            StrongBegin
                :: (List.map miToMe inlines |> List.concat)
                ++ [ StrongEnd ]

        MB.Strikethrough inlines ->
            StrikethroughBegin
                :: (List.map miToMe inlines |> List.concat)
                ++ [ StrikethroughEnd ]

        MB.CodeSpan string ->
            [ CodeSpan string ]

        MB.Text string ->
            [ Text string ]

        MB.HardLineBreak ->
            [ HardLineBreak ]
