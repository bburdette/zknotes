module SnListView exposing (..)

import Common
import Data exposing (ZkNoteId)
import DataUtil exposing (NlLink, ZniSet, emptyZniSet, zkNoteIdToString)
import DnDList
import DndPorts exposing (..)
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Border as EBd
import Element.Events as EE
import Element.Font as EF
import Element.Input as EI
import Html.Attributes as HA
import Html.Events as HE
import Json.Decode as JD
import NoteCache exposing (NoteCache)
import TSet
import TangoColors as TC
import Util
import ZkCommon as ZC


type Msg
    = Select ZkNoteId
    | Play ZkNoteId


type alias Model =
    { currentUuid : Maybe String
    , nlls : List NlLink
    , selected : ZniSet
    }


type Command
    = PlayNSave String
    | None


init : Maybe String -> List NlLink -> Model
init currentUuid nlls =
    { currentUuid = currentUuid
    , nlls = nlls
    , selected = emptyZniSet
    }


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        Select id ->
            ( { model
                | selected =
                    if TSet.member id model.selected then
                        TSet.remove id model.selected

                    else
                        TSet.insert id model.selected
              }
            , None
            )

        Play id ->
            ( { model
                | currentUuid = Just <| zkNoteIdToString id
              }
            , PlayNSave (zkNoteIdToString id)
            )


controlRow : ZkNoteId -> E.Element Msg
controlRow id =
    E.row [ E.width E.fill, E.spacing 3, E.padding 3 ]
        [ EI.button Common.buttonStyle
            { onPress = Just (Play id)
            , label = E.text "▶"
            }
        , ZC.golinkns id TC.black
        ]


