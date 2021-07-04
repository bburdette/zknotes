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
import TangoColors as TC
import UUID exposing (UUID)
import Util exposing (httpErrorString)


type alias Model =
    { userId : String
    , password : String
    , reset_key : UUID
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


initialModel : String -> UUID -> String -> Model
initialModel uid reset_key appname =
    { userId = uid
    , password = ""
    , reset_key = reset_key
    , appname = appname
    , sent = False
    }


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
                resetView model
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
        [ row [ width fill ] [ el [ centerX, Font.bold ] <| text <| "password reset" ]
        , row [ spacing 8 ] [ text "user id:", text model.userId ]
        , Input.newPassword [ width fill ]
            { onChange = PasswordUpdate
            , text = model.password
            , placeholder = Nothing
            , show = False
            , label = Input.labelLeft [] <| text "new password:"
            }
        , Input.button (buttonStyle ++ [ width fill ])
            { onPress = Just OkPressed
            , label = text "change to new password"
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
