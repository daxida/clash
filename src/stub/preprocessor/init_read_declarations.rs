use super::Renderable;
use crate::stub::{Cmd, Stub, VarType, VariableCommand};

const ALPHABET: [char; 18] = [
    'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// Change the Stub structure into: [ReadDeclarations, MainContents(old_cmds)]
/// This is relevant for Pascal.
#[derive(Debug, Clone)]
struct MainWrapper {
    // Read declarations that should go on top of the main function.
    // render declaration: int c;
    // render read (usual): int c;\nscanf("%d", c);
    pub forward_declarations: Vec<VariableCommand>,
    // The main function contents.
    pub main_content: Vec<Cmd>,
}

pub fn transform(stub: &mut Stub) {
    let mut max_nested_depth = 0;

    let mut forward_declarations: Vec<VariableCommand> = stub.commands.iter().filter_map(|cmd| {
        let (cmd, nested_depth) = unpack_cmd(cmd, 0);

        if nested_depth > max_nested_depth {
            max_nested_depth = nested_depth;
        }

        if let Cmd::Read(vars) = cmd {
            Some(vars.into_iter())
        } else {
            None
        }
    }).flatten().collect();

    let mut loop_vars: Vec<VariableCommand> = ALPHABET[0..max_nested_depth].iter().filter(|loop_var|
        forward_declarations.iter().all(|var_cmd| var_cmd.ident != loop_var.to_string())
    ).map(|loop_var|
        VariableCommand { 
            ident: loop_var.to_string(), 
            var_type: VarType::Int, 
            max_length: None, 
            input_comment: String::new() 
        }
    ).collect();

    forward_declarations.append(&mut loop_vars);

    let wrapper = MainWrapper {
        forward_declarations,
        main_content: stub.commands.drain(..).collect(),
    };

    stub.commands = vec![Cmd::External(Box::new(wrapper))];
}

fn unpack_cmd(cmd: &Cmd, nested_depth: usize) -> (Cmd, usize) {
    if let Cmd::Loop { count_var: _, command: subcmd } = cmd {
        unpack_cmd(subcmd, nested_depth + 1)
    } else {
        (cmd.clone(), nested_depth + 1)
    }
}

impl Renderable for MainWrapper {
    fn render(&self, renderer: &crate::stub::renderer::Renderer) -> String {
        let main_contents_str: String =
            self.main_content.iter().map(|cmd| renderer.render_command(cmd, 0)).collect();
        let main_contents: Vec<&str> = main_contents_str.lines().collect();

        let forward_declarations: Vec<String> =
            self.forward_declarations.iter().map(|vc| vc.render(renderer)).collect();

        let mut context = tera::Context::new();
        context.insert("forward_declarations", &forward_declarations);
        context.insert("main_contents", &main_contents);
        renderer.tera_render("main_wrapper", &mut context)
    }
}

impl Renderable for VariableCommand {
    fn render(&self, renderer: &crate::stub::renderer::Renderer) -> String {
        let mut context = tera::Context::from_serialize(self).expect("VariableCommand should be serializable");
        renderer.tera_render("forward_declaration", &mut context)
    }
}