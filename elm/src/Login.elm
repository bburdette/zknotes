module Login exposing (Cmd(..), Mode(..), Model, Msg(..), initialModel, invalidUserOrPwd, loginView, makeUrlP, registrationSent, registrationView, sentView, unregisteredUser, update, urlToState, userExists, view)

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


type Mode
    = RegistrationMode
    | LoginMode


type alias Model =
    { userId : String
    , password : String
    , email : String
    , captcha : String
    , captchaQ : ( String, Int )
    , seed : Seed
    , mode : Mode
    , sent : Bool
    , responseMessage : String
    , postLoginUrl : Maybe ( List String, Dict String String )
    , appname : String
    }


type Msg
    = IdUpdate String
    | CaptchaUpdate String
    | PasswordUpdate String
    | EmailUpdate String
    | SetMode Mode
    | TryLogin
    | TryRegister
    | CancelSent


type Cmd
    = Login
    | Register
    | None


initialModel : Maybe { uid : String, pwd : String } -> String -> Seed -> Model
initialModel mblogin appname seed =
    let
        ( newseed, cq, cans ) =
            Util.captchaQ seed

        ( uid, pwd ) =
            case mblogin of
                Just info ->
                    ( info.uid, info.pwd )

                Nothing ->
                    ( "", "" )
    in
    { userId = uid
    , password = pwd
    , email = ""
    , captcha = ""
    , captchaQ = ( cq, cans )
    , seed = newseed
    , mode = LoginMode
    , sent = False
    , responseMessage = ""
    , postLoginUrl = Nothing
    , appname = appname
    }


makeUrlP : Model -> ( String, Dict String String )
makeUrlP model =
    case model.mode of
        RegistrationMode ->
            ( "/registration", Dict.empty )

        LoginMode ->
            ( "/login", Dict.empty )


urlToState : List String -> Dict String String -> Model -> Model
urlToState segments parms model =
    { model
        | mode =
            case List.head segments of
                Just "login" ->
                    LoginMode

                Just "registration" ->
                    RegistrationMode

                _ ->
                    model.mode
    }


userExists : Model -> Model
userExists model =
    { model
        | responseMessage = "can't register - this user id already exixts!"
        , sent = False
    }


unregisteredUser : Model -> Model
unregisteredUser model =
    { model
        | responseMessage = "can't login - this user is not registered"
        , sent = False
    }


registrationSent : Model -> Model
registrationSent model =
    { model
        | responseMessage = "registration sent.  check your spam folder for email from " ++ model.appname ++ "!"
        , sent = False
    }


invalidUserOrPwd : Model -> Model
invalidUserOrPwd model =
    { model
        | responseMessage = "can't login - invalid user or password."
        , sent = False
    }


view : Util.Size -> Model -> Element Msg
view size model =
    column [ height fill, width fill, centerX, centerY, Background.color (Common.navbarColor 4) ]
        [ row [ width fill ]
            [ Common.navbar 0
                model.mode
                SetMode
                [ ( LoginMode, "Log in" ), ( RegistrationMode, "Register" ) ]
            ]
        , if model.sent then
            sentView model

          else
            case model.mode of
                LoginMode ->
                    loginView model

                RegistrationMode ->
                    registrationView model
        ]


loginView : Model -> Element Msg
loginView model =
    column [ spacing 5, width fill, padding 10, Background.color (Common.navbarColor 1) ]
        [ text <| "welcome to " ++ model.appname ++ "!  log in below:"
        , Input.text [ width fill ]
            { onChange = IdUpdate
            , text = model.userId
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "User id:"
            }
        , Input.currentPassword [ width fill ]
            { onChange = PasswordUpdate
            , text = model.password
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "password: "
            , show = False
            }
        , text model.responseMessage
        , Input.button (buttonStyle ++ [ width fill ])
            { onPress = Just TryLogin
            , label = text "log in"
            }
        , Input.button (buttonStyle ++ [ width fill ])
            { onPress = Just (SetMode RegistrationMode)
            , label = text "reset password"
            }
        ]


registrationView : Model -> Element Msg
registrationView model =
    column [ Background.color (Common.navbarColor 1), width fill ]
        [ row [] [ text <| "welcome to " ++ model.appname ++ "!  register your new account below:" ]
        , text "email: "
        , Input.text []
            { onChange = EmailUpdate
            , text = model.email
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "Email:"
            }
        , Input.text []
            { onChange = IdUpdate
            , text = model.userId
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "User id:"
            }
        , Input.currentPassword []
            { onChange = PasswordUpdate
            , text = model.password
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "password: "
            , show = False
            }
        , Input.text []
            { onChange = CaptchaUpdate
            , text = model.captcha
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text <| Tuple.first model.captchaQ
            }
        , text model.responseMessage
        , Input.button (buttonStyle ++ [ width fill ])
            { onPress = Just TryRegister
            , label = text "register"
            }
        ]


sentView : Model -> Element Msg
sentView model =
    column [ width fill ]
        [ text
            (case model.mode of
                LoginMode ->
                    "Logging in..."

                RegistrationMode ->
                    "Registration sent..."
            )
        ]


update : Msg -> Model -> ( Model, Cmd )
update msg model =
    case msg of
        IdUpdate id ->
            ( { model | userId = id, sent = False }, None )

        EmailUpdate txt ->
            ( { model | email = txt, sent = False }, None )

        PasswordUpdate txt ->
            ( { model | password = txt, sent = False }, None )

        CaptchaUpdate txt ->
            ( { model | captcha = txt, sent = False }, None )

        SetMode mode ->
            ( { model | mode = mode, sent = False, responseMessage = "" }, None )

        CancelSent ->
            ( { model | sent = False }, None )

        TryRegister ->
            let
                ( newseed, cq, cans ) =
                    Util.captchaQ model.seed

                newmod =
                    { model
                        | seed = newseed
                        , captchaQ = ( cq, cans )
                    }
            in
            if String.toInt model.captcha == (Just <| Tuple.second model.captchaQ) then
                ( { newmod | sent = True }, Register )

            else
                ( newmod, None )

        TryLogin ->
            ( { model | sent = True }, Login )
