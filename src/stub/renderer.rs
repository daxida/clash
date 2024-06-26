use anyhow::{Context as _, Result}; // To distinguish it from tera::Context
use itertools::Itertools;
use serde_json::json;
use tera::{Context, Tera};

use super::{Cmd, JoinTerm, Language, Stub, StubConfig, VariableCommand};

const ALPHABET: [char; 18] = [
    'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

pub struct Renderer {
    tera: Tera,
    lang: Language,
    stub: Stub,
}

impl Renderer {
    pub(super) fn new(config: StubConfig, stub: Stub) -> Result<Renderer> {
        Ok(Self {
            lang: config.language,
            tera: config.tera,
            stub,
        })
    }

    pub(super) fn tera_render(&self, template_name: &str, context: &mut Context) -> String {
        // Since these are (generally) shared across languages, it makes sense to
        // store it in the "global" context instead of accepting it as parameters.
        let format_symbols = json!({
            "Bool": "%b",
            "Float": "%f",
            "Int": "%d",
            "Long": "%lld",
            // The extra space at the start is an improvement over CG's fgetc(stdin).
            // It trims previous whitespace (" \n\r\t" etc.) if any.
            "String": " %[^\\n]",
            "Word": "%s",
        });
        context.insert("format_symbols", &format_symbols);

        self.tera
            .render(&format!("{template_name}.{}.jinja", self.lang.source_file_ext), context)
            .with_context(|| format!("Failed to render {} template.", template_name))
            .unwrap()
    }

    pub(super) fn render(&self) -> String {
        let mut context = Context::new();

        let code: String = self.stub.commands.iter().map(|cmd| self.render_command(cmd, 0)).collect();
        let code_lines: Vec<&str> = code.lines().collect();

        context.insert("statement", &self.stub.statement);
        context.insert("code_lines", &code_lines);

        self.tera_render("main", &mut context)
    }

    pub(super) fn render_command(&self, cmd: &Cmd, nesting_depth: usize) -> String {
        match cmd {
            Cmd::Read(vars) => self.render_read(vars, nesting_depth),
            Cmd::Write {
                lines,
                output_comment,
            } => self.render_write(lines, output_comment),
            Cmd::WriteJoin {
                join_terms,
                output_comment,
            } => self.render_write_join(join_terms, output_comment),
            Cmd::Loop { count_var, command } => self.render_loop(count_var, command, nesting_depth),
            Cmd::LoopLine { count_var, variables } => {
                self.render_loopline(count_var, variables, nesting_depth)
            }
            Cmd::External(cmd) => cmd.render(self),
        }
    }

    fn render_write(&self, lines: &[String], output_comments: &[String]) -> String {
        let mut context = Context::new();

        context.insert("messages", lines);
        context.insert("output_comments", output_comments);

        self.tera_render("write", &mut context)
    }

    fn render_write_join(&self, terms: &[JoinTerm], output_comments: &[String]) -> String {
        let mut context = Context::new();

        let terms: Vec<JoinTerm> = terms
            .iter()
            .cloned()
            .map(|mut term| {
                if term.var_type.is_some() {
                    term.ident = self.lang.variable_name_options.transform_variable_name(&term.ident);
                }
                term
            })
            .collect();

        context.insert("terms", &terms);
        context.insert("type_tokens", &self.lang.type_tokens);
        context.insert("output_comments", output_comments);

        self.tera_render("write_join", &mut context)
    }

    fn render_read(&self, vars: &Vec<VariableCommand>, nesting_depth: usize) -> String {
        match vars.as_slice() {
            [var] => self.render_read_one(var),
            _ => self.render_read_many(vars, nesting_depth),
        }
    }

    fn render_read_one(&self, var: &VariableCommand) -> String {
        let mut context = Context::new();
        let var = self.lang.variable_name_options.transform_variable_command(var);

        context.insert("var", &var);
        context.insert("type_tokens", &self.lang.type_tokens);

        self.tera_render("read_one", &mut context)
    }

    fn render_read_many(&self, vars: &[VariableCommand], nesting_depth: usize) -> String {
        let mut context = Context::new();
        let vars = vars
            .iter()
            .map(|var| self.lang.variable_name_options.transform_variable_command(var))
            .collect::<Vec<_>>();

        let types: Vec<_> = vars.iter().map(|r| &r.var_type).unique().collect();
        match types.as_slice() {
            [single_type] => context.insert("single_type", single_type),
            _ => context.insert("single_type", &false),
        }

        let index_ident = ALPHABET[nesting_depth];

        context.insert("vars", &vars);
        context.insert("type_tokens", &self.lang.type_tokens);
        context.insert("index_ident", &index_ident);

        self.tera_render("read_many", &mut context)
    }

    fn render_loop(&self, count_var: &str, cmd: &Cmd, nesting_depth: usize) -> String {
        let mut context = Context::new();
        let inner_text = self.render_command(cmd, nesting_depth + 1);
        let cased_count_var = self.lang.variable_name_options.transform_variable_name(count_var);
        let index_ident = ALPHABET[nesting_depth];
        context.insert("count_var", &cased_count_var);
        context.insert("inner", &inner_text.lines().collect::<Vec<&str>>());
        context.insert("index_ident", &index_ident);

        self.tera_render("loop", &mut context)
    }

    fn render_loopline(&self, count_var: &str, vars: &[VariableCommand], nesting_depth: usize) -> String {
        let vars = vars
            .iter()
            .map(|var| self.lang.variable_name_options.transform_variable_command(var))
            .collect::<Vec<_>>();

        let mut context = Context::new();

        let cased_count_var = self.lang.variable_name_options.transform_variable_name(count_var);
        let index_ident = ALPHABET[nesting_depth];

        context.insert("count_var", &cased_count_var);
        context.insert("vars", &vars);
        context.insert("type_tokens", &self.lang.type_tokens);
        context.insert("index_ident", &index_ident);

        self.tera_render("loopline", &mut context)
    }
}
