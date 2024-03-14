#![allow(clippy::while_let_on_iterator)]
use regex::Regex;

pub mod types;
pub use types::{Cmd, InputComment, JoinTerm, JoinTermType, LengthType, Stub, VariableCommand};

pub fn parse_generator_stub(generator: String) -> Stub {
    let generator = generator.replace('\n', " \n ").replace("\n  \n", "\n \n");
    let stream = generator.split(' ');
    Parser::new(stream).parse()
}

struct Parser<StreamType: Iterator> {
    stream: StreamType,
}

impl<'a, I: Iterator<Item = &'a str>> Parser<I> {
    fn new(stream: I) -> Self {
        Self { stream }
    }

    #[rustfmt::skip]
    fn parse(&mut self) -> Stub {
        let mut stub = Stub::new();

        while let Some(token) = self.stream.next() {
            match token {
                "read"      => stub.commands.push(self.parse_read()),
                "write"     => stub.commands.push(self.parse_write()),
                "loop"      => stub.commands.push(self.parse_loop()),
                "loopline"  => stub.commands.push(self.parse_loopline()),
                "OUTPUT"    => self.parse_output_comment(&mut stub),
                "INPUT"     => stub.input_comments.append(&mut self.parse_input_comment()),
                "STATEMENT" => stub.statement = self.parse_statement(),
                "\n" | ""   => continue,
                thing => panic!("Error parsing stub generator: {}", thing),
            };
        }

        stub
    }

    fn parse_read(&mut self) -> Cmd {
        Cmd::Read(self.parse_variable_list())
    }

    fn parse_write(&mut self) -> Cmd {
        let mut output: Vec<String> = Vec::new();

        while let Some(token) = self.stream.next() {
            let next_token = match token {
                "\n" => match self.stream.next() {
                    Some("\n") | None => break,
                    Some(str) => format!("\n{}", str),
                },
                join if join.contains("join(") => return self.parse_write_join(join),
                other => String::from(other),
            };

            output.push(next_token);
        }

        Cmd::Write {
            text: output.join(" "),
            output_text: String::new(),
        }
    }

    fn parse_write_join(&mut self, start: &str) -> Cmd {
        let mut raw_string = String::from(start);

        while let Some(token) = self.stream.next() {
            match token {
                "\n" => panic!("'join(' never closed"),
                last_term if last_term.contains(')') => {
                    raw_string.push_str(last_term);
                    break;
                }
                regular_term => raw_string.push_str(regular_term),
            }
        }

        self.skip_to_next_line();

        let terms_finder = Regex::new(r"join\((.+)\)").unwrap();
        let terms_string = terms_finder.captures(&raw_string).unwrap().get(1).unwrap().as_str();
        let term_splitter = Regex::new(r"\s*,\s*").unwrap();
        let terms: Vec<JoinTerm> = term_splitter
            .split(terms_string)
            .map(|term_str| {
                let literal_matcher = Regex::new("^\\\"(.+)\\\"$").unwrap();
                if let Some(mtch) = literal_matcher.captures(term_str) {
                    JoinTerm::new(mtch.get(1).unwrap().as_str().to_owned(), JoinTermType::Literal)
                } else {
                    JoinTerm::new(term_str.to_owned(), JoinTermType::Variable)
                }
            })
            .collect();

        Cmd::WriteJoin(terms)
    }

    fn parse_loop(&mut self) -> Cmd {
        let count_var = match self.stream.next() {
            Some("\n") | None => panic!("Loop stub not provided with loop count"),
            Some(other) => String::from(other),
        };

        let command = Box::new(self.parse_loopable());

        Cmd::Loop { count_var, command }
    }

    fn parse_loopline(&mut self) -> Cmd {
        let count_var = match self.stream.next() {
            Some("\n") | None => panic!("Loopline stub not provided with count identifier"),
            Some(other) => String::from(other),
        };

        let variables = self.parse_variable_list();

        Cmd::LoopLine { count_var, variables }
    }

