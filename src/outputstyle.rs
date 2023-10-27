use ansi_term::{Color, Style};

pub struct OutputStyle {
    pub title: Style,
    pub link: Style,
    pub variable: Option<Style>,
    pub constant: Option<Style>,
    pub bold: Option<Style>,
    pub monospace: Option<Style>,
    pub input: Style,
    pub input_whitespace: Option<Style>,
    pub output: Style,
    pub output_whitespace: Option<Style>,
}

impl OutputStyle {
    pub fn plain() -> Self {
        OutputStyle {
            title: Style::default(),
            link: Style::default(),
            variable: Some(Style::default()),
            constant: Some(Style::default()),
            bold: Some(Style::default()),
            monospace: Some(Style::default()),
            input: Style::default(),
            input_whitespace: None,
            output: Style::default(),
            output_whitespace: None,
        }
    }
}

impl Default for OutputStyle {
    fn default() -> Self {
        OutputStyle {
            title: Style::new().fg(Color::Yellow).bold(),
            link: Style::new().fg(Color::Yellow),
            variable: Some(Style::new().fg(Color::Yellow)),
            constant: Some(Style::new().fg(Color::Blue)),
            bold: Some(Style::new().italic()),
            monospace: Some(Style::default()),
            input: Style::new().fg(Color::White),
            input_whitespace: Some(Style::new().fg(Color::Black).dimmed()),
            output: Style::new().fg(Color::Green),
            output_whitespace: Some(Style::new().fg(Color::White).dimmed()),
        }
    }
}

pub struct TestCaseStyle {
    pub success: Style,
    pub failure: Style,
    pub error: Style,
    pub title: Style,
    pub stderr: Style,
    pub out: Style,
    pub whitespace: Option<Style>,
}

impl Default for TestCaseStyle {
    fn default() -> Self {
        TestCaseStyle {
            success: Style::new().on(Color::Green),
            failure: Style::new().on(Color::Red),
            error:  Style::new().on(Color::Red),
            title: Style::new().fg(Color::Yellow),
            stderr: Style::new().fg(Color::Red),
            out: Style::new().fg(Color::White),
            whitespace: Some(Style::new().fg(Color::RGB(43, 43, 43))),
        }
    }
}