view : Int -> Model -> E.Element Msg
view fontsize model =
    let
        lastselected : Maybe ZkNoteId
        lastselected =
            model.nlls
                |> List.reverse
                |> Util.foldUntil
                    (\nl _ ->
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
                let
                    selected =
                        TSet.member nl.id model.selected

                    r =
                        E.row
                            [ E.width E.fill
                            , EE.onClick (Select nl.id)
                            , EBk.color
                                (if selected then
                                    TC.lightCharcoal

                                 else
                                    TC.lightGray
                                )
                            , E.spacing 3
                            , if selected then
                                E.paddingXY 0 4

                              else
                                E.padding 0
                            ]
                            [ if selected then
                                E.paragraph
                                    [ E.width E.fill
                                    , E.htmlAttribute (HA.style "overflow-wrap" "anywhere")
                                    ]
                                <|
                                    [ E.text nl.title ]

                              else
                                E.paragraph
                                    [ E.width E.fill
                                    , E.height (E.px <| fontsize * 5 // 4)
                                    , E.htmlAttribute (HA.style "overflow-wrap" "anywhere")
                                    , E.htmlAttribute (HA.style "overflow" "clip")
                                    ]
                                <|
                                    [ E.text nl.title ]
                            , if Just nl.id == Maybe.map Data.Zni model.currentUuid then
                                E.el [ EF.bold ] (E.text "▶")

                              else
                                E.none
                            ]
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
            model.nlls
        )



-------------------------------------------------------------------------
-- Drag and Drop
-------------------------------------------------------------------------
-- type DragDropWhat
--     = Drag
--     | Drop
--     | DropH
--     | Ghost
--     | Inactive
--     | DdwItemEdit
-- nllDndSubscriptions : Model -> List (Sub Msg)
-- nllDndSubscriptions model =
--     [ nllDndSystem.subscriptions model.nllDnd ]
-- -- to be called from main.elm!
-- ghostView : Model -> NoteCache -> Int -> Maybe (Element Msg)
-- ghostView model _ _ =
--     nllDndSystem.info model.nllDnd
--         |> Maybe.andThen
--             (\{ dragIndex } ->
--                 model.nlls
--                     |> (List.head << List.drop dragIndex)
--                     |> Maybe.map (\b -> ( dragIndex, b ))
--             )
--         |> Maybe.map
--             (\( i, item ) ->
--                 E.el
--                     (E.alpha 0.5
--                         :: List.map E.htmlAttribute
--                             (nllDndSystem.ghostStyles model.nllDnd)
--                     )
--                     (viewItem Ghost 0 (Just i) item)
--             )
-- nllId : Int -> String
-- nllId i =
--     "nll_" ++ String.fromInt i
-- edButtonStyle : Msg -> List (E.Attribute Msg)
-- edButtonStyle m =
--     [ EBk.color TC.blue
--     , EF.color TC.white
--     , EBd.color TC.darkBlue
--     , E.paddingXY 3 3
--     , EBd.rounded 2
--     , E.htmlAttribute <|
--         HE.custom "click"
--             (JD.succeed
--                 { message = m
--                 , stopPropagation = True
--                 , preventDefault = False
--                 }
--             )
--     ]
-- dragHandleWidth : Int
-- dragHandleWidth =
--     10
-- dndRow : (Int -> String) -> DragDropWhat -> Int -> Bool -> Element Msg -> Element Msg
-- dndRow toid ddw i focus e =
--     let
--         bid =
--             toid i
--         baseAttr =
--             (if focus then
--                 [ EBd.width 5 ]
--              else
--                 []
--             )
--                 ++ [ E.width E.fill
--                    , E.height E.fill
--                    , E.padding 3
--                    , E.spacing 2
--                    , E.htmlAttribute (HA.id bid)
--                    ]
--         dragHandleAttrs =
--             [ E.width (E.px dragHandleWidth), E.height E.fill, EBk.color TC.gray, E.alignBottom ]
--         spacer =
--             E.el [ E.width (E.px dragHandleWidth), E.height E.fill ] E.none
--     in
--     case ddw of
--         Drag ->
--             E.row
--                 baseAttr
--                 [ E.el
--                     (E.htmlAttribute (HA.style "touch-action" "none")
--                         :: dragHandleAttrs
--                         ++ List.map E.htmlAttribute (nllDndSystem.dragEvents i bid)
--                     )
--                     E.none
--                 , spacer
--                 , E.el [ E.width E.fill ] e
--                 ]
--         Drop ->
--             E.row
--                 (E.htmlAttribute (HA.style "touch-action" "none")
--                     :: baseAttr
--                     ++ List.map E.htmlAttribute (nllDndSystem.dropEvents i bid)
--                 )
--                 [ E.el dragHandleAttrs E.none
--                 , spacer
--                 , E.el [ E.width E.fill ] e
--                 ]
--         DropH ->
--             E.row
--                 ((EBk.color TC.darkBlue
--                     :: E.htmlAttribute (HA.style "touch-action" "none")
--                     :: baseAttr
--                  )
--                     ++ List.map E.htmlAttribute (nllDndSystem.dropEvents i bid)
--                 )
--                 [ E.el dragHandleAttrs E.none
--                 , spacer
--                 , E.el [ E.width E.fill ] e
--                 ]
--         Ghost ->
--             E.row
--                 ((EBk.color TC.darkGreen
--                     :: E.htmlAttribute (HA.style "touch-action" "none")
--                     :: baseAttr
--                  )
--                     ++ List.map E.htmlAttribute (nllDndSystem.dragEvents i bid)
--                 )
--                 [ E.el dragHandleAttrs E.none
--                 , spacer
--                 , E.el [ E.width E.fill ] e
--                 ]
--         Inactive ->
--             E.row
--                 baseAttr
--                 [ E.el (dragHandleAttrs ++ [ EBk.color TC.lightGray ]) E.none
--                 , spacer
--                 , E.el [ E.width E.fill ] e
--                 ]
--         DdwItemEdit ->
--             E.row
--                 baseAttr
--                 [ E.el (dragHandleAttrs ++ [ EBk.color TC.lightGray ]) E.none
--                 , spacer
--                 , E.el [ E.width E.fill ] e
--                 ]
-- viewItem : DragDropWhat -> Int -> Maybe Int -> NlLink -> Element Msg
-- viewItem ddw i focusid nll =
--     E.column
--         [ E.spacing 0
--         , E.padding 3
--         , E.width (E.fill |> E.maximum 1000)
--         , E.centerX
--         , E.alignTop
--         , EBd.width 2
--         , EBd.color TC.darkGrey
--         , EBk.color TC.lightGrey
--         ]
--         [ dndRow nllId ddw i (Just i == focusid) (E.text nll.title) ]
-------------------------------------------------------------------------
-- END Drag and Drop
-------------------------------------------------------------------------
