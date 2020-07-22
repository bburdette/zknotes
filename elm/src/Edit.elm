module Edit exposing (Model, Msg(..), blockCells, cellView, code, codeBlock, defCell, heading, init, markdownBody, markdownView, mdCells, mkRenderer, rawTextToId, showRunState, update, view)

import Cellme.Cellme exposing (Cell, CellContainer(..), CellState, RunState(..), evalCellsFully, evalCellsOnce)
import Cellme.DictCellme exposing (CellDict(..), DictCell, dictCcr, getCd, mkCc)
import Dict exposing (Dict)
import Element exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Font as Font
import Element.Input as EI
import Element.Region
import Html exposing (Attribute, Html)
import Html.Attributes
import Markdown.Block as Block exposing (Block, Inline, ListItem(..), Task(..))
import Markdown.Html
import Markdown.Parser
import Markdown.Renderer
import Schelme.Show exposing (showTerm)


type Msg
    = OnMarkdownInput String
    | OnSchelmeCodeChanged String String


type alias Model =
    { md : String
    , cells : CellDict
    }


view : Model -> Element Msg
view model =
    Element.row [ Element.width Element.fill ]
        [ EI.multiline [ Element.width (Element.px 400) ]
            { onChange = OnMarkdownInput
            , text = model.md
            , placeholder = Nothing
            , label = EI.labelHidden "Markdown input"
            , spellcheck = False
            }
        , case markdownView (mkRenderer model.cells) model.md of
            Ok rendered ->
                Element.column
                    [ Element.spacing 30
                    , Element.padding 80
                    , Element.width (Element.fill |> Element.maximum 1000)
                    , Element.centerX
                    ]
                    rendered

            Err errors ->
                Element.text errors
        ]


markdownView : Markdown.Renderer.Renderer (Element Msg) -> String -> Result String (List (Element Msg))
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


mkRenderer : CellDict -> Markdown.Renderer.Renderer (Element Msg)
mkRenderer cellDict =
    { heading = heading
    , paragraph =
        Element.paragraph
            [ Element.spacing 15 ]
    , thematicBreak = Element.none
    , text = Element.text
    , strong = \content -> Element.row [ Font.bold ] content
    , emphasis = \content -> Element.row [ Font.italic ] content
    , codeSpan = code
    , link =
        \{ title, destination } body ->
            Element.newTabLink
                [ Element.htmlAttribute (Html.Attributes.style "display" "inline-flex") ]
                { url = destination
                , label =
                    Element.paragraph
                        [ Font.color (Element.rgb255 0 0 255)
                        ]
                        body
                }
    , hardLineBreak = Html.br [] [] |> Element.html
    , image =
        \image ->
            case image.title of
                Just title ->
                    Element.image [ Element.width Element.fill ] { src = image.src, description = image.alt }

                Nothing ->
                    Element.image [ Element.width Element.fill ] { src = image.src, description = image.alt }
    , blockQuote =
        \children ->
            Element.column
                [ EBd.widthEach { top = 0, right = 0, bottom = 0, left = 10 }
                , Element.padding 10
                , EBd.color (Element.rgb255 145 145 145)
                , EBk.color (Element.rgb255 245 245 245)
                ]
                children
    , unorderedList =
        \items ->
            Element.column [ Element.spacing 15 ]
                (items
                    |> List.map
                        (\(ListItem task children) ->
                            Element.row [ Element.spacing 5 ]
                                [ Element.row
                                    [ Element.alignTop ]
                                    ((case task of
                                        IncompleteTask ->
                                            EI.defaultCheckbox False

                                        CompletedTask ->
                                            EI.defaultCheckbox True

                                        NoTask ->
                                            Element.text "•"
                                     )
                                        :: Element.text " "
                                        :: children
                                    )
                                ]
                        )
                )
    , orderedList =
        \startingIndex items ->
            Element.column [ Element.spacing 15 ]
                (items
                    |> List.indexedMap
                        (\index itemBlocks ->
                            Element.row [ Element.spacing 5 ]
                                [ Element.row [ Element.alignTop ]
                                    (Element.text (String.fromInt (index + startingIndex) ++ " ") :: itemBlocks)
                                ]
                        )
                )
    , codeBlock = codeBlock
    , html =
        Markdown.Html.oneOf
            [ Markdown.Html.tag "cell"
                (\name schelmeCode renderedChildren ->
                    cellView cellDict renderedChildren name schelmeCode
                )
                |> Markdown.Html.withAttribute "name"
                |> Markdown.Html.withAttribute "schelmecode"
            ]
    , table = Element.column []
    , tableHeader = Element.column []
    , tableBody = Element.column []
    , tableRow = Element.row []
    , tableHeaderCell =
        \maybeAlignment children ->
            Element.paragraph [] children
    , tableCell = Element.paragraph []
    }


