module ViewLinearMd exposing (..)

import LinearMd exposing (MdElement(..), lpad, mbToMe, miToMe, viewMdElement, viewhtml)
import Array exposing (Array)
import Common exposing (buttonStyle)
import DnDList
import Markdown.Block as MB
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Element.Region as ER
import Html.Attributes as HA
-- import Markdown.MdElement as MdElement exposing (MdElement(..), Html(..), Inline(..), ListItem(..), Task(..), inlineFoldl)
import Markdown.Html
import TangoColors as TC

type alias Model =
    { blocks : Array EditMdElement
    , focusMdElement : Maybe Int
    , nextMdElementId : Int
    , cleanMdElements : Array EditMdElement
    , blockDnd : DnDList.Model
    }


type alias EditMdElement =
    { editid : Int
    , block : MdElement
    }


type Msg
    = AddMdElementPress
    | MdElementDndMsg DnDList.Msg
    | MdElementClicked Int
    | DeleteMdElement Int


init : List MB.Block -> Model
init blocks =
    let
        ba =
            blocks
                |> List.map mbToMe 
                |> List.concat
                |> List.indexedMap (\i b -> { editid = i, block = b })
                |> Array.fromList
    in
    { blocks = ba
    , focusMdElement = Nothing
    , nextMdElementId = Array.length ba
    , cleanMdElements = ba
    , blockDnd = blockDndSystem.model
    }


blockDndConfig : DnDList.Config EditMdElement
blockDndConfig =
    { beforeUpdate = \_ _ list -> list
    , movement = DnDList.Free
    , listen = DnDList.OnDrag
    , operation = DnDList.Swap
    }


blockDndSystem : DnDList.System EditMdElement Msg
blockDndSystem =
    DnDList.create blockDndConfig MdElementDndMsg


subscriptions : Model -> Sub Msg
subscriptions model =
    blockDndSystem.subscriptions model.blockDnd



-- ++ (Array.indexedMap
--         (\i -> viewMdElementDnd model.blockDnd i model.focusMdElement)
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
            (\i -> viewMdElementDnd model.blockDnd i model.focusMdElement)
            model.blocks
            |> Array.toList
        )


viewMdElementDnd : DnDList.Model -> Int -> Maybe Int -> EditMdElement -> Element Msg
viewMdElementDnd ddlmodel i focusid s =
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
    viewMdElement ddw i focusid s


type DragDropWhat
    = Drag
    | Drop
    | Ghost


viewMdElement : DragDropWhat -> Int -> Maybe Int -> EditMdElement -> Element Msg
viewMdElement ddw i focusid s =
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
         , EE.onMouseDown (MdElementClicked s.editid)
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
            [ viewIMdElement s.editid s.block

            -- , if focus then
            --     blockMenu
            --   else
            --     E.none
            ]
        , EI.button (E.alignTop :: E.alignRight :: buttonStyle)
            { onPress = Just (DeleteMdElement s.editid)
            , label = E.text "x"
            }
        ]


makeScrollId : Int -> String
makeScrollId i =
    "id-" ++ String.fromInt i


viewIMdElement : Int -> MdElement -> Element Msg
viewIMdElement editid b =
    E.row
        [ E.width E.fill
        , EBd.width 1
        , EBd.rounded 5
        , E.padding 8
        , E.spacing 8
        , E.htmlAttribute (HA.id (makeScrollId editid))
        ]
    <|
        [ LinearMd.viewMdElement b
        ]


ghostView : Model -> E.Element Msg
ghostView model =
    let
        dnd =
            model.blockDnd

        maybeDragItem : Maybe EditMdElement
        maybeDragItem =
            blockDndSystem.info dnd
                |> Maybe.andThen (\{ dragIndex } -> Array.get dragIndex model.blocks)
    in
    case maybeDragItem of
        Just item ->
            E.el
                (List.map E.htmlAttribute (blockDndSystem.ghostStyles dnd))
                (viewMdElement Ghost 0 (Just item.editid) item)

        Nothing ->
            E.none



