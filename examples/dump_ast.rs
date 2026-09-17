use patto::parser::AstNode;

fn dump(node: &AstNode, depth: usize) {
    let pad = "  ".repeat(depth);
    let kind = format!("{:?}", node.kind());
    let kind = kind.split(" {").next().unwrap_or(&kind).to_string();
    println!(
        "{pad}{kind} row={} span={:?} text={:?}",
        node.location().row,
        node.location().span,
        node.extract_str()
    );
    let contents = node.contents();
    if !contents.is_empty() {
        println!("{pad}  .contents:");
        for c in contents.iter() {
            dump(c, depth + 2);
        }
    }
    let children = node.children();
    if !children.is_empty() {
        println!("{pad}  .children:");
        for c in children.iter() {
            dump(c, depth + 2);
        }
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_ast <file.pn>");
    let input = std::fs::read_to_string(path).unwrap();
    let result = patto::parser::parse_text(&input);
    dump(&result.ast, 0);
    for e in &result.parse_errors {
        println!("ERROR: {:?}", e);
    }
}
