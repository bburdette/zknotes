module LinearMd exposing (..)

import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Markdown.Block as MB
import TangoColors as TC


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


toBlocks : List MdElement -> Result String (List MB.Block)
toBlocks elements =
    let
        st =
            List.foldl mdfProc { blocks = [], result = Ok toMB } elements
    in
    case st.result of
        Err e ->
            Err e

        Ok _ ->
            Ok (List.reverse st.blocks)


type alias State =
    { blocks : List MB.Block
    , result : Result String Mdfn
    }


mdfProc : MdElement -> State -> State
mdfProc elt state =
    -- let
    --     _ =
    --         Debug.log "mdfProc elt, blocks" ( elt, state.blocks )
    -- in
    case state.result of
        Err _ ->
            state

        Ok mdfn ->
            case mdfn elt of
                Err e ->
                    { state | result = Err e }

                Ok (Mdeb block) ->
                    { state | blocks = block :: state.blocks, result = Ok toMB }

                Ok (Mdei inline) ->
                    { state | result = Err "unexpected inline" }

                Ok (Mdf fn) ->
                    { state | result = Ok fn }


type alias Mdfn =
    MdElement -> Result String Mdf


type Mdf
    = Mdf Mdfn
    | Mdeb MB.Block
    | Mdei MB.Inline


unorderedListItem : MB.ListSpacing -> List (MB.ListItem MB.Block) -> MdElement -> Result String Mdf
unorderedListItem listSpacing items mde =
    case mde of
        UnorderedListItem task blocks ->
            Ok (Mdf (unorderedListItem listSpacing (MB.ListItem task blocks :: items)))

        UnorderedListEnd ->
            Ok (Mdeb (MB.UnorderedList listSpacing (List.reverse items)))

        _ ->
            Err "unexpected item"


orderedListItem : MB.ListSpacing -> Int -> List (List MB.Block) -> MdElement -> Result String Mdf
orderedListItem listSpacing offset items mde =
    case mde of
        OrderedListItem blocks ->
            Ok (Mdf (orderedListItem listSpacing offset (blocks :: items)))

        OrderedListEnd ->
            Ok (Mdeb (MB.OrderedList listSpacing offset items))

        _ ->
            Err "unexpected item"


accumInlines : MdElement -> (List MB.Inline -> MB.Inline) -> List MB.Inline -> Mdfn -> MdElement -> Result String Mdf
accumInlines endelt endcontainer accum mdfn elt =
    if elt == endelt then
        Ok (Mdei (endcontainer (List.reverse accum)))

    else
        case mdfn elt of
            Err e ->
                Err e

            Ok (Mdeb blocks) ->
                Err "accumInlines - expected inlines not blocks"

            Ok (Mdei inline) ->
                Ok (Mdf <| accumInlines endelt endcontainer (inline :: accum) mdfn)

            Ok (Mdf fn) ->
                Ok (Mdf <| accumInlines endelt endcontainer accum fn)


accumInlinesToBlock : MdElement -> (List MB.Inline -> MB.Block) -> List MB.Inline -> Mdfn -> MdElement -> Result String Mdf
accumInlinesToBlock endelt endcontainer accum mdfn elt =
    if elt == endelt then
        -- Ok (Mdeb (Debug.log "return" (endcontainer (List.reverse accum))))
        Ok (Mdeb (endcontainer (List.reverse accum)))

    else
        -- let
        --     _ =
        --         Debug.log "mdfn elt, endelt" ( elt, endelt )
        -- in
        case mdfn elt of
            Err e ->
                Err e

            Ok (Mdeb block) ->
                -- let
                --     _ =
                --         Debug.log "blocjk" block
                -- in
                Err "accumInlinesToBlock - expected inlines not blocks"

            Ok (Mdei inline) ->
                Ok (Mdf <| accumInlinesToBlock endelt endcontainer (inline :: accum) mdfn)

            Ok (Mdf fn) ->
                Ok (Mdf <| accumInlinesToBlock endelt endcontainer accum fn)


