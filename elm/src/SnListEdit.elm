port module SnListEdit exposing (..)

import Data exposing (ZkNoteId)
import DnDList
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Html.Attributes
import Html.Events as HE
import Json.Decode as JD
import Json.Encode as JE
import NoteCache exposing (NoteCache)
import SpecialNotes exposing (Notegraph)
import TangoColors as TC


type alias NlLink =
    { id : ZkNoteId, title : String }


type Msg
    = GraphFocusClick
    | EditItem Int
    | DnDMsg DnDList.Msg


type alias Model =
    { ng : Notegraph
    , nlls : List NlLink
    , nllDnd : DnDList.Model
    }


update : Msg -> Model -> Model
update msg model =
    case msg of
        GraphFocusClick ->
            let
                _ =
                    Debug.log "GraphFocusClick" msg
            in
            model

        EditItem i ->
            let
                _ =
                    Debug.log "EditItem" msg
            in
            model

        DnDMsg dmsg ->
            let
                _ =
                    Debug.log "DnDMsg" msg
            in
            model



-------------------------------------------------------------------------
-- Drag and Drop
-------------------------------------------------------------------------


type DragDropWhat
    = Drag
    | Drop
    | DropH
    | Ghost
    | Inactive
    | DdwItemEdit


dndIdentity : x -> x -> List a -> List a
dndIdentity _ _ l =
    l


nllDndSystem : DnDList.System NlLink Msg
nllDndSystem =
    DnDList.createWithTouch
        { beforeUpdate = dndIdentity
        , movement = DnDList.Vertical
        , listen = DnDList.OnDrop
        , operation = DnDList.Rotate
        }
        DnDMsg
        onPointerMove
        onPointerUp
        releasePointerCapture


type alias INlLink =
    { idx : Int
    , draggee : Bool
    , droppee : Bool
    , nll : NlLink
    }


dndINlLink : Int -> Int -> List INlLink -> List INlLink
dndINlLink dragIdx dropIdx iNlLinks =
    List.indexedMap
        (\i ib ->
            if i == dropIdx then
                { ib | droppee = True }

            else if i == dragIdx then
                { ib | draggee = True }

            else
                ib
        )
        iNlLinks


nllDndSystemUnaltered : DnDList.System INlLink Msg
nllDndSystemUnaltered =
    DnDList.createWithTouch
        { beforeUpdate = dndINlLink
        , movement = DnDList.Vertical
        , listen = DnDList.OnDrop
        , operation = DnDList.Unaltered
        }
        DnDMsg
        onPointerMove
        onPointerUp
        releasePointerCapture


port onPointerMove : (JE.Value -> msg) -> Sub msg


port onPointerUp : (JE.Value -> msg) -> Sub msg


port releasePointerCapture : JE.Value -> Cmd msg


dllDndSubscriptions : Model -> List (Sub Msg)
dllDndSubscriptions model =
    [ nllDndSystem.subscriptions model.nllDnd ]



-- to be called from main.elm!


ghostView : Model -> NoteCache -> Int -> Maybe (Element Msg)
ghostView model nc mdw =
    nllDndSystem.info model.nllDnd
        |> Maybe.andThen
            (\{ dragIndex } ->
                model.nlls
                    |> (List.head << List.drop dragIndex)
                    |> Maybe.map (\b -> ( dragIndex, b ))
            )
        |> Maybe.map
            (\( i, item ) ->
                E.el
                    (E.alpha 0.5
                        :: List.map E.htmlAttribute
                            (nllDndSystem.ghostStyles model.nllDnd)
                    )
                    (viewItem Ghost 0 (Just i) item)
            )


nllId : Int -> String
nllId i =
    "nll-" ++ String.fromInt i


edButtonStyle : Msg -> List (E.Attribute Msg)
edButtonStyle m =
    [ EBk.color TC.blue
    , EF.color TC.white
    , EBd.color TC.darkBlue
    , E.paddingXY 3 3
    , EBd.rounded 2
    , E.htmlAttribute <|
        HE.custom "click"
            (JD.succeed
                { message = m
                , stopPropagation = True
                , preventDefault = False
                }
            )
    ]


dragHandleWidth : Int
dragHandleWidth =
    10


dndRow : (Int -> String) -> DragDropWhat -> Int -> Bool -> Element Msg -> Element Msg
dndRow toid ddw i focus e =
    let
        bid =
            toid i

        baseAttr =
            (if focus then
                [ EBd.width 5 ]

             else
                []
            )
                ++ [ E.width E.fill
                   , E.height E.fill
                   , E.padding 3
                   , E.spacing 2
                   , E.htmlAttribute (Html.Attributes.id bid)
                   ]

        dragHandleAttrs =
            [ E.width (E.px dragHandleWidth), E.height E.fill, EBk.color TC.gray, E.alignBottom ]

        spacer =
            E.el [ E.width (E.px dragHandleWidth), E.height E.fill ] E.none
    in
    case ddw of
        Drag ->
            E.row
                baseAttr
                [ E.el
                    (E.htmlAttribute (Html.Attributes.style "touch-action" "none")
                        :: dragHandleAttrs
                        ++ List.map E.htmlAttribute (nllDndSystem.dragEvents i bid)
                    )
                    E.none
                , spacer
                , if focus then
                    E.el [ E.width E.fill ] e

                  else
                    E.el [ EE.onClick (EditItem i), E.width E.fill ] e
                ]

        Drop ->
            E.row
                (E.htmlAttribute (Html.Attributes.style "touch-action" "none")
                    :: baseAttr
                    ++ List.map E.htmlAttribute (nllDndSystem.dropEvents i bid)
                )
                [ E.el dragHandleAttrs E.none
                , spacer
                , E.el [ E.width E.fill ] e
                ]

        DropH ->
            E.row
                ((EBk.color TC.darkBlue
                    :: E.htmlAttribute (Html.Attributes.style "touch-action" "none")
                    :: baseAttr
                 )
                    ++ List.map E.htmlAttribute (nllDndSystem.dropEvents i bid)
                )
                [ E.el dragHandleAttrs E.none
                , spacer
                , E.el [ E.width E.fill ] e
                ]

        Ghost ->
            E.row
                ((EBk.color TC.darkGreen
                    :: E.htmlAttribute (Html.Attributes.style "touch-action" "none")
                    :: baseAttr
                 )
                    ++ List.map E.htmlAttribute (nllDndSystem.dragEvents i (nllId i))
                )
                [ E.el dragHandleAttrs E.none
                , spacer
                , E.el [ E.width E.fill ] e
                ]

        Inactive ->
            E.row
                baseAttr
                [ E.el (dragHandleAttrs ++ [ EBk.color TC.lightGray ]) E.none
                , spacer
                , E.el [ EE.onClick (EditItem i), E.width E.fill ] e
                ]

        DdwItemEdit ->
            E.row
                baseAttr
                [ E.el (dragHandleAttrs ++ [ EBk.color TC.lightGray ]) E.none
                , spacer
                , E.el [ E.width E.fill ] e
                ]


viewItem : DragDropWhat -> Int -> Maybe Int -> NlLink -> Element Msg
viewItem ddw i focusid nll =
    E.column
        [ E.spacing 0
        , E.padding 3
        , E.width (E.fill |> E.maximum 1000)
        , E.centerX
        , E.alignTop
        , EBd.width 2
        , EBd.color TC.darkGrey
        , EBk.color TC.lightGrey
        ]
        (List.map (dndRow nllId ddw i (Just i == focusid)) [ E.text nll.title ])



-------------------------------------------------------------------------
-- END Drag and Drop
-------------------------------------------------------------------------
