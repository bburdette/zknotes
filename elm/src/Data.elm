module Data exposing (..)

import Json.Decode as JD
import Json.Encode as JE
import Orgauth.Data
import Search as S
import UUID exposing (UUID)
import Url.Builder as UB
import Util exposing (andMap)



------------------------------------------------------------
-- getting, setting text selections in text edit areas.
------------------------------------------------------------


type alias SetSelection =
    { id : String
    , offset : Int
    , length : Int
    }


type alias TASelection =
    { text : String
    , offset : Int
    , what : String
    }


encodeSetSelection : SetSelection -> JE.Value
encodeSetSelection s =
    JE.object
        [ ( "id", JE.string s.id )
        , ( "offset", JE.int s.offset )
        , ( "length", JE.int s.length )
        ]


decodeTASelection : JD.Decoder TASelection
decodeTASelection =
    JD.succeed TASelection
        |> andMap (JD.field "text" JD.string)
        |> andMap (JD.field "offset" JD.int)
        |> andMap (JD.field "what" JD.string)



----------------------------------------
-- types sent to or from the server.
----------------------------------------


fromOaLd : Orgauth.Data.LoginData -> Result JD.Error LoginData
fromOaLd oald =
    JD.decodeValue
        (JD.succeed (LoginData oald.userid oald.name oald.admin)
            |> andMap (JD.field "zknote" JD.int)
            |> andMap (JD.field "homenote" (JD.maybe JD.int))
            |> andMap (JD.field "publicid" JD.int)
            |> andMap (JD.field "shareid" JD.int)
            |> andMap (JD.field "searchid" JD.int)
            |> andMap (JD.field "commentid" JD.int)
        )
        oald.data


type alias LoginData =
    { userid : Int
    , name : String
    , admin : Bool
    , zknote : Int
    , homenote : Maybe Int
    , publicid : Int
    , shareid : Int
    , searchid : Int
    , commentid : Int
    }


type alias ZkListNote =
    { id : Int
    , user : Int
    , title : String
    , createdate : Int
    , changeddate : Int
    , sysids : List Int
    }


type alias ZkListNoteSearchResult =
    { notes : List ZkListNote
    , offset : Int
    , what : String
    }


type alias ZkNoteSearchResult =
    { notes : List ZkNote
    , offset : Int
    , what : String
    }


type alias SavedZkNote =
    { id : Int
    , changeddate : Int
    }


type alias ZkNote =
    { id : Int
    , user : Int
    , username : String
    , usernote : Int
    , title : String
    , content : String
    , pubid : Maybe String
    , editable : Bool -- whether I'm allowed to edit the note.
    , editableValue : Bool -- whether the user has marked it editable.
    , showtitle : Bool
    , createdate : Int
    , changeddate : Int
    , sysids : List Int
    }


type alias SaveZkNote =
    { id : Maybe Int
    , pubid : Maybe String
    , title : String
    , content : String
    , editable : Bool
    , showtitle : Bool
    }


type alias ZkLink =
    { from : Int
    , to : Int
    , user : Int
    , zknote : Maybe Int
    , fromname : Maybe String
    , toname : Maybe String
    , delete : Maybe Bool
    }


type Direction
    = From
    | To


type alias EditLink =
    { otherid : Int
    , direction : Direction
    , user : Int
    , zknote : Maybe Int
    , othername : Maybe String
    , sysids : List Int
    }


type alias SaveZkLink =
    { otherid : Int
    , direction : Direction
    , user : Int
    , zknote : Maybe Int
    , delete : Maybe Bool
    }


type alias SaveZkNotePlusLinks =
    { note : SaveZkNote
    , links : List SaveZkLink
    }


type alias ZkLinks =
    { links : List ZkLink
    }


type alias SaveZkLinks =
    { saveLinks : List ZkLink
    , deleteLinks : List ZkLink
    }


type alias ImportZkNote =
    { title : String, content : String, fromLinks : List String, toLinks : List String }


type alias GetZkLinks =
    { zknote : Int
    }


type alias GetZkNoteComments =
    { zknote : Int
    , offset : Int
    , limit : Maybe Int
    }


type alias GetZkNoteEdit =
    { zknote : Int
    }


type alias ZkNoteEdit =
    { zknote : ZkNote
    , links : List EditLink
    , panelNote : Maybe ZkNote
    }



