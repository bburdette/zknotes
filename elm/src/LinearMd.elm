module LinearMd exposing (MdElement(..), lpad, mbToMe, miToMe, viewMdElement, viewhtml)

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


type alias State =
    { mdbstack : List Mdb
    , mdf : Mdf
    , result : Result String (List MB.Block)
    }


mdfProc : MdElement -> State -> State
mdfProc elt state =
    case state.result of 
        Err _ -> state
        Ok mbs -> 
            case state.mdf mbs elt of
                Err e -> {state | result - Err e}
                Ok nmbs ->  
        
    case List.head state.mdbstack of
        Just mdf ->
            case mdf of
                Mdbf fn ->
                    state

                Mdif fn ->
                    state

        Nothing ->
            state


toBlocks : List MdElement -> Result String (List MB.Block)
toBlocks elements =
    List.foldl
        mdfProc
        { mdbstack = [], mdf = Mdf toBlock, result = Ok [] }
        elements
        |> .result



-- stack 'o states.
-- state is a function.
-- state can





type Mdf
    = Mdfb (MdElement -> Result String ( Mdf, List MB.Block ))
    | Mdfi (MdElement -> Result String ( Mdf, List MB.Inline ))
    | Mdfe


-- type Mdb
--     = Mdbf (List MB.Block -> ( Mdf, List MB.Block ))
--     | Mdif (List MB.Inline -> ( Mdf, List MB.Block ))


unorderedListItem : MB.ListSpacing -> List (MB.ListItem MB.Block) ->  MdElement -> Result String ( Mdf, List MB.Block )
unorderedListItem listSpacing items  mde =
    case mde of
        UnorderedListItem task blocks ->
            Ok ( Mdf (unorderedListItem listSpacing (MB.ListItem task blocks :: items)), [] )

        UnorderedListEnd ->
            Ok ( Mdf toBlock, [MB.UnorderedList listSpacing (List.reverse items)] )

        _ ->
            Err "unexpected item"


orderedListItem : MB.ListSpacing -> Int -> List (List MB.Block)  -> MdElement -> Result String ( Mdf, List MB.Block )
orderedListItem listSpacing offset items mbs mde =
    case mde of
        OrderedListItem blocks ->
            Ok ( Mdf (orderedListItem listSpacing offset (blocks :: items)), [] )

        OrderedListEnd ->
            Ok ( Mdf toBlock, [MB.OrderedList listSpacing offset items ])

        _ ->
            Err "unexpected item"

-- blockQuote : List MB.Block -> 

accumInlinesToB : MdElement -> (List MB.Inline -> MB.Inline) -> List MB.Inline -> MdElement -> Result String (Mdf, List MB.Block)
accumInlinesToB endelt endcontainer accum elt =
    if elt == endelt then 
        Ok (Mdfe, endcontainer accum)
    else
         
    
accumInlines : MdElement -> (List MB.Inline -> MB.Inline) -> List MB.Inline -> MdElement -> Result String (Mdf, List MB.Inline)
accumInlines endelt endcontainer accum elt =
    case toInline elt of 
        Mdfb -> 
            Err "bad accum"
        Mdfi fn -> 
            

    
andThenInline : (MdElement -> Result String ( Mdf, List MB.Inline )) 
    -> (MdElement -> Result String ( Mdf, List MB.Inline )) 
    -> MdElement -> Result String ( Mdf, List MB.Inline )
