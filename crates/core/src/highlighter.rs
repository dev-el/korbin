use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

pub struct SyntaxHighlighter {
    highlighter: Highlighter,
    rust_config: HighlightConfiguration,
    md_config: HighlightConfiguration,
    md_inline_config: HighlightConfiguration,
    latex_config: HighlightConfiguration,
    highlight_names: Vec<String>,
    current_lang: String,
}

#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub text: String,
    pub range: std::ops::Range<usize>,
    pub highlight_name: Option<String>,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let highlight_names = [
            "keyword",
            "keyword.control",
            "operator",
            "punctuation",
            "punctuation.bracket",
            "punctuation.delimiter",
            "punctuation.special",
            "function",
            "function.method",
            "function.macro",
            "variable",
            "variable.parameter",
            "variable.builtin",
            "property",
            "type",
            "type.builtin",
            "constructor",
            "constant",
            "constant.builtin",
            "string",
            "string.special",
            "comment",
            "attribute",
            "label",
            "text.title",
            "text.literal",
            "text.uri",
            "text.reference",
            "text.strong",
            "text.emphasis",
            "text.list",
            "text.quote",
            "markup.heading",
            "markup.bold",
            "markup.italic",
            "markup.list",
            "markup.link",
            "markup.quote",
            "markup.raw",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

        let mut rust_config = HighlightConfiguration::new(
            tree_sitter_rust::LANGUAGE.into(),
            "rust",
            tree_sitter_rust::HIGHLIGHTS_QUERY,
            tree_sitter_rust::INJECTIONS_QUERY,
            "",
        )
        .expect("Failed to load Rust highlight configuration");
        let highlight_names_refs: Vec<&str> = highlight_names.iter().map(|s| s.as_str()).collect();
        rust_config.configure(&highlight_names_refs);

        let mut md_config = HighlightConfiguration::new(
            tree_sitter_md::LANGUAGE.into(),
            "markdown",
            tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
            tree_sitter_md::INJECTION_QUERY_BLOCK,
            "",
        )
        .expect("Failed to load Markdown highlight configuration");
        md_config.configure(&highlight_names_refs);

        let mut md_inline_config = HighlightConfiguration::new(
            tree_sitter_md::INLINE_LANGUAGE.into(),
            "markdown_inline",
            tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
            tree_sitter_md::INJECTION_QUERY_INLINE,
            "",
        )
        .expect("Failed to load Markdown Inline highlight configuration");
        md_inline_config.configure(&highlight_names_refs);

        let latex_query = r#"
        (command_name) @keyword.control
        (generic_command command: (command_name) @keyword.control)
        ((class_include command: _ @keyword.control))
        ((package_include command: _ @keyword.control))
        (line_comment) @comment
        (block_comment) @comment
        (comment) @comment
        (comment_environment) @comment
        (math_environment) @string.special
        (inline_formula) @string.special
        (displayed_equation) @string.special
        (begin) @keyword.control
        (end) @keyword.control
        (text) @text.literal
        "#;

        let mut latex_config = HighlightConfiguration::new(
            codebook_tree_sitter_latex::LANGUAGE.into(),
            "latex",
            latex_query,
            "",
            "",
        )
        .expect("Failed to load Latex highlight configuration");
        latex_config.configure(&highlight_names_refs);

        Self {
            highlighter: Highlighter::new(),
            rust_config,
            md_config,
            md_inline_config,
            latex_config,
            highlight_names,
            current_lang: "markdown".to_string(),
        }
    }

    pub fn set_language_from_path(&mut self, path: &str) {
        if path.ends_with(".rs") {
            self.current_lang = "rust".to_string();
        } else if path.ends_with(".md") {
            self.current_lang = "markdown".to_string();
        } else if path.ends_with(".tex") {
            self.current_lang = "latex".to_string();
        }
    }

    pub fn highlight(&mut self, text: &str) -> Vec<HighlightSpan> {
        let config = if self.current_lang == "rust" {
            &self.rust_config
        } else if self.current_lang == "latex" {
            &self.latex_config
        } else {
            &self.md_config
        };

        let highlights = self
            .highlighter
            .highlight(config, text.as_bytes(), None, |lang| {
                match lang {
                    "rust" => Some(&self.rust_config),
                    "markdown" => Some(&self.md_config),
                    "markdown_inline" => Some(&self.md_inline_config),
                    "latex" => Some(&self.latex_config),
                    _ => None,
                }
            })
            .expect("Failed to highlight text");

        let mut spans = Vec::new();
        let mut current_highlight = None;

        for event in highlights {
            match event {
                Ok(HighlightEvent::Source { start, end }) => {
                    let part = &text[start..end];
                    spans.push(HighlightSpan {
                        text: part.to_string(),
                        range: start..end,
                        highlight_name: current_highlight.clone(),
                    });
                }
                Ok(HighlightEvent::HighlightStart(index)) => {
                    current_highlight = self.highlight_names.get(index.0).cloned();
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    current_highlight = None;
                }
                Err(e) => {
                    eprintln!("Highlight error: {:?}", e);
                }
            }
        }

        spans
    }
}
