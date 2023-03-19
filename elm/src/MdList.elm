module MdList exposing (..)

import Array exposing (Array)
import Common exposing (buttonStyle)
import DnDList
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Html.Attributes as HA
import Markdown.Block as Block exposing (Block(..), Html(..), Inline(..), ListItem(..), Task(..), inlineFoldl)
import Markdown.Html
import TangoColors as TC


type alias Model =
    { blocks : Array EditBlock
    , focusBlock : Maybe Int
    , nextBlockId : Int
    , cleanBlocks : Array EditBlock
    , blockDnd : DnDList.Model
    }


type alias EditBlock =
    { editid : Int
    , block : Block
    }


type Msg
    = AddBlockPress
    | BlockDndMsg DnDList.Msg
    | BlockClicked Int
    | DeleteBlock Int


init : List Block -> Model
init blocks =
    let
        ba =
            blocks
                |> List.indexedMap (\i b -> { editid = i, block = b })
                |> Array.fromList
    in
    { blocks = ba
    , focusBlock = Nothing
    , nextBlockId = Array.length ba
    , cleanBlocks = ba
    , blockDnd = blockDndSystem.model
    }


blockDndConfig : DnDList.Config EditBlock
blockDndConfig =
    { beforeUpdate = \_ _ list -> list
    , movement = DnDList.Free
    , listen = DnDList.OnDrag
    , operation = DnDList.Swap
    }


blockDndSystem : DnDList.System EditBlock Msg
blockDndSystem =
    DnDList.create blockDndConfig BlockDndMsg


subscriptions : Model -> Sub Msg
subscriptions model =
    blockDndSystem.subscriptions model.blockDnd



-- ++ (Array.indexedMap
--         (\i -> viewBlockDnd model.blockDnd i model.focusBlock)
--         model.blocks
--         |> Array.toList
--    )


view : Model -> Element Msg
view model =
    -- let
    --     maxwidth =
    --         700
    --     maxheight =
    --         nd.size.height - 75
    -- in
    E.column
        [ E.alignTop
        , E.spacing 8
        , E.padding 8

        -- , E.width (E.maximum maxwidth E.fill)
        -- , E.height (E.maximum maxheight E.fill)
        , E.centerX

        -- , E.scrollbarY
        , E.htmlAttribute (HA.id "steplist")
        ]
    <|
        (Array.indexedMap
            (\i -> viewBlockDnd model.blockDnd i model.focusBlock)
            model.blocks
            |> Array.toList
        )


viewBlockDnd : DnDList.Model -> Int -> Maybe Int -> EditBlock -> Element Msg
viewBlockDnd ddlmodel i focusid s =
    let
        ddw =
            case
                blockDndSystem.info ddlmodel
                    |> Maybe.map .dragIndex
            of
                Just ix ->
                    if ix == i then
                        Ghost

                    else
                        Drop

                Nothing ->
                    Drag
    in
    viewBlock ddw i focusid s


type DragDropWhat
    = Drag
    | Drop
    | Ghost


viewBlock : DragDropWhat -> Int -> Maybe Int -> EditBlock -> Element Msg
viewBlock ddw i focusid s =
    let
        itemId : String
        itemId =
            "id-" ++ String.fromInt i

        focus =
            focusid == Just s.editid
    in
    E.row
        ([ E.width E.fill
         , E.spacing 8
         , E.htmlAttribute (HA.id itemId)
         , E.padding 10
         , EBd.rounded 5
         , EE.onMouseDown (BlockClicked s.editid)
         ]
            ++ (case ddw of
                    Drag ->
                        [ EBk.color <|
                            if focus then
                                TC.darkBlue

                            else
                                TC.blue
                        ]

                    Drop ->
                        EBk.color TC.yellow
                            :: List.map E.htmlAttribute (blockDndSystem.dropEvents i itemId)

                    Ghost ->
                        [ E.alpha 0.5, EBk.color TC.green ]
               )
        )
        [ E.column
            ([ E.width <| E.px 30
             , EBk.color TC.brown
             , E.height E.fill
             ]
                ++ List.map E.htmlAttribute (blockDndSystem.dragEvents i itemId)
            )
            []
        , E.column
            [ E.width E.fill
            , E.spacing 8
            ]
            [ viewIBlock s.editid s.block

            -- , if focus then
            --     blockMenu
            --   else
            --     E.none
            ]
        , EI.button (E.alignTop :: E.alignRight :: buttonStyle)
            { onPress = Just (DeleteBlock s.editid)
            , label = E.text "x"
            }
        ]


makeScrollId : Int -> String
makeScrollId i =
    "id-" ++ String.fromInt i


viewIBlock : Int -> Block -> Element Msg
viewIBlock editid b =
    E.row
        [ E.width E.fill
        , EBd.width 1
        , EBd.rounded 5
        , E.padding 8
        , E.spacing 8
        , E.htmlAttribute (HA.id (makeScrollId editid))
        ]
    <|
        [ viewMdBlock b
        ]


ghostView : Model -> E.Element Msg
ghostView model =
    let
        dnd =
            model.blockDnd

        maybeDragItem : Maybe EditBlock
        maybeDragItem =
            blockDndSystem.info dnd
                |> Maybe.andThen (\{ dragIndex } -> Array.get dragIndex model.blocks)
    in
    case maybeDragItem of
        Just item ->
            E.el
                (List.map E.htmlAttribute (blockDndSystem.ghostStyles dnd))
                (viewBlock Ghost 0 (Just item.editid) item)

        Nothing ->
            E.none


viewMdBlock : Block -> Element Msg
viewMdBlock b =
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


mdblockview : Result error (List Block) -> List (Element Msg)
mdblockview parsedMd =
    case parsedMd of
        Err e ->
            []

        Ok bs ->
            bs
                |> List.map
                    viewMdBlock


lpad : E.Attribute Msg
lpad =
    E.paddingEach
        { top = 0, right = 0, bottom = 0, left = 10 }


viewhtml : Html Block -> Element Msg
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


viewinline : Inline -> Element Msg
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