andThenInline fn1 fn2 elt =
    case fn2 elt of 
        (Mdfi newf -> 
             
    

toBlock :  MdElement -> Result String ( Mdf, List MB.Block )
toBlock  mde =
   case mde of
       HtmlBlock hblock ->
           Ok ( Mdf toBlock, [MB.HtmlBlock hblock] )
       UnorderedListStart listSpacing  -> -- (List (ListItem Block))
           Ok ( Mdf unorderedListItem listSpacing [], [] )
       UnorderedListItem _ _ ->
         Err "unexpected UnorderedListItem"
       UnorderedListEnd ->
         Err "unexpected UnorderedListEnd"
       OrderedListStart listSpacing offset  -> -- (List (List Block))
           Ok ( Mdf orderedListItem listSpacing offset [], [] )
       OrderedListItem _ ->
         Err "unexpected OrderedListItem"
       OrderedListEnd ->
         Err "unexpected OrderedListEnd"
       BlockQuoteStart ->  -- list blocks
            Ok ( Mdfb MB.BlockQuote toBlock, [])
       BlockQuoteEnd -> 
            Ok (Mdfe, mbs)
       -- Leaf Blocks With Inlines
       HeadingStart headingLevel -> 
            Ok (Mdfi MB.Heading )
       HeadingEnd -> 
       ParagraphStart -> 
            Ok (Mdfi MB.Paragraph )
       ParagraphEnd -> 
            Err "unexpected ParagraphEnd"
       Table heading data -> 
             Ok (Mdf toBlock [MB.Table heading data ])
       -- Leaf Blocks Without Inlines
       CodeBlock code 
            -> Ok (Mdf toBlock [MB.CodeBlock code ])
       ThematicBreak -> Ok (Mdf toBlock [MB.ThematicBreak ])
       -- Inlines
       HtmlInline hblock ->
            Err "unexpected HtmlInline; expected block"
       Link url maybeTitle inlines ->
            Err "unexpected Link; expected block"
       Image  url maybeTitle inlines ->
            Err "unexpected Image; expected block"
       EmphasisBegin ->
            Err "unexpected EmphasisBegin; expected block"
       EmphasisEnd ->
            Err "unexpected EmphasisEnd; expected block"
       StrongBegin ->
            Err "unexpected StrongBegin; expected block"
       StrongEnd ->
            Err "unexpected StrongEnd; expected block"
       StrikethroughBegin ->
            Err "unexpected StrikethroughBegin; expected block"
       StrikethroughEnd ->
            Err "unexpected StrikethroughEnd; expected block"
       CodeSpan String ->
            Err "unexpected CodeSpan; expected block"
       Text String ->
            Err "unexpected Text; expected block"
       HardLineBreak ->
            Err "unexpected HardLineBreak; expected block"

toInline : MdElement -> Result String (Mdf, List MB.Inline)
toInline mde = 
   case mde of
       HtmlBlock hblock ->
            Err "unexpectd HtmlBlock"
       UnorderedListStart listSpacing  -> -- (List (ListItem Block))
            Err "unexpectd UnorderedListStart"
       UnorderedListItem _ _ ->
         Err "unexpected UnorderedListItem"
       UnorderedListEnd ->
         Err "unexpected UnorderedListEnd"
       OrderedListStart listSpacing offset  -> -- (List (List Block))
            Err "unexpectd OrderedListStart"
       OrderedListItem _ ->
         Err "unexpected OrderedListItem"
       OrderedListEnd ->
         Err "unexpected OrderedListEnd"
       BlockQuoteStart ->  -- list blocks
            Err "unexpectd BlockQuoteStart"
       BlockQuoteEnd -> 
            Err "unexpectd BlockQuoteEnd"
       -- Leaf Blocks With Inlines
       HeadingStart headingLevel -> 
            Err "unexpectd HeadingStart"
       HeadingEnd -> 
            Err "unexpected HeadingEnd"
       ParagraphStart -> 
            Err "unexpectd ParagraphStart"
       ParagraphEnd -> 
            Err "unexpected ParagraphEnd"
       Table heading data -> 
            Err "unexpectd Table"
       -- Leaf Blocks Without Inlines
       CodeBlock code 
            Err "unexpectd CodeBlock"
       ThematicBreak -> 
            Err "unexpectd ThematicBreak"
       -- Inlines
       HtmlInline hblock ->
            Err "unexpectd HtmlInline"
       Link url maybeTitle inlines -> 
         Ok (Mdfe, [MB.Link url maybeTitle inlines])
       Image  url maybeTitle inlines ->
         Ok (Mdfe, [MB.Image url maybeTitle inlines])
       EmphasisBegin -> 
        Ok (Mdfi (accumInlines EmphasisEnd MB.Emphasis [] ))
       EmphasisEnd -> Err "unexpected EmphasisEnd"
            
       StrongBegin ->
        Ok (Mdfi (accumInlines StrongEnd MB.Emphasis [] ))
            
       StrongEnd ->Err "unexpected StrongEnd"
            
       StrikethroughBegin ->
        Ok (Mdfi (accumInlines StrikethroughEnd MB.Emphasis [] ))
            
       StrikethroughEnd ->Err "unexpected StrikethroughEnd"
            
       CodeSpan string -> 
            
            Ok(Mdfe, [MB.CodeSpan string])
       Text string ->
            Ok(Mdfe, [MB.CodeSpan string])
       HardLineBreak ->
            Ok(Mdfe, [MB.HardLineBreak])



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
