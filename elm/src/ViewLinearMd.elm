module ViewLinearMd exposing (..)

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
import LinearMd exposing (MdElement(..), lpad, mbToMe, miToMe, toBlocks, viewMdElement, viewhtml)
import Markdown.Block as MB
import Markdown.Html
import Markdown.Renderer as MR
import MdText as MT
import TangoColors as TC


type alias Model =
    { blocks : Array EditMdElement
    , focusMdElement : Maybe Int
    , nextMdElementId : Int
    , cleanMdElements : Array EditMdElement
    , blockDnd : DnDList.Model
    , built : Maybe String
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
    | BuildPress


type DragDropWhat
    = Drag
    | Drop
    | Ghost


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
    , built = Nothing
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
        EI.button Common.buttonStyle { onPress = Just BuildPress, label = E.text "build" }
            :: E.text (Maybe.withDefault "" model.built)
            :: (Array.indexedMap
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
                        Debug.log "ghost mode"
                            [ E.alpha 0.5, EBk.color TC.red ]
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


ghostView : Model -> Maybe (E.Element Msg)
ghostView model =
    let
        dnd =
            model.blockDnd

        maybeDragItem : Maybe EditMdElement
        maybeDragItem =
            blockDndSystem.info dnd
                |> Maybe.andThen (\{ dragIndex } -> Array.get dragIndex model.blocks)
    in
    maybeDragItem
        |> Maybe.map
            (\item ->
                E.el
                    (List.map E.htmlAttribute (blockDndSystem.ghostStyles dnd))
                    (viewMdElement Ghost 0 (Just item.editid) item)
            )



-- type Command
--     = None
--     | BlockDndCmd (Cmd Msg)


update : Model -> Msg -> ( Model, Cmd Msg )
update model msg =
    case msg of
        BuildPress ->
            let
                st =
                    model.blocks
                        |> Array.toList
                        |> List.map .block
                        |> LinearMd.toBlocks
                        |> Result.andThen
                            (\blocks ->
                                MR.render MT.stringRenderer blocks
                                    |> Result.map String.concat
                            )
            in
            ( { model
                | built =
                    Just <|
                        case st of
                            Ok s ->
                                s

                            Err s ->
                                s
              }
            , Cmd.none
            )

        AddMdElementPress ->
            ( model, Cmd.none )

        MdElementDndMsg dndmsg ->
            let
                ( blockDnD, blocks ) =
                    blockDndSystem.update dndmsg model.blockDnd (Array.toList model.blocks)
            in
            ( { model
                | blockDnd = blockDnD
                , blocks = Array.fromList blocks
              }
            , blockDndSystem.commands blockDnD
            )

        MdElementClicked int ->
            ( model, Cmd.none )

        DeleteMdElement int ->
            ( model, Cmd.none )
