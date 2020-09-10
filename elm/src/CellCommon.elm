module CellCommon exposing (blockCells, cellView, code, codeBlock, defCell, heading, inlineFoldl, markdownBody, markdownView, mdCells, mkRenderer, rawTextToId, showRunState)

import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Dict exposing (Dict)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region as ER
import Html exposing (Attribute, Html)
import Html.Attributes
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..))
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import Schelme.Show exposing (showTerm)


markdownView : Markdown.Renderer.Renderer (Element a) -> String -> Result String (List (Element a))
markdownView renderer markdown =
    markdown
        |> Markdown.Parser.parse
        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        |> Result.andThen (Markdown.Renderer.render renderer)


mdCells : String -> Result String CellDict
mdCells markdown =
    markdown
        |> Markdown.Parser.parse
        |> Result.mapError (\error -> error |> List.map Markdown.Parser.deadEndToString |> String.join "\n")
        |> Result.map blockCells


blockCells : List Block -> CellDict
blockCells blocks =
    blocks
        |> List.filterMap
            (\block ->
                case block of
                    Block.HtmlBlock (Block.HtmlElement tag attribs _) ->
                        if tag == "cell" then
                            let
                                am =
                                    Dict.fromList <| List.map (\trib -> ( trib.name, trib.value )) attribs
                            in
                            am
                                |> Dict.get "name"
                                |> Maybe.andThen
                                    (\name ->
                                        am
                                            |> Dict.get "schelmecode"
                                            |> Maybe.map (String.replace "/'" "\"")
                                            |> Maybe.andThen
                                                (\schelme ->
                                                    Just ( name, defCell schelme )
                                                )
                                    )

                        else
                            Nothing

                    _ ->
                        Nothing
            )
        |> Dict.fromList
        |> CellDict


defCell : String -> DictCell
defCell s =
    { code = s, prog = Err "", runstate = RsErr "" }


mkRenderer : CellDict -> (String -> String -> a) -> Markdown.Renderer.Renderer (Element a)
mkRenderer cellDict onchanged =
    { heading = heading
    , paragraph =
        E.paragraph
            [ E.spacing 15 ]
    , thematicBreak = E.none
    , text = E.text
    , strong = \content -> E.row [ Font.bold ] content
    , emphasis = \content -> E.row [ Font.italic ] content
    , codeSpan = code
    , link =
        \{ title, destination } body ->
            (if String.contains ":" destination then
                E.newTabLink

             else
                E.link
            )
                [ E.htmlAttribute (Html.Attributes.style "display" "inline-flex") ]
                { url = destination
                , label =
                    E.paragraph
                        [ Font.color (E.rgb255 0 0 255)
                        ]
                        body
                }
    , hardLineBreak = Html.br [] [] |> E.html
    , image =
        \image ->
            case image.title of
                Just title ->
                    E.image [ E.width E.fill ] { src = image.src, description = image.alt }

                Nothing ->
                    E.image [ E.width E.fill ] { src = image.src, description = image.alt }
    , blockQuote =
        \children ->
            E.column
                [ EBd.widthEach { top = 0, right = 0, bottom = 0, left = 10 }
                , E.padding 10
                , EBd.color (E.rgb255 145 145 145)
                , EBk.color (E.rgb255 245 245 245)
                ]
                children
    , unorderedList =
        \items ->
            E.column [ E.spacing 15 ]
                (items
                    |> List.map
                        (\(ListItem task children) ->
                            E.row [ E.spacing 5 ]
                                [ E.row
                                    [ E.alignTop ]
                                    ((case task of
                                        IncompleteTask ->
                                            EI.defaultCheckbox False

                                        CompletedTask ->
                                            EI.defaultCheckbox True

                                        NoTask ->
                                            E.text "•"
                                     )
                                        :: E.text " "
                                        :: children
                                    )
                                ]
                        )
                )
    , orderedList =
        \startingIndex items ->
            E.column [ E.spacing 15 ]
                (items
                    |> List.indexedMap
                        (\index itemBlocks ->
                            E.row [ E.spacing 5 ]
                                [ E.row [ E.alignTop ]
                                    (E.text (String.fromInt (index + startingIndex) ++ " ") :: itemBlocks)
                                ]
                        )
                )
    , codeBlock = codeBlock
    , html =
        Markdown.Html.oneOf
            [ Markdown.Html.tag "cell"
                (\name schelmeCode renderedChildren ->
                    cellView cellDict renderedChildren name schelmeCode onchanged
                )
                |> Markdown.Html.withAttribute "name"
                |> Markdown.Html.withAttribute "schelmecode"
            ]
    , table = E.column []
    , tableHeader = E.column []
    , tableBody = E.column []
    , tableRow = E.row []
    , tableHeaderCell =
        \maybeAlignment children ->
            E.paragraph [] children
    , tableCell = E.paragraph []
    }


