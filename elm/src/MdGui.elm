module MdGui exposing (Msg, coltrib, guiBlock, rowtrib, updateBlock)

import Either
import Element as E
import Element.Border as EBd
import Element.Input as EI
import Markdown.Block as MB exposing (..)
import MdCommon as MC
import Set
import TangoColors as TC
import Toop


type Msg
    = CbLanguage String
    | CbBody String
    | CellName String
    | CellScript String
    | SearchText String
    | NoteIdText String
    | ImageText String
    | ImageUrl String
    | ImageWidth String
    | VideoSrc String
    | VideoText String
    | VideoWidth String
    | VideoHeight String
    | AudioSrc String
    | AudioText String
    | NoteSrc String
    | NoteText String
    | NoteShowTitle Bool
    | NoteShowContents Bool
    | NoteShowText Bool
    | NoteShowFile Bool
    | NoteShowCreatedate Bool
    | NoteShowChangedate Bool
    | NoteShowLink Bool
    | ListItemMsg Int Msg
    | LinkUrl String
    | LinkTitle String
    | ImageTitle String
    | CodeSpanStr String
    | TextStr String


rowtrib =
    [ E.spacing 3, E.width E.fill ]


coltrib =
    [ E.spacing 3, E.width E.fill ]


guiBlock : MB.Block -> E.Element Msg
guiBlock block =
    case block of
        HtmlBlock htmlBlock ->
            guiHtml htmlBlock

        UnorderedList listSpacing listItems ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "unordered list"
                , E.column coltrib <|
                    List.indexedMap
                        (\i (ListItem t li) ->
                            E.map (ListItemMsg i)
                                (E.column coltrib <| List.indexedMap (\ii item -> E.map (ListItemMsg ii) (guiBlock item)) li)
                        )
                        listItems
                ]

        OrderedList listSpacing startIndex blockLists ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "ordered list"
                , E.column coltrib <|
                    List.indexedMap
                        (\bli bl ->
                            E.map (ListItemMsg bli)
                                (E.column coltrib <| List.indexedMap (\i b -> E.map (ListItemMsg i) (guiBlock b)) bl)
                        )
                        blockLists
                ]

        BlockQuote blocks ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "blockquote"
                , E.column coltrib <|
                    List.indexedMap (\i b -> E.map (ListItemMsg i) (guiBlock b)) blocks
                ]

        Heading headingLevel inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "heading"
                , E.column coltrib <| List.indexedMap (\i inline -> E.map (ListItemMsg i) (guiInline inline)) inlines
                ]

        Paragraph inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "paragraph"
                , E.column coltrib <| List.indexedMap (\i inline -> E.map (ListItemMsg i) (guiInline inline)) inlines
                ]

        Table headings inlines ->
            E.none

        CodeBlock cb ->
            E.column [ E.width E.fill ]
                [ EI.text []
                    { onChange = CbLanguage
                    , text = cb.language |> Maybe.withDefault ""
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "language")
                    }
                , EI.multiline [ E.width E.fill ]
                    { onChange = CbBody
                    , text = cb.body
                    , placeholder = Nothing
                    , label = EI.labelLeft [] (E.text "body")
                    , spellcheck = False
                    }
                ]

        ThematicBreak ->
            E.text "----------------------"


