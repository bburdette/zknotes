module MdList exposing (mdblockview)

import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Markdown.Block as Block exposing (Block(..), Html(..), Inline, ListItem(..), Task(..), inlineFoldl)
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
                                E.column []
                                    [ E.text "HtmlBlock"
                                    , case htmlb of
                                        HtmlElement string listHtmlAttribute listChildren ->
                                            E.el
                                                [ E.paddingEach
                                                    { top = 0, right = 0, bottom = 0, left = 10 }
                                                ]
                                            <|
                                                E.text <|
                                                    "HtmlElement "
                                                        ++ string

                                        HtmlComment string ->
                                            E.el
                                                [ E.paddingEach
                                                    { top = 0, right = 0, bottom = 0, left = 10 }
                                                ]
                                            <|
                                                E.text <|
                                                    "HtmlComment "
                                                        ++ string

                                        ProcessingInstruction string ->
                                            E.el
                                                [ E.paddingEach
                                                    { top = 0, right = 0, bottom = 0, left = 10 }
                                                ]
                                            <|
                                                E.text <|
                                                    "ProcessingInstruction "
                                                        ++ string

                                        HtmlDeclaration string1 string2 ->
                                            E.el
                                                [ E.paddingEach
                                                    { top = 0, right = 0, bottom = 0, left = 10 }
                                                ]
                                            <|
                                                E.text <|
                                                    "HtmlDeclaration "
                                                        ++ string1
                                                        ++ " "
                                                        ++ string2

                                        Cdata string ->
                                            E.el
                                                [ E.paddingEach
                                                    { top = 0, right = 0, bottom = 0, left = 10 }
                                                ]
                                            <|
                                                E.text <|
                                                    "Cdata "
                                                        ++ string
                                    ]

                            UnorderedList listSpacing blockListItems ->
                                E.text "UnorderedList"

                            OrderedList listSpacing offset listListBlocks ->
                                E.text "OrderedList"

                            BlockQuote blocks ->
                                E.text "BlockQuote"

                            Heading headingLevel inlines ->
                                E.text "Heading"

                            Paragraph inlines ->
                                E.text "Paragraph"

                            Table headings inlines ->
                                E.text "Table"

                            CodeBlock bodyLanguage ->
                                E.text "CodeBlock"

                            ThematicBreak ->
                                E.text "ThematicBreak"
                    )



-- = HtmlInline (Html Block)
-- | Link String (Maybe String) (List Inline)
-- | Image String (Maybe String) (List Inline)
-- | Emphasis (List Inline)
-- | Strong (List Inline)
-- | Strikethrough (List Inline)
-- | CodeSpan String
-- | Text String
-- | HardLineBreak
