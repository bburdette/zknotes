module LinearMd exposing (..)

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