----------------------------------------
-- Json encoders/decoders
----------------------------------------


encodeGetZkLinks : GetZkLinks -> JE.Value
encodeGetZkLinks gzl =
    JE.object
        [ ( "zknote", JE.int gzl.zknote )
        ]


encodeGetZkNoteEdit : GetZkNoteEdit -> JE.Value
encodeGetZkNoteEdit gzl =
    JE.object
        [ ( "zknote", JE.int gzl.zknote )
        ]


encodeGetZkNoteComments : GetZkNoteComments -> JE.Value
encodeGetZkNoteComments x =
    JE.object <|
        [ ( "zknote", JE.int x.zknote )
        , ( "offset", JE.int x.offset )
        ]
            ++ (case x.limit of
                    Just l ->
                        [ ( "limit", JE.int l )
                        ]

                    Nothing ->
                        []
               )


encodeZkLinks : ZkLinks -> JE.Value
encodeZkLinks zklinks =
    JE.object
        [ ( "links", JE.list encodeZkLink zklinks.links )
        ]


encodeDirection : Direction -> JE.Value
encodeDirection direction =
    case direction of
        To ->
            JE.string "To"

        From ->
            JE.string "From"


decodeDirection : JD.Decoder Direction
decodeDirection =
    JD.string
        |> JD.andThen
            (\s ->
                case s of
                    "From" ->
                        JD.succeed From

                    "To" ->
                        JD.succeed To

                    wat ->
                        JD.fail ("not a direction: " ++ wat)
            )


encodeSaveZkLink : SaveZkLink -> JE.Value
encodeSaveZkLink s =
    [ Just ( "otherid", JE.int s.otherid )
    , Just ( "direction", encodeDirection s.direction )
    , Just ( "user", JE.int s.user )
    , s.zknote |> Maybe.map (\n -> ( "zknote", JE.int n ))
    , s.delete |> Maybe.map (\n -> ( "delete", JE.bool n ))
    ]
        |> List.filterMap identity
        |> JE.object


decodeZkLinks : JD.Decoder ZkLinks
decodeZkLinks =
    JD.map ZkLinks
        (JD.field "links" (JD.list decodeZkLink))


encodeZkLink : ZkLink -> JE.Value
encodeZkLink zklink =
    JE.object <|
        [ ( "from", JE.int zklink.from )
        , ( "to", JE.int zklink.to )
        , ( "user", JE.int zklink.user )
        ]
            ++ (zklink.delete
                    |> Maybe.map (\b -> [ ( "delete", JE.bool b ) ])
                    |> Maybe.withDefault []
               )
            ++ (zklink.zknote
                    |> Maybe.map
                        (\id ->
                            [ ( "linkzknote", JE.int id ) ]
                        )
                    |> Maybe.withDefault
                        []
               )


decodeZkLink : JD.Decoder ZkLink
decodeZkLink =
    JD.map7 ZkLink
        (JD.field "from" JD.int)
        (JD.field "to" JD.int)
        (JD.field "user" JD.int)
        (JD.maybe (JD.field "linkzknote" JD.int))
        (JD.maybe (JD.field "fromname" JD.string))
        (JD.maybe (JD.field "toname" JD.string))
        (JD.succeed Nothing)


decodeEditLink : JD.Decoder EditLink
decodeEditLink =
    JD.map6 EditLink
        (JD.field "otherid" JD.int)
        (JD.field "direction" decodeDirection)
        (JD.field "user" JD.int)
        (JD.maybe (JD.field "zknote" JD.int))
        (JD.maybe (JD.field "othername" JD.string))
        (JD.field "sysids" (JD.list JD.int))


saveZkNote : ZkNote -> SaveZkNote
saveZkNote fzn =
    { id = Just fzn.id
    , pubid = fzn.pubid
    , title = fzn.title
    , content = fzn.content
    , editable = fzn.editableValue
    , showtitle = fzn.showtitle
    }


encodeSaveZkNotePlusLinks : SaveZkNotePlusLinks -> JE.Value
encodeSaveZkNotePlusLinks s =
    JE.object
        [ ( "note", encodeSaveZkNote s.note )
        , ( "links", JE.list encodeSaveZkLink s.links )
        ]


