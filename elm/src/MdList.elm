module MdList exposing (mdblockview)

import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Markdown.Block as Block exposing (Block(..), Html(..), Inline(..), ListItem(..), Task(..), inlineFoldl)
import Markdown.Html


mdblockview parsedMd =
    case parsedMd of
        Err e ->
            []

        Ok bs ->
            bs
                |> List.map
                    (\b ->
                        case b of
                            HtmlBlock htmlb ->
                                viewhtml htmlb

                            UnorderedList listSpacing blockListItems ->
                                E.text "UnorderedList"

                            OrderedList listSpacing offset listListBlocks ->
                                E.text "OrderedList"

                            BlockQuote blocks ->
                                E.text "BlockQuote"

                            Heading headingLevel inlines ->
                                E.column []
                                    [ E.text "Heading"
                                    , E.column [] (List.map viewinline inlines)
                                    ]

                            Paragraph inlines ->
                                E.column []
                                    [ E.text "Paragraph"
                                    , E.column [] (List.map viewinline inlines)
                                    ]

                            Table headings inlines ->
                                E.column []
                                    [ E.text "Table"

                                    -- , E.column [] (List.map viewinline inlines)
                                    ]

                            CodeBlock bodyLanguage ->
                                E.text "CodeBlock"

                            ThematicBreak ->
                                E.text "ThematicBreak"
                    )


lpad =
    E.paddingEach
        { top = 0, right = 0, bottom = 0, left = 10 }


viewhtml htmlb =
    E.column []
        [ E.text "HtmlBlock"
        , case htmlb of
            HtmlElement string listHtmlAttribute listChildren ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "HtmlElement "
                            ++ string

            HtmlComment string ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "HtmlComment "
                            ++ string

            ProcessingInstruction string ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "ProcessingInstruction "
                            ++ string

            HtmlDeclaration string1 string2 ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "HtmlDeclaration "
                            ++ string1
                            ++ " "
                            ++ string2

            Cdata string ->
                E.el
                    [ lpad ]
                <|
                    E.text <|
                        "Cdata "
                            ++ string
        ]


viewinline inline =
    case inline of
        HtmlInline html ->
            E.row [ lpad ]
                [ E.text <| "HtmlInline "
                , viewhtml html
                ]

        Link string maybeString inlines ->
            E.row [ lpad ]
                (E.text
                    "Link "
                    :: List.map viewinline inlines
                )

        Image string maybeString inlines ->
            E.row [ lpad ]
                (E.text
                    "Image "
                    :: List.map viewinline inlines
                )

        Emphasis inlines ->
            E.row [ lpad ]
                (E.text
                    "Emphasis "
                    :: List.map viewinline inlines
                )

        Strong inlines ->
            E.row [ lpad ]
                (E.text
                    "Strong "
                    :: List.map viewinline inlines
                )

        Strikethrough inlines ->
            E.row [ lpad ]
                (E.text
                    "Strikethrough "
                    :: List.map viewinline inlines
                )

        CodeSpan string ->
            E.text <| "CodeSpan " ++ string

        Text string ->
            E.text <| "Text " ++ string

        HardLineBreak ->
            E.text <| "HardLineBreak "