accumBlocks : MdElement -> (List MB.Block -> MB.Block) -> List MB.Block -> Mdfn -> MdElement -> Result String Mdf
accumBlocks endelt endcontainer accum mdfn elt =
    if elt == endelt then
        Ok (Mdeb (endcontainer (List.reverse accum)))

    else
        case mdfn elt of
            Err e ->
                Err e

            Ok (Mdeb block) ->
                Ok (Mdf <| accumBlocks endelt endcontainer (block :: accum) mdfn)

            Ok (Mdei _) ->
                Err "expected blocks not inlines"

            Ok (Mdf fn) ->
                Ok (Mdf <| accumBlocks endelt endcontainer accum fn)


toMB : MdElement -> Result String Mdf
toMB mde =
    case mde of
        HtmlBlock hblock ->
            Ok (Mdeb <| MB.HtmlBlock hblock)

        UnorderedListStart listSpacing ->
            -- (List (ListItem Block))
            Ok (Mdf (unorderedListItem listSpacing []))

        UnorderedListItem _ _ ->
            Err "unexpected UnorderedListItem"

        UnorderedListEnd ->
            Err "unexpected UnorderedListEnd"

        OrderedListStart listSpacing offset ->
            -- (List (List Block))
            Ok (Mdf <| orderedListItem listSpacing offset [])

        OrderedListItem _ ->
            Err "unexpected OrderedListItem"

        OrderedListEnd ->
            Err "unexpected OrderedListEnd"

        BlockQuoteStart ->
            -- list blocks
            Ok (Mdf (accumBlocks BlockQuoteEnd MB.BlockQuote [] toMB))

        BlockQuoteEnd ->
            Err "unexpected BlockQuoteEnd"

        -- Leaf Blocks With Inlines
        HeadingStart headingLevel ->
            Ok (Mdf (accumInlinesToBlock HeadingEnd (MB.Heading headingLevel) [] toMB))

        HeadingEnd ->
            Err "unexpected HeadingEnd"

        ParagraphStart ->
            Ok (Mdf (accumInlinesToBlock ParagraphEnd MB.Paragraph [] toMB))

        ParagraphEnd ->
            Err "unexpected ParagraphEnd"

        Table heading data ->
            Ok (Mdeb (MB.Table heading data))

        -- Leaf Blocks Without Inlines
        CodeBlock code ->
            Ok (Mdeb (MB.CodeBlock code))

        ThematicBreak ->
            Ok (Mdeb MB.ThematicBreak)

        -- Inlines
        HtmlInline hblock ->
            Err "unexpectd HtmlInline"

        Link url maybeTitle inlines ->
            Ok (Mdei (MB.Link url maybeTitle inlines))

        Image url maybeTitle inlines ->
            Ok (Mdei (MB.Image url maybeTitle inlines))

        EmphasisBegin ->
            Ok (Mdf (accumInlines EmphasisEnd MB.Emphasis [] toMB))

        EmphasisEnd ->
            Err "unexpected EmphasisEnd"

        StrongBegin ->
            Ok (Mdf (accumInlines StrongEnd MB.Emphasis [] toMB))

        StrongEnd ->
            Err "unexpected StrongEnd"

        StrikethroughBegin ->
            Ok (Mdf (accumInlines StrikethroughEnd MB.Emphasis [] toMB))

        StrikethroughEnd ->
            Err "unexpected StrikethroughEnd"

        CodeSpan string ->
            Ok (Mdei (MB.CodeSpan string))

        Text string ->
            Ok (Mdei (MB.Text string))

        HardLineBreak ->
            Ok (Mdei MB.HardLineBreak)


viewMdElement : MdElement -> Element msg
viewMdElement b =
    E.el [ EF.color TC.white ] <|
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
                E.column []
                    [ E.text "Link"
                    , E.text <| "url " ++ url
                    , E.text <| "title " ++ Maybe.withDefault "" maybeTitle
                    ]

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
