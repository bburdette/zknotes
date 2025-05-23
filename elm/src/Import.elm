module Import exposing
    ( Command(..)
    , LinkHalf
    , Links
    , Model
    , Msg(..)
    , WClass(..)
    , decodeLinks
    , importview
    , init
    , jsplit
    , noteLink
    , parseContent
    , rbrak
    , showLh
    , update
    , view
    , zkLinkName
    , zklKey
    )

import Cellme.Cellme exposing (CellContainer(..), RunState(..))
import Cellme.DictCellme exposing (CellDict(..))
import Common
import Data exposing (ZkNoteId)
import DataUtil exposing (zniEq)
import Dialog as D
import Element as E exposing (Element)
import Element.Background as EBk
import Element.Font as EF
import Element.Input as EI
import File as F
import Json.Decode as JD
import Markdown.Block exposing (ListItem(..), Task(..))
import MdCommon exposing (..)
import SearchStackPanel as SP
import TDict exposing (TDict)
import TangoColors as TC
import Task
import Url as U
import Url.Parser as UP exposing ((</>))
import Util


type Msg
    = SavePress
    | CancelPress
    | FilesPress
    | LinkPress Data.ZkListNote
    | RemoveLink ZkNoteId
    | SPMsg SP.Msg
    | DialogMsg D.Msg
    | FilesSelected F.File (List F.File)
    | FileLoaded String String
    | Noop


type alias LinkHalf =
    { from : Bool
    , to : Bool
    , title : String
    }


type alias Links =
    { fromlinks : List String
    , tolinks : List String
    }


type alias Model =
    { ld : DataUtil.LoginData
    , notes : List Data.ImportZkNote
    , globlinks : TDict ZkNoteId String LinkHalf
    , dialog : Maybe D.Model
    }


type Command
    = None
    | SaveExit (List Data.ImportZkNote)
    | SelectFiles
    | Cancel
    | Command (Cmd Msg)
    | SPMod (SP.Model -> ( SP.Model, SP.Command ))


decodeLinks : JD.Decoder Links
decodeLinks =
    JD.map2 Links
        (JD.field "fromlinks" (JD.list JD.string))
        (JD.field "tolinks" (JD.list JD.string))


addLinks : Data.ImportZkNote -> LinkHalf -> Data.ImportZkNote
addLinks izn lh =
    { izn
        | fromLinks =
            if lh.from then
                lh.title :: izn.fromLinks

            else
                izn.fromLinks
        , toLinks =
            if lh.to then
                lh.title :: izn.toLinks

            else
                izn.toLinks
    }


zkLinkName : Data.ZkLink -> ZkNoteId -> String
zkLinkName zklink noteid =
    if zniEq noteid zklink.from then
        zklink.toname |> Maybe.withDefault (DataUtil.zkNoteIdToString zklink.to)

    else if zniEq noteid zklink.to then
        zklink.fromname |> Maybe.withDefault (DataUtil.zkNoteIdToString zklink.from)

    else
        "link error"


showLh : ZkNoteId -> LinkHalf -> Element Msg
showLh id lh =
    E.row [ E.spacing 8, E.width E.fill ]
        [ case ( lh.from, lh.to ) of
            ( False, False ) ->
                E.text ""

            ( True, False ) ->
                E.text "<-"

            ( False, True ) ->
                E.text "->"

            ( True, True ) ->
                E.text "<- ->"
        , E.row
            [ E.clipX
            , E.centerY
            , E.height E.fill
            , E.width E.fill
            ]
            [ E.text lh.title
            ]
        , EI.button (Common.buttonStyle ++ [ E.alignRight ])
            { onPress = Just (RemoveLink id)
            , label = E.text "X"
            }
        ]


type WClass
    = Narrow
    | Medium
    | Wide


view : Util.Size -> Model -> SP.Model -> Data.ZkListNoteSearchResult -> Element Msg
view size model spmodel zknSearchResult =
    case model.dialog of
        Just dialog ->
            D.view size dialog |> E.map DialogMsg

        Nothing ->
            importview size model spmodel zknSearchResult


