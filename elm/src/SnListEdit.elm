module SnListEdit exposing (..)

import Common exposing (buttonStyle)
import Data exposing (ZkNoteId)
import DataUtil exposing (ZniSet, emptyZniSet)
import DnDList
import DndPorts exposing (..)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Html.Attributes
import Html.Events as HE
import Json.Decode as JD
import NoteCache exposing (NoteCache)
import SpecialNotes exposing (Notegraph)
import TSet
import TangoColors as TC
import Util
import ZkCommon as ZC


type alias NlLink =
    { id : ZkNoteId, title : String }


type Msg
    = GraphFocusClick
    | EditItem Int
    | DnDMsg DnDList.Msg
    | Select ZkNoteId
    | Remove


type alias Model =
    { ng : Notegraph
    , nlls : List NlLink
    , nllDnd : DnDList.Model
    , selected : ZniSet
    }


init : Notegraph -> List NlLink -> Model
init ng nlls =
    { ng = ng
    , nlls = nlls
    , nllDnd = nllDndSystem.model
    , selected = emptyZniSet
    }


commands : Model -> Cmd Msg
commands model =
    nllDndSystem.commands model.nllDnd


update : Msg -> Model -> Model
update msg model =
    case msg of
        GraphFocusClick ->
            model

        EditItem _ ->
            model

        DnDMsg dmsg ->
            let
                ( nm, lst ) =
                    nllDndSystem.update dmsg model.nllDnd model.nlls
            in
            { model | nllDnd = nm, nlls = lst }

        Select id ->
            { model
                | selected =
                    if TSet.member id model.selected then
                        TSet.remove id model.selected

                    else
                        TSet.insert id model.selected
            }

        Remove ->
            { model
                | nlls =
                    model.nlls
                        |> List.filter
                            (\nl ->
                                not <|
                                    TSet.member nl.id model.selected
                            )
                , selected = emptyZniSet
            }


controlRow : ZkNoteId -> E.Element Msg
controlRow id =
    E.row [ E.width E.fill ]
        [ EI.button Common.buttonStyle
            { onPress = Just Remove
            , label = E.text "x"
            }
        , ZC.golinkns id TC.black
        ]


view : Model -> E.Element Msg
view model =
    let
        mbinfo =
            nllDndSystem.info model.nllDnd

        lastselected =
            model.nlls
                |> List.reverse
                |> Util.foldUntil
                    (\nl x ->
                        if TSet.member nl.id model.selected then
                            Util.Stop (Just nl.id)

                        else
                            Util.Go Nothing
                    )
                    Nothing
    in
    E.column []
        (List.indexedMap
            (\i nl ->
                dndRow
                    nllId
                    (case mbinfo of
                        Nothing ->
                            Drag

                        Just { dragIndex, dropIndex } ->
                            if i == dragIndex then
                                Ghost

                            else if i == dropIndex then
                                DropH

                            else
                                Drop
                    )
                    i
                    False
                    (let
                        r =
                            E.row
                                [ E.width E.fill
                                , EE.onClick (Select nl.id)
                                , EBk.color
                                    (if TSet.member nl.id model.selected then
                                        TC.lightCharcoal

                                     else
                                        TC.lightGray
                                    )
                                ]
                                [ E.text nl.title ]
                     in
                     E.column [ E.width E.fill ]
                        (r
                            :: (if lastselected == Just nl.id then
                                    [ controlRow nl.id ]

                                else
                                    [ E.none ]
                               )
                        )
                    )
            )
            model.nlls
        )



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


nllDndSubscriptions : Model -> List (Sub Msg)
nllDndSubscriptions model =
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
    "nll_" ++ String.fromInt i


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
                    ++ List.map E.htmlAttribute (nllDndSystem.dragEvents i bid)
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
        [ dndRow nllId ddw i (Just i == focusid) (E.text nll.title) ]



-------------------------------------------------------------------------
-- END Drag and Drop
-------------------------------------------------------------------------