encodeSaveZkNote : SaveZkNote -> JE.Value
encodeSaveZkNote zkn =
    JE.object <|
        (Maybe.map (\id -> [ ( "id", JE.int id ) ]) zkn.id
            |> Maybe.withDefault []
        )
            ++ (Maybe.map (\pubid -> [ ( "pubid", JE.string pubid ) ]) zkn.pubid
                    |> Maybe.withDefault []
               )
            ++ [ ( "title", JE.string zkn.title )
               , ( "content", JE.string zkn.content )
               , ( "editable", JE.bool zkn.editable )
               , ( "showtitle", JE.bool zkn.showtitle )
               ]


decodeZkListNote : JD.Decoder ZkListNote
decodeZkListNote =
    JD.map6 ZkListNote
        (JD.field "id" JD.int)
        (JD.field "user" JD.int)
        (JD.field "title" JD.string)
        (JD.field "createdate" JD.int)
        (JD.field "changeddate" JD.int)
        (JD.field "sysids" (JD.list JD.int))


decodeZkListNoteSearchResult : JD.Decoder ZkListNoteSearchResult
decodeZkListNoteSearchResult =
    JD.map3 ZkListNoteSearchResult
        (JD.field "notes" (JD.list decodeZkListNote))
        (JD.field "offset" JD.int)
        (JD.field "what" JD.string)


decodeZkNoteSearchResult : JD.Decoder ZkNoteSearchResult
decodeZkNoteSearchResult =
    JD.map3 ZkNoteSearchResult
        (JD.field "notes" (JD.list decodeZkNote))
        (JD.field "offset" JD.int)
        (JD.field "what" JD.string)


decodeSavedZkNote : JD.Decoder SavedZkNote
decodeSavedZkNote =
    JD.map2 SavedZkNote
        (JD.field "id" JD.int)
        (JD.field "changeddate" JD.int)


decodeZkNote : JD.Decoder ZkNote
decodeZkNote =
    JD.succeed ZkNote
        |> andMap (JD.field "id" JD.int)
        |> andMap (JD.field "user" JD.int)
        |> andMap (JD.field "username" JD.string)
        |> andMap (JD.field "usernote" JD.int)
        |> andMap (JD.field "title" JD.string)
        |> andMap (JD.field "content" JD.string)
        |> andMap (JD.field "pubid" (JD.maybe JD.string))
        |> andMap (JD.field "editable" JD.bool)
        |> andMap (JD.field "editableValue" JD.bool)
        |> andMap (JD.field "showtitle" JD.bool)
        |> andMap (JD.field "createdate" JD.int)
        |> andMap (JD.field "changeddate" JD.int)
        |> andMap (JD.field "sysids" <| JD.list JD.int)


decodeZkNoteEdit : JD.Decoder ZkNoteEdit
decodeZkNoteEdit =
    JD.map3 ZkNoteEdit
        (JD.field "zknote" decodeZkNote)
        (JD.field "links" (JD.list decodeEditLink))
        (JD.succeed Nothing)


decodeLoginData : JD.Decoder LoginData
decodeLoginData =
    JD.succeed LoginData
        |> andMap (JD.field "userid" JD.int)
        |> andMap (JD.field "name" JD.string)
        |> andMap (JD.field "admin" JD.bool)
        |> andMap (JD.field "data" (JD.field "zknote" JD.int))
        |> andMap (JD.field "data" (JD.field "homenote" (JD.maybe JD.int)))
        |> andMap (JD.field "data" (JD.field "publicid" JD.int))
        |> andMap (JD.field "data" (JD.field "shareid" JD.int))
        |> andMap (JD.field "data" (JD.field "searchid" JD.int))
        |> andMap (JD.field "data" (JD.field "commentid" JD.int))


encodeImportZkNote : ImportZkNote -> JE.Value
encodeImportZkNote izn =
    JE.object
        [ ( "title", JE.string izn.title )
        , ( "content", JE.string izn.content )
        , ( "fromLinks", JE.list JE.string izn.fromLinks )
        , ( "toLinks", JE.list JE.string izn.toLinks )
        ]



----------------------------------------
-- misc functions
----------------------------------------


editNoteLink : Int -> String
editNoteLink noteid =
    UB.absolute [ "editnote", String.fromInt noteid ] []


flipDirection : Direction -> Direction
flipDirection direction =
    case direction of
        To ->
            From

        From ->
            To