cellView : CellDict -> List (Element Msg) -> String -> String -> Element Msg
cellView (CellDict cellDict) renderedChildren name schelmeCode =
    Element.column
        [ EBd.shadow
            { offset = ( 0.3, 0.3 )
            , size = 2
            , blur = 0.5
            , color = Element.rgba255 0 0 0 0.22
            }
        , Element.padding 20
        , Element.spacing 30
        , Element.centerX
        , Font.center
        ]
        (Element.row [ Element.spacing 20 ]
            [ Element.el
                [ Font.bold
                , Font.size 30
                ]
                (Element.text name)
            , EI.text []
                { onChange = OnSchelmeCodeChanged name
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
                    (Element.text "<reserr>")
            ]
            :: renderedChildren
        )


showRunState : DictCell -> Element Msg
showRunState cell =
    Element.el [ Element.width Element.fill ] <|
        case cell.runstate of
            RsOk term ->
                Element.text <| showTerm term

            RsErr s ->
                Element.el [ Font.color <| Element.rgb 1 0.1 0.1 ] <| Element.text <| "err: " ++ s

            RsUnevaled ->
                Element.text <| "unevaled"

            RsBlocked _ id ->
                Element.text <| "blocked on cell: " ++ dictCcr.showId id


rawTextToId : String -> String
rawTextToId rawText =
    rawText
        |> String.toLower
        |> String.replace " " ""


heading : { level : Block.HeadingLevel, rawText : String, children : List (Element msg) } -> Element msg
heading { level, rawText, children } =
    Element.paragraph
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
        , Element.Region.heading (Block.headingLevelToInt level)
        , Element.htmlAttribute
            (Html.Attributes.attribute "name" (rawTextToId rawText))
        , Element.htmlAttribute
            (Html.Attributes.id (rawTextToId rawText))
        ]
        children


code : String -> Element msg
code snippet =
    Element.el
        [ EBk.color
            (Element.rgba 0 0 0 0.04)
        , EBd.rounded 2
        , Element.paddingXY 5 3
        , Font.family
            [ Font.external
                { url = "https://fonts.googleapis.com/css?family=Source+Code+Pro"
                , name = "Source Code Pro"
                }
            ]
        ]
        (Element.text snippet)


codeBlock : { body : String, language : Maybe String } -> Element msg
codeBlock details =
    Element.el
        [ EBk.color (Element.rgba 0 0 0 0.03)
        , Element.htmlAttribute (Html.Attributes.style "white-space" "pre")
        , Element.padding 20
        , Element.width Element.fill
        , Font.family
            [ Font.external
                { url = "https://fonts.googleapis.com/css?family=Source+Code+Pro"
                , name = "Source Code Pro"
                }
            ]
        ]
        (Element.text details.body)


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


init : Model
init =
    let
        cells =
            Debug.log "newcells"
                (markdownBody
                    |> mdCells
                    |> Result.withDefault (CellDict Dict.empty)
                )

        ( cc, result ) =
            evalCellsFully
                (mkCc cells)
    in
    { md = markdownBody
    , cells = Debug.log "evaled cells: " <| getCd cc
    }


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        OnMarkdownInput newMarkdown ->
            let
                cells =
                    Debug.log "newcells"
                        (newMarkdown
                            |> mdCells
                            |> Result.withDefault (CellDict Dict.empty)
                        )

                ( cc, result ) =
                    evalCellsFully
                        (mkCc cells)
            in
            ( { model
                | md = newMarkdown
                , cells = Debug.log "evaled cells: " <| getCd cc
              }
            , Cmd.none
            )

        OnSchelmeCodeChanged name string ->
            let
                (CellDict cd) =
                    model.cells

                ( cc, result ) =
                    evalCellsFully
                        (mkCc
                            (Dict.insert name (defCell string) cd
                                |> CellDict
                            )
                        )
            in
            ( { model
                | cells = getCd cc
              }
            , Cmd.none
            )
