module WindowKeys exposing (Key, WindowKeyCmd(..), send)

import Json.Decode as JD
import Json.Encode as JE


type alias Key =
    { key : String
    , ctrl : Bool
    , alt : Bool
    , shift : Bool
    , preventDefault : Bool
    }


type WindowKeyCmd
    = SetWindowKeys (List Key)


type alias WindowKeyMsg =
    Key


encodeKey : Key -> JE.Value
encodeKey key =
    JE.object
        [ ( "key", JE.string key.key )
        , ( "ctrl", JE.bool key.ctrl )
        , ( "alt", JE.bool key.alt )
        , ( "shift", JE.bool key.shift )
        , ( "preventDefault", JE.bool key.preventDefault )
        ]


decodeKey : JD.Decoder Key
decodeKey =
    JD.map5 Key
        (JD.field "key" JD.string)
        (JD.field "ctrl" JD.bool)
        (JD.field "alt" JD.bool)
        (JD.field "shift" JD.bool)
        (JD.field "preventDefault" JD.bool)


encodeCmd : WindowKeyCmd -> JE.Value
encodeCmd c =
    case c of
        SetWindowKeys keys ->
            JE.object
                [ ( "cmd", JE.string "SetWindowKeys" )
                , ( "keys", JE.list encodeKey keys )
                ]


send : (JE.Value -> Cmd msg) -> WindowKeyCmd -> Cmd msg
send portfn wsc =
    portfn (encodeCmd wsc)


receive : (Result JD.Error WindowKeyMsg -> msg) -> (JD.Value -> msg)
receive wsmMsg =
    \v ->
        JD.decodeValue decodeKey v
            |> wsmMsg