importview : Util.Size -> Model -> SP.Model -> Data.ZkListNoteSearchResult -> Element Msg
importview size model spmodel zknSearchResult =
    let
        wclass =
            if size.width < 800 then
                Narrow

            else if size.width > 1700 then
                Wide

            else
                Medium

        isdirty =
            not <| List.isEmpty model.notes

        showLinks =
            E.row [ EF.bold ] [ E.text "links" ]
                :: List.map
                    (\( a, b ) -> showLh a b)
                    (TDict.toList model.globlinks)

        searchPanel =
            let
                spwidth =
                    case wclass of
                        Narrow ->
                            E.fill

                        Medium ->
                            E.px 400

                        Wide ->
                            E.px 400
            in
            E.column
                [ E.spacing 8
                , E.alignTop
                , E.alignRight
                , E.width spwidth
                ]
                ((E.map SPMsg <|
                    SP.view False (wclass == Narrow) 0 spmodel
                 )
                    :: (List.map
                            (\zkln ->
                                let
                                    tolinked =
                                        TDict.get zkln.id model.globlinks
                                            |> Maybe.map
                                                (\lh -> lh.to)
                                            |> Maybe.withDefault
                                                False
                                in
                                E.row [ E.spacing 8, E.width E.fill ]
                                    [ EI.button
                                        (if tolinked then
                                            Common.disabledButtonStyle

                                         else
                                            Common.buttonStyle
                                        )
                                        { onPress =
                                            if tolinked then
                                                Nothing

                                            else
                                                Just (LinkPress zkln)
                                        , label = E.text "link"
                                        }
                                    , E.row
                                        [ E.clipX
                                        , E.height E.fill
                                        , E.width E.fill
                                        ]
                                        [ E.text zkln.title
                                        ]
                                    ]
                            )
                        <|
                            zknSearchResult.notes
                       )
                )
    in
    E.column
        [ E.width E.fill, E.spacing 8, E.padding 8 ]
        [ E.row [ E.width E.fill, E.spacing 8 ]
            [ E.row [ EF.bold ] [ E.text model.ld.name ]
            ]
        , E.row [ E.width E.fill, E.spacing 8 ]
            [ EI.button Common.buttonStyle { onPress = Just FilesPress, label = E.text "select files" }
            , EI.button Common.buttonStyle { onPress = Just CancelPress, label = E.text "cancel" }
            , if isdirty then
                EI.button (Common.buttonStyle ++ [ EBk.color TC.darkYellow ])
                    { onPress = Just SavePress, label = E.text "save" }

              else
                E.none
            ]
        , E.row
            [ E.width E.fill
            , E.spacing 8
            , E.alignTop
            ]
            [ E.column [ E.alignTop, E.spacing 8 ]
                (List.map
                    (\note ->
                        E.row [ E.spacing 8 ]
                            (E.text note.title
                                :: (List.map E.text note.fromLinks
                                        ++ List.map E.text note.toLinks
                                   )
                            )
                    )
                    model.notes
                    ++ showLinks
                )
            , searchPanel
            ]
        ]


zklKey : { a | from : Int, to : Int } -> String
zklKey zkl =
    String.fromInt zkl.from ++ ":" ++ String.fromInt zkl.to


init : DataUtil.LoginData -> Model
init ld =
    { ld = ld
    , notes = []
    , globlinks = TDict.empty DataUtil.zkNoteIdToString DataUtil.trustedZkNoteIdFromString
    , dialog = Nothing
    }


noteLink : String -> Maybe Int
noteLink str =
    -- hack allows parsing /note/<N>
    -- other urls will be invalid which is fine.
    U.fromString ("http://wat" ++ str)
        |> Maybe.andThen
            (UP.parse (UP.s "note" </> UP.int))


parseContent : String -> Result JD.Error Links
parseContent c =
    JD.decodeString decodeLinks c


jsplit : String -> ( Maybe String, String )
jsplit s =
    if String.left 1 s == "{" then
        rbrak s 1 1 (String.length s)

    else
        ( Nothing, s )


rbrak : String -> Int -> Int -> Int -> ( Maybe String, String )
rbrak s c i l =
    if i == l then
        ( Nothing, s )

    else
        let
            f =
                String.slice i (i + 1) s
        in
        if f == "{" then
            rbrak s (c + 1) (i + 1) l

        else if f == "}" then
            if c == 1 then
                ( Just <| String.slice 0 (i + 1) s, String.slice (i + 1) l s )

            else
                rbrak s (c - 1) (i + 1) l

        else
            rbrak s c (i + 1) l


update : Msg -> Model -> ( Model, Command )
update msg model =
    case msg of
        SavePress ->
            let
                gl =
                    TDict.values model.globlinks
            in
            ( model
            , SaveExit <|
                List.map (\n -> List.foldl (\glk note -> addLinks note glk) n gl)
                    model.notes
            )

        CancelPress ->
            ( model, Cancel )

        FilesPress ->
            ( model, SelectFiles )

        FileLoaded name content ->
            let
                ( mbjs, s ) =
                    jsplit content

                links =
                    mbjs
                        |> Maybe.map
                            (\js ->
                                parseContent js
                                    |> Result.withDefault { fromlinks = [], tolinks = [] }
                            )
                        |> Maybe.withDefault { fromlinks = [], tolinks = [] }
            in
            ( { model
                | notes =
                    model.notes
                        ++ [ { title = name
                             , content = s
                             , fromLinks = links.fromlinks
                             , toLinks = links.tolinks
                             }
                           ]
              }
            , None
            )

        FilesSelected f fs ->
            ( model
            , Command <|
                Cmd.batch <|
                    List.map
                        (\file ->
                            Task.perform (FileLoaded (F.name file)) (F.toString file)
                        )
                        (f :: fs)
            )

        LinkPress zkln ->
            -- add a zklink, or newlink?
            ( { model
                | globlinks = TDict.insert zkln.id { title = zkln.title, to = True, from = False } model.globlinks
              }
            , None
            )

        RemoveLink id ->
            ( { model
                | globlinks = TDict.remove id model.globlinks
              }
            , None
            )

        DialogMsg dm ->
            case model.dialog of
                Just dmod ->
                    case D.update dm dmod of
                        D.Cancel ->
                            ( { model | dialog = Nothing }, None )

                        D.Ok ->
                            ( { model | dialog = Nothing }, None )

                        D.Dialog dmod2 ->
                            ( { model | dialog = Just dmod2 }, None )

                Nothing ->
                    ( model, None )

        SPMsg m ->
            ( model
            , SPMod (SP.update m)
            )

        Noop ->
            ( model, None )