cellView : CellDict -> List (Element a) -> String -> String -> (String -> String -> a) -> Element a
cellView (CellDict cellDict) renderedChildren name schelmeCode onchanged =
    E.column
        [ EBd.shadow
            { offset = ( 0.3, 0.3 )
            , size = 2
            , blur = 0.5
            , color = E.rgba255 0 0 0 0.22
            }
        , E.padding 20
        , E.spacing 30
        , E.centerX
        , Font.center
        ]
        (E.row [ E.spacing 20 ]
            [ E.el
                [ Font.bold
                , Font.size 30
                ]
                (E.text name)
            , EI.text []
                { onChange = onchanged name
                , placeholder = Nothing
                , label = EI.labelHidden name
                , text =
                    cellDict
                        |> Dict.get name
                        |> Maybe.map .code
                        |> Maybe.withDefault "<err>"
                }
            , cellDict
                |> Dict.get name
                |> Maybe.map showRunState
                |> Maybe.withDefault
                    (E.text "<reserr>")
            ]
            :: renderedChildren
        )


showRunState : DictCell -> Element a
showRunState cell =
    E.el [ E.width E.fill ] <|
        case cell.runstate of
            RsOk term ->
                E.text <| showTerm term

            RsErr s ->
                E.el [ Font.color <| E.rgb 1 0.1 0.1 ] <| E.text <| "err: " ++ s

            RsUnevaled ->
                E.text <| "unevaled"

            RsBlocked _ id ->
                E.text <| "blocked on cell: " ++ dictCcr.showId id


rawTextToId : String -> String
rawTextToId rawText =
    rawText
        |> String.toLower
        |> String.replace " " ""


heading : { level : Block.HeadingLevel, rawText : String, children : List (Element msg) } -> Element msg
heading { level, rawText, children } =
    E.paragraph
        [ Font.size
            (case level of
                Block.H1 ->
                    36

                Block.H2 ->
                    24

                _ ->
                    20
            )
        , Font.bold
        , Font.family [ Font.typeface "Montserrat" ]
        , ER.heading (Block.headingLevelToInt level)
        , E.htmlAttribute
            (Html.Attributes.attribute "name" (rawTextToId rawText))
        , E.htmlAttribute
            (Html.Attributes.id (rawTextToId rawText))
        ]
        children


code : String -> Element msg
code snippet =
    E.el
        [ EBk.color
            (E.rgba 0 0 0 0.04)
        , EBd.rounded 2
        , E.paddingXY 5 3
        , Font.family
            [ Font.external
                { url = "https://fonts.googleapis.com/css?family=Source+Code+Pro"
                , name = "Source Code Pro"
                }
            ]
        ]
        (E.text snippet)


codeBlock : { body : String, language : Maybe String } -> Element msg
codeBlock details =
    E.el
        [ EBk.color (E.rgba 0 0 0 0.03)
        , E.htmlAttribute (Html.Attributes.style "white-space" "pre")
        , E.padding 20
        , E.width E.fill
        , Font.family
            [ Font.external
                { url = "https://fonts.googleapis.com/css?family=Source+Code+Pro"
                , name = "Source Code Pro"
                }
            ]
        ]
        (E.text details.body)


markdownBody : String
markdownBody =
    """# Markdown Schelme Cells!

###[elm-markdown](https://github.com/dillonkearns/elm-markdown) + [schelme](https://github.com/bburdette/schelme) + [cellme](https://github.com/bburdette/cellme) + [elm-ui](https://github.com/mdgriffith/elm-ui)

#####Kind of a spreadsheet,  but with named cells instead of a grid.

<cell
  name="inches"
  schelmeCode="5"
>
</cell>

<cell
  name="millimeters"
  schelmeCode="(* (cv /'inches/') 25.4)"
>
</cell>

<cell
  name="furlongs"
  schelmeCode="(/ (cv /'inches/') (* 12 660))"
>
</cell>
"""



{- pullLinks : List Block -> List String
   pullLinks blocks =
       blocks
           |> inlineFoldl
               (\inline links ->
                   case inline of
                       Block.Link str mbstr moreinlines ->
                           str :: links

                       _ ->
                           links
               )
               []


   testPullLinks =
       [ Block.Heading Block.H1 [ Block.Text "Document" ]
       , Block.Heading Block.H2 [ Block.Link "/note/50" (Just "interesting document") [] ]
       , Block.Heading Block.H3 [ Block.Text "Subsection" ]
       , Block.Heading Block.H2 [ Block.Link "/note/51" (Just "more interesting document") [] ]
       ]
           |> pullLinks
-}


inlineFoldl : (Inline -> acc -> acc) -> acc -> List Block -> acc
inlineFoldl function top_acc list =
    let
        bfn =
            \block acc ->
                case block of
                    Block.HtmlBlock html ->
                        acc

                    Block.UnorderedList listItems ->
                        List.foldl
                            (\(ListItem _ inlines) liacc ->
                                List.foldl function liacc inlines
                            )
                            acc
                            listItems

                    Block.OrderedList int lists ->
                        List.foldl
                            (\inlines lacc ->
                                List.foldl function lacc inlines
                            )
                            acc
                            lists

                    Block.BlockQuote _ ->
                        acc

                    Block.Heading _ inlines ->
                        List.foldl function acc inlines

                    Block.Paragraph inlines ->
                        List.foldl function acc inlines

                    Block.Table labels lists ->
                        let
                            lacc =
                                List.foldl
                                    (\inlines iacc ->
                                        List.foldl function iacc inlines
                                    )
                                    acc
                                    (List.map .label labels)
                        in
                        List.foldl
                            (\inlines iacc ->
                                List.foldl function iacc inlines
                            )
                            lacc
                            lists

                    Block.CodeBlock _ ->
                        acc

                    Block.ThematicBreak ->
                        acc
    in
    Block.foldl bfn top_acc list
