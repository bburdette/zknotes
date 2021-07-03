module ResetPassword exposing (Cmd(..), Model, Msg(..), initialModel, resetView, sentView, update, view)

import Common exposing (buttonStyle)
import Dict exposing (Dict)
import Element exposing (..)
import Element.Background as Background
import Element.Border as Border
import Element.Events exposing (onClick)
import Element.Font as Font
import Element.Input as Input
import Html exposing (Html)
import Random exposing (Seed)
import TangoColors as Color
import Util exposing (httpErrorString)


type alias Model =
    { userId : String
    , password : String
    , reset_key : String
    , sent : Bool
    , appname : String
    }


type Msg
    = PasswordUpdate String
    | OkPressed
    | Noop


type Cmd
    = Ok
    | None


initialModel : String -> String -> String -> Model
initialModel uid reset_key appname =
    { userId = uid
    , password = ""
    , reset_key = reset_key
    , appname = appname
    , sent = False
    }



{- makeUrlP : Model -> ( String, Dict String String )
   makeUrlP model =
       case model.mode of
           RegistrationMode ->
               ( "/registration", Dict.empty )

           LoginMode ->
               ( "/login", Dict.empty )

           ResetMode ->
               ( "/reset", Dict.empty )
-}
{- urlToState : List String -> Dict String String -> Model -> Model
   urlToState segments parms model =
       { model
           | mode =
               case List.head segments of
                   Just "login" ->
                       LoginMode

                   Just "reset" ->
                       ResetMode

                   Just "registration" ->
                       RegistrationMode

                   _ ->
                       model.mode
       }

-}


view : Util.Size -> Model -> Element Msg
view size model =
    column [ width fill, height (px size.height) ]
        [ column
            [ centerX
            , centerY
            , Background.color (Common.navbarColor 1)
            , Border.rounded 10
            , padding 10
            ]
            [ if model.sent then
                sentView model

              else
                none
            ]
        ]


resetView : Model -> Element Msg
resetView model =
    column
        [ spacing 8
        , width fill
        , padding 10
        , Background.color (Common.navbarColor 1)
        ]
        [ text <| "forgot your password?"
        , Input.text [ width fill ]
            { onChange = always Noop
            , text = model.userId
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "User id:"
            }
        , Input.button (buttonStyle ++ [ width fill ])
            { onPress = Just OkPressed
            , label = text "send reset email"
            }
        ]


sentView : Model -> Element Msg
sentView model =
    column
        [ spacing 8
        , width fill
        , padding 10
        , Background.color (Common.navbarColor 1)
        ]
        [ text <| "password change sent..."
        ]


update : Msg -> Model -> ( Model, Cmd )
update msg model =
    case msg of
        PasswordUpdate txt ->
            ( { model | password = txt, sent = False }, None )

        OkPressed ->
            ( { model | sent = True }, Ok )

        Noop ->
            ( model, None )
