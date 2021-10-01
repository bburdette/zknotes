module WindowKeys exposing (Key, WindowKeyCmd(..), receive, send)

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



{- use send to make a  convenience function,
   like so:
         port sendKeyCommand : JE.Value -> Cmd msg
         wssend =
             WindowKey.send sendKeyCommand

   then you can call (makes a Cmd):
         wssend <|
             (SetWindowKeys
                [ { key = "Tab", ctrl = True, alt = True, shift = False, preventDefault = True }
                , { key = "s", ctrl = True, alt = False, shift = False, preventDefault = True }])
-}


send : (JE.Value -> Cmd msg) -> WindowKeyCmd -> Cmd msg
send portfn wsc =
    portfn (encodeCmd wsc)



{- make a subscription function with receive and a port, like so:
         port receiveKeyMsg : (JD.Value -> msg) -> Sub msg
         keyreceive =
             receiveSocketMsg <| WindowKey.receive WsMsg
   Where WkMessage is defined in your app like this:
         type Msg
             = WkMsg (Result JD.Error WindowKey.WindowKeyMsg)
             | <other message types>
   then in your application subscriptions:
         subscriptions =
             \_ -> keyreceive
-}


receive : (Result JD.Error WindowKeyMsg -> msg) -> (JD.Value -> msg)
receive toKeyMsg =
    \v ->
        JD.decodeValue decodeKey v
            |> toKeyMsg
