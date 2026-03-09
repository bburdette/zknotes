port module DndPorts exposing (..)

import Json.Encode as JE


port onPointerMove : (JE.Value -> msg) -> Sub msg


port onPointerUp : (JE.Value -> msg) -> Sub msg


port releasePointerCapture : JE.Value -> Cmd msg


