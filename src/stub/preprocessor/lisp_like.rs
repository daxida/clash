use super::Renderable;
use crate::stub::{Cmd, Stub};

/// Edit Stub to allow for rendering lisp-like syntax.
///
/// Specifically, this preprocessor will:
/// - batch consecutive reads together
/// - embed remaining commands inside the scope of a read_batch
///
/// The purpose is to support languages such as lisp and clojure which
/// initialize multiple variables in one statement and also must have the scope
/// of said variables explicitly wrapped.
pub fn transform(stub: &mut Stub) {
    let mut old_commands = stub.commands.drain(..).rev().peekable();

    let mut cmds = Vec::new();
    let mut reads = Vec::new();

    while let Some(mut cmd) = old_commands.next() {
        if matches!(cmd, Cmd::Loop { .. }) {
            cmd = transform_loop(cmd);
        }

        let is_read = matches!(cmd, Cmd::Read(_));

        if is_read {
            reads.push(cmd)
        } else {
            cmds.push(cmd)
        }

        if !reads.is_empty() && (!is_read || old_commands.peek().is_none()) {
            let read_batch = ReadBatch {
                line_readers: reads.drain(..).rev().collect(),
                nested_cmds: cmds.drain(..).rev().collect(),
            };

            cmds.push(Cmd::External(Box::new(read_batch)));
        }
    }

    cmds.reverse();
    drop(old_commands);
    stub.commands = cmds;
}

fn transform_loop(cmd: Cmd) -> Cmd {
    match cmd {
        Cmd::Loop { command, count_var } => Cmd::Loop {
            count_var,
            command: Box::new(transform_loop(*command)),
        },
        Cmd::Read(_) => {
            let read_batch = ReadBatch {
                line_readers: vec![cmd],
                nested_cmds: Vec::new(),
            };

            Cmd::External(Box::new(read_batch))
        }
        _ => cmd,
    }
}

#[derive(Debug, Clone)]
struct ReadBatch {
    pub line_readers: Vec<Cmd>,
    pub nested_cmds: Vec<Cmd>,
}

impl Renderable for ReadBatch {
    fn render(&self, renderer: &crate::stub::renderer::Renderer) -> String {
        // Swap line_readers and nested_cmds order
        let nested_string: String =
            self.nested_cmds.iter().map(|cmd| renderer.render_command(cmd, 0)).collect();
        let nested_lines: Vec<&str> = nested_string.lines().filter(|ln| !ln.is_empty()).collect();

        let read_lines: String =
            self.line_readers.iter().map(|cmd| renderer.render_command(cmd, 0)).collect();
        let read_lines: Vec<&str> = read_lines.lines().filter(|ln| !ln.is_empty()).collect();

        // print!("{:?}\n\n", read_lines);
        print!("{:?}\n\n", nested_lines);
        let mut context = tera::Context::new();
        context.insert("read_lines", &read_lines);
        context.insert("nested_lines", &nested_lines);
        renderer.tera_render("read_batch", &mut context)
    }
}
