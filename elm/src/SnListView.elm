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
