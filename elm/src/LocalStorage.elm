port module LocalStorage exposing (ReceiveValue, clearLocalStorage, getLocalVal, localVal, storeLocalVal)

{-| store a name,value pair in local storage.
-}


port storeLocalVal : { name : String, value : String } -> Cmd msg


{-| retrieve a value for 'name' if it exists. 'for' is passed through to help
with routing when the result is returned.
-}
port getLocalVal : { for : String, name : String } -> Cmd msg


{-| remove all values from local storage
-}
port clearLocalStorage : () -> Cmd msg


{-| value struct that is returned after a getLocalVal call
-}
type alias ReceiveValue =
    { for : String, name : String, value : Maybe String }


{-| subscription
-}
port localVal : (ReceiveValue -> msg) -> Sub msg