guiInline : MB.Inline -> E.Element Msg
guiInline inline =
    case inline of
        HtmlInline block ->
            guiHtml block

        Link url mbtitle inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "link"
                , E.column coltrib <|
                    [ EI.text []
                        { onChange = LinkUrl
                        , text = url
                        , placeholder = Nothing
                        , label = EI.labelLeft [] (E.text "url")
                        }

                    -- title is not used, so don't bother with editing it!
                    -- , EI.text []
                    --     { onChange = LinkTitle
                    --     , text = mbtitle |> Maybe.withDefault ""
                    --     , placeholder = Nothing
                    --     , label = EI.labelLeft [] (E.text "title")
                    --     }
                    , E.column coltrib <| List.indexedMap (\i inl -> E.map (ListItemMsg i) (guiInline inl)) inlines
                    ]
                ]

        Image src mbtitle inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "image"
                , E.column coltrib <|
                    [ EI.text []
                        { onChange = ImageUrl
                        , text = src
                        , placeholder = Nothing
                        , label = EI.labelLeft [] (E.text "url")
                        }

                    -- title is not used, so don't bother with editing it!
                    -- , EI.text []
                    --     { onChange = ImageTitle
                    --     , text = mbtitle |> Maybe.withDefault ""
                    --     , placeholder = Nothing
                    --     , label = EI.labelLeft [] (E.text "title")
                    --     }
                    , E.column coltrib <| List.indexedMap (\i inl -> E.map (ListItemMsg i) (guiInline inl)) inlines
                    ]
                ]

        Emphasis inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "emphasis"
                , E.column coltrib (List.indexedMap (\i inl -> E.map (ListItemMsg i) (guiInline inl)) inlines)
                ]

        Strong inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "strong"
                , E.column coltrib (List.indexedMap (\i inl -> E.map (ListItemMsg i) (guiInline inl)) inlines)
                ]

        Strikethrough inlines ->
            E.row rowtrib
                [ E.el [ E.alignTop ] <| E.text "strikethrough"
                , E.column coltrib (List.indexedMap (\i inl -> E.map (ListItemMsg i) (guiInline inl)) inlines)
                ]

        CodeSpan s ->
            EI.text []
                { onChange = CodeSpanStr
                , text = s
                , placeholder = Nothing
                , label = EI.labelLeft [] (E.text "code")
                }

        Text s ->
            EI.text []
                { onChange = TextStr
                , text = s
                , placeholder = Nothing
                , label = EI.labelLeft [] (E.text "text")
                }

        HardLineBreak ->
            E.none


updateListItem : Int -> Msg -> List (ListItem Block) -> List (ListItem Block)
updateListItem idx msg listitems =
    case List.head (List.drop idx listitems) of
        Just (ListItem task blocks) ->
            case msg of
                ListItemMsg bi bmsg ->
                    List.take idx listitems
                        ++ [ ListItem task <| updateListBlock bi bmsg blocks ]
                        ++ List.drop (idx + 1) listitems

                _ ->
                    listitems

        _ ->
            listitems


updateListBlock : Int -> Msg -> List Block -> List Block
updateListBlock idx msg blocks =
    case List.head (List.drop idx blocks) of
        Just block ->
            List.take idx blocks
                ++ updateBlock msg block
                ++ List.drop (idx + 1) blocks

        Nothing ->
            blocks


updateListListBlock : Int -> Msg -> List (List Block) -> List (List Block)
updateListListBlock idx msg blocklists =
    case ( List.head (List.drop idx blocklists), msg ) of
        ( Just blocklist, ListItemMsg i lim ) ->
            List.take idx blocklists
                ++ [ updateListBlock i lim blocklist ]
                ++ List.drop (idx + 1) blocklists

        _ ->
            blocklists


updateListInline : Int -> Msg -> List Inline -> List Inline
updateListInline idx msg inlines =
    case List.head (List.drop idx inlines) of
        Just inline ->
            let
                a =
                    List.take
                        idx
                        inlines
                        ++ updateInline msg inline
                        ++ List.drop (idx + 1) inlines
            in
            a

        Nothing ->
            inlines


updateInline : Msg -> MB.Inline -> List MB.Inline
updateInline msg inline =
    case inline of
        HtmlInline block ->
            [ HtmlInline <| updateHtml msg block ]

        Link url mbtitle inlines ->
            case msg of
                LinkUrl s ->
                    [ Link s mbtitle inlines ]

                LinkTitle s ->
                    [ Link url
                        (if String.isEmpty s then
                            Nothing

                         else
                            Just s
                        )
                        inlines
                    ]

                ListItemMsg idx lim ->
                    [ Link url mbtitle <| updateListInline idx lim inlines ]

                _ ->
                    [ inline ]

        Image src mbtitle inlines ->
            case msg of
                ImageUrl s ->
                    [ Image s mbtitle inlines ]

                ImageTitle s ->
                    [ Image src
                        (if String.isEmpty s then
                            Nothing

                         else
                            Just s
                        )
                        inlines
                    ]

                ListItemMsg idx lim ->
                    [ Image src mbtitle <| updateListInline idx lim inlines ]

                _ ->
                    [ inline ]

        Emphasis inlines ->
            case msg of
                ListItemMsg idx lim ->
                    [ Emphasis <| updateListInline idx lim inlines ]

                _ ->
                    [ inline ]

        Strong inlines ->
            case msg of
                ListItemMsg idx lim ->
                    [ Strong <| updateListInline idx lim inlines ]

                _ ->
                    [ inline ]

        Strikethrough inlines ->
            case msg of
                ListItemMsg idx lim ->
                    [ Strikethrough <| updateListInline idx lim inlines ]

                _ ->
                    [ inline ]

        CodeSpan _ ->
            case msg of
                CodeSpanStr str ->
                    [ CodeSpan str ]

                _ ->
                    [ inline ]

        Text _ ->
            case msg of
                TextStr str ->
                    [ Text str ]

                _ ->
                    [ inline ]

        HardLineBreak ->
            [ inline ]


