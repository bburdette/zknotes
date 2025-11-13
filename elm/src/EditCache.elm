module EditCache exposing (EdEntry, EditZkNote, ExistingStuff)

import Data exposing (EditLink, FileStatus, ZkNoteAndLinks, ZkNoteId)
import DataUtil
import Dict exposing (Dict)
import EdMarkdown exposing (EdMarkdown)
import Orgauth.Data exposing (UserId)
import TDict exposing (TDict)


type alias EdEntry =
    { edm : EdMarkdown
    , original : Maybe ZkNoteAndLinks
    , zklDict : Dict String EditLink
    }


{-| fields that only exist in a saved note.
-}
type alias ExistingStuff =
    { id : ZkNoteId
    , user : UserId
    , username : String
    , usernote : ZkNoteId
    , filestatus : FileStatus
    , createdate : Int
    , changeddate : Int
    , server : String
    }


{-| note fields.
-}
type alias EditZkNote =
    { xstuff : Maybe ExistingStuff
    , title : String
    , content : String
    , editable : Bool
    , editableValue : Bool
    , showtitle : Bool
    , pubid : Maybe String
    , deleted : Bool
    , sysids : List ZkNoteId
    }



-- new notes need fake ids or something!
-- situation:  have a note, embed another note in it, but that is a new note!
--   hao to do?


type alias EditIdGen =
    { nextId : Int
    }


getId : EditIdGen -> ( Int, EditIdGen )
getId eig =
    ( eig.nextId, { eig | nextId = eig.nextId + 1 } )


type EditId
    = EiZkn ZkNoteId
    | EiNew Int


editIdToString : EditId -> String
editIdToString ei =
    case ei of
        EiZkn zni ->
            -- 'should be' a uuid
            "z" ++ DataUtil.zkNoteIdToString zni

        EiNew i ->
            "n" ++ String.fromInt i


stringToEditId : String -> EditId
stringToEditId s =
    case String.left 1 s of
        "z" ->
            String.dropLeft 1 s
                |> DataUtil.trustedZkNoteIdFromString
                |> EiZkn

        "n" ->
            String.dropLeft 1 s
                |> String.toInt
                |> Maybe.withDefault -1
                |> EiNew

        _ ->
            EiNew -1


type alias EdDict =
    TDict EditId String EdEntry


emptyEdDict : EdDict
emptyEdDict =
    TDict.empty editIdToString stringToEditId
