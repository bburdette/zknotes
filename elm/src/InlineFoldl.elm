module InlineFoldl exposing (inlineFoldl)

import Markdown.Block as Block exposing (Block(..), Html(..), Inline(..), ListItem(..), Task(..), foldl)


{-| Fold over all inlines within a list of blocks to yield a value.

    import Markdown.Block as Block exposing (..)

    pullLinks : List Block -> List String
    pullLinks blocks =
        blocks
            |> inlineFoldl
                (\inline links ->
                    case inline of
                        Link str mbstr moreinlines ->
                            str :: links
                        _ ->
                            links
                )
                []

    [ Heading H1 [ Text "Document" ]
    , Heading H2 [ Link "/note/50" (Just "interesting document") [] ]
    , Heading H3 [ Text "Subsection" ]
    , Heading H2 [ Link "/note/51" (Just "more interesting document") [] ]
    ]
        |> pullLinks
    -->  ["/note/51", "/note/50"]

-}
inlineFoldl : (Inline -> acc -> acc) -> acc -> List Block -> acc
inlineFoldl ifunction top_acc list =
    let
        -- change a simple inline accum function to one that will fold over
        -- inlines contained within other inlines.
        inlineFoldF : (Inline -> acc -> acc) -> Inline -> acc -> acc
        inlineFoldF =
            \ifn inline acc ->
                case inline of
                    HtmlInline hblock ->
                        let
                            hiacc =
                                ifn inline acc
                        in
                        case hblock of
                            HtmlElement _ _ blocks ->
                                inlineFoldl ifn hiacc blocks

                            HtmlComment _ ->
                                ifn inline hiacc

                            ProcessingInstruction _ ->
                                ifn inline hiacc

                            HtmlDeclaration _ _ ->
                                ifn inline hiacc

                            Cdata _ ->
                                ifn inline hiacc

                    Link _ _ inlines ->
                        let
                            iacc =
                                ifn inline acc
                        in
                        List.foldl ifn iacc inlines

                    Image _ _ inlines ->
                        let
                            iacc =
                                ifn inline acc
                        in
                        List.foldl ifn iacc inlines

                    Emphasis inlines ->
                        let
                            iacc =
                                ifn inline acc
                        in
                        List.foldl ifn iacc inlines

                    Strong inlines ->
                        let
                            iacc =
                                ifn inline acc
                        in
                        List.foldl ifn iacc inlines

                    CodeSpan _ ->
                        ifn inline acc

                    Text _ ->
                        ifn inline acc

                    HardLineBreak ->
                        ifn inline acc

        function =
            inlineFoldF ifunction

        bfn =
            \block acc ->
                case block of
                    HtmlBlock html ->
                        acc

                    UnorderedList listItems ->
                        List.foldl
                            (\(ListItem _ inlines) liacc ->
                                List.foldl function liacc inlines
                            )
                            acc
                            listItems

                    OrderedList int lists ->
                        List.foldl
                            (\inlines lacc ->
                                List.foldl function lacc inlines
                            )
                            acc
                            lists

                    BlockQuote _ ->
                        acc

                    Heading _ inlines ->
                        List.foldl function acc inlines

                    Paragraph inlines ->
                        List.foldl function acc inlines

                    Table labels listlists ->
                        let
                            llacc =
                                List.foldl
                                    (\inlines iacc ->
                                        List.foldl function iacc inlines
                                    )
                                    acc
                                    (List.map .label labels)
                        in
                        List.foldl
                            (\lists lacc ->
                                List.foldl
                                    (\inlines iacc ->
                                        List.foldl function iacc inlines
                                    )
                                    lacc
                                    lists
                            )
                            llacc
                            listlists

                    CodeBlock _ ->
                        acc

                    ThematicBreak ->
                        acc
    in
    foldl bfn top_acc list