updateBlock : Msg -> MB.Block -> List MB.Block
updateBlock msg block =
    case block of
        HtmlBlock htmlBlock ->
            updateHtml msg htmlBlock
                |> HtmlBlock
                |> List.singleton

        UnorderedList listSpacing listItems ->
            case msg of
                ListItemMsg i ulmsg ->
                    [ UnorderedList listSpacing (updateListItem i ulmsg listItems) ]

                _ ->
                    [ block ]

        OrderedList listSpacing startIndex blockLists ->
            case msg of
                ListItemMsg i bmsg ->
                    [ OrderedList listSpacing startIndex (updateListListBlock i bmsg blockLists) ]

                _ ->
                    [ block ]

        BlockQuote blocks ->
            case msg of
                ListItemMsg i ulmsg ->
                    [ BlockQuote (updateListBlock i ulmsg blocks) ]

                _ ->
                    [ block ]

        Heading headingLevel inlines ->
            case msg of
                ListItemMsg i hmsg ->
                    [ Heading headingLevel (updateListInline i hmsg inlines) ]

                _ ->
                    [ block ]

        Paragraph inlines ->
            case msg of
                ListItemMsg i hmsg ->
                    [ Paragraph (updateListInline i hmsg inlines) ]

                _ ->
                    [ block ]

        Table headings inlines ->
            [ block ]

        CodeBlock cb ->
            case msg of
                CbLanguage s ->
                    [ CodeBlock
                        { cb
                            | language =
                                if s == "" then
                                    Nothing

                                else
                                    Just s
                        }
                    ]

                CbBody s ->
                    [ CodeBlock { cb | body = s } ]

                _ ->
                    [ block ]

        ThematicBreak ->
            [ block ]


updateHtml : Msg -> Html a -> Html a
updateHtml msg block =
    case block of
        HtmlElement tag attribs _ ->
            updateHtmlElement msg tag attribs

        HtmlComment _ ->
            block

        ProcessingInstruction _ ->
            block

        HtmlDeclaration _ _ ->
            block

        Cdata _ ->
            block


updateAttrib : String -> Maybe String -> List HtmlAttribute -> List HtmlAttribute
updateAttrib name mbvalue attribs =
    case mbvalue of
        Nothing ->
            List.filter (\l -> l.name /= name) attribs

        Just v ->
            attribs
                |> List.foldr
                    (\a l ->
                        case l of
                            Either.Left found ->
                                Either.Left (a :: found)

                            Either.Right notfound ->
                                if a.name == name then
                                    Either.Left ({ a | value = v } :: notfound)

                                else
                                    Either.Right (a :: notfound)
                    )
                    (Either.Right [])
                |> (\r ->
                        case r of
                            Either.Left list ->
                                list

                            Either.Right list ->
                                { name = name, value = v } :: list
                   )


