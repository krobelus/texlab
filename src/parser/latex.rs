mod lexer;

use rowan::{GreenNode, GreenNodeBuilder};

use crate::syntax::latex::SyntaxKind::{self, *};

use self::lexer::Lexer;

#[derive(Debug, Clone, Copy)]
struct ParserContext {
    allow_environment: bool,
    allow_comma: bool,
}

impl Default for ParserContext {
    fn default() -> Self {
        Self {
            allow_environment: true,
            allow_comma: true,
        }
    }
}

#[derive(Debug)]
struct Parser<'a> {
    lexer: Lexer<'a>,
    builder: GreenNodeBuilder<'static>,
}

impl<'a> Parser<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            lexer: Lexer::new(text),
            builder: GreenNodeBuilder::new(),
        }
    }

    fn eat(&mut self) {
        let (kind, text) = self.lexer.eat().unwrap();
        self.builder.token(kind.into(), text);
    }

    fn peek(&self) -> Option<SyntaxKind> {
        self.lexer.peek()
    }

    fn expect(&mut self, kind: SyntaxKind) {
        if self.peek() == Some(kind) {
            self.eat();
            self.trivia();
        } else {
            self.builder.token(MISSING.into(), "");
        }
    }

    fn expect2(&mut self, kind1: SyntaxKind, kind2: SyntaxKind) {
        if self
            .peek()
            .filter(|&kind| kind == kind1 || kind == kind2)
            .is_some()
        {
            self.eat();
            self.trivia();
        } else {
            self.builder.token(MISSING.into(), "");
        }
    }

    fn trivia(&mut self) {
        while self
            .peek()
            .filter(|&kind| matches!(kind, LINE_BREAK | WHITESPACE | COMMENT))
            .is_some()
        {
            self.eat();
        }
    }

    pub fn parse(mut self) -> GreenNode {
        self.builder.start_node(ROOT.into());
        self.preamble();
        while self.peek().is_some() {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
        self.builder.finish()
    }

    fn content(&mut self, context: ParserContext) {
        match self.peek().unwrap() {
            LINE_BREAK | WHITESPACE | COMMENT | VERBATIM => self.eat(),
            L_CURLY if context.allow_environment => self.curly_group(),
            L_CURLY => self.curly_group_without_environments(),
            L_BRACK | L_PAREN => self.mixed_group(),
            R_CURLY | R_BRACK | R_PAREN => {
                self.builder.start_node(ERROR.into());
                self.eat();
                self.builder.finish_node();
            }
            WORD | COMMA => self.text(context),
            EQUALITY_SIGN => self.eat(),
            DOLLAR => self.formula(),
            GENERIC_COMMAND_NAME => self.generic_command(),
            BEGIN_ENVIRONMENT_NAME if context.allow_environment => self.environment(),
            BEGIN_ENVIRONMENT_NAME => self.generic_command(),
            END_ENVIRONMENT_NAME => self.generic_command(),
            BEGIN_EQUATION_NAME => self.equation(),
            END_EQUATION_NAME => self.generic_command(),
            MISSING | ERROR => self.eat(),
            PART_NAME => self.part(),
            CHAPTER_NAME => self.chapter(),
            SECTION_NAME => self.section(),
            SUBSECTION_NAME => self.subsection(),
            SUBSUBSECTION_NAME => self.subsubsection(),
            PARAGRAPH_NAME => self.paragraph(),
            SUBPARAGRAPH_NAME => self.subparagraph(),
            ENUM_ITEM_NAME => self.enum_item(),
            CAPTION_NAME => self.caption(),
            CITATION_NAME => self.citation(),
            PACKAGE_INCLUDE_NAME => self.package_include(),
            CLASS_INCLUDE_NAME => self.class_include(),
            LATEX_INCLUDE_NAME => self.latex_include(),
            BIBLATEX_INCLUDE_NAME => self.biblatex_include(),
            BIBTEX_INCLUDE_NAME => self.bibtex_include(),
            GRAPHICS_INCLUDE_NAME => self.graphics_include(),
            SVG_INCLUDE_NAME => self.svg_include(),
            INKSCAPE_INCLUDE_NAME => self.inkscape_include(),
            VERBATIM_INCLUDE_NAME => self.verbatim_include(),
            IMPORT_NAME => self.import(),
            LABEL_DEFINITION_NAME => self.label_definition(),
            LABEL_REFERENCE_NAME => self.label_reference(),
            LABEL_REFERENCE_RANGE_NAME => self.label_reference_range(),
            LABEL_NUMBER_NAME => self.label_number(),
            COMMAND_DEFINITION_NAME => self.command_definition(),
            MATH_OPERATOR_NAME => self.math_operator(),
            GLOSSARY_ENTRY_DEFINITION_NAME => self.glossary_entry_definition(),
            GLOSSARY_ENTRY_REFERENCE_NAME => self.glossary_entry_reference(),
            ACRONYM_DEFINITION_NAME => self.acronym_definition(),
            ACRONYM_DECLARATION_NAME => self.acronym_declaration(),
            ACRONYM_REFERENCE_NAME => self.acronym_reference(),
            THEOREM_DEFINITION_NAME => self.theorem_definition(),
            COLOR_REFERENCE_NAME => self.color_reference(),
            COLOR_DEFINITION_NAME => self.color_definition(),
            COLOR_SET_DEFINITION_NAME => self.color_set_definition(),
            TIKZ_LIBRARY_IMPORT_NAME => self.tikz_library_import(),
            ENVIRONMENT_DEFINITION_NAME => self.environment_definition(),
            BEGIN_BLOCK_COMMENT_NAME => self.block_comment(),
            END_BLOCK_COMMENT_NAME => self.generic_command(),
            GRAPHICS_PATH_NAME => self.graphics_path(),
            _ => unreachable!(),
        }
    }

    fn text(&mut self, context: ParserContext) {
        self.builder.start_node(TEXT.into());
        self.eat();
        while self
            .peek()
            .filter(|&kind| {
                matches!(kind, LINE_BREAK | WHITESPACE | COMMENT | WORD | COMMA)
                    && (context.allow_comma || kind != COMMA)
            })
            .is_some()
        {
            self.eat();
        }
        self.builder.finish_node();
    }

    fn curly_group(&mut self) {
        self.builder.start_node(CURLY_GROUP.into());
        self.eat();
        while self
            .peek()
            .filter(|&kind| !matches!(kind, R_CURLY))
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn curly_group_impl(&mut self) {
        self.builder.start_node(CURLY_GROUP.into());
        self.eat();
        while let Some(kind) = self.peek() {
            match kind {
                R_CURLY => break,
                BEGIN_ENVIRONMENT_NAME => self.begin(),
                END_ENVIRONMENT_NAME => self.end(),
                _ => self.content(ParserContext::default()),
            };
        }
        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn curly_group_without_environments(&mut self) {
        self.builder.start_node(CURLY_GROUP.into());
        self.eat();
        while self
            .peek()
            .filter(|&kind| !matches!(kind, R_CURLY))
            .is_some()
        {
            self.content(ParserContext {
                allow_environment: false,
                allow_comma: true,
            });
        }
        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn curly_group_word(&mut self) {
        self.builder.start_node(CURLY_GROUP_WORD.into());
        self.eat();
        self.trivia();
        match self.peek() {
            Some(WORD) => {
                self.key();
            }
            Some(kind) if kind.is_command_name() => {
                self.content(ParserContext::default());
            }
            Some(_) | None => {
                self.builder.token(MISSING.into(), "");
            }
        }
        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn curly_group_word_list(&mut self) {
        self.builder.start_node(CURLY_GROUP_WORD_LIST.into());
        self.eat();

        while self
            .peek()
            .filter(|&kind| matches!(kind, LINE_BREAK | WHITESPACE | COMMENT | WORD | COMMA))
            .is_some()
        {
            if self.peek() == Some(WORD) {
                self.key();
            } else {
                self.eat();
            }
        }

        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn curly_group_command(&mut self) {
        self.builder.start_node(CURLY_GROUP_COMMAND.into());
        self.eat();
        self.trivia();
        match self.peek() {
            Some(kind) if kind.is_command_name() => {
                self.eat();
                self.trivia();
            }
            Some(_) | None => {
                self.builder.token(MISSING.into(), "");
            }
        }
        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn brack_group(&mut self) {
        self.builder.start_node(BRACK_GROUP.into());
        self.eat();
        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    R_CURLY
                        | R_BRACK
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                        | PARAGRAPH_NAME
                        | SUBPARAGRAPH_NAME
                        | ENUM_ITEM_NAME
                        | END_ENVIRONMENT_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.expect(R_BRACK);
        self.builder.finish_node();
    }

    fn brack_group_word(&mut self) {
        self.builder.start_node(BRACK_GROUP_WORD.into());
        self.eat();
        self.trivia();
        match self.peek() {
            Some(WORD) => {
                self.key();
            }
            Some(_) | None => {
                self.builder.token(MISSING.into(), "");
            }
        }
        self.expect(R_BRACK);
        self.builder.finish_node();
    }

    fn mixed_group(&mut self) {
        self.builder.start_node(MIXED_GROUP.into());
        self.eat();
        self.trivia();
        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    R_CURLY
                        | R_BRACK
                        | R_PAREN
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                        | PARAGRAPH_NAME
                        | SUBPARAGRAPH_NAME
                        | ENUM_ITEM_NAME
                        | END_ENVIRONMENT_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.expect2(R_BRACK, R_PAREN);
        self.builder.finish_node();
    }

    fn key(&mut self) {
        self.builder.start_node(KEY.into());
        self.eat();
        while self
            .peek()
            .filter(|&kind| matches!(kind, WHITESPACE | COMMENT | WORD))
            .is_some()
        {
            self.eat();
        }

        self.trivia();
        self.builder.finish_node();
    }

    fn value(&mut self) {
        self.builder.start_node(VALUE.into());
        while let Some(kind) = self.lexer.peek() {
            match kind {
                COMMA | R_BRACK | R_CURLY => break,
                _ => self.content(ParserContext {
                    allow_environment: true,
                    allow_comma: false,
                }),
            };
        }
        self.builder.finish_node();
    }

    fn key_value_pair(&mut self) {
        self.builder.start_node(KEY_VALUE_PAIR.into());
        self.key();
        if self.peek() == Some(EQUALITY_SIGN) {
            self.eat();
            self.trivia();
            if self
                .peek()
                .filter(|&kind| {
                    !matches!(
                        kind,
                        END_ENVIRONMENT_NAME | R_CURLY | R_BRACK | R_PAREN | COMMA
                    )
                })
                .is_some()
            {
                self.value();
            } else {
                self.builder.token(MISSING.into(), "");
            }
        }

        self.builder.finish_node();
    }

    fn key_value_body(&mut self) {
        self.builder.start_node(KEY_VALUE_BODY.into());
        while let Some(kind) = self.peek() {
            match kind {
                LINE_BREAK | WHITESPACE | COMMENT => self.eat(),
                WORD => {
                    self.key_value_pair();
                    if self.peek() == Some(COMMA) {
                        self.eat();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        self.builder.finish_node();
    }

    fn group_key_value(&mut self, node_kind: SyntaxKind, right_kind: SyntaxKind) {
        self.builder.start_node(node_kind.into());
        self.eat();
        self.trivia();
        self.key_value_body();
        self.expect(right_kind);
        self.builder.finish_node();
    }

    fn curly_group_key_value(&mut self) {
        self.group_key_value(CURLY_GROUP_KEY_VALUE, R_CURLY);
    }

    fn brack_group_key_value(&mut self) {
        self.group_key_value(BRACK_GROUP_KEY_VALUE, R_BRACK);
    }

    fn formula(&mut self) {
        self.builder.start_node(FORMULA.into());
        self.eat();
        self.trivia();
        while self
            .peek()
            .filter(|&kind| !matches!(kind, R_CURLY | END_ENVIRONMENT_NAME | DOLLAR))
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.expect(DOLLAR);
        self.builder.finish_node();
    }

    fn generic_command(&mut self) {
        self.builder.start_node(GENERIC_COMMAND.into());
        self.eat();
        while let Some(kind) = self.peek() {
            match kind {
                LINE_BREAK | WHITESPACE | COMMENT => self.eat(),
                L_CURLY => self.curly_group(),
                L_BRACK | L_PAREN => self.mixed_group(),
                _ => break,
            }
        }
        self.builder.finish_node();
    }

    fn equation(&mut self) {
        self.builder.start_node(EQUATION.into());
        self.eat();
        while self
            .peek()
            .filter(|&kind| !matches!(kind, END_ENVIRONMENT_NAME | R_CURLY | END_EQUATION_NAME))
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.expect(END_EQUATION_NAME);
        self.builder.finish_node();
    }

    fn begin(&mut self) {
        self.builder.start_node(BEGIN.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.peek() == Some(L_BRACK) {
            self.brack_group();
        }
        self.builder.finish_node();
    }

    fn end(&mut self) {
        self.builder.start_node(END.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn environment(&mut self) {
        self.builder.start_node(ENVIRONMENT.into());
        self.begin();

        while self
            .peek()
            .filter(|&kind| !matches!(kind, R_CURLY | END_ENVIRONMENT_NAME))
            .is_some()
        {
            self.content(ParserContext::default());
        }

        if self.peek() == Some(END_ENVIRONMENT_NAME) {
            self.end();
        } else {
            self.builder.token(MISSING.into(), "");
        }
        self.builder.finish_node();
    }

    fn preamble(&mut self) {
        self.builder.start_node(PREAMBLE.into());
        while self
            .peek()
            .filter(|&kind| kind != END_ENVIRONMENT_NAME)
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn part(&mut self) {
        self.builder.start_node(PART.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| !matches!(kind, END_ENVIRONMENT_NAME | R_CURLY | PART_NAME))
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn chapter(&mut self) {
        self.builder.start_node(CHAPTER.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME | R_CURLY | PART_NAME | CHAPTER_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn section(&mut self) {
        self.builder.start_node(SECTION.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME | R_CURLY | PART_NAME | CHAPTER_NAME | SECTION_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn subsection(&mut self) {
        self.builder.start_node(SUBSECTION.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME
                        | R_CURLY
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn subsubsection(&mut self) {
        self.builder.start_node(SUBSUBSECTION.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME
                        | R_CURLY
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                        | SUBSUBSECTION_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn paragraph(&mut self) {
        self.builder.start_node(PARAGRAPH.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME
                        | R_CURLY
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                        | SUBSUBSECTION_NAME
                        | PARAGRAPH_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn subparagraph(&mut self) {
        self.builder.start_node(SUBPARAGRAPH.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME
                        | R_CURLY
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                        | SUBSUBSECTION_NAME
                        | PARAGRAPH_NAME
                        | SUBPARAGRAPH_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn enum_item(&mut self) {
        self.builder.start_node(ENUM_ITEM.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_BRACK) {
            self.brack_group();
        }

        while self
            .peek()
            .filter(|&kind| {
                !matches!(
                    kind,
                    END_ENVIRONMENT_NAME
                        | R_CURLY
                        | PART_NAME
                        | CHAPTER_NAME
                        | SECTION_NAME
                        | SUBSECTION_NAME
                        | SUBSUBSECTION_NAME
                        | PARAGRAPH_NAME
                        | SUBPARAGRAPH_NAME
                        | ENUM_ITEM_NAME
                )
            })
            .is_some()
        {
            self.content(ParserContext::default());
        }
        self.builder.finish_node();
    }

    fn block_comment(&mut self) {
        self.builder.start_node(BLOCK_COMMENT.into());
        self.eat();

        if self.peek() == Some(VERBATIM) {
            self.eat();
        }

        if self.peek() == Some(END_BLOCK_COMMENT_NAME) {
            self.eat();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn caption(&mut self) {
        self.builder.start_node(CAPTION.into());
        self.eat();
        self.trivia();

        if self.peek() == Some(L_BRACK) {
            self.brack_group();
        }

        if self.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn citation(&mut self) {
        self.builder.start_node(CITATION.into());
        self.eat();
        self.trivia();
        for _ in 0..2 {
            if self.lexer.peek() == Some(L_BRACK) {
                self.brack_group();
            }
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word_list();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn generic_include(&mut self, kind: SyntaxKind, options: bool) {
        self.builder.start_node(kind.into());
        self.eat();
        self.trivia();
        if options && self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_key_value();
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_path_list();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn curly_group_path(&mut self) {
        self.builder.start_node(CURLY_GROUP_WORD.into());
        self.eat();
        self.trivia();

        while let Some(kind) = self.lexer.peek() {
            match kind {
                COMMENT | WORD | EQUALITY_SIGN | COMMA | L_BRACK | R_BRACK
                | GENERIC_COMMAND_NAME => self.path(),
                L_CURLY => self.curly_group_path(),
                WHITESPACE => self.eat(),
                _ => break,
            };
        }

        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn curly_group_path_list(&mut self) {
        self.builder.start_node(CURLY_GROUP_WORD_LIST.into());
        self.eat();
        self.trivia();

        while let Some(kind) = self.peek() {
            match kind {
                COMMENT | WORD | EQUALITY_SIGN | L_BRACK | R_BRACK | GENERIC_COMMAND_NAME => {
                    self.path()
                }
                WHITESPACE | LINE_BREAK | COMMA => self.eat(),
                L_CURLY => self.curly_group_path(),
                _ => break,
            };
        }

        self.expect(R_CURLY);
        self.builder.finish_node();
    }

    fn path(&mut self) {
        self.builder.start_node(KEY.into());
        self.eat();

        while let Some(kind) = self.peek() {
            match kind {
                WHITESPACE | COMMENT | WORD | EQUALITY_SIGN | L_BRACK | R_BRACK
                | GENERIC_COMMAND_NAME => self.eat(),
                L_CURLY => self.curly_group_path(),
                _ => break,
            };
        }

        self.builder.finish_node();
    }

    fn package_include(&mut self) {
        self.generic_include(PACKAGE_INCLUDE, true);
    }

    fn class_include(&mut self) {
        self.generic_include(CLASS_INCLUDE, true);
    }

    fn latex_include(&mut self) {
        self.generic_include(LATEX_INCLUDE, false);
    }

    fn biblatex_include(&mut self) {
        self.generic_include(BIBLATEX_INCLUDE, true);
    }

    fn bibtex_include(&mut self) {
        self.generic_include(BIBTEX_INCLUDE, false);
    }

    fn graphics_include(&mut self) {
        self.generic_include(GRAPHICS_INCLUDE, true);
    }

    fn svg_include(&mut self) {
        self.generic_include(SVG_INCLUDE, true);
    }

    fn inkscape_include(&mut self) {
        self.generic_include(INKSCAPE_INCLUDE, true);
    }

    fn verbatim_include(&mut self) {
        self.generic_include(VERBATIM_INCLUDE, false);
    }

    fn import(&mut self) {
        self.builder.start_node(IMPORT.into());
        self.eat();
        self.trivia();

        for _ in 0..2 {
            if self.lexer.peek() == Some(L_CURLY) {
                self.curly_group_word();
            } else {
                self.builder.token(MISSING.into(), "");
            }
        }

        self.builder.finish_node();
    }

    fn label_definition(&mut self) {
        self.builder.start_node(LABEL_DEFINITION.into());
        self.eat();
        self.trivia();
        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }
        self.builder.finish_node();
    }

    fn label_reference(&mut self) {
        self.builder.start_node(LABEL_REFERENCE.into());
        self.eat();
        self.trivia();
        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word_list();
        } else {
            self.builder.token(MISSING.into(), "");
        }
        self.builder.finish_node();
    }

    fn label_reference_range(&mut self) {
        self.builder.start_node(LABEL_REFERENCE_RANGE.into());
        self.eat();
        self.trivia();

        for _ in 0..2 {
            if self.lexer.peek() == Some(L_CURLY) {
                self.curly_group_word();
            } else {
                self.builder.token(MISSING.into(), "");
            }
        }

        self.builder.finish_node();
    }

    fn label_number(&mut self) {
        self.builder.start_node(LABEL_NUMBER.into());
        self.eat();
        self.trivia();
        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group();
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn command_definition(&mut self) {
        self.builder.start_node(COMMAND_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_command();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_word();

            if self.lexer.peek() == Some(L_BRACK) {
                self.brack_group();
            }
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_impl();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn math_operator(&mut self) {
        self.builder.start_node(MATH_OPERATOR.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_command();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_impl();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn glossary_entry_definition(&mut self) {
        self.builder.start_node(GLOSSARY_ENTRY_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_key_value();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn glossary_entry_reference(&mut self) {
        self.builder.start_node(GLOSSARY_ENTRY_REFERENCE.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_key_value();
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn acronym_definition(&mut self) {
        self.builder.start_node(ACRONYM_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_key_value();
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        }

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group();
        }

        for _ in 0..2 {
            if self.lexer.peek() == Some(L_CURLY) {
                self.curly_group();
            }
        }

        self.builder.finish_node();
    }

    fn acronym_declaration(&mut self) {
        self.builder.start_node(ACRONYM_DECLARATION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_key_value();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn acronym_reference(&mut self) {
        self.builder.start_node(ACRONYM_REFERENCE.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_key_value();
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn theorem_definition(&mut self) {
        self.builder.start_node(THEOREM_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_word();
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_word();
        }

        self.builder.finish_node();
    }

    fn color_reference(&mut self) {
        self.builder.start_node(COLOR_REFERENCE.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn color_definition(&mut self) {
        self.builder.start_node(COLOR_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn color_set_definition(&mut self) {
        self.builder.start_node(COLOR_SET_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_word();
        }

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word_list();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        for _ in 0..3 {
            if self.lexer.peek() == Some(L_CURLY) {
                self.curly_group_word();
            } else {
                self.builder.token(MISSING.into(), "");
            }
        }

        self.builder.finish_node();
    }

    fn tikz_library_import(&mut self) {
        self.builder.start_node(TIKZ_LIBRARY_IMPORT.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word_list();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        self.builder.finish_node();
    }

    fn environment_definition(&mut self) {
        self.builder.start_node(ENVIRONMENT_DEFINITION.into());
        self.eat();
        self.trivia();

        if self.lexer.peek() == Some(L_CURLY) {
            self.curly_group_word();
        } else {
            self.builder.token(MISSING.into(), "");
        }

        if self.lexer.peek() == Some(L_BRACK) {
            self.brack_group_word();
            if self.lexer.peek() == Some(L_BRACK) {
                self.brack_group();
            }
        }

        for _ in 0..2 {
            if self.lexer.peek() == Some(L_CURLY) {
                self.curly_group_without_environments();
            } else {
                self.builder.token(MISSING.into(), "");
            }
        }

        self.builder.finish_node();
    }

    fn graphics_path(&mut self) {
        self.builder.start_node(GRAPHICS_PATH.into());
        self.eat();
        self.trivia();

        let checkpoint = self.builder.checkpoint();
        if self.lexer.peek() == Some(L_CURLY) {
            self.eat();
            self.trivia();

            if matches!(
                self.lexer.peek(),
                Some(WORD | EQUALITY_SIGN | L_BRACK | R_BRACK | GENERIC_COMMAND_NAME)
            ) {
                self.builder
                    .start_node_at(checkpoint, CURLY_GROUP_WORD.into());
                self.path();
            } else {
                self.builder.start_node_at(checkpoint, CURLY_GROUP.into());
                while matches!(self.lexer.peek(), Some(L_CURLY)) {
                    self.curly_group_path();
                }
            }

            self.expect(R_CURLY);
            self.builder.finish_node();
        }

        self.builder.finish_node();
    }
}

pub fn parse_latex(text: &str) -> GreenNode {
    Parser::new(text).parse()
}

#[cfg(test)]
mod tests {
    use crate::syntax::latex;

    use super::parse_latex;

    #[test]
    fn test_parse() {
        insta::glob!("test_data/latex/{,**/}*.txt", |path| {
            let text = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
            let root = latex::SyntaxNode::new_root(parse_latex(&text));
            insta::assert_debug_snapshot!(root);
        });
    }
}