    fn parse_variable(token: &str) -> VariableCommand {
        let mut iter = token.split(':');
        let identifier = String::from(iter.next().unwrap());
        let var_type = iter.next().expect("Error in stub generator: missing type");
        let length_regex = Regex::new(r"(word|string)\((\w+)\)").unwrap();
        let length_captures = length_regex.captures(var_type);

        // Trim because the stub generator may contain sneaky newlines
        match var_type.trim_end() {
            "int" => VariableCommand::Int { name: identifier },
            "float" => VariableCommand::Float { name: identifier },
            "long" => VariableCommand::Long { name: identifier },
            "bool" => VariableCommand::Bool { name: identifier },
            _ => {
                let caps = length_captures
                    .unwrap_or_else(|| panic!("Failed to parse variable type for token: {}", &token));
                let new_type = caps.get(1).unwrap().as_str();
                let length = caps.get(2).unwrap().as_str();
                let max_length = String::from(length);
                let length_type = LengthType::from(length);
                match new_type {
                    "word" => VariableCommand::Word {
                        name: identifier,
                        max_length,
                        length_type,
                    },
                    "string" => VariableCommand::String {
                        name: identifier,
                        max_length,
                        length_type,
                    },
                    _ => panic!("Unexpected error"),
                }
            }
        }
    }

    fn parse_variable_list(&mut self) -> Vec<VariableCommand> {
        let mut vars = Vec::new();

        while let Some(token) = self.stream.next() {
            let var: VariableCommand = match token {
                _ if String::from(token).contains(':') => Self::parse_variable(token),
                "\n" => break,
                unexp => panic!("Error in stub generator, found {unexp} while searching for stub variables"),
            };

            vars.push(var);
        }

        vars
    }

    fn parse_loopable(&mut self) -> Cmd {
        match self.stream.next() {
            Some("read") => self.parse_read(),
            Some("write") => self.parse_write(),
            Some("loopline") => self.parse_loopline(),
            Some("loop") => self.parse_loop(),
            Some(thing) => panic!("Error parsing loop command in stub generator, got: {}", thing),
            None => panic!("Loop with no arguments in stub generator"),
        }
    }

    fn parse_output_comment(&mut self, stub: &mut Stub) {
        let output_text_parsed = self.parse_statement();
        self.recursively_backward_update_output(stub, output_text_parsed)
    }

    fn recursively_backward_update_output(&mut self, stub: &mut Stub, output_text_parsed: String) {
        let mut new_stub_cmds: Vec<Cmd> = Vec::new();

        for previous_cmd in &stub.commands {
            let new_cmd = match previous_cmd {
                Cmd::Write {
                    text,
                    ref output_text,
                } if output_text.is_empty() => Cmd::Write {
                    text: text.to_string(),
                    output_text: output_text_parsed.clone(),
                },
                Cmd::Loop { count_var, command } => {
                    // Recur
                    let mut inner_stub = Stub::new(); // temporary wrapper
                    inner_stub.commands.push(*command.clone());
                    self.recursively_backward_update_output(&mut inner_stub, output_text_parsed.clone());
                    Cmd::Loop {
                        count_var: count_var.clone(),
                        command: Box::new(inner_stub.commands.pop().unwrap()),
                    }
                }
                _ => previous_cmd.clone(),
            };

            new_stub_cmds.push(new_cmd);
        }

        stub.commands = new_stub_cmds;
    }

    fn parse_input_comment(&mut self) -> Vec<InputComment> {
        self.skip_to_next_line();
        let mut comments = Vec::new();

        while let Some(token) = self.stream.next() {
            let comment = match token {
                "\n" => break,
                _ => match token.strip_suffix(':') {
                    Some(variable) => InputComment::new(String::from(variable), self.read_to_end_of_line()),
                    None => {
                        self.skip_to_next_line();
                        continue
                    }
                },
            };

            comments.push(comment)
        }

        comments
    }

    fn parse_statement(&mut self) -> String {
        self.skip_to_next_line();
        self.parse_text_block()
    }

    fn read_to_end_of_line(&mut self) -> String {
        let mut upto_end_of_line = Vec::new();

        while let Some(token) = self.stream.next() {
            match token {
                "\n" => break,
                other => upto_end_of_line.push(other),
            }
        }

        upto_end_of_line.join(" ")
    }

    fn skip_to_next_line(&mut self) {
        while let Some(token) = self.stream.next() {
            if token == "\n" {
                break
            }
        }
    }

    fn parse_text_block(&mut self) -> String {
        let mut text_block: Vec<String> = Vec::new();

        while let Some(token) = self.stream.next() {
            let next_token = match token {
                "\n" => match self.stream.next() {
                    Some("\n") | None => break,
                    Some(str) => format!("\n{}", str),
                },
                other => String::from(other),
            };

            text_block.push(next_token);
        }

        text_block.join(" ")
    }
}