updateHtmlElement : Msg -> String -> List HtmlAttribute -> Html a
updateHtmlElement msg tag attribs =
    case tag of
        "cell" ->
            case msg of
                CellName s ->
                    HtmlElement tag (updateAttrib "name" (Just s) attribs) []

                CellScript s ->
                    HtmlElement tag (updateAttrib "script" (Just s) attribs) []

                _ ->
                    HtmlElement tag attribs []

        "search" ->
            case msg of
                SearchText s ->
                    HtmlElement tag (updateAttrib "search" (Just s) attribs) []

                _ ->
                    HtmlElement tag attribs []

        "panel" ->
            case msg of
                NoteIdText s ->
                    HtmlElement tag (updateAttrib "noteid" (Just s) attribs) []

                _ ->
                    HtmlElement tag attribs []

        "image" ->
            case msg of
                ImageText s ->
                    HtmlElement tag (updateAttrib "text" (Just s) attribs) []

                ImageUrl s ->
                    HtmlElement tag (updateAttrib "url" (Just s) attribs) []

                ImageWidth s ->
                    HtmlElement tag
                        (updateAttrib "width"
                            (if s == "" then
                                Nothing

                             else
                                Just s
                            )
                            attribs
                        )
                        []

                _ ->
                    HtmlElement tag attribs []

        "video" ->
            case msg of
                VideoText s ->
                    HtmlElement tag (updateAttrib "text" (Just s) attribs) []

                VideoSrc s ->
                    HtmlElement tag (updateAttrib "src" (Just s) attribs) []

                VideoWidth s ->
                    HtmlElement tag
                        (updateAttrib "width"
                            (if s == "" then
                                Nothing

                             else
                                Just s
                            )
                            attribs
                        )
                        []

                VideoHeight s ->
                    HtmlElement tag
                        (updateAttrib "height"
                            (if s == "" then
                                Nothing

                             else
                                Just s
                            )
                            attribs
                        )
                        []

                _ ->
                    HtmlElement tag attribs []

        "audio" ->
            case msg of
                AudioSrc s ->
                    HtmlElement tag (updateAttrib "src" (Just s) attribs) []

                AudioText s ->
                    HtmlElement tag (updateAttrib "text" (Just s) attribs) []

                _ ->
                    HtmlElement tag attribs []

        "note" ->
            case msg of
                NoteIdText s ->
                    HtmlElement tag (updateAttrib "id" (Just s) attribs) []

                NoteText s ->
                    let
                        mb =
                            case s of
                                "" ->
                                    Nothing

                                _ ->
                                    Just s
                    in
                    HtmlElement tag (updateAttrib "text" mb attribs) []

                NoteShowTitle b ->
                    HtmlElement tag (updateShowAttrib "title" b attribs) []

                NoteShowContents b ->
                    HtmlElement tag (updateShowAttrib "contents" b attribs) []

                NoteShowText b ->
                    HtmlElement tag (updateShowAttrib "text" b attribs) []

                NoteShowFile b ->
                    HtmlElement tag (updateShowAttrib "file" b attribs) []

                NoteShowCreatedate b ->
                    HtmlElement tag (updateShowAttrib "createdate" b attribs) []

                NoteShowChangedate b ->
                    HtmlElement tag (updateShowAttrib "changedate" b attribs) []

                NoteShowLink b ->
                    HtmlElement tag (updateShowAttrib "link" b attribs) []

                _ ->
                    HtmlElement tag attribs []

        _ ->
            HtmlElement tag attribs []


updateShowAttrib : String -> Bool -> List HtmlAttribute -> List HtmlAttribute
updateShowAttrib which on attribs =
    case findAttrib "show" attribs of
        Just s ->
            (if on then
                s
                    |> String.words
                    |> Set.fromList
                    |> Set.insert which
                    |> Set.toList
                    |> List.intersperse " "
                    |> String.concat

             else
                s
                    |> String.words
                    |> List.filter (\w -> w /= which)
                    |> List.intersperse " "
                    |> String.concat
            )
                |> (\a ->
                        updateAttrib "show"
                            (case a of
                                "" ->
                                    Nothing

                                _ ->
                                    Just a
                            )
                            attribs
                   )

        Nothing ->
            if on then
                updateAttrib "show" (Just which) attribs

            else
                attribs


guiHtml : Html Block -> E.Element Msg
guiHtml block =
    case block of
        HtmlElement tag attribs _ ->
            guiHtmlElement tag attribs

        HtmlComment _ ->
            E.none

        ProcessingInstruction _ ->
            E.none

        HtmlDeclaration _ _ ->
            E.none

        Cdata _ ->
            E.none


findAttrib : String -> List HtmlAttribute -> Maybe String
findAttrib name attribs =
    attribs
        |> List.filter (\a -> a.name == name)
        |> List.map .value
        |> List.head


