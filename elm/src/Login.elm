module Login exposing (Cmd(..), Mode(..), Model, Msg(..), initialModel, invalidUserOrPwd, loginView, makeUrlP, onWkKeyPress, registrationSent, registrationView, sentView, unregisteredUser, update, urlToState, userExists, view)

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
import Toop
import Util exposing (httpErrorString)
import WindowKeys as WK


type Mode
    = RegistrationMode
    | LoginMode
    | ResetMode


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
    | LoginPressed
    | RegisterPressed
    | ResetPressed
    | CancelSent


type Cmd
    = Login
    | Register
    | Reset
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

        ResetMode ->
            ( "/reset", Dict.empty )


urlToState : List String -> Dict String String -> Model -> Model
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
    column [ width fill, height (px size.height) ]
        [ column
            [ centerX
            , centerY
            , width <| maximum 450 fill
            , height <| maximum 420 fill
            , Background.color (Common.navbarColor 1)
            , Border.rounded 10
            , padding 10
            ]
            [ row [ width fill ]
                [ Common.navbar 0
                    model.mode
                    SetMode
                    [ ( LoginMode, "log in" )
                    , ( RegistrationMode, "register" )
                    , ( ResetMode, "reset" )
                    ]
                ]
            , if model.sent then
                sentView model

              else
                case model.mode of
                    LoginMode ->
                        loginView model

                    ResetMode ->
                        resetView model

                    RegistrationMode ->
                        registrationView model
            ]
        ]


loginView : Model -> Element Msg
loginView model =
    column
        [ spacing 8
        , width fill
        , height fill
        , padding 10
        , Background.color (Common.navbarColor 1)
        ]
        [ text <| "welcome to " ++ model.appname ++ "!"
        , text <| "log in below:"
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
        , Input.button (buttonStyle ++ [ width fill, alignBottom ])
            { onPress = Just LoginPressed
            , label = text "log in"
            }
        ]


resetView : Model -> Element Msg
resetView model =
    column
        [ spacing 8
        , width fill
        , height fill
        , padding 10
        , Background.color (Common.navbarColor 1)
        ]
        [ text <| "forgot your password?"
        , Input.text [ width fill ]
            { onChange = IdUpdate
            , text = model.userId
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "User id:"
            }
        , text model.responseMessage
        , Input.button (buttonStyle ++ [ width fill, alignBottom ])
            { onPress = Just ResetPressed
            , label = text "send reset email"
            }
        ]


registrationView : Model -> Element Msg
registrationView model =
    column [ Background.color (Common.navbarColor 1), width fill, height fill, spacing 8, padding 8 ]
        [ text <| "welcome to " ++ model.appname ++ "!"
        , text <| "register your new account below:"
        , Input.text []
            { onChange = EmailUpdate
            , text = model.email
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "email:"
            }
        , Input.text []
            { onChange = IdUpdate
            , text = model.userId
            , placeholder = Nothing
            , label = Input.labelLeft [] <| text "user id:"
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
        , Input.button (buttonStyle ++ [ width fill, alignBottom ])
            { onPress = Just RegisterPressed
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

                ResetMode ->
                    "Reset sent..."
            )
        ]


onWkKeyPress : WK.Key -> Model -> ( Model, Cmd )
onWkKeyPress key model =
    case Toop.T4 key.key key.ctrl key.alt key.shift of
        Toop.T4 "Enter" False False False ->
            case model.mode of
                LoginMode ->
                    update LoginPressed model

                RegistrationMode ->
                    update RegisterPressed model

                ResetMode ->
                    update ResetPressed model

        _ ->
            ( model, None )


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

        RegisterPressed ->
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

        LoginPressed ->
            ( { model | sent = True }, Login )

        ResetPressed ->
            ( { model | sent = True }, Reset )