guiHtmlElement : String -> List HtmlAttribute -> E.Element Msg
guiHtmlElement tag attribs =
    case tag of
        "cell" ->
            case ( findAttrib "name" attribs, findAttrib "script" attribs ) of
                ( Just name, Just script ) ->
                    E.column coltrib
                        [ EI.text []
                            { onChange = CellName
                            , text = name
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "name")
                            }
                        , EI.multiline []
                            { onChange = CellScript
                            , text = script
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "script")
                            , spellcheck = False
                            }
                        ]

                _ ->
                    E.none

        "search" ->
            case findAttrib "search" attribs of
                Just search ->
                    E.column coltrib
                        [ EI.text []
                            { onChange = SearchText
                            , text = search
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "search")
                            }
                        ]

                _ ->
                    E.none

        "panel" ->
            case findAttrib "noteid" attribs of
                Just noteid ->
                    E.column coltrib
                        [ EI.text []
                            { onChange = NoteIdText
                            , text = noteid
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "noteid")
                            }
                        ]

                _ ->
                    E.none

        "image" ->
            case ( findAttrib "text" attribs, findAttrib "url" attribs, findAttrib "width" attribs ) of
                ( Just name, Just url, mbwidth ) ->
                    E.column coltrib
                        [ EI.text []
                            { onChange = ImageText
                            , text = name
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "text")
                            }
                        , EI.text []
                            { onChange = ImageUrl
                            , text = url
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "url")
                            }
                        , EI.text []
                            { onChange = ImageWidth
                            , text = mbwidth |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "width")
                            }
                        ]

                _ ->
                    E.none

        "video" ->
            case Toop.T4 (findAttrib "src" attribs) (findAttrib "text" attribs) (findAttrib "width" attribs) (findAttrib "height" attribs) of
                Toop.T4 (Just src) mbtext mbwidth mbheight ->
                    E.column coltrib
                        [ EI.text []
                            { onChange = VideoSrc
                            , text = src
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "src")
                            }
                        , EI.text []
                            { onChange = VideoText
                            , text = mbtext |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbtext")
                            }
                        , EI.text []
                            { onChange = VideoWidth
                            , text = mbwidth |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbwidth")
                            }
                        , EI.text []
                            { onChange = VideoHeight
                            , text = mbheight |> Maybe.withDefault ""
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "mbheight")
                            }
                        ]

                _ ->
                    E.none

        "audio" ->
            case Toop.T2 (findAttrib "src" attribs) (findAttrib "text" attribs) of
                Toop.T2 (Just src) (Just text) ->
                    E.column coltrib
                        [ EI.text []
                            { onChange = AudioSrc
                            , text = src
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "src")
                            }
                        , EI.text []
                            { onChange = AudioText
                            , text = text
                            , placeholder = Nothing
                            , label = EI.labelLeft [] (E.text "text")
                            }
                        ]

                _ ->
                    E.none

        "note" ->
            case ( findAttrib "id" attribs, findAttrib "show" attribs, findAttrib "text" attribs ) of
                ( Just noteid, mbshow, mbtext ) ->
                    let
                        ns =
                            mbshow
                                |> Maybe.map MC.parseNoteShow
                                |> Maybe.withDefault
                                    { title = False
                                    , contents = False
                                    , text = False
                                    , file = False
                                    , createdate = False
                                    , changedate = False
                                    , link = False
                                    }
                    in
                    E.row rowtrib
                        [ E.el [ E.alignTop ] <| E.text "note"
                        , E.column coltrib
                            [ EI.text []
                                { onChange = NoteSrc
                                , text = noteid
                                , placeholder = Nothing
                                , label = EI.labelLeft [] (E.text "noteid")
                                }
                            , E.row [ E.width E.fill, E.spacing 3 ]
                                [ E.el [ E.alignTop ] <| E.text "show"
                                , E.column [ EBd.color TC.grey, EBd.width 1, E.padding 8, E.spacing 3 ]
                                    [ EI.checkbox []
                                        { onChange = NoteShowTitle
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.title
                                        , label = EI.labelRight [] (E.text "title")
                                        }
                                    , EI.checkbox []
                                        { onChange = NoteShowContents
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.contents
                                        , label = EI.labelRight [] (E.text "contents")
                                        }
                                    , EI.checkbox []
                                        { onChange = NoteShowText
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.text
                                        , label = EI.labelRight [] (E.text "text")
                                        }
                                    , EI.checkbox []
                                        { onChange = NoteShowFile
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.file
                                        , label = EI.labelRight [] (E.text "file")
                                        }
                                    , EI.checkbox []
                                        { onChange = NoteShowCreatedate
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.createdate
                                        , label = EI.labelRight [] (E.text "createdate")
                                        }
                                    , EI.checkbox []
                                        { onChange = NoteShowChangedate
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.changedate
                                        , label = EI.labelRight [] (E.text "changedate")
                                        }
                                    , EI.checkbox []
                                        { onChange = NoteShowLink
                                        , icon = EI.defaultCheckbox
                                        , checked = ns.link
                                        , label = EI.labelRight [] (E.text "link")
                                        }
                                    ]
                                ]
                            , EI.text []
                                { onChange = NoteText
                                , text = mbtext |> Maybe.withDefault ""
                                , placeholder = Nothing
                                , label = EI.labelLeft [] (E.text "text")
                                }
                            ]
                        ]

                _ ->
                    E.none

        _ ->
            E.none
